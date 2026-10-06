// Atlas OS — multi-channel gateway core (RFC 29 §3.B, A3.0).
//
// The channel-agnostic half of "steer from anywhere": it turns the Kernel Bus
// events worth interrupting a human for into an outbound message with the
// actions that resolve them, and parses an inbound `/command` back into a
// `ChannelAction`. Pure and network-free, so it is fully testable; the
// per-provider adapters (Telegram via `teloxide`, …) are thin layers on top and
// arrive in a later increment behind the `multi-channel` feature (default off).

use crate::core::bus::{BusEvent, BusEventKind};

/// An action an operator can take from a channel, either by tapping a button or
/// typing the matching `/command` (RFC 29 §3.B).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelAction {
    /// `/pause` — stop the running work (RFC 21).
    Pause,
    /// `/resume` — continue.
    Resume,
    /// `/steer <text>` — inject guidance (RFC 25 §3.9).
    Steer(String),
    /// `/approve <id>` — grant an approval (RFC 18).
    Approve(String),
    /// `/deny <id>` — refuse an approval.
    Deny(String),
    /// `/status` — ask for a snapshot.
    Status,
}

/// One quick-action button offered beside a message: a short label plus the
/// action it triggers. Adapters render these as inline buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelButton {
    pub label: String,
    pub action: ChannelAction,
}

/// An outbound message: the text plus any quick-action buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelMessage {
    pub text: String,
    pub buttons: Vec<ChannelButton>,
}

impl ChannelMessage {
    fn with(text: impl Into<String>, buttons: Vec<(&str, ChannelAction)>) -> Self {
        Self {
            text: text.into(),
            buttons: buttons
                .into_iter()
                .map(|(label, action)| ChannelButton {
                    label: label.to_string(),
                    action,
                })
                .collect(),
        }
    }
}

/// RFC 29 §3.B — map a Kernel Bus event to an operator-facing message, or
/// `None` for the many events not worth interrupting a human for (those stay in
/// the HUD). Only attention-worthy events are forwarded.
pub fn format_event(event: &BusEvent) -> Option<ChannelMessage> {
    match &event.kind {
        BusEventKind::ApprovalRequest {
            approval_id,
            agent_id,
            action,
        } => Some(ChannelMessage::with(
            format!("Approval needed for `{action}` (agent {agent_id})."),
            vec![
                ("Approve", ChannelAction::Approve(approval_id.to_string())),
                ("Deny", ChannelAction::Deny(approval_id.to_string())),
            ],
        )),
        BusEventKind::CostThresholdCrossed {
            agent_id,
            threshold,
            cumulative,
        } => Some(ChannelMessage::with(
            format!(
                "Cost ${cumulative:.2} crossed the ${threshold:.2} threshold (agent {agent_id})."
            ),
            vec![("Pause", ChannelAction::Pause)],
        )),
        BusEventKind::DoomLoopDetected { agent_id, count } => Some(ChannelMessage::with(
            format!("Doom loop: {count} repeats (agent {agent_id})."),
            vec![("Pause", ChannelAction::Pause)],
        )),
        BusEventKind::GoalDriftDetected { agent_id, drift } => Some(ChannelMessage::with(
            format!("Goal drift {drift:.2} (agent {agent_id})."),
            vec![("Pause", ChannelAction::Pause)],
        )),
        _ => None,
    }
}

/// Parse an inbound channel text into an action. Recognizes the RFC 29 §3.B
/// command set (`/pause`, `/resume`, `/steer <text>`, `/approve <id>`,
/// `/deny <id>`, `/status`); unknown or argument-less text is `None`, never a
/// silent no-op action.
pub fn parse_command(text: &str) -> Option<ChannelAction> {
    let text = text.trim();
    let (cmd, rest) = match text.split_once(char::is_whitespace) {
        Some((c, r)) => (c, r.trim()),
        None => (text, ""),
    };
    let cmd = cmd.trim_start_matches('/').to_ascii_lowercase();
    match cmd.as_str() {
        "pause" => Some(ChannelAction::Pause),
        "resume" | "continue" => Some(ChannelAction::Resume),
        "status" => Some(ChannelAction::Status),
        "steer" if !rest.is_empty() => Some(ChannelAction::Steer(rest.to_string())),
        "approve" | "apr" if !rest.is_empty() => Some(ChannelAction::Approve(rest.to_string())),
        "deny" if !rest.is_empty() => Some(ChannelAction::Deny(rest.to_string())),
        _ => None,
    }
}

/// The seam a per-provider adapter implements (Telegram via `teloxide`, …):
/// deliver one outbound message. Everything above this line is adapter-free and
/// unit-tested; the adapter only has to transport.
pub trait Channel {
    fn send(&self, message: &ChannelMessage) -> Result<(), String>;
}

/// Routes Kernel Bus events to a [`Channel`] and inbound text to actions. The
/// adapter implements [`Channel`]; this routing logic is adapter-free and
/// testable with a recording channel (no network).
pub struct Dispatcher<C: Channel> {
    channel: C,
}

impl<C: Channel> Dispatcher<C> {
    pub fn new(channel: C) -> Self {
        Self { channel }
    }

    /// Forward an event to the channel if it is attention-worthy. Returns
    /// whether anything was sent (so a caller can log/skip cheaply).
    pub fn on_event(&self, event: &BusEvent) -> Result<bool, String> {
        match format_event(event) {
            Some(msg) => {
                self.channel.send(&msg)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Parse inbound text into an action for the caller to apply. `None` for
    /// unknown text (never a silent no-op action).
    pub fn on_inbound(&self, text: &str) -> Option<ChannelAction> {
        parse_command(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bus::BusEvent;
    use std::cell::RefCell;

    fn event(kind: BusEventKind) -> BusEvent {
        BusEvent::new(kind)
    }

    #[test]
    fn approval_request_offers_approve_and_deny() {
        let id = uuid::Uuid::new_v4();
        let msg = format_event(&event(BusEventKind::ApprovalRequest {
            approval_id: id,
            agent_id: uuid::Uuid::new_v4(),
            action: "fs.write".into(),
        }))
        .expect("approval is attention-worthy");
        assert!(msg.text.contains("fs.write"));
        assert_eq!(msg.buttons.len(), 2);
        assert_eq!(
            msg.buttons[0].action,
            ChannelAction::Approve(id.to_string())
        );
        assert_eq!(msg.buttons[1].action, ChannelAction::Deny(id.to_string()));
    }

    #[test]
    fn cost_and_loop_events_offer_pause() {
        for kind in [
            BusEventKind::CostThresholdCrossed {
                agent_id: uuid::Uuid::new_v4(),
                threshold: 5.0,
                cumulative: 5.5,
            },
            BusEventKind::DoomLoopDetected {
                agent_id: uuid::Uuid::new_v4(),
                count: 3,
            },
            BusEventKind::GoalDriftDetected {
                agent_id: uuid::Uuid::new_v4(),
                drift: 0.4,
            },
        ] {
            let msg = format_event(&event(kind)).expect("attention-worthy");
            assert_eq!(msg.buttons[0].action, ChannelAction::Pause);
        }
    }

    #[test]
    fn ordinary_events_are_not_forwarded() {
        assert!(format_event(&event(BusEventKind::AgentHeartbeat {
            agent_id: uuid::Uuid::new_v4(),
        }))
        .is_none());
        assert!(format_event(&event(BusEventKind::HudServed { hud_port: 8080 })).is_none());
    }

    #[test]
    fn parses_the_command_set() {
        assert_eq!(parse_command("/pause"), Some(ChannelAction::Pause));
        assert_eq!(parse_command("pause"), Some(ChannelAction::Pause));
        assert_eq!(parse_command("/resume"), Some(ChannelAction::Resume));
        assert_eq!(parse_command("/status"), Some(ChannelAction::Status));
        assert_eq!(
            parse_command("/steer do the thing"),
            Some(ChannelAction::Steer("do the thing".into()))
        );
        assert_eq!(
            parse_command("/approve abc-123"),
            Some(ChannelAction::Approve("abc-123".into()))
        );
        assert_eq!(
            parse_command("/deny abc-123"),
            Some(ChannelAction::Deny("abc-123".into()))
        );
    }

    #[test]
    fn rejects_unknown_and_argumentless_commands() {
        assert_eq!(parse_command("hello world"), None);
        assert_eq!(parse_command("/steer"), None, "steer needs text");
        assert_eq!(parse_command("/approve"), None, "approve needs an id");
        assert_eq!(parse_command("/deny"), None, "deny needs an id");
        assert_eq!(parse_command(""), None);
    }

    #[derive(Default)]
    struct RecordingChannel {
        sent: RefCell<Vec<ChannelMessage>>,
    }
    impl Channel for RecordingChannel {
        fn send(&self, message: &ChannelMessage) -> Result<(), String> {
            self.sent.borrow_mut().push(message.clone());
            Ok(())
        }
    }

    #[test]
    fn dispatcher_forwards_only_attention_events() {
        let ch = RecordingChannel::default();
        let d = Dispatcher::new(ch);
        let sent = d
            .on_event(&event(BusEventKind::ApprovalRequest {
                approval_id: uuid::Uuid::new_v4(),
                agent_id: uuid::Uuid::new_v4(),
                action: "exec.run".into(),
            }))
            .unwrap();
        assert!(sent);
        let skipped = d
            .on_event(&event(BusEventKind::AgentHeartbeat {
                agent_id: uuid::Uuid::new_v4(),
            }))
            .unwrap();
        assert!(!skipped);
        assert_eq!(d.channel.sent.borrow().len(), 1);
    }

    #[test]
    fn dispatcher_parses_inbound_commands() {
        let d = Dispatcher::new(RecordingChannel::default());
        assert_eq!(d.on_inbound("/pause"), Some(ChannelAction::Pause));
        assert_eq!(d.on_inbound("random"), None);
    }
}
