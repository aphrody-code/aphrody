// SPDX-License-Identifier: Apache-2.0
//! Concrete [`ToolExecutor`](aphrody_toolcall::ToolExecutor)s for the aphrody
//! agent engine.
//!
//! These are the real, production capabilities the engine's
//! [`ToolRegistry`](aphrody_toolcall::ToolRegistry) exposes to the model:
//!
//! - [`ShellExecTool`] — runs a command (argv form, no shell parsing) with a
//!   configurable timeout and output cap, capturing the combined stdout/stderr
//!   stream and exit code. Optionally streams output chunks to a sink so the
//!   engine can emit
//!   [`EventMsg::ExecCommandOutputDelta`](aphrody_agent_proto::EventMsg::ExecCommandOutputDelta)
//!   events live.
//! - [`ApplyPatchTool`] — parses an `*** Begin Patch ... *** End Patch`
//!   document via [`aphrody_patch::parse_patch`] and applies it through a
//!   pluggable [`PatchFileSystem`](aphrody_patch::PatchFileSystem) (real disk by
//!   default, injectable for tests).
//!
//! # Safety posture
//!
//! Per the aphrody autonomy contract, both tools are **permissive by default**:
//! no command allow/deny-listing, no path jailing. Guardrails, when wanted, are
//! opt-in via the builder configuration (timeout / output cap) — the tools never
//! refuse work on their own and do not depend on `aphrody-guard`.

mod apply_patch;
mod shell;

pub use apply_patch::ApplyPatchTool;
pub use shell::DEFAULT_MAX_OUTPUT_BYTES;
pub use shell::DEFAULT_TIMEOUT;
pub use shell::OutputSink;
pub use shell::ShellExecConfig;
pub use shell::ShellExecTool;
