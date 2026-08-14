// Atlas OS — Calendar payloads & mappings (RFC 28 Section G).
//
// Types shared by both halves of §G (WRITE `calendar-ics` and READ
// `calendar-graph`):
//
// * `BusySource` — provenance of a `calendar_busy_windows` row. Mirrors
//   the CHECK constraint of the M18 schema (`graph | ics_local |
//   manual`). Serde form is `snake_case` so the JSON exposed to the HUD
//   / CLI matches the SQL column verbatim.
//
// * `BusyWindow` — in-memory projection of one calendar_busy_windows
//   row. Consumed by the Planning engine (RFC 12 §3): `overlaps(start,
//   end)` decides if a proactive turn can be enqueued now. `weight`
//   defaults to 1.0 (Graph poller inflates OPENCODE-tagged events to
//   1.0; untagged events stay 0.5 — soft busy).
//
// * `IcsMission` — minimal mission projection the ICS writer needs to
//   emit a `VEVENT`. Carried from `Journal::missions_recent(30 days)`
//   to avoid the ICS writer knowing about the full `Mission` envelope.
//
// * `IcsMissionStatus` — the subset of `MissionStatus` the ICS writer
//   maps to iCal `STATUS` values per RFC 5545 §3.1.20. Mapping table:
//   | mission status   | iCal STATUS  |
//   | ---------------- | ------------ |
//   | received         | TENTATIVE    |  (in queue, not yet planned)
//   | planning         | TENTATIVE    |
//   | in_progress      | CONFIRMED    |
//   | validation       | CONFIRMED    |
//   | completed        | CONFIRMED    |
//   | failed           | CANCELLED    |
//
//   We mirror `crate::journal::MissionStatus` rather than depending on
//   it directly so the calendar module stays usable from CLI contexts
//   that don't drag the full `plugins` tree.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Provenance of a busy-window row. Serde renames to match the SQL
/// CHECK constraint verbatim — `snake_case`, lowercase.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusySource {
    /// MS Graph `/me/calendarView` poller.
    Graph,
    /// Locally subscribed `webcal://` or `.ics` file mirror.
    IcsLocal,
    /// Hand-added by the operator (`opencode calendar busy add`).
    Manual,
}

impl BusySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Graph => "graph",
            Self::IcsLocal => "ics_local",
            Self::Manual => "manual",
        }
    }
}

impl std::fmt::Display for BusySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSourceError(pub String);

impl std::fmt::Display for UnknownSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown busy source: {}", self.0)
    }
}

impl std::error::Error for UnknownSourceError {}

impl FromStr for BusySource {
    type Err = UnknownSourceError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "graph" => Ok(Self::Graph),
            "ics_local" => Ok(Self::IcsLocal),
            "manual" => Ok(Self::Manual),
            other => Err(UnknownSourceError(other.into())),
        }
    }
}

/// In-memory busy window. `weight` 0.0..=1.0 — used by the Planning
/// engine's `next_free_slot` to skip events with `weight >=
/// FREE_SLOT_THRESHOLD` (default 0.5 — soft-busy events don't block).
#[derive(Clone, Debug, PartialEq)]
pub struct BusyWindow {
    pub id: i64,
    pub source: BusySource,
    pub external_id: String,
    pub subject: String,
    pub body: Option<String>,
    pub starts_at: i64,
    pub ends_at: i64,
    pub weight: f64,
}

impl BusyWindow {
    /// True if `[start, end)` overlaps `[self.starts_at, self.ends_at)`.
    /// Half-open intervals: a window ending at `t` doesn't block a turn
    /// that starts at `t`.
    pub fn overlaps(&self, start: i64, end: i64) -> bool {
        self.starts_at < end && start < self.ends_at
    }
}

/// Subset of `crate::journal::MissionStatus` used by the ICS writer.
/// Kept in this module (not imported from `journal`) so the calendar
/// crate keeps a stable surface even if `journal::MissionStatus` grows
/// more states later (e.g. a `paused` state from RFC 27 would default
/// to `Tentative` until §G is updated).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IcsMissionStatus {
    Received,
    Planning,
    InProgress,
    Validation,
    Completed,
    Failed,
}

impl IcsMissionStatus {
    /// Convert to the iCal `STATUS` token per RFC 5545 §3.1.20.
    /// `received` and `planning` map to `TENTATIVE` (the calendar UI
    /// shows them as faint bars); `in_progress` / `validation` /
    /// `completed` map to `CONFIRMED`; `failed` is `CANCELLED`.
    pub fn to_ical_status(self) -> &'static str {
        match self {
            Self::Received => "TENTATIVE",
            Self::Planning => "TENTATIVE",
            Self::InProgress => "CONFIRMED",
            Self::Validation => "CONFIRMED",
            Self::Completed => "CONFIRMED",
            Self::Failed => "CANCELLED",
        }
    }

    /// Convert to the `CATEGORIES` token stored in the VEVENT. This
    /// keeps the raw mission status visible in calendar UIs that filter
    /// by category, bypassing the lossy iCal STATUS mapping above.
    pub fn to_category(self) -> &'static str {
        match self {
            Self::Received => "received",
            Self::Planning => "planning",
            Self::InProgress => "in_progress",
            Self::Validation => "validation",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl std::str::FromStr for IcsMissionStatus {
    type Err = UnknownStatusError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "received" => Ok(Self::Received),
            "planning" => Ok(Self::Planning),
            "in_progress" => Ok(Self::InProgress),
            "validation" => Ok(Self::Validation),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            other => Err(UnknownStatusError(other.into())),
        }
    }
}

impl std::fmt::Display for IcsMissionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_category())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownStatusError(pub String);

impl std::fmt::Display for UnknownStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown mission status: {}", self.0)
    }
}

impl std::error::Error for UnknownStatusError {}

/// Minimal mission projection consumed by the ICS writer. Carried from
/// the Journal via a SQL SELECT so the writer doesn't bind to the
/// full `Mission` envelope nor depend on `journal` directly.
#[derive(Clone, Debug, PartialEq)]
pub struct IcsMission {
    pub id: String,
    pub label: String,
    pub status: IcsMissionStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl IcsMission {
    /// iCal `UID` per RFC 5545 §3.8.4.7 — stable across retries (the
    /// user unsubscribes and resubscribes the feed, the events keep
    /// their identities).
    pub fn ical_uid(&self) -> String {
        format!("mission-{}@opencode.dev", self.id)
    }

    /// True when `created_at` and `updated_at` are >= 24 h apart —
    /// the ICS writer emits an all-day `VEVENT` rather than a timed
    /// one (RFC 5545 §3.6.1: `DTSTART;VALUE=DATE:YYYYMMDD`).
    pub fn is_all_day(&self) -> bool {
        (self.updated_at - self.created_at).num_hours().abs() >= 24
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn busy_source_roundtrip_string() {
        for s in [BusySource::Graph, BusySource::IcsLocal, BusySource::Manual] {
            let st = s.to_string();
            let back: BusySource = st.parse().unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn busy_source_rejects_unknown_token() {
        let e = "garbage".parse::<BusySource>().unwrap_err();
        assert!(format!("{e}").contains("garbage"));
    }

    #[test]
    fn busy_source_serde_uses_snake_case() {
        let s = serde_json::to_string(&BusySource::IcsLocal).unwrap();
        assert_eq!(s, "\"ics_local\"");
    }

    #[test]
    fn overlaps_half_open_intervals() {
        let w = BusyWindow {
            id: 1,
            source: BusySource::Graph,
            external_id: "x".into(),
            subject: "Sprint planning".into(),
            body: None,
            starts_at: 1_000,
            ends_at: 2_000,
            weight: 1.0,
        };
        assert!(w.overlaps(1_500, 1_800), "fully inside");
        assert!(w.overlaps(500, 1_500), "left overlap");
        assert!(w.overlaps(1_500, 3_000), "right overlap");
        assert!(!w.overlaps(2_000, 3_000), "ends_at is exclusive");
        assert!(
            !w.overlaps(500, 1_000),
            "starts_at is inclusive (boundary not overlap)"
        );
    }

    #[test]
    fn ics_status_mapping_follows_rfc_5545() {
        use IcsMissionStatus::*;
        assert_eq!(Received.to_ical_status(), "TENTATIVE");
        assert_eq!(Planning.to_ical_status(), "TENTATIVE");
        assert_eq!(InProgress.to_ical_status(), "CONFIRMED");
        assert_eq!(Validation.to_ical_status(), "CONFIRMED");
        assert_eq!(Completed.to_ical_status(), "CONFIRMED");
        assert_eq!(Failed.to_ical_status(), "CANCELLED");
    }

    #[test]
    fn ics_status_to_category_round_trips() {
        use IcsMissionStatus::*;
        for (s, expected) in [
            (Received, "received"),
            (Planning, "planning"),
            (InProgress, "in_progress"),
            (Validation, "validation"),
            (Completed, "completed"),
            (Failed, "failed"),
        ] {
            let cat = s.to_category();
            assert_eq!(cat, expected);
            let back: IcsMissionStatus = cat.parse().unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn ics_mission_uid_is_stable() {
        let m = IcsMission {
            id: "abc-123".into(),
            label: "do thing".into(),
            status: IcsMissionStatus::InProgress,
            created_at: ts("2026-08-02T12:00:00Z"),
            updated_at: ts("2026-08-02T12:00:00Z"),
        };
        assert_eq!(m.ical_uid(), "mission-abc-123@opencode.dev");
    }

    #[test]
    fn ics_mission_is_all_day_when_delta_ge_24h() {
        let base = ts("2026-08-02T12:00:00Z");
        let timed = IcsMission {
            id: "x".into(),
            label: "x".into(),
            status: IcsMissionStatus::InProgress,
            created_at: base,
            updated_at: base + chrono::Duration::hours(3),
        };
        assert!(!timed.is_all_day());
        let all_day = IcsMission {
            id: "y".into(),
            label: "y".into(),
            status: IcsMissionStatus::InProgress,
            created_at: base,
            updated_at: base + chrono::Duration::hours(48),
        };
        assert!(all_day.is_all_day());
    }
}
