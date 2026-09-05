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
//! no path jailing, and no command refusal unless explicitly opted in. The
//! [`ShellExecTool`] consults [`aphrody_guard`] command-safety as an **opt-in**
//! backstop: only when `APHRODY_GUARD` is enabled does it refuse a
//! provably-destructive command ([`Decision::Forbidden`](aphrody_guard::Decision::Forbidden)
//! — `rm -rf /`, `git push --force`, a fork bomb, …) before spawning. With the
//! guard off (the default), behaviour is unchanged: every command runs, so a
//! fully-autonomous agent is never impeded. `Allow`/`Prompt` commands always run;
//! only the known-destructive class is hard-blocked. Resource guardrails
//! (timeout / output cap) remain configurable via the builder.

mod apply_patch;
mod computer_use;
mod read_file;
mod search_text;
mod shell;

pub use apply_patch::ApplyPatchTool;
pub use computer_use::ComputerUseTool;
pub use read_file::ReadFileTool;
pub use search_text::SearchTextTool;
pub use shell::DEFAULT_MAX_OUTPUT_BYTES;
pub use shell::DEFAULT_TIMEOUT;
pub use shell::OutputSink;
pub use shell::ShellExecConfig;
pub use shell::ShellExecTool;
