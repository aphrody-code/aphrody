//! mrx-detect — recognise the shape of a JS/Rust monorepo root and its
//! sub-workspaces.
//!
//! What we recognise (no clones, just stat()):
//!   * Bun                — `bun.lock`, `bun.lockb`, `bunfig.toml`
//!   * Turborepo          — `turbo.json`, `turbo.jsonc`
//!   * pnpm               — `pnpm-workspace.yaml`, `pnpm-lock.yaml`
//!   * npm                — `package-lock.json`
//!   * Yarn               — `yarn.lock`
//!   * Lerna              — `lerna.json`
//!   * Nx                 — `nx.json`
//!   * Deno               — `deno.json` / `deno.jsonc` / `deno.lock`
//!   * Cargo workspace    — root `Cargo.toml` with `[workspace]`
//!   * Bun/Node workspaces — root `package.json` with `"workspaces": [...]`

use std::path::Path;

use mrx_core::RootKind;

/// Detect everything at the repository root in a single pass.
pub fn detect_root(root: &Path) -> RootKind {
    let mut k = RootKind::default();

    for lf in &[
        "bun.lock",
        "bun.lockb",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "Cargo.lock",
        "deno.lock",
    ] {
        if root.join(lf).exists() {
            k.lockfiles.push((*lf).to_string());
        }
    }

    // Bun (lock OR config)
    if root.join("bun.lock").exists()
        || root.join("bun.lockb").exists()
        || root.join("bunfig.toml").exists()
    {
        push_unique(&mut k.package_managers, "bun");
    }

    // pnpm
    if root.join("pnpm-lock.yaml").exists() || root.join("pnpm-workspace.yaml").exists() {
        push_unique(&mut k.package_managers, "pnpm");
    }
    k.has_pnpm_workspaces = root.join("pnpm-workspace.yaml").exists();

    // npm
    if root.join("package-lock.json").exists() {
        push_unique(&mut k.package_managers, "npm");
    }

    // yarn
    if root.join("yarn.lock").exists() {
        push_unique(&mut k.package_managers, "yarn");
    }

    // Turbo
    if root.join("turbo.json").exists() || root.join("turbo.jsonc").exists() {
        push_unique(&mut k.task_runners, "turbo");
        k.has_turbo = true;
    }

    // Nx
    if root.join("nx.json").exists() {
        push_unique(&mut k.task_runners, "nx");
        k.has_nx = true;
    }

    // Lerna
    if root.join("lerna.json").exists() {
        push_unique(&mut k.task_runners, "lerna");
        k.has_lerna = true;
    }

    // Deno
    if root.join("deno.json").exists() || root.join("deno.jsonc").exists() {
        k.has_deno = true;
        push_unique(&mut k.package_managers, "deno");
    }

    // Cargo workspace?
    if let Ok(s) = std::fs::read_to_string(root.join("Cargo.toml")) {
        k.has_cargo_workspace = s.contains("[workspace]");
    }

    // Bun/Node workspaces? (package.json with "workspaces")
    if let Ok(s) = std::fs::read_to_string(root.join("package.json"))
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&s)
        && v.get("workspaces").is_some()
    {
        k.has_bun_workspaces = true;
        push_unique(&mut k.package_managers, "bun");
    }

    k
}

/// Per-workspace runtime detection (called for each detected workspace dir).
pub fn detect_workspace_runtimes(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if dir.join("bunfig.toml").exists() || dir.join("bun.lock").exists() {
        out.push("bun".into());
    }
    if dir.join("turbo.json").exists() {
        out.push("turbo".into());
    }
    if dir.join("Cargo.toml").exists() {
        out.push("cargo".into());
    }
    if dir.join("package.json").exists() && !out.iter().any(|r| r == "bun") {
        out.push("node".into());
    }
    if dir.join("deno.json").exists() || dir.join("deno.jsonc").exists() {
        out.push("deno".into());
    }
    out
}

fn push_unique(v: &mut Vec<String>, s: &str) {
    if !v.iter().any(|x| x == s) {
        v.push(s.to_string());
    }
}
