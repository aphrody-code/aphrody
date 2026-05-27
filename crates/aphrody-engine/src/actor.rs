// SPDX-License-Identifier: Apache-2.0
//! A long-lived session actor that drives [`run_turn`](crate::run_turn) from a
//! stream of [`Submission`]s.
//!
//! [`spawn_session`] wires the SQ/EQ pattern end to end: the caller submits
//! [`Op`]s over a channel and receives [`Event`]s on another. A
//! [`Op::UserInput`] starts a turn; [`Op::ExecApproval`] / [`Op::PatchApproval`]
//! answer a pending approval; [`Op::Interrupt`] trips the running turn's
//! interrupt flag; [`Op::Shutdown`] drains and stops the actor.

use std::sync::Arc;

use aphrody_agent_proto::{Event, Op, Submission};
use aphrody_model_client::ModelClient;
use aphrody_rollout::RolloutRecorder;
use aphrody_toolcall::ToolRegistry;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::approval::ApprovalGate;
use crate::config::AutonomyMode;
use crate::session::Session;
use crate::sink::EventSink;
use crate::turn::{InterruptFlag, run_turn_with_interrupt};

/// Handle to a spawned session actor.
///
/// Submit work with [`SessionHandle::submit`] and drain results from
/// [`SessionHandle::events`]. Dropping all clones of the submission sender (or
/// sending [`Op::Shutdown`]) lets the actor finish and exit; [`join`](Self::join)
/// awaits its task.
#[derive(Debug)]
pub struct SessionHandle {
    submissions: mpsc::UnboundedSender<Submission>,
    events: Option<mpsc::UnboundedReceiver<Event>>,
    task: JoinHandle<Session>,
}

impl SessionHandle {
    /// Submit an op to the actor.
    ///
    /// # Errors
    /// Returns the submission back if the actor has already stopped.
    pub fn submit(&self, op: Op) -> Result<(), Submission> {
        let submission = Submission::new(op);
        self.submissions.send(submission).map_err(|e| e.0)
    }

    /// Take ownership of the event receiver (callable once).
    pub fn events(&mut self) -> Option<mpsc::UnboundedReceiver<Event>> {
        self.events.take()
    }

    /// Await the actor's task, recovering the final [`Session`] state.
    ///
    /// # Errors
    /// Returns a [`tokio::task::JoinError`] if the actor task panicked.
    pub async fn join(self) -> Result<Session, tokio::task::JoinError> {
        self.task.await
    }
}

/// Spawn a session actor driven by `client` + `tools`, optionally recording to
/// `rollout`.
///
/// Returns a [`SessionHandle`] owning the submission sender, the event
/// receiver, and the actor task handle.
#[must_use]
pub fn spawn_session(
    mut session: Session,
    client: Arc<dyn ModelClient>,
    tools: Arc<ToolRegistry>,
    rollout: Option<RolloutRecorder>,
) -> SessionHandle {
    let (sub_tx, mut sub_rx) = mpsc::unbounded_channel::<Submission>();
    let (event_sink, event_rx) = EventSink::unbounded();

    let task = tokio::spawn(async move {
        let gated = session.config().autonomy == AutonomyMode::Gated;
        // The approval sender is only used when gated; we keep it alive so the
        // gate's channel does not close between turns.
        let (approval_tx, mut gate) = ApprovalGate::channel();

        while let Some(submission) = sub_rx.recv().await {
            match submission.op {
                Op::UserInput { items } => {
                    let interrupt = InterruptFlag::new();
                    // Forward any interrupt/approval ops that arrive *during*
                    // the turn into the running loop.
                    let approvals = if gated { Some(&mut gate) } else { None };
                    let control = drive_turn(
                        &mut session,
                        client.as_ref(),
                        tools.as_ref(),
                        rollout.as_ref(),
                        items,
                        &event_sink,
                        approvals,
                        &mut sub_rx,
                        &approval_tx,
                        &interrupt,
                    )
                    .await;
                    if let Err(err) = control.result {
                        event_sink.emit(aphrody_agent_proto::EventMsg::Error {
                            message: err.to_string(),
                        });
                    }
                    if control.shutdown {
                        // A Shutdown / closed channel was observed mid-turn.
                        break;
                    }
                }
                Op::ExecApproval { .. } | Op::PatchApproval { .. } => {
                    // An approval that arrives outside a turn has nothing to
                    // answer; forward it to the gate so an in-flight wait (if
                    // any) can observe it, ignoring send failures.
                    let _ = approval_tx.send(submission.op);
                }
                Op::Interrupt => {
                    // No turn is currently running (we are between turns); a
                    // standalone interrupt is a no-op other than acknowledging.
                }
                Op::Compact => {
                    // Compaction is delegated to a future context-management
                    // pass; for now it is acknowledged without altering state.
                }
                Op::Shutdown => break,
            }
        }

        if let Some(rollout) = rollout {
            let _ = rollout.shutdown().await;
        }
        session
    });

    SessionHandle {
        submissions: sub_tx,
        events: Some(event_rx),
        task,
    }
}

/// The outcome of [`drive_turn`]: the turn's result plus whether a shutdown
/// (explicit [`Op::Shutdown`] or a closed submission channel) was observed while
/// the turn ran, so the actor loop can exit.
struct TurnControl {
    result: Result<(), crate::EngineError>,
    shutdown: bool,
}

/// Drive a single turn while concurrently servicing interrupt/approval ops that
/// arrive on the submission channel.
#[allow(clippy::too_many_arguments)]
async fn drive_turn(
    session: &mut Session,
    client: &dyn ModelClient,
    tools: &ToolRegistry,
    rollout: Option<&RolloutRecorder>,
    items: Vec<aphrody_agent_proto::InputItem>,
    events: &EventSink,
    approvals: Option<&mut ApprovalGate>,
    sub_rx: &mut mpsc::UnboundedReceiver<Submission>,
    approval_tx: &mpsc::UnboundedSender<Op>,
    interrupt: &InterruptFlag,
) -> TurnControl {
    // Run the turn as a child future and, in parallel, pump incoming control
    // submissions (Interrupt -> trip the flag; approvals -> forward to gate).
    let turn = run_turn_with_interrupt(
        session, client, tools, rollout, items, events, approvals, interrupt,
    );
    tokio::pin!(turn);

    let mut shutdown = false;
    loop {
        tokio::select! {
            biased;
            // Poll the turn first so it makes progress; control ops are only
            // serviced when the turn future is pending at an await point. This
            // lets a turn run to natural completion even if a Shutdown is
            // already queued behind the UserInput that started it.
            outcome = &mut turn => {
                return TurnControl { result: outcome.map(|_| ()), shutdown };
            }
            maybe_sub = sub_rx.recv() => {
                match maybe_sub {
                    Some(sub) => match sub.op {
                        Op::Interrupt => interrupt.trip(),
                        op @ (Op::ExecApproval { .. } | Op::PatchApproval { .. }) => {
                            let _ = approval_tx.send(op);
                        }
                        // Buffering a new UserInput mid-turn is out of scope;
                        // it is dropped with a trace so the turn can finish.
                        Op::UserInput { .. } => {
                            tracing::warn!("ignoring UserInput received mid-turn");
                        }
                        Op::Compact => {}
                        Op::Shutdown => {
                            shutdown = true;
                            interrupt.trip();
                        }
                    },
                    None => {
                        shutdown = true;
                        interrupt.trip();
                    }
                }
            }
        }
    }
}
