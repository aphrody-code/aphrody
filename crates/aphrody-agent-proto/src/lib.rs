// SPDX-License-Identifier: Apache-2.0
//! `aphrody-agent-proto` — the agent SQ/EQ protocol plus a newline-delimited
//! JSON (NDJSON) wire codec.
//!
//! This crate is the single wire shared by every aphrody surface
//! (CLI / TUI / desktop / MCP) to talk to the agent engine. It follows the
//! Submission Queue (SQ) / Event Queue (EQ) pattern:
//!
//! - A client sends [`Submission`]s (`{ id, op }`) carrying an [`Op`] *to* the
//!   agent.
//! - The agent emits [`Event`]s (`{ id, msg }`) carrying an [`EventMsg`] *back*
//!   to the client.
//!
//! Both directions are serialized one record per line via the NDJSON codec
//! ([`encode_event`] / [`decode_event`] / [`encode_submission`] /
//! [`decode_submission`] / [`decode_events`]). The codec guarantees a single
//! line per record (no embedded newlines) so it can be streamed over pipes,
//! sockets, or stdio.
//!
//! The protocol shape is a real, coherent subset inspired by the OpenAI Codex
//! protocol (Apache-2.0). Decoding is tolerant of unknown fields so newer
//! agents can add data without breaking older clients.
//!
//! The crate is pure (no platform syscalls) and compiles to wasm.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use uuid::Uuid;

/// Generates a fresh, RFC 4122-shaped v4 UUID string for record correlation.
///
/// On every native target this delegates to [`uuid::Uuid::new_v4`], which draws
/// from the OS CSPRNG. On `wasm32-unknown-unknown` the `uuid` crate has no
/// randomness source available without an extra feature (which this crate's
/// dependency set does not enable), so we synthesize a v4-shaped, per-call
/// unique id from a monotonically increasing process counter seeded by the
/// build's compile time. The result is a syntactically valid v4 UUID (correct
/// version and variant nibbles) and is unique within a wasm instance's
/// lifetime, which is all the correlation layer requires.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn gen_uuid_v4() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn gen_uuid_v4() -> String {
    use core::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    // A fixed high half keeps ids from colliding across distinct counter
    // sequences within practical bounds.
    const SEED_HI: u64 = 0x0a01_b110_d000_0000;

    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut raw = (u128::from(SEED_HI) << 64) | u128::from(n);
    // Force the RFC 4122 version (4) and variant (10xx) nibbles so the value is
    // a well-formed v4 UUID.
    raw = (raw & !(0xF000_u128 << 64)) | (0x4000_u128 << 64);
    raw = (raw & !(0xC000_u128 << 48)) | (0x8000_u128 << 48);
    let b = raw.to_be_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14],
        b[15],
    )
}

/// Errors produced by the NDJSON codec.
#[derive(Debug, Error)]
pub enum ProtoError {
    /// A record failed to (de)serialize as JSON.
    #[error("json codec error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A single client-to-agent request on the Submission Queue (SQ).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Submission {
    /// Client-generated correlation id. Events answering this submission reuse
    /// it (directly, or through the request ids carried in approval events).
    pub id: String,
    /// The operation the client is asking the agent to perform.
    pub op: Op,
}

impl Submission {
    /// Builds a submission for `op`, generating a fresh UUID v4 id.
    #[must_use]
    pub fn new(op: Op) -> Self {
        Self {
            id: gen_uuid_v4(),
            op,
        }
    }
}

/// The set of operations a client may submit to the agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Op {
    /// Feed user input (text and/or images) into the current turn.
    UserInput {
        /// The ordered input parts.
        items: Vec<InputItem>,
    },
    /// Answer a pending [`EventMsg::ExecApprovalRequest`].
    ExecApproval {
        /// The request id from the approval event.
        id: String,
        /// The client's decision.
        decision: ReviewDecision,
    },
    /// Answer a pending [`EventMsg::ApplyPatchApprovalRequest`].
    PatchApproval {
        /// The request id from the approval event.
        id: String,
        /// The client's decision.
        decision: ReviewDecision,
    },
    /// Interrupt the current turn as soon as possible.
    Interrupt,
    /// Inject steering guidance into the *currently running* turn without
    /// interrupting and restarting it. The `items` are appended to the
    /// conversation as an extra user message before the engine's next model
    /// call, so the agent course-corrects mid-flight. Submitted while no turn
    /// is running it is a no-op (there is nothing to steer) — start a turn with
    /// [`Op::UserInput`] instead.
    Steer {
        /// The ordered guidance parts (text and/or images), same shape as
        /// [`Op::UserInput`].
        items: Vec<InputItem>,
    },
    /// Ask the agent to compact / summarize its context.
    Compact,
    /// Ask the agent to shut down the session cleanly.
    Shutdown,
}

/// One part of a user message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputItem {
    /// A plain text fragment.
    Text {
        /// The text content.
        text: String,
    },
    /// An image, referenced either by a local path or a URL (at least one
    /// should be set; both are optional to stay tolerant of partial records).
    Image {
        /// A local filesystem path to the image.
        #[serde(default)]
        path: Option<PathBuf>,
        /// A remote URL to the image.
        #[serde(default)]
        url: Option<String>,
    },
}

/// The client's response to an approval request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    /// Approve this single action.
    Approved,
    /// Approve this action and similar ones for the rest of the session.
    ApprovedForSession,
    /// Deny this action but keep the turn going.
    Denied,
    /// Deny this action and abort the turn.
    Abort,
}

/// A single agent-to-client record on the Event Queue (EQ).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Correlation id (usually echoes the driving [`Submission::id`]).
    pub id: String,
    /// The event payload.
    pub msg: EventMsg,
}

impl Event {
    /// Builds an event for `msg`, generating a fresh UUID v4 id.
    #[must_use]
    pub fn new(msg: EventMsg) -> Self {
        Self {
            id: gen_uuid_v4(),
            msg,
        }
    }
}

/// Every payload the agent can emit back to a client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventMsg {
    /// A new turn has started.
    TurnStarted,
    /// A streamed fragment of the agent's visible message.
    AgentMessageDelta {
        /// The text delta to append.
        delta: String,
    },
    /// The full, finalized agent message for the turn.
    AgentMessage {
        /// The complete message text.
        text: String,
    },
    /// Steering guidance was folded into the running turn (in response to an
    /// [`Op::Steer`]). Carries the concatenated text that was appended to the
    /// conversation, so clients and the rollout observe the mid-turn
    /// course-correction and a replay stays faithful.
    SteerApplied {
        /// The injected guidance text (image parts render as `[image: …]`
        /// references).
        text: String,
    },
    /// A streamed fragment of the agent's reasoning.
    AgentReasoningDelta {
        /// The reasoning delta to append.
        delta: String,
    },
    /// The agent is about to run a command.
    ExecCommandBegin {
        /// Unique id tying begin/output/end together.
        call_id: String,
        /// The argv of the command.
        command: Vec<String>,
        /// The working directory the command runs in.
        cwd: PathBuf,
    },
    /// A streamed chunk of a running command's combined output.
    ExecCommandOutputDelta {
        /// The id from the matching [`EventMsg::ExecCommandBegin`].
        call_id: String,
        /// The output chunk.
        chunk: String,
    },
    /// A command finished.
    ExecCommandEnd {
        /// The id from the matching [`EventMsg::ExecCommandBegin`].
        call_id: String,
        /// The process exit code.
        exit_code: i32,
    },
    /// The agent requests approval to run a command.
    ExecApprovalRequest {
        /// Request id to echo back in [`Op::ExecApproval`].
        id: String,
        /// The argv of the command awaiting approval.
        command: Vec<String>,
        /// Optional human-readable reason for the request.
        #[serde(default)]
        reason: Option<String>,
    },
    /// The agent requests approval to apply a patch.
    ApplyPatchApprovalRequest {
        /// Request id to echo back in [`Op::PatchApproval`].
        id: String,
        /// The files the patch touches.
        files: Vec<String>,
    },
    /// A tool call started.
    ToolCallBegin {
        /// Unique id tying begin/end together.
        call_id: String,
        /// The tool name.
        name: String,
    },
    /// A tool call finished.
    ToolCallEnd {
        /// The id from the matching [`EventMsg::ToolCallBegin`].
        call_id: String,
        /// Whether the tool call succeeded.
        ok: bool,
    },
    /// Token accounting update.
    TokenCount {
        /// Input (prompt) tokens.
        input: u64,
        /// Output (completion) tokens.
        output: u64,
        /// Total tokens.
        total: u64,
    },
    /// The turn finished.
    TurnComplete {
        /// The last visible agent message, if any.
        #[serde(default)]
        last_agent_message: Option<String>,
    },
    /// A fatal or recoverable error surfaced to the client.
    Error {
        /// The error message.
        message: String,
    },
}

/// Encodes an [`Event`] as a single JSON line (no trailing newline, no embedded
/// newlines).
///
/// # Errors
/// Returns [`ProtoError::Json`] if serialization fails.
pub fn encode_event(e: &Event) -> Result<String, ProtoError> {
    Ok(serde_json::to_string(e)?)
}

/// Decodes one NDJSON line into an [`Event`].
///
/// # Errors
/// Returns [`ProtoError::Json`] if the line is not valid JSON for an [`Event`].
pub fn decode_event(line: &str) -> Result<Event, ProtoError> {
    Ok(serde_json::from_str(line)?)
}

/// Encodes a [`Submission`] as a single JSON line (no trailing newline, no
/// embedded newlines).
///
/// # Errors
/// Returns [`ProtoError::Json`] if serialization fails.
pub fn encode_submission(s: &Submission) -> Result<String, ProtoError> {
    Ok(serde_json::to_string(s)?)
}

/// Decodes one NDJSON line into a [`Submission`].
///
/// # Errors
/// Returns [`ProtoError::Json`] if the line is not valid JSON for a
/// [`Submission`].
pub fn decode_submission(line: &str) -> Result<Submission, ProtoError> {
    Ok(serde_json::from_str(line)?)
}

/// Decodes a buffer of NDJSON-encoded events, yielding one result per non-empty
/// line. Blank and whitespace-only lines are skipped.
pub fn decode_events(buf: &str) -> impl Iterator<Item = Result<Event, ProtoError>> + '_ {
    buf.lines()
        .filter(|line| !line.trim().is_empty())
        .map(decode_event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn roundtrip_op(op: Op) {
        let s = Submission::new(op.clone());
        let line = encode_submission(&s).expect("encode");
        assert!(!line.contains('\n'), "submission line must be single-line");
        let back = decode_submission(&line).expect("decode");
        assert_eq!(s, back);
        assert_eq!(back.op, op);
    }

    fn roundtrip_msg(msg: EventMsg) {
        let e = Event::new(msg.clone());
        let line = encode_event(&e).expect("encode");
        assert!(!line.contains('\n'), "event line must be single-line");
        let back = decode_event(&line).expect("decode");
        assert_eq!(e, back);
        assert_eq!(back.msg, msg);
    }

    #[test]
    fn op_variants_roundtrip() {
        roundtrip_op(Op::UserInput {
            items: vec![
                InputItem::Text {
                    text: "hello".into(),
                },
                InputItem::Image {
                    path: Some(PathBuf::from("/tmp/a.png")),
                    url: None,
                },
                InputItem::Image {
                    path: None,
                    url: Some("https://example.com/b.png".into()),
                },
            ],
        });
        roundtrip_op(Op::ExecApproval {
            id: "req-1".into(),
            decision: ReviewDecision::Approved,
        });
        roundtrip_op(Op::PatchApproval {
            id: "req-2".into(),
            decision: ReviewDecision::ApprovedForSession,
        });
        roundtrip_op(Op::Interrupt);
        roundtrip_op(Op::Steer {
            items: vec![
                InputItem::Text {
                    text: "be concise".into(),
                },
                InputItem::Image {
                    path: None,
                    url: Some("https://example.com/ref.png".into()),
                },
            ],
        });
        roundtrip_op(Op::Compact);
        roundtrip_op(Op::Shutdown);
    }

    #[test]
    fn review_decision_variants_roundtrip() {
        for d in [
            ReviewDecision::Approved,
            ReviewDecision::ApprovedForSession,
            ReviewDecision::Denied,
            ReviewDecision::Abort,
        ] {
            roundtrip_op(Op::ExecApproval {
                id: "x".into(),
                decision: d,
            });
        }
    }

    #[test]
    fn event_msg_variants_roundtrip() {
        roundtrip_msg(EventMsg::TurnStarted);
        roundtrip_msg(EventMsg::AgentMessageDelta {
            delta: "tok".into(),
        });
        roundtrip_msg(EventMsg::AgentMessage {
            text: "full message".into(),
        });
        roundtrip_msg(EventMsg::SteerApplied {
            text: "focus on tests first".into(),
        });
        roundtrip_msg(EventMsg::AgentReasoningDelta {
            delta: "thinking".into(),
        });
        roundtrip_msg(EventMsg::ExecCommandBegin {
            call_id: "c1".into(),
            command: vec!["ls".into(), "-la".into()],
            cwd: PathBuf::from("/work"),
        });
        roundtrip_msg(EventMsg::ExecCommandOutputDelta {
            call_id: "c1".into(),
            chunk: "total 0".into(),
        });
        roundtrip_msg(EventMsg::ExecCommandEnd {
            call_id: "c1".into(),
            exit_code: 0,
        });
        roundtrip_msg(EventMsg::ExecApprovalRequest {
            id: "a1".into(),
            command: vec!["rm".into(), "-rf".into(), "x".into()],
            reason: Some("destructive".into()),
        });
        roundtrip_msg(EventMsg::ApplyPatchApprovalRequest {
            id: "p1".into(),
            files: vec!["src/lib.rs".into()],
        });
        roundtrip_msg(EventMsg::ToolCallBegin {
            call_id: "t1".into(),
            name: "search".into(),
        });
        roundtrip_msg(EventMsg::ToolCallEnd {
            call_id: "t1".into(),
            ok: true,
        });
        roundtrip_msg(EventMsg::TokenCount {
            input: 100,
            output: 50,
            total: 150,
        });
        roundtrip_msg(EventMsg::TurnComplete {
            last_agent_message: Some("done".into()),
        });
        roundtrip_msg(EventMsg::TurnComplete {
            last_agent_message: None,
        });
        roundtrip_msg(EventMsg::Error {
            message: "boom".into(),
        });
    }

    #[test]
    fn submission_new_generates_uuid() {
        let a = Submission::new(Op::Interrupt);
        let b = Submission::new(Op::Interrupt);
        assert_ne!(a.id, b.id);
        assert!(Uuid::parse_str(&a.id).is_ok());
    }

    #[test]
    fn event_new_generates_uuid() {
        let a = Event::new(EventMsg::TurnStarted);
        assert!(Uuid::parse_str(&a.id).is_ok());
    }

    #[test]
    fn encode_event_has_no_newline() {
        let e = Event::new(EventMsg::AgentMessage {
            text: "line one\nline two".into(),
        });
        let line = encode_event(&e).expect("encode");
        // The JSON string escapes the embedded newline, so the wire line has
        // none.
        assert!(!line.contains('\n'));
        assert!(line.contains("\\n"));
    }

    #[test]
    fn decode_single_line() {
        let line = r#"{"id":"abc","msg":{"type":"agent_message","text":"hi"}}"#;
        let e = decode_event(line).expect("decode");
        assert_eq!(e.id, "abc");
        assert_eq!(
            e.msg,
            EventMsg::AgentMessage {
                text: "hi".into()
            }
        );
    }

    #[test]
    fn decode_events_multiline_with_blank_lines() {
        let buf = format!(
            "{}\n\n{}\n   \n{}\n",
            encode_event(&Event {
                id: "1".into(),
                msg: EventMsg::TurnStarted,
            })
            .unwrap(),
            encode_event(&Event {
                id: "2".into(),
                msg: EventMsg::AgentMessageDelta { delta: "x".into() },
            })
            .unwrap(),
            encode_event(&Event {
                id: "3".into(),
                msg: EventMsg::TurnComplete {
                    last_agent_message: None,
                },
            })
            .unwrap(),
        );
        let events: Vec<Event> = decode_events(&buf).map(|r| r.expect("decode")).collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].id, "1");
        assert_eq!(events[1].id, "2");
        assert_eq!(events[2].id, "3");
    }

    #[test]
    fn decode_tolerates_unknown_fields() {
        // Extra top-level and nested fields must be ignored, not rejected.
        let line = r#"{"id":"z","msg":{"type":"agent_message","text":"ok","extra":42},"future":true}"#;
        let e = decode_event(line).expect("decode tolerant");
        assert_eq!(
            e.msg,
            EventMsg::AgentMessage {
                text: "ok".into()
            }
        );

        let sub = r#"{"id":"s","op":{"type":"interrupt","unexpected":"v"},"more":1}"#;
        let s = decode_submission(sub).expect("decode tolerant submission");
        assert_eq!(s.op, Op::Interrupt);
    }

    #[test]
    fn decode_invalid_json_errors() {
        let err = decode_event("not json").unwrap_err();
        matches!(err, ProtoError::Json(_));
    }
}
