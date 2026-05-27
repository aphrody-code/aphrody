// SPDX-License-Identifier: Apache-2.0
//! The approval gate used in [`AutonomyMode::Gated`](crate::AutonomyMode::Gated).
//!
//! When gated, the engine emits an
//! [`ExecApprovalRequest`](aphrody_agent_proto::EventMsg::ExecApprovalRequest)
//! and then blocks the turn until the surface replies with an
//! [`Op::ExecApproval`](aphrody_agent_proto::Op::ExecApproval) carrying the
//! matching request id. The gate is the receiving half of that approval
//! channel.

use aphrody_agent_proto::{Op, ReviewDecision};
use tokio::sync::mpsc;

/// Receives approval decisions from the client/surface.
///
/// Decisions arrive as [`Op`] values (the same submission enum the surface
/// already speaks). The gate filters for
/// [`Op::ExecApproval`] / [`Op::PatchApproval`] matching the request id it is
/// waiting on; [`Op::Interrupt`] is surfaced as an [`ReviewDecision::Abort`] so
/// a user interrupt while awaiting approval cleanly stops the turn. Other ops
/// are ignored while a decision is pending.
#[derive(Debug)]
pub struct ApprovalGate {
    rx: mpsc::UnboundedReceiver<Op>,
}

impl ApprovalGate {
    /// Wrap an existing receiver of approval [`Op`]s.
    #[must_use]
    pub fn new(rx: mpsc::UnboundedReceiver<Op>) -> Self {
        Self { rx }
    }

    /// Build a fresh approval channel, returning the gate and its sender.
    #[must_use]
    pub fn channel() -> (mpsc::UnboundedSender<Op>, Self) {
        let (tx, rx) = mpsc::unbounded_channel();
        (tx, Self::new(rx))
    }

    /// Await the decision for the approval request identified by `request_id`.
    ///
    /// Ops that do not answer this request are skipped. An
    /// [`Op::Interrupt`] is treated as [`ReviewDecision::Abort`]. Returns
    /// `None` if the channel closes before a matching decision arrives.
    pub async fn wait_for(&mut self, request_id: &str) -> Option<ReviewDecision> {
        while let Some(op) = self.rx.recv().await {
            match op {
                Op::ExecApproval { id, decision } | Op::PatchApproval { id, decision }
                    if id == request_id =>
                {
                    return Some(decision);
                }
                Op::Interrupt => return Some(ReviewDecision::Abort),
                // A decision for a different request, or an unrelated op, is
                // ignored while we wait for the one we asked about.
                _ => {}
            }
        }
        None
    }
}
