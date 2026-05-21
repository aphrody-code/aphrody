// SPDX-License-Identifier: Apache-2.0
//! `aphrody-skills` — unified skills infrastructure.
//!
//! Merges three formerly standalone crates:
//!
//! - [`runtime`] — SKILL.md aggregator: frontmatter parser, discovery,
//!   manifest builder, install, validate, lockfile, sources (local + git +
//!   registry). Formerly `aphrody-skills-runtime`.
//! - [`hooks`] — runtime hook dispatcher: event types (PreToolUse,
//!   PostToolUse, Stop, UserPromptSubmit), glob-filtered rules, async
//!   subprocess execution with timeout. Formerly `aphrody-hooks`.
//! - [`permissions`] — tool-level permission engine: allow/ask/deny rules
//!   with glob + regex matchers, ordered precedence, JSON-serialisable
//!   policy files. Formerly `aphrody-permissions`.

#![forbid(unsafe_code)]

pub mod runtime;
pub mod hooks;
pub mod permissions;
