// SPDX-License-Identifier: Apache-2.0
//! Auto-generate `docs/SUMMARY.md` (mdBook table of contents) from the
//! `docs/` directory tree.
//!
//! This crate ships both as a library and as a binary:
//!
//! - **Binary** (`aphrody-summary`): walks the repo's `docs/` tree, mirrors
//!   the top-level `README.md` / `CHANGELOG.md` / etc. into `docs/_root/`,
//!   and writes the assembled `SUMMARY.md` to disk. CI uses
//!   `cargo run -p aphrody-summary -- --check` to assert the file is up to
//!   date with the tree.
//! - **Library**: exposes [`generate`] for in-process consumers (the
//!   `aphrody-terminal-llm` "docs preview" pane invokes this directly to
//!   render the markdown TOC inline, instead of shelling out to
//!   `cargo run`).
//!
//! ## Library usage
//!
//! ```no_run
//! // Returns the SUMMARY.md markdown body as a `String`. Walks the docs
//! // tree starting from the workspace root resolved via `CARGO_MANIFEST_DIR`
//! // (falling back to the current directory).
//! let markdown = aphrody_summary::generate().expect("docs/ readable");
//! assert!(markdown.contains("# Summary"));
//! ```
//!
//! No filesystem writes occur on the [`generate`] path beyond the side
//! effect of mirroring root docs into `docs/_root/` (matching the binary's
//! behavior — kept in sync so the markdown links resolve identically
//! whether produced by the binary or the library).

#![deny(unsafe_code)]

use std::{
    collections::{BTreeMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// Root-level files that get mirrored under `docs/_root/` so they have a
/// stable in-tree URL for the mdBook summary.
pub const ROOT_DOCS_TO_MIRROR: &[&str] =
    &["CHANGELOG.md", "CONTRIBUTING.md", "CODE_OF_CONDUCT.md", "SECURITY.md", "BENCHMARKS.md"];

/// Canonical ordering for top-level `docs/*.md` entries.
pub const TOP_LEVEL_ORDER: &[&str] =
    &["README.md", "PLAN.md", "DESIGN.md", "GOOGLE.md", "AWESOME.md", "libc.md", "bun-rs.md"];

/// Directory name (inside `docs/`) where root docs are mirrored.
pub const ROOT_MIRROR_DIRNAME: &str = "_root";

/// Group title rendered for the mirrored root docs section.
pub const ROOT_MIRROR_GROUP_TITLE: &str = "Project-Wide";

#[derive(Debug, Clone)]
enum Entry {
    File { name: String, rel_path: String },
    Dir { name: String, entries: Vec<Entry> },
}

fn titleize(name: &str) -> String {
    let base = name.strip_suffix(".md").unwrap_or(name);
    let parts: Vec<&str> = base.split(|c| c == '-' || c == '_').collect();
    let mut title = Vec::new();
    for part in parts {
        if part.is_empty() {
            title.push(String::new());
        } else {
            let mut c = part.chars();
            if let Some(first) = c.next() {
                let capitalized = format!("{}{}", first.to_uppercase(), c.as_str());
                title.push(capitalized);
            }
        }
    }
    title.join("-")
}

/// Resolve the workspace root for this crate.
///
/// Uses `CARGO_MANIFEST_DIR` when set (i.e. when invoked via cargo), and
/// strips the trailing `crates/aphrody-summary` segment. Falls back to the
/// current working directory.
pub fn repo_root() -> Result<PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| env::current_dir().unwrap().display().to_string());
    let mut path = PathBuf::from(manifest_dir);
    if path.ends_with("crates\\aphrody-summary") || path.ends_with("crates/aphrody-summary") {
        path.pop();
        path.pop();
    }
    Ok(path)
}

fn mirror_root_docs(repo_root: &Path, mirror_dir: &Path) -> Result<()> {
    fs::create_dir_all(mirror_dir)
        .with_context(|| format!("Failed to create {}", mirror_dir.display()))?;
    for name in ROOT_DOCS_TO_MIRROR {
        let src = repo_root.join(name);
        let dst = mirror_dir.join(name);
        if src.exists() {
            fs::copy(&src, &dst).with_context(|| {
                format!("Failed to copy {} to {}", src.display(), dst.display())
            })?;
        }
    }
    Ok(())
}

fn scan(dir: &Path, docs_root: &Path) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut names = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        names.push((name, entry.path()));
    }
    names.sort_by(|a, b| a.0.cmp(&b.0));

    let skip_names: HashSet<&str> = ["SUMMARY.md", ".DS_Store"].into_iter().collect();
    let skip_dir_prefixes = ['_', '.'];

    for (name, full_path) in names {
        if skip_names.contains(name.as_str()) {
            continue;
        }
        if skip_dir_prefixes.iter().any(|&p| name.starts_with(p)) && name != ROOT_MIRROR_DIRNAME {
            continue;
        }

        let st = match fs::metadata(&full_path) {
            Ok(st) => st,
            Err(_) => continue, // skip broken symlinks
        };

        if st.is_dir() {
            let inner = scan(&full_path, docs_root)?;
            if !inner.is_empty() {
                entries.push(Entry::Dir { name, entries: inner });
            }
        } else if name.ends_with(".md") {
            let rel_path = full_path.strip_prefix(docs_root)?.to_string_lossy().replace('\\', "/");
            entries.push(Entry::File { name, rel_path });
        }
    }

    Ok(entries)
}

fn render_top_level(entries: &[Entry]) -> String {
    let mut files = BTreeMap::new();
    for entry in entries {
        if let Entry::File { name, rel_path } = entry {
            files.insert(name.clone(), rel_path.clone());
        }
    }

    let mut ordered = Vec::new();
    for name in TOP_LEVEL_ORDER {
        if let Some(rel_path) = files.remove(*name) {
            let title = if *name == "README.md" { "Accueil".to_string() } else { titleize(name) };
            ordered.push(format!("- [{}]({})", title, rel_path));
        }
    }

    for (name, rel_path) in files {
        let title = titleize(&name);
        ordered.push(format!("- [{}]({})", title, rel_path));
    }

    ordered.join("\n")
}

fn render_group(name: &str, entries: &[Entry], depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let mut lines = Vec::new();

    let readme = entries.iter().find(|e| {
        if let Entry::File { name, .. } = e { name.to_lowercase() == "readme.md" } else { false }
    });

    let group_name = if name == ROOT_MIRROR_DIRNAME {
        ROOT_MIRROR_GROUP_TITLE.to_string()
    } else {
        titleize(name)
    };

    if let Some(Entry::File { rel_path, .. }) = readme {
        lines.push(format!("{}- [{}]({})", indent, group_name, rel_path));
    } else {
        lines.push(format!("{}- [{}]()", indent, group_name));
    }

    let mut files = Vec::new();
    let mut subdirs = Vec::new();
    for entry in entries {
        match entry {
            Entry::File { name, .. } if name.to_lowercase() != "readme.md" => files.push(entry),
            Entry::Dir { .. } => subdirs.push(entry),
            _ => {},
        }
    }

    for f in files {
        if let Entry::File { name, rel_path } = f {
            let title = titleize(name);
            lines.push(format!("{}  - [{}]({})", indent, title, rel_path));
        }
    }

    for sd in subdirs {
        if let Entry::Dir { name, entries } = sd {
            lines.push(render_group(name, entries, depth + 1));
        }
    }

    lines.join("\n")
}

/// Build the SUMMARY.md markdown body from a specific repo + docs root.
///
/// This is the lower-level entry point. Most consumers want [`generate`]
/// which resolves the repo root automatically.
pub fn build(repo_root: &Path, docs_root: &Path) -> Result<String> {
    let mirror_dir = docs_root.join(ROOT_MIRROR_DIRNAME);
    mirror_root_docs(repo_root, &mirror_dir)?;

    let entries = scan(docs_root, docs_root)?;
    let top_level = render_top_level(&entries);

    let mut groups = Vec::new();
    for entry in &entries {
        if let Entry::Dir { name, entries: inner } = entry {
            groups.push(render_group(name, inner, 0));
        }
    }

    let banner = [
        "<!--",
        "  AUTO-GENERATED by aphrody-summary — DO NOT EDIT BY HAND.",
        "  Re-run with:  cargo run -p aphrody-summary",
        "-->",
        "",
        "# Summary",
        "",
    ]
    .join("\n");

    let mut output = vec![banner, top_level, String::new()];
    output.extend(groups);
    output.push(String::new());

    Ok(output.join("\n"))
}

/// Generate the full SUMMARY.md markdown for this workspace and return it
/// as a `String`. No file is written.
///
/// Used by the `aphrody-terminal-llm` "docs preview" pane to render the
/// table of contents inline without spawning a child process.
pub fn generate() -> Result<String> {
    let repo_root = repo_root()?;
    let docs_root = repo_root.join("docs");
    build(&repo_root, &docs_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titleize_basic() {
        assert_eq!(titleize("foo-bar.md"), "Foo-Bar");
        assert_eq!(titleize("hello_world.md"), "Hello-World");
        assert_eq!(titleize("README.md"), "README");
    }

    #[test]
    fn build_emits_banner_and_summary_heading() {
        // Build against a synthetic docs tree under tempdir so this test is
        // hermetic and doesn't depend on the live repo layout.
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo_root = tmp.path();
        let docs_root = repo_root.join("docs");
        fs::create_dir_all(&docs_root).unwrap();
        fs::write(docs_root.join("README.md"), "# hi").unwrap();
        fs::write(docs_root.join("PLAN.md"), "# plan").unwrap();

        let md = build(repo_root, &docs_root).expect("build ok");
        assert!(md.contains("# Summary"), "missing # Summary heading: {md}");
        assert!(md.contains("AUTO-GENERATED by aphrody-summary"), "missing banner: {md}");
        assert!(md.contains("[Accueil]"), "README.md should render as Accueil: {md}");
    }
}
