// SPDX-License-Identifier: Apache-2.0
//! Per-tool permission descriptors.
//!
//! Every builtin tool advertises a [`PermissionDescriptor`] alongside its
//! [`ToolDescriptor`](crate::ToolDescriptor). The descriptor is consumed by
//! the `aphrody-permissions` crate (allow / ask / deny gate, glob matchers,
//! `settings.json` compatibility) before [`Tool::invoke`](crate::Tool::invoke)
//! is reached.
//!
//! This module owns the *shape* of that handshake: enum variants, free-form
//! tags ("fs:read", "fs:write", "net:fetch") and the rule of thumb for which
//! tools should default to which gate. The actual matching engine lives in
//! `aphrody-permissions`.

use serde::{Deserialize, Serialize};

/// Coarse permission grade applied to a tool by default.
///
/// The default is conservative: anything that mutates the filesystem or
/// emits a network request lands on [`Permission::Ask`]; pure reads stay on
/// [`Permission::Allow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Always run without confirmation. Read-only, side-effect-free.
    Allow,
    /// Prompt the user (via `aphrody-permissions`) before each invocation.
    Ask,
    /// Refuse to run unconditionally.
    Deny,
}

impl Permission {
    /// Returns the lower-case wire token used by `settings.json` entries
    /// (`{"permission": "allow"}` / `"ask"` / `"deny"`).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Ask => "ask",
            Self::Deny => "deny",
        }
    }
}

/// Permission profile for a single tool.
///
/// Carries the tool name plus an optional list of free-form `scopes`
/// describing what the tool touches (`fs:read`, `fs:write`, `net:fetch`,
/// `memory:write`, …). `aphrody-permissions` joins these scopes against the
/// active `settings.json` allow/ask/deny lists, falling back to
/// [`Self::default_permission`] when no rule matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionDescriptor {
    /// Tool name (must match [`ToolDescriptor::name`](crate::ToolDescriptor::name)).
    pub tool_name: String,
    /// Free-form scope tags. Conventionally `prefix:subject` (e.g.
    /// `"fs:read"`, `"net:fetch"`).
    pub scopes: Vec<String>,
    /// Permission level applied when no `settings.json` rule matches.
    pub default_permission: Permission,
}

impl PermissionDescriptor {
    /// Construct a permissive descriptor (`Allow`) with no scopes. Used by
    /// the default [`Tool::permission`](crate::Tool::permission) impl.
    #[must_use]
    pub fn allow(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            scopes: Vec::new(),
            default_permission: Permission::Allow,
        }
    }

    /// Construct an `Ask` descriptor for a write-class or network tool.
    #[must_use]
    pub fn ask(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            scopes: Vec::new(),
            default_permission: Permission::Ask,
        }
    }

    /// Append a scope tag and return `self` (builder style).
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Replace the scopes vector with the supplied iterator.
    #[must_use]
    pub fn with_scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_as_str_round_trip() {
        for p in [Permission::Allow, Permission::Ask, Permission::Deny] {
            let s = p.as_str();
            let v = serde_json::to_string(&p).expect("serialize");
            assert!(v.contains(s));
        }
    }

    #[test]
    fn allow_descriptor_is_permissive() {
        let d = PermissionDescriptor::allow("read_file");
        assert_eq!(d.default_permission, Permission::Allow);
        assert!(d.scopes.is_empty());
    }

    #[test]
    fn ask_with_scopes_chains() {
        let d = PermissionDescriptor::ask("write_file")
            .with_scope("fs:write")
            .with_scope("fs:create");
        assert_eq!(d.default_permission, Permission::Ask);
        assert_eq!(d.scopes, vec!["fs:write", "fs:create"]);
    }

    #[test]
    fn with_scopes_replaces_existing() {
        let d = PermissionDescriptor::ask("net")
            .with_scope("net:fetch")
            .with_scopes(["net:get", "net:post"]);
        assert_eq!(d.scopes, vec!["net:get", "net:post"]);
    }
}
