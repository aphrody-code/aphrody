// SPDX-License-Identifier: Apache-2.0
//! # aphrody-skills-forge
//!
//! Programmatic creation, registry loading, and linting of the aphrody
//! `SKILL.md` catalog.
//!
//! This crate complements [`aphrody-skills-runtime`](../aphrody_skills_runtime)
//! (which owns the discovery walker / install manifest / upstream-source
//! registry) by providing the **authoring** surface:
//!
//! - [`registry::load_registry`] — recursively collect every `SKILL.md` under a
//!   tree and return strongly-typed [`skill::Skill`] records.
//! - [`scaffold::scaffold_skill`] — generate a fresh
//!   `<root>/.claude/skills/<name>/SKILL.md` with valid frontmatter and a
//!   three-section body, refusing to overwrite an existing directory.
//! - [`lint::lint_skill`] — apply the canonical SKILL.md style rules from
//!   `docs/cargo/SKILLS.md` and return a list of [`lint::LintWarning`]s
//!   tagged with severity + line number.
//!
//! The DTOs are kebab-case validated, serde-serialisable, and `schemars`-aware
//! so the same types can drive a JSON-schema export for downstream tooling.
//!
//! All public APIs return [`ForgeError`] on failure; no panics except on
//! programmer error inside lazily-compiled regex literals (guarded by tests).

pub mod lint;
pub mod registry;
pub mod scaffold;
pub mod skill;

pub use lint::{LintSeverity, LintWarning, lint_skill};
pub use registry::{ForgeError, load_registry};
pub use scaffold::scaffold_skill;
pub use skill::{Skill, SkillFrontmatter, SkillRuntime};
