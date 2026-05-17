// SPDX-License-Identifier: Apache-2.0
//! Auto-generate `docs/SUMMARY.md` (mdBook table of contents) from the docs/
//! directory tree.
//!
//! Usage:
//!   cargo run -p aphrody-summary
//!   cargo run -p aphrody-summary -- --check    # CI: fail if SUMMARY.md drifts

use std::{
    collections::{BTreeMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

const ROOT_DOCS_TO_MIRROR: &[&str] =
    &["CHANGELOG.md", "CONTRIBUTING.md", "CODE_OF_CONDUCT.md", "SECURITY.md", "BENCHMARKS.md"];
const TOP_LEVEL_ORDER: &[&str] =
    &["README.md", "PLAN.md", "DESIGN.md", "GOOGLE.md", "AWESOME.md", "libc.md", "bun-rs.md"];
const ROOT_MIRROR_DIRNAME: &str = "_root";
const ROOT_MIRROR_GROUP_TITLE: &str = "Project-Wide";

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

fn get_repo_root() -> Result<PathBuf> {
    // Assuming this runs from workspace root or crate root
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

fn build(repo_root: &Path, docs_root: &Path) -> Result<String> {
    let mirror_dir = docs_root.join(ROOT_MIRROR_DIRNAME);
    mirror_root_docs(repo_root, &mirror_dir)?;

    let entries = scan(docs_root, docs_root)?;
    let top_level = render_top_level(&entries);

    let mut groups = Vec::new();
    for entry in &entries {
        if let Entry::Dir { name, entries: inner } = entry {
            groups.push(render_group(&name, &inner, 0));
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

    let mut output = vec![banner, top_level, "".to_string()];
    output.extend(groups);
    output.push("".to_string());

    Ok(output.join("\n"))
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let check_mode = args.contains(&"--check".to_string());

    let repo_root = get_repo_root()?;
    let docs_root = repo_root.join("docs");
    let summary_path = docs_root.join("SUMMARY.md");

    let generated = build(&repo_root, &docs_root)?;

    if check_mode {
        let existing = fs::read_to_string(&summary_path).unwrap_or_default();
        if existing.trim() != generated.trim() {
            eprintln!("✗ docs/SUMMARY.md is out of date. Run: cargo run -p aphrody-summary");
            std::process::exit(1);
        }
        println!("✓ docs/SUMMARY.md is up to date.");
        return Ok(());
    }

    fs::write(&summary_path, &generated)?;
    let line_count = generated.lines().count();
    println!("✓ Generated docs/SUMMARY.md ({} lines)", line_count);

    Ok(())
}
