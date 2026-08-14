// Atlas OS — Doom-loop detector (RFC 19 §6).
//
// The detector is a rolling-window counter keyed on
// `{tool_name}|{normalized_input_hash}` (RFC 19 §6 `detector.key`).
// When the SAME key appears `min_repeats` times inside a
// `window_seconds` window, the detector fires — the runner translates
// the firing into an `ExecutionMode`-dependent action (RFC 19 §6
// `on_match`).
//
// Pure data structure: the detector does NOT own a clock. The host
// feeds it `(key, ts_secs)` observations via `observe`; the detector
// purges entries older than `window_seconds` on every call. This
// keeps the detector testable without sleeps.

use std::collections::VecDeque;

/// RFC 19 §6 `anti_infinite_loop.detector` configuration. Defaults
/// match the YAML block in the RFC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoomLoopConfig {
    pub window_seconds: u32,
    pub min_repeats: u32,
}

impl DoomLoopConfig {
    pub const DEFAULT: Self = Self {
        window_seconds: 60,
        min_repeats: 3,
    };
}

impl Default for DoomLoopConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One observation: the deduplication key plus a monotonically
/// increasing timestamp (seconds since session start). The host can
/// supply `now()` cheaply; the detector doesn't care about absolute
/// time, only deltas relative to the rolling window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sample {
    key: u64,
    ts_secs: u64,
}

/// Rolling-window doom-loop detector. One instance per agent (or per
/// mission in single-agent mode).
#[derive(Clone, Debug)]
pub struct DoomLoopDetector {
    cfg: DoomLoopConfig,
    samples: VecDeque<Sample>,
    /// Number of times the detector has FIRED for this session. The
    /// runner uses this to enforce `max_recovery_attempts = 2`
    /// (RFC 19 §6 `on_doom_loop.max_recovery_attempts`).
    pub fire_count: u32,
}

impl DoomLoopDetector {
    pub fn new(cfg: DoomLoopConfig) -> Self {
        Self {
            cfg,
            samples: VecDeque::new(),
            fire_count: 0,
        }
    }

    /// RFC 19 §6 `detector.key` — build the canonical key string from
    /// the tool name and a stable hash of the normalised input. The
    /// runner passes the resulting string to `observe_key` so tests can
    /// assert on the literal key without needing the hasher to be
    /// stable across builds.
    pub fn build_key(tool: &str, normalized_input: &str) -> String {
        format!("{}|{}", tool, normalized_input)
    }

    /// Record an observation. The `ts_secs` argument should be the
    /// current session wallclock (whatever clock the host feeds); the
    /// detector uses it ONLY to age out samples outside the window.
    ///
    /// Returns `Some(key)` when the rolling window fires — the same
    /// key has been observed `min_repeats` times within
    /// `window_seconds` AND the detector hasn't already fired for
    /// that.WINDOW. Subsequent duplicate observations inside the same
    /// window do NOT re-fire (the runner needs one trigger to act on;
    /// re-arming only happens once the window expires).
    pub fn observe(&mut self, key: &str, ts_secs: u64) -> Option<String> {
        // Age out samples outside the window.
        let cutoff = ts_secs.saturating_sub(self.cfg.window_seconds as u64);
        while let Some(front) = self.samples.front() {
            if front.ts_secs < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }
        let h = fxhash(key);
        self.samples.push_back(Sample { key: h, ts_secs });
        // Count matching samples inside the window.
        let matches: u32 = self
            .samples
            .iter()
            .filter(|s| s.key == h)
            .count()
            .try_into()
            .unwrap_or(u32::MAX);
        if matches >= self.cfg.min_repeats {
            self.fire_count = self.fire_count.saturating_add(1);
            // Drain all matching samples so the detector re-arms only
            // for fresh activity after the window expires.
            self.samples.retain(|s| s.key != h);
            return Some(key.to_string());
        }
        None
    }

    /// True when the runner should hard-deny further tool calls
    /// (RFC 19 §6 `max_recovery_attempts` reached).
    pub fn exhausts_recovery(self, max_recovery_attempts: u32) -> bool {
        self.fire_count >= max_recovery_attempts
    }

    /// Reset the detector (used when the runner explicitly re-arms
    /// after a user steer).
    pub fn reset(&mut self) {
        self.samples.clear();
        self.fire_count = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Constant-foldable string hash. The detector only ever compares
/// hashes for equality inside a session, so we don't need a
/// cryptographic primitive; we need stability inside one process.
fn fxhash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_repeats_do_not_fire() {
        let mut d = DoomLoopDetector::new(DoomLoopConfig::DEFAULT);
        let key = DoomLoopDetector::build_key("bash", "ls");
        assert!(d.observe(&key, 0).is_none());
        assert!(d.observe(&key, 1).is_none());
        assert_eq!(d.fire_count, 0);
    }

    #[test]
    fn three_repeats_inside_window_fire_once() {
        let mut d = DoomLoopDetector::new(DoomLoopConfig::DEFAULT);
        let key = DoomLoopDetector::build_key("bash", "ls -la");
        assert!(d.observe(&key, 0).is_none());
        assert!(d.observe(&key, 1).is_none());
        let fired = d.observe(&key, 2).expect("third repeat fires");
        assert_eq!(fired, key);
        // A fourth observation inside the same window is drained by
        // the fire — the detector only fires once per window now.
        // (Adding the same key again after draining starts a fresh
        // rolling window with one sample.)
        let s3 = d.observe(&key, 3);
        assert!(s3.is_none(), "post-fire drain: no immediate re-fire");
        assert_eq!(d.fire_count, 1);
    }

    #[test]
    fn samples_age_out_of_window_do_not_fire() {
        let mut d = DoomLoopDetector::new(DoomLoopConfig::DEFAULT);
        let key = DoomLoopDetector::build_key("read", "file.txt");
        d.observe(&key, 0);
        d.observe(&key, 1);
        // 100s later — outside the 60s window.
        assert!(d.observe(&key, 100).is_none(), "aged-out samples excluded");
        assert_eq!(d.fire_count, 0);
    }

    #[test]
    fn different_tools_do_not_cross_count() {
        let mut d = DoomLoopDetector::new(DoomLoopConfig::DEFAULT);
        let k1 = DoomLoopDetector::build_key("bash", "ls");
        let k2 = DoomLoopDetector::build_key("read", "ls");
        // 2 identical bash|ls + 1 read|ls + 1 bash|ls = 3 total for
        // bash|ls (k1), but spread out. min_repeats=3 ⇒ fires on
        // the third k1 observation.
        d.observe(&k1, 0);
        d.observe(&k2, 1);
        assert!(d.observe(&k1, 2).is_none(), "still only 2 layoffs for k1");
        let fire = d.observe(&k1, 3);
        assert!(fire.is_some(), "third k1 observation fires");
        assert_eq!(fire.unwrap(), k1);
    }

    #[test]
    fn exhausts_recovery_after_two_fires() {
        let mut d = DoomLoopDetector::new(DoomLoopConfig {
            window_seconds: 1000,
            min_repeats: 2,
        });
        let key = "k".to_string();
        d.observe(&key, 0);
        // t=1: matches=2 → fire #1, drain.
        assert!(d.observe(&key, 1).is_some());
        // t=2: fresh window starts.
        d.observe(&key, 2);
        // t=3: matches=2 → fire #2, drain.
        assert!(d.observe(&key, 3).is_some());
        assert!(d.exhausts_recovery(2));
    }

    #[test]
    fn reset_clears_state() {
        let mut d = DoomLoopDetector::new(DoomLoopConfig {
            window_seconds: 1000,
            min_repeats: 2,
        });
        d.observe("k", 0);
        assert!(d.observe("k", 1).is_some());
        d.reset();
        assert!(d.is_empty());
        assert_eq!(d.fire_count, 0);
        // After reset the detector re-arms from zero. Need 2 samples.
        assert!(d.observe("k", 3).is_none(), "only 1 sample -> no fire");
        assert!(d.observe("k", 4).is_some(), "2 samples -> fire");
    }
}
