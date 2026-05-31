// SPDX-License-Identifier: Apache-2.0
//! Pure, terminal-free application state for the agent TUI.
//!
//! Everything in this module is deterministic and side-effect free: it maps
//! incoming protocol [`EventMsg`]s onto a scrollable transcript and turns key
//! presses into protocol [`Submission`]s. This is the part that carries the
//! tests, because it never touches a real terminal.

use aphrody_agent_proto::EventMsg;
use aphrody_agent_proto::InputItem;
use aphrody_agent_proto::Op;
use aphrody_agent_proto::ReviewDecision;
use aphrody_agent_proto::Submission;

/// A single rendered line-group in the conversation transcript.
///
/// Cells are appended in arrival order and rendered top-to-bottom. Streaming
/// cells (exec output, tool calls) are mutated in place by [`AppState::apply_event`]
/// as more deltas arrive, keyed by their `call_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptCell {
    /// A message the user submitted.
    UserMessage(String),
    /// A finalized visible message from the agent.
    AgentMessage(String),
    /// A block of the agent's streamed reasoning.
    Reasoning(String),
    /// A tool invocation. `done` is `None` while running, then `Some(ok)`.
    ToolCall {
        /// Display name of the tool.
        name: String,
        /// Correlation id tying begin/end together.
        call_id: String,
        /// `None` while running, `Some(true/false)` once finished.
        done: Option<bool>,
    },
    /// A shell command execution with streamed combined output.
    Exec {
        /// Correlation id tying begin/output/end together.
        call_id: String,
        /// The argv of the command.
        command: Vec<String>,
        /// Accumulated combined output.
        output: String,
        /// `None` while running, `Some(code)` once finished.
        exit_code: Option<i32>,
    },
    /// An error surfaced by the agent.
    ErrorCell(String),
    /// A local, client-side informational notice (never sent on the wire).
    Notice(String),
}

/// An approval the agent is waiting on, surfaced to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingApproval {
    /// The agent wants to run a command.
    Exec {
        /// Request id to echo back in [`Op::ExecApproval`].
        id: String,
        /// The argv awaiting approval.
        command: Vec<String>,
        /// Optional human-readable reason.
        reason: Option<String>,
    },
    /// The agent wants to apply a patch.
    Patch {
        /// Request id to echo back in [`Op::PatchApproval`].
        id: String,
        /// The files the patch touches.
        files: Vec<String>,
    },
}

/// The outcome of handling a key press.
///
/// Returned by [`AppState::on_key`] so the event loop (or a test) can act on it
/// without the state machine needing to own a terminal or a channel.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Nothing observable happened (e.g. a plain edit of the input buffer).
    None,
    /// A submission should be forwarded to the agent.
    Submit(Submission),
    /// The user asked to quit the application.
    Quit,
}

/// The full, pure state of the agent surface.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// The conversation transcript, oldest first.
    transcript: Vec<TranscriptCell>,
    /// In-flight visible agent message, accumulated from deltas.
    agent_buffer: String,
    /// In-flight reasoning text, accumulated from deltas.
    reasoning_buffer: String,
    /// The current input line.
    input: String,
    /// Cursor position as a character (not byte) index into `input`.
    cursor: usize,
    /// Whether a turn is currently in progress.
    turn_active: bool,
    /// Latest `(input, output, total)` token accounting, if any.
    token_count: Option<(u64, u64, u64)>,
    /// The approval currently blocking the turn, if any.
    pending_approval: Option<PendingApproval>,
    /// Number of transcript lines scrolled up from the bottom (0 == latest).
    scroll_offset: u16,
}

impl AppState {
    /// Builds an empty state with a single welcome notice.
    #[must_use]
    pub fn new() -> Self {
        let mut s = Self::default();
        s.transcript.push(TranscriptCell::Notice(
            "aphrody agent ready. Type a message and press Enter.".to_string(),
        ));
        s
    }

    /// The conversation transcript, oldest first.
    #[must_use]
    pub fn transcript(&self) -> &[TranscriptCell] {
        &self.transcript
    }

    /// The current input line.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The cursor position as a character index into [`AppState::input`].
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Whether a turn is currently in progress.
    #[must_use]
    pub const fn turn_active(&self) -> bool {
        self.turn_active
    }

    /// The latest `(input, output, total)` token counts, if reported.
    #[must_use]
    pub const fn token_count(&self) -> Option<(u64, u64, u64)> {
        self.token_count
    }

    /// The approval currently awaiting a decision, if any.
    #[must_use]
    pub const fn pending_approval(&self) -> Option<&PendingApproval> {
        self.pending_approval.as_ref()
    }

    /// The current vertical scroll offset (lines above the latest content).
    #[must_use]
    pub const fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    /// The in-flight agent message buffer (visible while streaming).
    #[must_use]
    pub fn agent_buffer(&self) -> &str {
        &self.agent_buffer
    }

    /// Applies a single protocol event to the state.
    ///
    /// This is the heart of the state machine: it accumulates streamed deltas,
    /// finalizes messages, opens and closes exec / tool cells by `call_id`, and
    /// records pending approvals. It performs no I/O.
    pub fn apply_event(&mut self, msg: EventMsg) {
        match msg {
            EventMsg::TurnStarted => {
                self.turn_active = true;
                self.agent_buffer.clear();
                self.reasoning_buffer.clear();
            }
            EventMsg::AgentMessageDelta { delta } => {
                self.agent_buffer.push_str(&delta);
            }
            EventMsg::AgentMessage { text } => {
                // A finalized message supersedes whatever we streamed.
                self.agent_buffer.clear();
                self.transcript.push(TranscriptCell::AgentMessage(text));
            }
            EventMsg::SteerApplied { text } => {
                // Mid-turn guidance was folded into the conversation; show it in
                // the transcript as the user message it effectively is.
                self.transcript.push(TranscriptCell::UserMessage(text));
            }
            EventMsg::AgentReasoningDelta { delta } => {
                self.reasoning_buffer.push_str(&delta);
            }
            EventMsg::ExecCommandBegin {
                call_id,
                command,
                cwd: _,
            } => {
                self.transcript.push(TranscriptCell::Exec {
                    call_id,
                    command,
                    output: String::new(),
                    exit_code: None,
                });
            }
            EventMsg::ExecCommandOutputDelta { call_id, chunk } => {
                if let Some(TranscriptCell::Exec { output, .. }) = self.find_exec_mut(&call_id) {
                    output.push_str(&chunk);
                }
            }
            EventMsg::ExecCommandEnd {
                call_id,
                exit_code: code,
            } => {
                if let Some(TranscriptCell::Exec { exit_code, .. }) = self.find_exec_mut(&call_id) {
                    *exit_code = Some(code);
                }
            }
            EventMsg::ExecApprovalRequest {
                id,
                command,
                reason,
            } => {
                self.pending_approval = Some(PendingApproval::Exec {
                    id,
                    command,
                    reason,
                });
            }
            EventMsg::ApplyPatchApprovalRequest { id, files } => {
                self.pending_approval = Some(PendingApproval::Patch { id, files });
            }
            EventMsg::ToolCallBegin { call_id, name } => {
                self.transcript.push(TranscriptCell::ToolCall {
                    name,
                    call_id,
                    done: None,
                });
            }
            EventMsg::ToolCallEnd { call_id, ok } => {
                if let Some(TranscriptCell::ToolCall { done, .. }) = self.find_tool_mut(&call_id) {
                    *done = Some(ok);
                }
            }
            EventMsg::TokenCount {
                input,
                output,
                total,
            } => {
                self.token_count = Some((input, output, total));
            }
            EventMsg::TurnComplete { last_agent_message } => {
                // Flush any streamed-but-not-finalized agent text so nothing is
                // silently dropped at end of turn.
                if !self.agent_buffer.is_empty() {
                    let text = std::mem::take(&mut self.agent_buffer);
                    self.transcript.push(TranscriptCell::AgentMessage(text));
                } else if let Some(text) = last_agent_message {
                    // Only synthesize from the closing summary if we have not
                    // already emitted a finalized AgentMessage with the same
                    // text during this turn.
                    let already_present = matches!(
                        self.transcript.last(),
                        Some(TranscriptCell::AgentMessage(t)) if *t == text
                    );
                    if !already_present {
                        self.transcript.push(TranscriptCell::AgentMessage(text));
                    }
                }
                if !self.reasoning_buffer.is_empty() {
                    let text = std::mem::take(&mut self.reasoning_buffer);
                    self.transcript.push(TranscriptCell::Reasoning(text));
                }
                self.turn_active = false;
            }
            EventMsg::Error { message } => {
                self.transcript.push(TranscriptCell::ErrorCell(message));
                self.turn_active = false;
            }
        }
    }

    /// Finds a mutable reference to the open exec cell with `call_id`.
    fn find_exec_mut(&mut self, call_id: &str) -> Option<&mut TranscriptCell> {
        self.transcript.iter_mut().rev().find(|c| {
            matches!(c, TranscriptCell::Exec { call_id: id, .. } if id == call_id)
        })
    }

    /// Finds a mutable reference to the tool-call cell with `call_id`.
    fn find_tool_mut(&mut self, call_id: &str) -> Option<&mut TranscriptCell> {
        self.transcript.iter_mut().rev().find(|c| {
            matches!(c, TranscriptCell::ToolCall { call_id: id, .. } if id == call_id)
        })
    }

    /// Handles a key press, returning the resulting [`Action`].
    ///
    /// Behaviour:
    /// - When an approval is pending, `y`/`n`/`a` resolve it (Approved / Denied
    ///   / Abort) and clear the pending state.
    /// - `Enter` with non-empty input submits a [`Op::UserInput`] and clears the
    ///   line.
    /// - `Ctrl-C` interrupts an active turn, or shuts down when idle.
    /// - `Esc` (with empty input) quits.
    /// - Printable characters, `Backspace`, and arrow keys edit the input line.
    pub fn on_key(&mut self, key: KeyPress) -> Action {
        // Approval prompt intercepts the relevant keys first.
        if self.pending_approval.is_some() {
            match key {
                KeyPress::Char('y') => return self.resolve_approval(ReviewDecision::Approved),
                KeyPress::Char('n') => return self.resolve_approval(ReviewDecision::Denied),
                KeyPress::Char('a') => return self.resolve_approval(ReviewDecision::Abort),
                // Any other key while a decision is pending is ignored so a
                // stray edit cannot leak through.
                _ => return Action::None,
            }
        }

        match key {
            KeyPress::CtrlC => {
                let op = if self.turn_active {
                    Op::Interrupt
                } else {
                    Op::Shutdown
                };
                Action::Submit(Submission::new(op))
            }
            KeyPress::Enter => {
                let text = self.input.trim_end_matches(['\r', '\n']).to_string();
                if text.is_empty() {
                    return Action::None;
                }
                self.transcript
                    .push(TranscriptCell::UserMessage(text.clone()));
                self.input.clear();
                self.cursor = 0;
                Action::Submit(Submission::new(Op::UserInput {
                    items: vec![InputItem::Text { text }],
                }))
            }
            KeyPress::Esc => {
                if self.input.is_empty() {
                    Action::Quit
                } else {
                    self.input.clear();
                    self.cursor = 0;
                    Action::None
                }
            }
            KeyPress::Char(c) => {
                self.insert_char(c);
                Action::None
            }
            KeyPress::Backspace => {
                self.delete_char();
                Action::None
            }
            KeyPress::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                Action::None
            }
            KeyPress::Right => {
                let max = self.input.chars().count();
                self.cursor = (self.cursor + 1).min(max);
                Action::None
            }
            KeyPress::Home => {
                self.cursor = 0;
                Action::None
            }
            KeyPress::End => {
                self.cursor = self.input.chars().count();
                Action::None
            }
            KeyPress::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_add(5);
                Action::None
            }
            KeyPress::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
                Action::None
            }
        }
    }

    /// Resolves the pending approval with `decision` and clears it.
    fn resolve_approval(&mut self, decision: ReviewDecision) -> Action {
        let Some(pending) = self.pending_approval.take() else {
            return Action::None;
        };
        let op = match pending {
            PendingApproval::Exec { id, .. } => Op::ExecApproval { id, decision },
            PendingApproval::Patch { id, .. } => Op::PatchApproval { id, decision },
        };
        Action::Submit(Submission::new(op))
    }

    /// Inserts `c` at the cursor, respecting char boundaries.
    fn insert_char(&mut self, c: char) {
        let byte_idx = self.byte_index();
        self.input.insert(byte_idx, c);
        self.cursor += 1;
    }

    /// Deletes the char left of the cursor, respecting char boundaries.
    fn delete_char(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let target = self.cursor - 1;
        let before = self.input.chars().take(target);
        let after = self.input.chars().skip(self.cursor);
        self.input = before.chain(after).collect();
        self.cursor = target;
    }

    /// Maps the char-index cursor to a byte index into `input`.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.cursor)
            .unwrap_or(self.input.len())
    }
}

/// A normalized key event, decoupled from any backend.
///
/// The event loop maps backend (crossterm) key events into this enum so the
/// pure state machine never depends on crossterm types and stays trivially
/// testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPress {
    /// A printable character.
    Char(char),
    /// The Enter / Return key.
    Enter,
    /// The Backspace key.
    Backspace,
    /// The Escape key.
    Esc,
    /// `Ctrl-C`.
    CtrlC,
    /// Cursor left.
    Left,
    /// Cursor right.
    Right,
    /// Jump to start of line.
    Home,
    /// Jump to end of line.
    End,
    /// Scroll the transcript up.
    PageUp,
    /// Scroll the transcript down.
    PageDown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use aphrody_agent_proto::Op;
    use pretty_assertions::assert_eq;

    fn type_str(state: &mut AppState, s: &str) {
        for c in s.chars() {
            state.on_key(KeyPress::Char(c));
        }
    }

    #[test]
    fn streaming_agent_message_finalizes_and_clears_buffer() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::TurnStarted);
        assert!(s.turn_active());
        s.apply_event(EventMsg::AgentMessageDelta { delta: "He".into() });
        s.apply_event(EventMsg::AgentMessageDelta {
            delta: "llo".into(),
        });
        assert_eq!(s.agent_buffer(), "Hello");
        s.apply_event(EventMsg::AgentMessage {
            text: "Hello".into(),
        });
        s.apply_event(EventMsg::TurnComplete {
            last_agent_message: Some("Hello".into()),
        });

        assert!(!s.turn_active());
        assert_eq!(s.agent_buffer(), "");
        let finalized: Vec<_> = s
            .transcript()
            .iter()
            .filter(|c| matches!(c, TranscriptCell::AgentMessage(_)))
            .collect();
        assert_eq!(finalized.len(), 1, "exactly one finalized message");
        assert_eq!(
            finalized[0],
            &TranscriptCell::AgentMessage("Hello".into())
        );
    }

    #[test]
    fn turn_complete_flushes_unfinalized_buffer() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::TurnStarted);
        s.apply_event(EventMsg::AgentMessageDelta {
            delta: "partial".into(),
        });
        s.apply_event(EventMsg::TurnComplete {
            last_agent_message: None,
        });
        assert_eq!(s.agent_buffer(), "");
        assert_eq!(
            s.transcript().last(),
            Some(&TranscriptCell::AgentMessage("partial".into()))
        );
    }

    #[test]
    fn tool_call_lifecycle_by_call_id() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::ToolCallBegin {
            call_id: "t1".into(),
            name: "search".into(),
        });
        match s.transcript().last() {
            Some(TranscriptCell::ToolCall { done, name, .. }) => {
                assert_eq!(done, &None);
                assert_eq!(name, "search");
            }
            other => panic!("expected open tool call, got {other:?}"),
        }
        s.apply_event(EventMsg::ToolCallEnd {
            call_id: "t1".into(),
            ok: true,
        });
        match s.transcript().last() {
            Some(TranscriptCell::ToolCall { done, .. }) => assert_eq!(done, &Some(true)),
            other => panic!("expected closed tool call, got {other:?}"),
        }
    }

    #[test]
    fn exec_output_concatenates_and_records_exit_code() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::ExecCommandBegin {
            call_id: "c1".into(),
            command: vec!["echo".into(), "hi".into()],
            cwd: std::path::PathBuf::from("/work"),
        });
        s.apply_event(EventMsg::ExecCommandOutputDelta {
            call_id: "c1".into(),
            chunk: "foo".into(),
        });
        s.apply_event(EventMsg::ExecCommandOutputDelta {
            call_id: "c1".into(),
            chunk: "bar".into(),
        });
        s.apply_event(EventMsg::ExecCommandEnd {
            call_id: "c1".into(),
            exit_code: 0,
        });
        match s.transcript().last() {
            Some(TranscriptCell::Exec {
                output, exit_code, ..
            }) => {
                assert_eq!(output, "foobar");
                assert_eq!(exit_code, &Some(0));
            }
            other => panic!("expected exec cell, got {other:?}"),
        }
    }

    #[test]
    fn output_delta_for_unknown_call_id_is_ignored() {
        let mut s = AppState::new();
        // No begin: a stray delta must not panic or create a cell.
        let before = s.transcript().len();
        s.apply_event(EventMsg::ExecCommandOutputDelta {
            call_id: "ghost".into(),
            chunk: "x".into(),
        });
        assert_eq!(s.transcript().len(), before);
    }

    #[test]
    fn typing_then_enter_submits_user_input_and_clears() {
        let mut s = AppState::new();
        type_str(&mut s, "hi");
        assert_eq!(s.input(), "hi");
        let action = s.on_key(KeyPress::Enter);
        match action {
            Action::Submit(sub) => assert_eq!(
                sub.op,
                Op::UserInput {
                    items: vec![InputItem::Text { text: "hi".into() }]
                }
            ),
            other => panic!("expected submit, got {other:?}"),
        }
        assert_eq!(s.input(), "");
        assert_eq!(s.cursor(), 0);
        assert_eq!(
            s.transcript().last(),
            Some(&TranscriptCell::UserMessage("hi".into()))
        );
    }

    #[test]
    fn enter_with_empty_input_does_nothing() {
        let mut s = AppState::new();
        assert_eq!(s.on_key(KeyPress::Enter), Action::None);
    }

    #[test]
    fn ctrl_c_interrupts_active_turn_else_shuts_down() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::TurnStarted);
        match s.on_key(KeyPress::CtrlC) {
            Action::Submit(sub) => assert_eq!(sub.op, Op::Interrupt),
            other => panic!("expected interrupt, got {other:?}"),
        }
        s.apply_event(EventMsg::TurnComplete {
            last_agent_message: None,
        });
        match s.on_key(KeyPress::CtrlC) {
            Action::Submit(sub) => assert_eq!(sub.op, Op::Shutdown),
            other => panic!("expected shutdown, got {other:?}"),
        }
    }

    #[test]
    fn esc_quits_when_input_empty_else_clears() {
        let mut s = AppState::new();
        type_str(&mut s, "draft");
        assert_eq!(s.on_key(KeyPress::Esc), Action::None);
        assert_eq!(s.input(), "");
        assert_eq!(s.on_key(KeyPress::Esc), Action::Quit);
    }

    #[test]
    fn exec_approval_sets_pending_and_y_approves() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::ExecApprovalRequest {
            id: "req-1".into(),
            command: vec!["rm".into(), "-rf".into()],
            reason: Some("destructive".into()),
        });
        assert!(s.pending_approval().is_some());
        match s.on_key(KeyPress::Char('y')) {
            Action::Submit(sub) => assert_eq!(
                sub.op,
                Op::ExecApproval {
                    id: "req-1".into(),
                    decision: ReviewDecision::Approved,
                }
            ),
            other => panic!("expected approval, got {other:?}"),
        }
        assert!(s.pending_approval().is_none());
    }

    #[test]
    fn patch_approval_n_denies() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::ApplyPatchApprovalRequest {
            id: "p1".into(),
            files: vec!["src/lib.rs".into()],
        });
        match s.on_key(KeyPress::Char('n')) {
            Action::Submit(sub) => assert_eq!(
                sub.op,
                Op::PatchApproval {
                    id: "p1".into(),
                    decision: ReviewDecision::Denied,
                }
            ),
            other => panic!("expected denial, got {other:?}"),
        }
        assert!(s.pending_approval().is_none());
    }

    #[test]
    fn approval_a_aborts_and_typing_blocked_while_pending() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::ExecApprovalRequest {
            id: "req-2".into(),
            command: vec!["ls".into()],
            reason: None,
        });
        // Typing is suppressed while a decision is pending.
        assert_eq!(s.on_key(KeyPress::Char('z')), Action::None);
        assert_eq!(s.input(), "");
        match s.on_key(KeyPress::Char('a')) {
            Action::Submit(sub) => assert_eq!(
                sub.op,
                Op::ExecApproval {
                    id: "req-2".into(),
                    decision: ReviewDecision::Abort,
                }
            ),
            other => panic!("expected abort, got {other:?}"),
        }
    }

    #[test]
    fn token_count_and_error_recorded() {
        let mut s = AppState::new();
        s.apply_event(EventMsg::TokenCount {
            input: 10,
            output: 5,
            total: 15,
        });
        assert_eq!(s.token_count(), Some((10, 5, 15)));
        s.apply_event(EventMsg::Error {
            message: "boom".into(),
        });
        assert_eq!(
            s.transcript().last(),
            Some(&TranscriptCell::ErrorCell("boom".into()))
        );
        assert!(!s.turn_active());
    }

    #[test]
    fn unicode_input_edits_on_char_boundaries() {
        let mut s = AppState::new();
        type_str(&mut s, "héllo");
        assert_eq!(s.cursor(), 5);
        s.on_key(KeyPress::Left);
        s.on_key(KeyPress::Backspace); // delete 'l' before cursor
        assert_eq!(s.input(), "hélo");
        s.on_key(KeyPress::Home);
        assert_eq!(s.cursor(), 0);
        s.on_key(KeyPress::End);
        assert_eq!(s.cursor(), 4);
    }
}
