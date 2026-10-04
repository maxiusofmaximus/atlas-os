// Atlas OS — Planning availability (RFC 20 Fase 23 v3.1.2.0, research/52).
//
// Pure interval arithmetic over the calendar busy windows: given `now`, an
// estimated turn duration (`eta`) and a weight threshold, decide whether a
// proactive turn can start. This is the `next_free_slot` that RFC 28 §G.4
// item 8 assumed and the repo lacked.
//
// No LLM, no clock read inside the pure core: `next_free_slot` is deterministic
// (unit-tested + a golden EVAL task), and `availability_now` is the thin
// Journal-backed wrapper that reads the M18 busy windows.

use crate::calendar::queue::BusyWindowRow;
use crate::journal::Journal;

/// Policy for deciding whether a proactive turn can start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurnPolicy {
    /// Estimated duration of the turn, in milliseconds.
    pub eta_ms: i64,
    /// Busy windows with `weight >= threshold` block the turn
    /// (`manual`/timed ICS default to 1.0; graph to 0.5; all-day to 0.0).
    pub weight_threshold: f64,
    /// Do not look further than this for a free slot, in milliseconds.
    pub horizon_ms: i64,
}

impl Default for TurnPolicy {
    fn default() -> Self {
        Self {
            eta_ms: 15 * 60 * 1000,
            // Conservative: only *hard* busy blocks; soft-busy (0.5) does not.
            weight_threshold: 1.0,
            horizon_ms: 24 * 60 * 60 * 1000,
        }
    }
}

/// Decision for the proactive turn engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Availability {
    /// A free slot of `eta` starts now.
    RunNow,
    /// The next free slot opens at this epoch (ms).
    WaitUntil(i64),
    /// No free slot within the policy horizon.
    Blocked,
}

/// Earliest `now`-aligned slot of `policy.eta_ms` free of blocking windows.
/// Half-open intervals, matching `BusyWindowQueue::overlapping`.
pub fn next_free_slot(busy: &[BusyWindowRow], now_ms: i64, policy: &TurnPolicy) -> Availability {
    let mut blockers: Vec<(i64, i64)> = busy
        .iter()
        .filter(|w| w.weight >= policy.weight_threshold && w.ends_at > now_ms)
        .map(|w| (w.starts_at, w.ends_at))
        .collect();
    blockers.sort_by_key(|(start, _)| *start);

    let deadline = now_ms.saturating_add(policy.horizon_ms);
    let mut cursor = now_ms;

    // Push the cursor past any blocker overlapping [cursor, cursor + eta).
    // Bounded by the number of blockers (each pass either moves forward or stops).
    loop {
        let mut moved = false;
        for &(start, end) in &blockers {
            if start < cursor + policy.eta_ms && cursor < end {
                cursor = end;
                moved = true;
                if cursor > deadline {
                    return Availability::Blocked;
                }
            }
        }
        if !moved {
            break;
        }
    }

    if cursor > deadline {
        return Availability::Blocked;
    }
    if cursor <= now_ms {
        Availability::RunNow
    } else {
        Availability::WaitUntil(cursor)
    }
}

/// Journal-backed availability: reads the M18 busy windows overlapping the
/// policy horizon and applies `next_free_slot`.
pub fn availability_now(
    journal: &Journal,
    now_ms: i64,
    policy: &TurnPolicy,
) -> anyhow::Result<Availability> {
    let busy =
        journal.busy_windows_overlapping(now_ms, now_ms.saturating_add(policy.horizon_ms))?;
    Ok(next_free_slot(&busy, now_ms, policy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::payload::BusySource;

    fn win(start: i64, end: i64, weight: f64) -> BusyWindowRow {
        BusyWindowRow {
            id: 0,
            source: BusySource::Manual,
            external_id: "e".into(),
            subject: "busy".into(),
            body: None,
            starts_at: start,
            ends_at: end,
            weight,
            recorded_at: 0,
        }
    }

    #[test]
    fn no_busy_runs_now() {
        assert_eq!(
            next_free_slot(&[], 1_000_000, &TurnPolicy::default()),
            Availability::RunNow
        );
    }

    #[test]
    fn window_covering_now_waits_until_it_ends() {
        let busy = [win(999_000, 1_500_000, 1.0)];
        assert_eq!(
            next_free_slot(&busy, 1_000_000, &TurnPolicy::default()),
            Availability::WaitUntil(1_500_000)
        );
    }

    #[test]
    fn future_window_overlapping_eta_waits() {
        // eta = 15 min; a window starting in 5 min within the eta overlaps.
        let busy = [win(1_300_000, 1_400_000, 1.0)];
        assert_eq!(
            next_free_slot(&busy, 1_000_000, &TurnPolicy::default()),
            Availability::WaitUntil(1_400_000)
        );
    }

    #[test]
    fn future_window_after_eta_runs_now() {
        // eta = 900_000; window starts after now + eta → half-open → free now.
        let busy = [win(2_000_000, 2_100_000, 1.0)];
        assert_eq!(
            next_free_slot(&busy, 1_000_000, &TurnPolicy::default()),
            Availability::RunNow
        );
    }

    #[test]
    fn soft_busy_respects_threshold() {
        let busy = [win(999_000, 1_500_000, 0.5)];
        let conservative = TurnPolicy::default(); // threshold 1.0
        assert_eq!(
            next_free_slot(&busy, 1_000_000, &conservative),
            Availability::RunNow
        );
        let strict = TurnPolicy {
            weight_threshold: 0.5,
            ..TurnPolicy::default()
        };
        assert_eq!(
            next_free_slot(&busy, 1_000_000, &strict),
            Availability::WaitUntil(1_500_000)
        );
    }

    #[test]
    fn chained_windows_wait_for_the_last_end() {
        let busy = [win(999_000, 1_200_000, 1.0), win(1_150_000, 1_600_000, 1.0)];
        assert_eq!(
            next_free_slot(&busy, 1_000_000, &TurnPolicy::default()),
            Availability::WaitUntil(1_600_000)
        );
    }

    #[test]
    fn beyond_horizon_is_blocked() {
        let policy = TurnPolicy {
            horizon_ms: 60_000,
            ..TurnPolicy::default()
        };
        let busy = [win(999_000, 1_500_000, 1.0)];
        assert_eq!(
            next_free_slot(&busy, 1_000_000, &policy),
            Availability::Blocked
        );
    }
}
