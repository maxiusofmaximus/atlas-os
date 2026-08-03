// OpenCode OS — Retry policy with jitter and bail-out (RFC 28 §H.5).
//
// When the orchestrator receives a 429 rate-limit (without a
// `SpendLimitError` payload) or a transient 5xx, a `RetryPolicy`
// decides whether to retry, how long to wait, and when to give up.
//
// The policy combines three concerns:
//
// 1. **Exponential backoff** — `base * 2^attempt` capped at
//    `max_delay`. Default base 1 s, max 30 s.
// 2. **Jitter ±25 %** — sampled per attempt to avoid thundering-herd
//    on shared provider rate windows. `delay = base * (1 + U[-0.25,
//    +0.25])` (Cline #10963).
// 3. **Bail-out threshold** — when the computed delay exceeds the
//    operator-controlled threshold (default 60 s, RFC 28 §H.5),
//    the policy returns `BailDecision::GiveUp` instead of waiting; the
//    orchestrator pauses the mission and surfaces a HUD card (the
//    user-not-the-loop decides whether to wait it out or switch
//    provider).
//
// Bail-out explicitly does NOT apply to `SpendLimitError` — that's
// the contract of §H.1/Cline #10207 ("SpendLimitError exempted from
// auto-retry"): the spend-limit card always pauses the loop, never
// waits. The orchestrator lifts `SpendLimitError` out before
// consulting `RetryPolicy::decide`.
//
// Pattern adapted from Cline (Apache-2.0) PRs:
// * #10963 — jitter ±25 % + Retry-After / x-ratelimit-reset parse
//   (the parse half lives in `parse_error.rs`).
// * #10141 — bail-out threshold (default 60 s).
// https://github.com/cline/cline

use chrono::Duration;
use rand::Rng;

/// Bail-out outcome for `RetryPolicy::decide`. `Wait` carries the
/// computed delay (after jitter); `GiveUp` carries the reason used by
/// the HUD's "paused mission" card (e.g. "computed delay > 60 s
/// threshold").
#[derive(Clone, Debug, PartialEq)]
pub enum BailDecision {
    /// Wait `delay` before the next attempt.
    Wait { delay: Duration, attempt: u32 },
    /// Stop retrying; surface a paused-mission card to the user.
    GiveUp { reason: String, attempt: u32 },
}

/// Tunable retry policy. All times are explicit so tests are
/// deterministic; production callers build defaults via
/// `RetryPolicy::default()`.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    /// Base delay (1 s by default). The delay at attempt `n` (n=0
    /// for the first retry) is `backoff(n) * (1 + jitter)` where
    /// `backoff(n) = base * 2^n` capped at `max_delay`.
    pub base: Duration,
    /// Cap on the pre-jitter delay (30 s by default).
    pub max_delay: Duration,
    /// Bail-out threshold. When the post-jitter delay exceeds this,
    /// `RetryPolicy::decide` returns `GiveUp`. Default 60 s per RFC.
    pub bail_out_threshold: Duration,
    /// Maximum attempts before declaring failure (default 5). After
    /// `max_attempts` the policy gives up regardless of delay size.
    pub max_attempts: u32,
    /// Jitter amplitude (default 0.25 = ±25 %). The sampled multiplier
    /// is `1.0 + U(-jitter, +jitter)` on the unjittered delay.
    pub jitter: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base: Duration::seconds(1),
            max_delay: Duration::seconds(30),
            bail_out_threshold: Duration::seconds(60),
            max_attempts: 5,
            jitter: 0.25,
        }
    }
}

impl RetryPolicy {
    /// Compute the un-jittered backoff delay for attempt `n`
    /// (n=0 → first retry). `base * 2^n` capped at `max_delay`.
    pub fn backoff(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.base;
        }
        // Use i64 multiplication; protect against overflow that an
        // adversarial `attempt` could trigger (32 << the cap is hit at
        // attempt ~5 with base=1s, so we are well within bounds, but
        // we still saturate at `max_delay`).
        let mut multiplier: i64 = 1;
        for _ in 0..attempt {
            multiplier = multiplier.saturating_mul(2);
            let candidate = self.base * multiplier as i32;
            if candidate >= self.max_delay {
                return self.max_delay;
            }
        }
        self.base * multiplier as i32
    }

    /// Sample a jittered multipler in `[1 - jitter, 1 + jitter]`.
    /// `Rng` is passed by `&mut` so tests can use a fixed seed.
    pub fn jitter_multiplier<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        if self.jitter <= 0.0 {
            return 1.0;
        }
        let lo = 1.0 - self.jitter;
        let hi = 1.0 + self.jitter;
        rng.gen_range(lo..=hi)
    }

    /// Decide whether to wait or give up for `attempt` (n=0 ⇒ the
    /// first retry after the original failure). The caller threads
    /// the returned `attempt` field into the next call so the policy
    /// monotonically advances through its backoff curve.
    pub fn decide<R: Rng + ?Sized>(&self, attempt: u32, rng: &mut R) -> BailDecision {
        if attempt >= self.max_attempts {
            return BailDecision::GiveUp {
                reason: format!("max_attempts ({}) reached", self.max_attempts),
                attempt,
            };
        }
        let raw = self.backoff(attempt);
        let factor = self.jitter_multiplier(rng);
        // Saturating cast to i64 nanos; the multipliers are within 4x so
        // no overflow concern.
        let jittered_nanos = (raw.num_nanoseconds().unwrap_or(0) as f64 * factor) as i64;
        let delay = Duration::nanoseconds(jittered_nanos);
        if delay > self.bail_out_threshold {
            return BailDecision::GiveUp {
                reason: format!(
                    "computed delay {}s exceeds bail-out threshold {}s",
                    delay.num_seconds(),
                    self.bail_out_threshold.num_seconds()
                ),
                attempt,
            };
        }
        BailDecision::Wait { delay, attempt }
    }

    /// Same as `decide` but uses `thread_rng()` — convenience for
    /// production callers that don't need deterministic output.
    pub fn decide_thread_rng(&self, attempt: u32) -> BailDecision {
        let mut rng = rand::thread_rng();
        self.decide(attempt, &mut rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn fixed_rng(seed: u64) -> StdRng {
        StdRng::seed_from_u64(seed)
    }

    #[test]
    fn backoff_grows_exponentially_until_cap() {
        let p = RetryPolicy::default();
        assert_eq!(p.backoff(0), Duration::seconds(1));
        assert_eq!(p.backoff(1), Duration::seconds(2));
        assert_eq!(p.backoff(2), Duration::seconds(4));
        assert_eq!(p.backoff(3), Duration::seconds(8));
        assert_eq!(p.backoff(4), Duration::seconds(16));
        assert_eq!(p.backoff(5), Duration::seconds(30), "capped at max_delay");
        assert_eq!(p.backoff(99), Duration::seconds(30));
    }

    #[test]
    fn jitter_multiplier_in_range_for_default_policy() {
        let p = RetryPolicy::default();
        let mut rng = fixed_rng(42);
        for _ in 0..100 {
            let m = p.jitter_multiplier(&mut rng);
            assert!(
                (0.75..=1.25).contains(&m),
                "jittered multiplier out of ±25% range: {m}"
            );
        }
    }

    #[test]
    fn jitter_multiplier_is_one_when_jitter_is_zero() {
        let p = RetryPolicy {
            jitter: 0.0,
            ..RetryPolicy::default()
        };
        let mut rng = fixed_rng(1);
        for _ in 0..10 {
            assert_eq!(p.jitter_multiplier(&mut rng), 1.0);
        }
    }

    #[test]
    fn decide_returns_wait_with_bounded_delay() {
        let p = RetryPolicy::default();
        let mut rng = fixed_rng(7);
        for attempt in 0..5 {
            let d = p.decide(attempt, &mut rng);
            match d {
                BailDecision::Wait { delay, .. } => {
                    let secs = delay.num_seconds();
                    assert!(
                        (0..=60).contains(&secs),
                        "attempt {attempt}: delay {secs}s outside [0, 60]s"
                    );
                }
                BailDecision::GiveUp { .. } => {
                    // Not expected for attempts < max_attempts on the
                    // default curve (max jittered delay at attempt 4 is
                    // 16 * 1.25 = 20s, well under the 60s threshold).
                    panic!("attempt {attempt}: unexpected GiveUp");
                }
            }
        }
    }

    #[test]
    fn decide_gives_up_when_max_attempts_reached() {
        let p = RetryPolicy::default();
        let mut rng = fixed_rng(1);
        let d = p.decide(p.max_attempts, &mut rng);
        assert!(matches!(d, BailDecision::GiveUp { .. }));
        if let BailDecision::GiveUp { reason, .. } = d {
            assert!(reason.contains("max_attempts"), "reason was: {reason}");
        }
    }

    #[test]
    fn decide_gives_up_when_delay_exceeds_bail_out_threshold() {
        // base 10 s, max 100 s, threshold 15 s — attempt 1 is 20s
        // unjittered; with jitter up to +25% it's 25s, well over 15s.
        let p = RetryPolicy {
            base: Duration::seconds(10),
            max_delay: Duration::seconds(100),
            bail_out_threshold: Duration::seconds(15),
            max_attempts: 10,
            jitter: 0.0, // deterministic: 20s > 15s threshold.
        };
        let mut rng = fixed_rng(1);
        let d = p.decide(1, &mut rng);
        assert!(
            matches!(d, BailDecision::GiveUp { .. }),
            "expected GiveUp, got: {d:?}"
        );
        if let BailDecision::GiveUp { reason, .. } = d {
            assert!(
                reason.contains("bail-out") || reason.contains("threshold"),
                "reason was: {reason}"
            );
        }
    }

    #[test]
    fn decide_with_low_threshold_bails_out_immediately_at_zero() {
        // base 100s (above threshold of 60s) → attempt 0 gives up.
        let p = RetryPolicy {
            base: Duration::seconds(100),
            max_delay: Duration::seconds(100),
            bail_out_threshold: Duration::seconds(60),
            max_attempts: 10,
            jitter: 0.0,
        };
        let mut rng = fixed_rng(1);
        let d = p.decide(0, &mut rng);
        assert!(matches!(d, BailDecision::GiveUp { .. }));
    }

    #[test]
    fn decide_at_boundary_threshold_returns_wait() {
        // base = threshold exactly; jitter 0 so the decision is Wait
        // (delay == threshold, the policy is `>` not `>=`).
        let p = RetryPolicy {
            base: Duration::seconds(60),
            max_delay: Duration::seconds(60),
            bail_out_threshold: Duration::seconds(60),
            max_attempts: 10,
            jitter: 0.0,
        };
        let mut rng = fixed_rng(1);
        let d = p.decide(0, &mut rng);
        assert!(matches!(d, BailDecision::Wait { .. }));
    }
}
