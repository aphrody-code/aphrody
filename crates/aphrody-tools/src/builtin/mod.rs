// SPDX-License-Identifier: Apache-2.0
//! Builtin tools — Rust ports of the gemini-cli `packages/core/src/tools`
//! TypeScript surface.
//!
//! ## Ported tools
//!
//! | Module                | Origin (`gemini-cli`)         | Surface                          |
//! |-----------------------|-------------------------------|----------------------------------|
//! | [`read_file`]         | `read-file.ts`                | `read_file({file_path, …})`      |
//! | [`write_file`]        | `write-file.ts`               | `write_file({file_path, content})` |
//! | [`edit`]              | `edit.ts`                     | `edit({file_path, old_string, new_string, replace_all?})` |
//! | [`glob_tool`]         | `glob.ts`                     | `glob({pattern, path?})`         |
//! | [`grep`]              | `grep.ts` + `ripGrep.ts`      | `grep({pattern, path?, glob?})`  |
//! | [`ls`]                | `ls.ts`                       | `ls({path, recursive?, depth?})` |
//! | [`web_fetch`]         | `web-fetch.ts`                | `web_fetch({url, method?, …})`   |
//! | [`memory_tool`]       | `memoryTool.ts`               | `memory({op, …})` (in-memory store) |
//!
//! ## Not ported here
//!
//! - `shell.ts` / `shellBackgroundTools.ts` → delegated to the dedicated
//!   `aphrody-shell` crate (process spawning + cancellation lives there).
//! - `mcp-tool.ts` / `mcp-client*.ts` → extended via the existing
//!   `gemini-runtime`'s `McpProxyTool`. This crate exposes the [`Tool`]
//!   trait the proxy implements but doesn't re-implement the MCP client
//!   transport.
//! - `read-many-files.ts` → trivial composition of [`read_file`] + the
//!   registry's iteration, intentionally not duplicated.
//! - `web-search.ts` → provider-specific (Gemini grounding); ported in
//!   `gemini-runtime` under the search-grounding feature flag.
//!
//! ## Registration
//!
//! Use [`register_builtins`] to bulk-install every builtin into a
//! [`ToolRegistry`](crate::ToolRegistry). The function never panics: it
//! returns the first registry error encountered.

pub mod edit;
pub mod glob_tool;
pub mod grep;
pub mod ls;
pub mod memory_tool;
pub mod read_file;
pub mod scraping;
pub mod web_fetch;
pub mod write_file;

use crate::{ToolRegistry, ToolsResult};

/// Bulk-register every builtin descriptor into `registry`.
///
/// # Errors
///
/// Returns the first registry error encountered (duplicate name, invalid
/// schema, …). The registry keeps every successful insertion that
/// preceded the failure.
pub fn register_builtins(registry: &mut ToolRegistry) -> ToolsResult<()> {
    registry.register(read_file::descriptor())?;
    registry.register(write_file::descriptor())?;
    registry.register(edit::descriptor())?;
    registry.register(glob_tool::descriptor())?;
    registry.register(grep::descriptor())?;
    registry.register(ls::descriptor())?;
    registry.register(web_fetch::descriptor())?;
    registry.register(memory_tool::descriptor())?;
    Ok(())
}

/// Names of every builtin, sorted alphabetically. Useful for matching
/// against settings.json allow-lists.
#[must_use]
pub fn builtin_names() -> Vec<&'static str> {
    let mut v = vec![
        edit::NAME,
        glob_tool::NAME,
        grep::NAME,
        ls::NAME,
        memory_tool::NAME,
        read_file::NAME,
        web_fetch::NAME,
        write_file::NAME,
    ];
    v.sort_unstable();
    v
}
