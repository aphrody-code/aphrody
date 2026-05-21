// SPDX-License-Identifier: Apache-2.0
//! Shared types serialised to `path.json` and `monorepo-map.json`.
//!
//! Kept dependency-free (only serde) so downstream consumers can depend on
//! just the type layer without pulling in heavy audit/watch deps.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Audit report (path.json) — pattern findings + hardening status.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    #[serde(rename = "Production Ready")]
    ProductionReady,
    Findings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditReport {
    pub audit_name: &'static str,
    pub status: Status,
    pub date: String,
    pub last_updated: String,
    pub generator: &'static str,
    pub scope: Scope,
    pub findings: BTreeMap<String, FindingGroup>,
    pub infrastructure_exceptions: InfraExceptions,
    pub submodules: SubmoduleSection,
    pub conclusion: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scope {
    pub directories_audited: Vec<String>,
    pub excluded: Vec<String>,
    pub host: String,
    pub os: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FindingGroup {
    pub pattern: String,
    pub status: Status,
    pub matches: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InfraExceptions {
    pub note: String,
    pub directories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmoduleSection {
    pub tracked: Vec<Submodule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Submodule {
    pub path: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added: Option<String>,
}

// ---------------------------------------------------------------------------
// Full monorepo map (monorepo-map.json) — exhaustive snapshot.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct MonorepoMap {
    pub generated_at: String,
    pub root: String,
    pub host: String,
    pub os: String,
    /// Detected root-level package managers / task runners / lockfiles.
    pub root_kind: RootKind,
    /// blake3 hash of root config files — for downstream cache invalidation.
    pub content_hash: String,
    pub stats: MapStats,
    pub workspaces: Vec<Workspace>,
    pub submodules: Vec<Submodule>,
}

/// What the repository root looks like (Bun? Turborepo? pnpm? Cargo workspace?).
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct RootKind {
    /// e.g. `["turbo"]`, `["nx"]`, `["turbo", "lerna"]`
    pub task_runners: Vec<String>,
    /// e.g. `["bun", "npm"]`
    pub package_managers: Vec<String>,
    /// e.g. `["bun.lock", "Cargo.lock"]`
    pub lockfiles: Vec<String>,
    pub has_cargo_workspace: bool,
    pub has_bun_workspaces: bool,
    pub has_pnpm_workspaces: bool,
    pub has_turbo: bool,
    pub has_nx: bool,
    pub has_lerna: bool,
    pub has_deno: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MapStats {
    pub total_files: usize,
    pub total_workspaces: usize,
    pub total_submodules: usize,
    pub bytes_scanned: u64,
    pub scan_duration_ms: u128,
    pub languages: BTreeMap<String, LangStat>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct LangStat {
    pub files: usize,
    pub bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workspace {
    pub path: String,
    pub kind: WorkspaceKind,
    /// e.g. `["bun", "turbo", "cargo"]` — runtimes detected at this workspace.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtimes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub file_count: usize,
    pub bytes: u64,
    pub languages: BTreeMap<String, LangStat>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceKind {
    /// `package.json` workspace (Bun/Node).
    Node,
    /// `Cargo.toml` workspace member.
    Rust,
    /// Both `package.json` and `Cargo.toml` (e.g. napi-rs crates).
    Hybrid,
}

/// Summary returned by [`crate::audit::run`] for the CLI to log.
#[derive(Debug)]
pub struct RunResult {
    pub status: Status,
    pub submodules: usize,
    pub workspaces: usize,
    pub total_findings: usize,
    pub duration_ms: u128,
}
