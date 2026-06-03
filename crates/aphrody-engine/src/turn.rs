// SPDX-License-Identifier: Apache-2.0
//! The turn loop: the heart of the engine.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use aphrody_agent_proto::{Event, EventMsg, InputItem, ReviewDecision};
use aphrody_model_client::{ChatTurn, ModelClient, ModelStreamEvent, Part, ToolDecl};
use aphrody_rollout::RolloutRecorder;
use aphrody_toolcall::{ToolOutput, ToolRegistry, ToolSafety};
use futures::StreamExt;
use serde_json::{Value, json};

use crate::approval::ApprovalGate;
use crate::config::AutonomyMode;
use crate::error::EngineError;
use crate::session::Session;
use crate::sink::EventSink;

/// A cooperative cancellation flag shared with the engine.
///
/// The engine checks the flag between steps (before each model call and before
/// each tool call). Setting it requests a clean interruption: the engine emits
/// a [`TurnComplete`](EventMsg::TurnComplete) with `interrupted = true` and
/// returns. Cloning shares the same underlying flag.
#[derive(Debug, Clone, Default)]
pub struct InterruptFlag(Arc<AtomicBool>);

impl InterruptFlag {
    /// Create a fresh, un-tripped flag.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Request interruption.
    pub fn trip(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether interruption has been requested.
    #[must_use]
    pub fn is_tripped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A cooperative queue of *steering* input shared with the engine.
///
/// A client or supervisor pushes [`InputItem`]s with [`SteerQueue::push`] while
/// a turn is running. The engine drains the queue at the start of every loop
/// iteration — before the next model call — and folds the items into the
/// conversation as an extra user message, emitting
/// [`SteerApplied`](EventMsg::SteerApplied). This lets a caller course-correct
/// an in-flight turn without interrupting and restarting it. Cloning shares the
/// same underlying queue.
///
/// The queue is pure (no syscalls) and compiles to wasm; a poisoned lock is
/// recovered rather than propagated, because a panic in another holder must not
/// drop already-queued steering.
#[derive(Debug, Clone, Default)]
pub struct SteerQueue(Arc<Mutex<Vec<InputItem>>>);

impl SteerQueue {
    /// Create a fresh, empty steer queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append steering `items` for the running turn to pick up. Empty pushes are
    /// dropped.
    pub fn push(&self, items: Vec<InputItem>) {
        if items.is_empty() {
            return;
        }
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        guard.extend(items);
    }

    /// Take all currently queued steering items, leaving the queue empty.
    #[must_use]
    pub fn drain(&self) -> Vec<InputItem> {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *guard)
    }

    /// Whether the queue currently holds no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        guard.is_empty()
    }
}

/// The result of a completed (or interrupted) turn.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TurnOutcome {
    /// The last visible agent message produced this turn, if any.
    pub last_agent_message: Option<String>,
    /// How many model<->tool round trips ran (the number of model calls).
    pub tool_iterations: usize,
    /// Whether the turn ended because of an [`InterruptFlag`] trip or an
    /// approval interrupt rather than natural completion.
    pub interrupted: bool,
    /// Whether the turn stopped because it hit
    /// [`max_tool_iterations`](crate::EngineConfig::max_tool_iterations).
    pub hit_iteration_cap: bool,
}

/// A single tool call the model requested, awaiting dispatch.
struct PendingCall {
    id: String,
    name: String,
    arguments: Value,
}

/// Emit `msg` to the sink and, when a recorder is present, mirror it to the
/// rollout as a serialized [`Event`].
fn emit(events: &EventSink, rollout: Option<&RolloutRecorder>, msg: EventMsg) {
    let event = Event::new(msg);
    if let Some(rec) = rollout {
        match serde_json::to_value(&event) {
            Ok(value) => rec.record(value),
            Err(err) => tracing::warn!("failed to serialize event for rollout: {err}"),
        }
    }
    events.send_event(event);
}

/// Convert user [`InputItem`]s into the [`Part`]s of a single user message.
///
/// Text items map directly. Image items are rendered as a descriptive text
/// part for the MVP (the model client does not yet carry inline image bytes);
/// an item with neither a path nor a URL is dropped.
fn input_to_parts(input: Vec<InputItem>) -> Vec<Part> {
    let mut parts = Vec::with_capacity(input.len());
    for item in input {
        match item {
            InputItem::Text { text } => parts.push(Part::Text(text)),
            InputItem::Image { path, url } => {
                let reference = match (path, url) {
                    (Some(path), _) => Some(format!("[image: {}]", path.display())),
                    (None, Some(url)) => Some(format!("[image: {url}]")),
                    (None, None) => None,
                };
                if let Some(reference) = reference {
                    parts.push(Part::Text(reference));
                }
            }
        }
    }
    parts
}

/// Derive the model-visible [`ToolDecl`] list from the registry.
fn tool_decls(tools: &ToolRegistry) -> Vec<ToolDecl> {
    tools
        .model_visible_definitions()
        .into_iter()
        .map(|definition| {
            let decl = definition.to_gemini_declaration();
            ToolDecl {
                name: definition.name.clone(),
                description: definition.description.clone(),
                parameters: decl
                    .get("parameters")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            }
        })
        .collect()
}

/// Drive one full agent turn to completion.
///
/// Appends `input` to `session`'s history, then loops up to
/// [`max_tool_iterations`](crate::EngineConfig::max_tool_iterations) times:
/// stream the model, surface text / reasoning / token events, execute any
/// requested tool calls (honoring [`AutonomyMode`]), re-inject their results,
/// and repeat until the model stops calling tools. Every emitted event is sent
/// to `events` and, when `rollout` is set, mirrored to the recorder.
///
/// `approvals` is only consulted in [`AutonomyMode::Gated`]; pass `None` in
/// [`AutonomyMode::FullAuto`].
///
/// # Errors
///
/// Returns [`EngineError::Model`] if the model stream errors, [`EngineError::Tool`]
/// for a hard registry error, [`EngineError::Aborted`] if an approval decision
/// aborts the turn, or [`EngineError::ApprovalChannelClosed`] if a gated turn's
/// approval channel closes before a decision arrives.
#[allow(clippy::too_many_arguments)]
pub async fn run_turn(
    session: &mut Session,
    client: &dyn ModelClient,
    tools: &ToolRegistry,
    rollout: Option<&RolloutRecorder>,
    input: Vec<InputItem>,
    events: &EventSink,
    mut approvals: Option<&mut ApprovalGate>,
) -> Result<TurnOutcome, EngineError> {
    let interrupt = InterruptFlag::new();
    run_turn_with_interrupt(
        session,
        client,
        tools,
        rollout,
        input,
        events,
        approvals.take(),
        &interrupt,
    )
    .await
}

/// Like [`run_turn`] but with an explicit [`InterruptFlag`] the caller can trip
/// from another task to cancel the turn cooperatively.
///
/// This is the no-steering shorthand for [`run_turn_with_controls`]: it runs the
/// turn with a fresh, never-fed [`SteerQueue`], so no mid-turn guidance can be
/// injected.
#[allow(clippy::too_many_arguments)]
pub async fn run_turn_with_interrupt(
    session: &mut Session,
    client: &dyn ModelClient,
    tools: &ToolRegistry,
    rollout: Option<&RolloutRecorder>,
    input: Vec<InputItem>,
    events: &EventSink,
    mut approvals: Option<&mut ApprovalGate>,
    interrupt: &InterruptFlag,
) -> Result<TurnOutcome, EngineError> {
    let steer = SteerQueue::new();
    run_turn_with_controls(
        session,
        client,
        tools,
        rollout,
        input,
        events,
        approvals.take(),
        interrupt,
        &steer,
    )
    .await
}

/// The full turn loop with both cooperative controls: an [`InterruptFlag`] to
/// cancel and a [`SteerQueue`] to inject mid-turn guidance.
///
/// At the top of each iteration (after the interrupt and iteration-cap checks,
/// before the next model call) the loop drains `steer`; any queued items are
/// appended to the conversation as an extra user message and surfaced as an
/// [`EventMsg::SteerApplied`]. Everything else matches
/// [`run_turn`]'s contract.
///
/// # Errors
///
/// Same as [`run_turn`]: [`EngineError::Model`] on a model-stream error,
/// [`EngineError::Aborted`] if an approval aborts the turn, or
/// [`EngineError::ApprovalChannelClosed`] if a gated turn's approval channel
/// closes before a decision arrives.
#[allow(clippy::too_many_arguments)]
pub async fn run_turn_with_controls(
    session: &mut Session,
    client: &dyn ModelClient,
    tools: &ToolRegistry,
    rollout: Option<&RolloutRecorder>,
    input: Vec<InputItem>,
    events: &EventSink,
    mut approvals: Option<&mut ApprovalGate>,
    interrupt: &InterruptFlag,
    steer: &SteerQueue,
) -> Result<TurnOutcome, EngineError> {
    let autonomy = session.config().autonomy;
    let system = session.config().system_prompt.clone();
    let max_iterations = session.config().max_tool_iterations;

    // 1. Append the user input.
    let parts = input_to_parts(input);
    session.push_user(parts);

    // 2. Announce the turn.
    emit(events, rollout, EventMsg::TurnStarted);

    let mut outcome = TurnOutcome::default();

    loop {
        if interrupt.is_tripped() {
            outcome.interrupted = true;
            emit(
                events,
                rollout,
                EventMsg::TurnComplete {
                    last_agent_message: outcome.last_agent_message.clone(),
                },
            );
            return Ok(outcome);
        }

        if outcome.tool_iterations >= max_iterations {
            outcome.hit_iteration_cap = true;
            emit(
                events,
                rollout,
                EventMsg::Error {
                    message: format!("max tool iterations reached ({max_iterations})"),
                },
            );
            emit(
                events,
                rollout,
                EventMsg::TurnComplete {
                    last_agent_message: outcome.last_agent_message.clone(),
                },
            );
            return Ok(outcome);
        }

        // 3·. Fold in any steering guidance queued since the previous model
        // call, as an extra user message, so the upcoming call course-corrects.
        let steered = steer.drain();
        if !steered.is_empty() {
            let parts = input_to_parts(steered);
            if !parts.is_empty() {
                let text = parts
                    .iter()
                    .filter_map(|part| match part {
                        Part::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                session.push_user(parts);
                emit(events, rollout, EventMsg::SteerApplied { text });
            }
        }

        // 3a. Build the turn snapshot.
        let chat_turn = ChatTurn {
            system: system.clone(),
            messages: session.history().to_vec(),
            tools: tool_decls(tools),
        };

        // 3b. Stream the model.
        let mut stream = client.stream(chat_turn).await.map_err(EngineError::Model)?;
        outcome.tool_iterations += 1;

        let mut text_buf = String::new();
        let mut pending_calls: Vec<PendingCall> = Vec::new();

        while let Some(event) = stream.next().await {
            match event {
                Ok(ModelStreamEvent::Created) => {}
                Ok(ModelStreamEvent::TextDelta(delta)) => {
                    text_buf.push_str(&delta);
                    emit(events, rollout, EventMsg::AgentMessageDelta { delta });
                }
                Ok(ModelStreamEvent::ReasoningDelta(delta)) => {
                    emit(events, rollout, EventMsg::AgentReasoningDelta { delta });
                }
                Ok(ModelStreamEvent::ToolCall {
                    id,
                    name,
                    arguments,
                }) => {
                    pending_calls.push(PendingCall {
                        id,
                        name,
                        arguments,
                    });
                }
                Ok(ModelStreamEvent::Completed {
                    usage,
                    finish_reason: _,
                }) => {
                    if let Some(usage) = usage {
                        emit(
                            events,
                            rollout,
                            EventMsg::TokenCount {
                                input: usage.input_tokens,
                                output: usage.output_tokens,
                                total: usage.total_tokens,
                            },
                        );
                    }
                    break;
                }
                Err(err) => {
                    emit(
                        events,
                        rollout,
                        EventMsg::Error {
                            message: err.to_string(),
                        },
                    );
                    return Err(EngineError::Model(err));
                }
            }
        }

        // 3c. Finalize any visible text into a model message.
        if !text_buf.is_empty() {
            emit(
                events,
                rollout,
                EventMsg::AgentMessage {
                    text: text_buf.clone(),
                },
            );
            session.push_model_text(text_buf.clone());
            outcome.last_agent_message = Some(text_buf);
        }

        // 3d. No tool calls => the turn is complete.
        if pending_calls.is_empty() {
            emit(
                events,
                rollout,
                EventMsg::TurnComplete {
                    last_agent_message: outcome.last_agent_message.clone(),
                },
            );
            return Ok(outcome);
        }

        // 3e. Dispatch each requested tool call, re-injecting its result.
        for call in pending_calls {
            if interrupt.is_tripped() {
                outcome.interrupted = true;
                emit(
                    events,
                    rollout,
                    EventMsg::TurnComplete {
                        last_agent_message: outcome.last_agent_message.clone(),
                    },
                );
                return Ok(outcome);
            }

            // Record the model's request in the history before running it.
            session.push_model_function_call(&call.name, call.arguments.clone());
            emit(
                events,
                rollout,
                EventMsg::ToolCallBegin {
                    call_id: call.id.clone(),
                    name: call.name.clone(),
                },
            );

            let executor = tools.get(&call.name);

            // Command-safety: refuse a known-destructive call before approval or
            // execution, in any autonomy mode (defense-in-depth over the tool's
            // own checks). The default ToolSafety::Safe leaves every existing
            // tool's behaviour unchanged.
            if executor
                .as_ref()
                .is_some_and(|exec| exec.safety(&call.arguments) == ToolSafety::Refuse)
            {
                let output = ToolOutput::error(format!(
                    "tool call `{}` refused: command-safety classified it as known-destructive",
                    call.name
                ));
                finish_tool(session, events, rollout, &call, &output);
                continue;
            }

            // Gated approval, if configured.
            if autonomy == AutonomyMode::Gated {
                match approve(&call, events, rollout, approvals.as_deref_mut()).await? {
                    Gate::Approved => {}
                    Gate::Denied => {
                        let output = ToolOutput::error(format!(
                            "tool call `{}` denied by approval policy",
                            call.name
                        ));
                        finish_tool(session, events, rollout, &call, &output);
                        continue;
                    }
                    Gate::Abort => {
                        outcome.interrupted = true;
                        emit(
                            events,
                            rollout,
                            EventMsg::TurnComplete {
                                last_agent_message: outcome.last_agent_message.clone(),
                            },
                        );
                        return Ok(outcome);
                    }
                }
            }

            // Execute the tool.
            let output = match executor {
                None => ToolOutput::error(format!("unknown tool `{}`", call.name)),
                Some(executor) => match executor.handle(call.arguments.clone()).await {
                    Ok(output) => output,
                    Err(err) => ToolOutput::error(format!(
                        "tool `{}` failed: {err}",
                        call.name
                    )),
                },
            };

            finish_tool(session, events, rollout, &call, &output);
        }

        // 3f. Re-loop: the next model call sees the appended tool results.
    }
}

/// Outcome of an approval check.
enum Gate {
    Approved,
    Denied,
    Abort,
}

/// Resolve the approval for `call` per the gate, emitting the request event.
async fn approve(
    call: &PendingCall,
    events: &EventSink,
    rollout: Option<&RolloutRecorder>,
    approvals: Option<&mut ApprovalGate>,
) -> Result<Gate, EngineError> {
    emit(
        events,
        rollout,
        EventMsg::ExecApprovalRequest {
            id: call.id.clone(),
            command: vec![call.name.clone()],
            reason: Some(format!("tool call `{}`", call.name)),
        },
    );

    let Some(gate) = approvals else {
        // Gated mode without a wired channel cannot obtain a decision; deny the
        // call so the turn can proceed safely rather than block forever.
        return Ok(Gate::Denied);
    };

    match gate.wait_for(&call.id).await {
        Some(ReviewDecision::Approved | ReviewDecision::ApprovedForSession) => Ok(Gate::Approved),
        Some(ReviewDecision::Denied) => Ok(Gate::Denied),
        Some(ReviewDecision::Abort) => Ok(Gate::Abort),
        None => Err(EngineError::ApprovalChannelClosed),
    }
}

/// Emit the tool-call end event and append the function response to history.
fn finish_tool(
    session: &mut Session,
    events: &EventSink,
    rollout: Option<&RolloutRecorder>,
    call: &PendingCall,
    output: &ToolOutput,
) {
    emit(
        events,
        rollout,
        EventMsg::ToolCallEnd {
            call_id: call.id.clone(),
            ok: !output.is_error,
        },
    );
    session.push_tool_response(
        &call.name,
        json!({ "output": output.content, "is_error": output.is_error }),
    );
}
