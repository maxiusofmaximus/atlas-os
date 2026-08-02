// OpenCode OS — Toast payload types (RFC 28 Section F).
//
// `ToastKind` enum + `ToastStatus` enum + `ToastPayload` DTO + the
// `opencode://` deep-link parser. The enum is the semantic
// classification surfaced to the HUD (Mission Control cards) and to
// the future §G calendar reminder path and the §H model-ready path.
// keeps SQL `TEXT` representation stable across migrations (no
// integer codes — that'd break a future rename).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::toast::error::{Result as ToastResult, ToastFacadeError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToastKind {
    ModelReady,
    TurnEnd,
    ValidationFailed,
    CalendarReminder,
    Critical,
    Info,
}

impl fmt::Display for ToastKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ToastKind::ModelReady => "model_ready",
            ToastKind::TurnEnd => "turn_end",
            ToastKind::ValidationFailed => "validation_failed",
            ToastKind::CalendarReminder => "calendar_reminder",
            ToastKind::Critical => "critical",
            ToastKind::Info => "info",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for ToastKind {
    type Err = ToastFacadeError;
    fn from_str(s: &str) -> ToastResult<Self> {
        Ok(match s {
            "model_ready" => ToastKind::ModelReady,
            "turn_end" => ToastKind::TurnEnd,
            "validation_failed" => ToastKind::ValidationFailed,
            "calendar_reminder" => ToastKind::CalendarReminder,
            "critical" => ToastKind::Critical,
            "info" => ToastKind::Info,
            other => {
                return Err(ToastFacadeError::InvalidKind(other.to_string()));
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToastStatus {
    Pending,
    Fired,
    Dismissed,
    Failed,
}

impl fmt::Display for ToastStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ToastStatus::Pending => "pending",
            ToastStatus::Fired => "fired",
            ToastStatus::Dismissed => "dismissed",
            ToastStatus::Failed => "failed",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for ToastStatus {
    type Err = ToastFacadeError;
    fn from_str(s: &str) -> ToastResult<Self> {
        Ok(match s {
            "pending" => ToastStatus::Pending,
            "fired" => ToastStatus::Fired,
            "dismissed" => ToastStatus::Dismissed,
            "failed" => ToastStatus::Failed,
            other => {
                return Err(ToastFacadeError::InvalidKind(format!(
                    "toast_status:{other}"
                )));
            }
        })
    }
}

/// In-memory rendering of a queued toast. Produced by
/// `ToastQueue::next_pending()` and consumed by the dispatcher; never
/// serialised directly to the database (the DB stores the raw columns).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToastPayload {
    pub id: i64,
    pub kind: ToastKind,
    pub title: String,
    pub body: Option<String>,
    pub deep_link: Option<String>,
    pub fire_at: i64,
}

/// Build a canonical `opencode://` deep-link.
pub fn build_deep_link(mission_id: &str, card_id: Option<&str>) -> String {
    match card_id {
        Some(c) => format!("opencode://mission/{mission_id}/card/{c}"),
        None => format!("opencode://mission/{mission_id}"),
    }
}

/// Parse an `opencode://` deep-link back into `(mission_id, card_id)`.
pub fn parse_deep_link(link: &str) -> ToastResult<(String, Option<String>)> {
    let rest = link
        .strip_prefix("opencode://")
        .ok_or_else(|| ToastFacadeError::InvalidDeepLink(link.to_string()))?;
    let rest = rest
        .strip_prefix("mission/")
        .ok_or_else(|| ToastFacadeError::InvalidDeepLink(link.to_string()))?;
    let (mission_id, tail) = match rest.split_once('/') {
        Some((m, t)) => (m.to_string(), t),
        None => (rest.to_string(), ""),
    };
    if mission_id.is_empty() {
        return Err(ToastFacadeError::InvalidDeepLink(link.to_string()));
    }
    let card_id = if tail.is_empty() {
        None
    } else {
        let card = tail
            .strip_prefix("card/")
            .ok_or_else(|| ToastFacadeError::InvalidDeepLink(link.to_string()))?;
        if card.is_empty() {
            return Err(ToastFacadeError::InvalidDeepLink(link.to_string()));
        }
        Some(card.to_string())
    };
    Ok((mission_id, card_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn kind_round_trips_via_serde() {
        let js = serde_json::to_string(&ToastKind::ModelReady).unwrap();
        assert_eq!(js, "\"model_ready\"");
        let back: ToastKind = serde_json::from_str(&js).unwrap();
        assert_eq!(back, ToastKind::ModelReady);
    }

    #[test]
    fn kind_display_matches_from_str() {
        for k in [
            ToastKind::ModelReady,
            ToastKind::TurnEnd,
            ToastKind::ValidationFailed,
            ToastKind::CalendarReminder,
            ToastKind::Critical,
            ToastKind::Info,
        ] {
            let s = k.to_string();
            let back: ToastKind = s.parse().unwrap();
            assert_eq!(k, back, "round-trip failed for {k:?}");
        }
    }

    #[test]
    fn unknown_kind_is_invalid_kind_error() {
        let res = ToastKind::from_str("unknown");
        assert!(matches!(res, Err(ToastFacadeError::InvalidKind(_))));
    }

    #[test]
    fn status_round_trips() {
        for s in [
            ToastStatus::Pending,
            ToastStatus::Fired,
            ToastStatus::Dismissed,
            ToastStatus::Failed,
        ] {
            let str = s.to_string();
            let back: ToastStatus = str.parse().unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn deep_link_mission_only() {
        let link = build_deep_link("abc-123", None);
        assert_eq!(link, "opencode://mission/abc-123");
        let (m, c) = parse_deep_link(&link).unwrap();
        assert_eq!(m, "abc-123");
        assert_eq!(c, None);
    }

    #[test]
    fn deep_link_with_card() {
        let link = build_deep_link("m1", Some("c2"));
        assert_eq!(link, "opencode://mission/m1/card/c2");
        let (m, c) = parse_deep_link(&link).unwrap();
        assert_eq!(m, "m1");
        assert_eq!(c.as_deref(), Some("c2"));
    }

    #[test]
    fn deep_link_rejects_other_schemes() {
        let res = parse_deep_link("https://example.com");
        assert!(matches!(res, Err(ToastFacadeError::InvalidDeepLink(_))));
    }

    #[test]
    fn deep_link_rejects_no_mission_segment() {
        let res = parse_deep_link("opencode://not-mission/abc");
        assert!(matches!(res, Err(ToastFacadeError::InvalidDeepLink(_))));
    }
}
