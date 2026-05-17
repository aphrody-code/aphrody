// SPDX-License-Identifier: Apache-2.0
//! Merge logic for layered configuration.
//!
//! Precedence (highest first):
//!
//! 1. `~/.aphrody/terminal.json` (the base [`crate::TerminalConfig`] argument).
//! 2. `~/.claude.json`           ([`crate::shims::ClaudeShim`]).
//! 3. `./.mcp.json`              ([`crate::shims::McpShim`]).
//! 4. `~/.claude/settings.json`  ([`crate::shims::SettingsShim`]).
//! 5. Built-in [`crate::TerminalConfig::default`].
//!
//! Higher-precedence sources win on scalar fields.  For map-like fields
//! ([`crate::TerminalConfig::keymap`]) the merge is a *union*: the base map's
//! entries are preserved and the shim entries fill in only the keys the base
//! does not already define.

use crate::{TerminalConfig, shims::Shim};

/// Apply each shim to `base` in **reverse** precedence order.
///
/// Caller must pass the shims in increasing precedence (lowest first):
/// `[Settings, Mcp, Claude]`.  The base config always wins on conflict.
///
/// Returns the merged [`TerminalConfig`].
#[must_use]
pub fn merge(mut base: TerminalConfig, shims: Vec<Shim>) -> TerminalConfig {
    // Apply low-precedence shims first so higher-precedence ones overwrite them.
    for shim in shims {
        match shim {
            Shim::Settings(s) => apply_settings(&mut base, &s),
            Shim::Mcp(m) => apply_mcp(&mut base, &m),
            Shim::Claude(c) => apply_claude(&mut base, &c),
        }
    }
    base
}

fn apply_settings(base: &mut TerminalConfig, shim: &crate::shims::SettingsShim) {
    // theme: only fill if base.theme is at its default (empty string).
    if base.theme.name.is_empty() {
        if let Some(t) = &shim.theme {
            base.theme.name = t.clone();
        }
    }
    // permissions: append shim allow/deny into TerminalConfig.permissions only
    // when the base list is empty (lower precedence).
    if base.permissions.allow.is_empty() {
        base.permissions.allow.extend(shim.permissions.allow.iter().cloned());
    }
    if base.permissions.deny.is_empty() {
        base.permissions.deny.extend(shim.permissions.deny.iter().cloned());
    }
}

fn apply_mcp(base: &mut TerminalConfig, shim: &crate::shims::McpShim) {
    // mcp_servers: union, base wins on key conflict.
    for (name, entry) in &shim.servers {
        base.mcp_servers
            .entry(name.clone())
            .or_insert_with(|| entry.clone());
    }
}

fn apply_claude(base: &mut TerminalConfig, shim: &crate::shims::ClaudeShim) {
    // theme: only fill if base.theme is still at its default.
    if base.theme.name.is_empty() {
        if let Some(t) = &shim.theme {
            base.theme.name = t.clone();
        }
    }
    // keymap: union, base wins on key conflict.
    for (k, v) in &shim.keymap {
        base.keymap.entry(k.clone()).or_insert_with(|| v.clone());
    }
}
