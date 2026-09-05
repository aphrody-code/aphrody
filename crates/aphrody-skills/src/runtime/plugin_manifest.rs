// SPDX-License-Identifier: Apache-2.0
//
// Plugin-manifest skill discovery — port of
// `vercel-labs/skills::plugin-manifest.ts::getPluginSkillPaths`.
//
// If a `.claude-plugin/marketplace.json` (multi-plugin catalog) or a
// `.claude-plugin/plugin.json` (single plugin) exists under a base path, this
// resolves the `skills/` directory of each declared plugin RELATIVE to the
// manifest — never a hardcoded location. The plugin install path is therefore
// irrelevant (dev tree, project-local `.claude/plugins/`, or a global install
// all work identically), which matters because the dev plugin path is
// temporary. Path-traversal is guarded: every resolved directory must stay
// contained within the base, and declared skill paths must be `./`-relative
// per the Claude Code plugin convention.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MarketplaceManifest {
    metadata: Option<MarketplaceMeta>,
    #[serde(default)]
    plugins: Vec<PluginEntry>,
}

#[derive(Debug, Deserialize)]
struct MarketplaceMeta {
    #[serde(rename = "pluginRoot")]
    plugin_root: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PluginEntry {
    /// A local string path, or a remote `{ source, repo }` object (skipped).
    source: Option<serde_json::Value>,
    #[serde(default)]
    skills: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PluginManifest {
    #[serde(default)]
    skills: Vec<String>,
}

/// Claude Code convention: declared paths must start with `./`.
fn is_valid_relative(p: &str) -> bool {
    p.starts_with("./")
}

/// Lexical normalisation (no filesystem access): collapse `.` and `..`.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            },
            Component::CurDir => {},
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Path-traversal guard: `target` must equal or sit under `base`.
fn contained_in(target: &Path, base: &Path) -> bool {
    let nb = normalize(base);
    let nt = normalize(target);
    nt == nb || nt.starts_with(&nb)
}

fn add_plugin_skill_paths(
    plugin_base: &Path,
    skills: &[String],
    base: &Path,
    out: &mut BTreeSet<PathBuf>,
) {
    if !contained_in(plugin_base, base) {
        return;
    }
    for skill in skills {
        if !is_valid_relative(skill) {
            continue;
        }
        if let Some(parent) = plugin_base.join(skill).parent() {
            let dir = parent.to_path_buf();
            if contained_in(&dir, base) {
                out.insert(normalize(&dir));
            }
        }
    }
    // Always add the conventional `skills/` directory for discovery.
    out.insert(normalize(&plugin_base.join("skills")));
}

/// Extract skill-search directories declared by plugin manifests under `base`.
///
/// Handles `marketplace.json` (multi-plugin) and `plugin.json` (single
/// plugin). Only local string sources are resolved; remote sources are
/// skipped. Returns directories that should be scanned for child
/// `<name>/SKILL.md` entries.
#[must_use]
pub fn plugin_skill_dirs(base: &Path) -> Vec<PathBuf> {
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();

    // marketplace.json (multi-plugin catalog).
    let marketplace = base.join(".claude-plugin").join("marketplace.json");
    if let Ok(raw) = std::fs::read_to_string(&marketplace) {
        if let Ok(m) = serde_json::from_str::<MarketplaceManifest>(&raw) {
            let plugin_root = m.metadata.and_then(|x| x.plugin_root);
            let valid_root = plugin_root.as_deref().is_none_or(is_valid_relative);
            if valid_root {
                for plugin in &m.plugins {
                    let source = match &plugin.source {
                        None => None,
                        Some(serde_json::Value::String(s)) => Some(s.clone()),
                        // Remote `{ source, repo }` object — not a local path.
                        Some(_) => continue,
                    };
                    if let Some(s) = &source {
                        if !is_valid_relative(s) {
                            continue;
                        }
                    }
                    let mut plugin_base = base.to_path_buf();
                    if let Some(root) = &plugin_root {
                        plugin_base = plugin_base.join(root);
                    }
                    if let Some(s) = &source {
                        plugin_base = plugin_base.join(s);
                    }
                    add_plugin_skill_paths(&plugin_base, &plugin.skills, base, &mut dirs);
                }
            }
        }
    }

    // plugin.json (single plugin rooted at base).
    let plugin_json = base.join(".claude-plugin").join("plugin.json");
    if let Ok(raw) = std::fs::read_to_string(&plugin_json) {
        if let Ok(m) = serde_json::from_str::<PluginManifest>(&raw) {
            add_plugin_skill_paths(base, &m.skills, base, &mut dirs);
        }
    }

    dirs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn marketplace_resolves_plugin_skills_dir() {
        let td = tempdir().unwrap();
        let base = td.path();
        let cp = base.join(".claude-plugin");
        fs::create_dir_all(&cp).unwrap();
        fs::write(
            cp.join("marketplace.json"),
            r#"{"plugins":[{"name":"demo","source":"./demo"}]}"#,
        )
        .unwrap();

        let dirs = plugin_skill_dirs(base);
        assert!(
            dirs.contains(&normalize(&base.join("demo").join("skills"))),
            "{dirs:?}"
        );
    }

    #[test]
    fn plugin_json_adds_conventional_skills_dir() {
        let td = tempdir().unwrap();
        let base = td.path();
        let cp = base.join(".claude-plugin");
        fs::create_dir_all(&cp).unwrap();
        fs::write(cp.join("plugin.json"), r#"{"name":"x"}"#).unwrap();

        let dirs = plugin_skill_dirs(base);
        assert!(dirs.contains(&normalize(&base.join("skills"))), "{dirs:?}");
    }

    #[test]
    fn explicit_skill_paths_add_parent_dir() {
        let td = tempdir().unwrap();
        let base = td.path();
        let cp = base.join(".claude-plugin");
        fs::create_dir_all(&cp).unwrap();
        fs::write(
            cp.join("plugin.json"),
            r#"{"skills":["./bundled/my-skill/SKILL.md"]}"#,
        )
        .unwrap();

        let dirs = plugin_skill_dirs(base);
        assert!(
            dirs.contains(&normalize(&base.join("bundled").join("my-skill"))),
            "{dirs:?}"
        );
    }

    #[test]
    fn traversal_outside_base_is_rejected() {
        let td = tempdir().unwrap();
        let base = td.path();
        let cp = base.join(".claude-plugin");
        fs::create_dir_all(&cp).unwrap();
        // `../escape` would resolve outside base → must be dropped.
        fs::write(
            cp.join("plugin.json"),
            r#"{"skills":["./../escape/SKILL.md"]}"#,
        )
        .unwrap();

        let dirs = plugin_skill_dirs(base);
        for d in &dirs {
            assert!(contained_in(d, base), "leaked outside base: {d:?}");
        }
    }

    #[test]
    fn remote_object_source_is_skipped() {
        let td = tempdir().unwrap();
        let base = td.path();
        let cp = base.join(".claude-plugin");
        fs::create_dir_all(&cp).unwrap();
        fs::write(
            cp.join("marketplace.json"),
            r#"{"plugins":[{"name":"r","source":{"source":"github","repo":"a/b"}}]}"#,
        )
        .unwrap();

        // Remote source contributes no local dir.
        assert!(plugin_skill_dirs(base).is_empty());
    }

    #[test]
    fn no_manifest_yields_nothing() {
        let td = tempdir().unwrap();
        assert!(plugin_skill_dirs(td.path()).is_empty());
    }
}
