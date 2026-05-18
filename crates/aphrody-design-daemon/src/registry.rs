// SPDX-License-Identifier: Apache-2.0
//! Design-systems registry — loads `design-systems/<id>/` directories into an
//! in-memory catalogue.
//!
//! ## Source-of-truth format (what open-design ships today)
//!
//! Each design system lives under `design-systems/<id>/` with at least one
//! `DESIGN.md` whose first H1 is the title and an optional `> Category: …`
//! blockquote line beneath it. Sibling files `tokens.css` and
//! `components.html` are picked up when present. This loader is a faithful
//! Rust transcription of `apps/daemon/src/design-systems.ts` —
//! specifically `listDesignSystems()` / `readDesignSystem()` /
//! `readDesignSystemAssets()` — so the same 149-brand corpus can be served
//! from the aphrody daemon without any data conversion.
//!
//! ## Forward-compatible JSON manifest sidecar
//!
//! When a directory contains `manifest.json` we parse it as
//! [`DesignSystemManifest`] and use those fields verbatim (id, title,
//! category, palette swatches). This keeps the door open for fully-typed
//! systems while preserving zero-config compatibility with the existing
//! Markdown-driven catalogue.
//!
//! ## Performance
//!
//! Loading is a single synchronous scan: `read_dir` → per-entry
//! `read_to_string` of `DESIGN.md` (≈8 KB median, ≤64 KB tail).
//! 149 entries × 8 KB = ~1.2 MB cold read on first boot, fully amortised
//! into the OS file cache thereafter. No async needed.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// The product surface a design system targets. Mirrors open-design's
/// `'web' | 'image' | 'video' | 'audio'` discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DesignSystemSurface {
    Web,
    Image,
    Video,
    Audio,
}

impl Default for DesignSystemSurface {
    fn default() -> Self {
        Self::Web
    }
}

/// In-memory representation of one design system. Matches the
/// `DesignSystemSummary` shape from the upstream TS daemon so the wire
/// format is stable across the port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSystemRecord {
    /// Directory name, used as the stable lookup key.
    pub id: String,
    /// Title — from the H1 of `DESIGN.md`, or `manifest.json:title`, or
    /// (fallback) the directory name.
    pub title: String,
    /// Category bucket — e.g. `"E-Commerce & Retail"`. Defaults to
    /// `"Uncategorized"` when no `> Category: …` line is present.
    pub category: String,
    /// Short prose summary — first paragraph beneath the H1 with the
    /// `> Category: …` line scrubbed.
    pub summary: String,
    /// Hex swatches extracted from `DESIGN.md` (e.g. `#ff385c`). Capped at
    /// 8 entries to match open-design's hero-color limit.
    pub swatches: Vec<String>,
    /// Surface family this system targets.
    pub surface: DesignSystemSurface,
    /// `tokens.css` contents if present. Held in memory because typical
    /// brand stylesheets are ≤16 KB and call sites need them on the hot
    /// prompt-assembly path.
    pub tokens_css: Option<String>,
    /// `components.html` contents if present. Worked-example fixture the
    /// system prompt can inline so the model has a concrete reference.
    pub fixture_html: Option<String>,
    /// Filesystem directory the record was loaded from. Useful for
    /// downstream tools that want to re-read assets or watch for changes.
    pub root: PathBuf,
}

/// Optional `manifest.json` sidecar shape. Every field is optional — the
/// loader falls back to `DESIGN.md`-driven values for anything missing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DesignSystemManifest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub surface: Option<DesignSystemSurface>,
    #[serde(alias = "palette", alias = "swatches")]
    pub palette: Vec<String>,
}

/// In-process catalogue of every design system on disk. Use [`load_from_dir`]
/// to populate, then [`list`] / [`get`] / [`count`] to query.
///
/// [`load_from_dir`]: Self::load_from_dir
/// [`list`]: Self::list
/// [`get`]: Self::get
/// [`count`]: Self::count
#[derive(Debug, Default, Clone)]
pub struct DesignSystemRegistry {
    by_id: BTreeMap<String, DesignSystemRecord>,
}

impl DesignSystemRegistry {
    /// Build an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan `dir` and load every immediate subdirectory that contains a
    /// `DESIGN.md` or a `manifest.json`. Directories without either are
    /// silently skipped, matching the upstream behaviour.
    ///
    /// # Errors
    ///
    /// Returns an error if `dir` cannot be read. Individual entry parse
    /// failures are logged via `tracing::warn!` and skipped — one broken
    /// manifest never blocks the rest of the catalogue.
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let mut out = Self::new();
        let entries = fs::read_dir(dir)
            .with_context(|| format!("read_dir {}", dir.display()))?;
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!(?err, "read_dir entry error, skipping");
                    continue;
                }
            };
            let entry_path = entry.path();
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(err) => {
                    tracing::warn!(?err, path = %entry_path.display(), "file_type error, skipping");
                    continue;
                }
            };
            if !(file_type.is_dir() || file_type.is_symlink()) {
                continue;
            }
            let id = match entry_path.file_name().and_then(|s| s.to_str()) {
                Some(s) => s.to_owned(),
                None => continue,
            };
            // Skip dot-dirs (`_schema`, `.git`, …) — they're meta, not brands.
            if id.starts_with('_') || id.starts_with('.') {
                continue;
            }
            match Self::load_one(&entry_path, id.clone()) {
                Ok(Some(record)) => {
                    out.by_id.insert(id, record);
                }
                Ok(None) => {
                    // No DESIGN.md and no manifest.json — not a design system dir.
                }
                Err(err) => {
                    tracing::warn!(?err, path = %entry_path.display(), "skipping malformed entry");
                }
            }
        }
        Ok(out)
    }

    fn load_one(dir: &Path, id: String) -> Result<Option<DesignSystemRecord>> {
        let design_md_path = dir.join("DESIGN.md");
        let manifest_path = dir.join("manifest.json");
        let has_design = design_md_path.is_file();
        let has_manifest = manifest_path.is_file();
        if !has_design && !has_manifest {
            return Ok(None);
        }

        let manifest: DesignSystemManifest = if has_manifest {
            let raw = fs::read_to_string(&manifest_path)
                .with_context(|| format!("read {}", manifest_path.display()))?;
            serde_json::from_str(&raw)
                .with_context(|| format!("parse manifest {}", manifest_path.display()))?
        } else {
            DesignSystemManifest::default()
        };

        let design_md = if has_design {
            fs::read_to_string(&design_md_path)
                .with_context(|| format!("read {}", design_md_path.display()))?
        } else {
            String::new()
        };

        let title = manifest
            .title
            .clone()
            .or_else(|| manifest.name.clone())
            .or_else(|| extract_title(&design_md))
            .unwrap_or_else(|| id.clone());

        let category = manifest
            .category
            .clone()
            .or_else(|| extract_category(&design_md))
            .unwrap_or_else(|| "Uncategorized".to_owned());

        let summary = manifest
            .summary
            .clone()
            .or_else(|| manifest.description.clone())
            .unwrap_or_else(|| summarize(&design_md));

        let mut swatches = if manifest.palette.is_empty() {
            extract_swatches(&design_md)
        } else {
            manifest.palette.clone()
        };
        swatches.truncate(8);

        let surface = manifest.surface.unwrap_or_default();

        let tokens_css = read_optional(&dir.join("tokens.css"))?;
        let fixture_html = read_optional(&dir.join("components.html"))?;

        Ok(Some(DesignSystemRecord {
            id: manifest.id.unwrap_or(id),
            title,
            category,
            summary,
            swatches,
            surface,
            tokens_css,
            fixture_html,
            root: dir.to_owned(),
        }))
    }

    /// Number of loaded design systems.
    #[must_use]
    pub fn count(&self) -> usize {
        self.by_id.len()
    }

    /// Fetch a record by its `id` (directory name).
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&DesignSystemRecord> {
        self.by_id.get(id)
    }

    /// Borrow every record, sorted lexicographically by `id`
    /// (`BTreeMap` iteration order).
    pub fn list(&self) -> impl Iterator<Item = &DesignSystemRecord> {
        self.by_id.values()
    }

    /// Owned snapshot useful for serialising the whole catalogue.
    #[must_use]
    pub fn to_vec(&self) -> Vec<DesignSystemRecord> {
        self.by_id.values().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Markdown helpers — direct port of design-systems.ts
// ---------------------------------------------------------------------------

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("read {}", path.display())),
    }
}

fn extract_title(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return Some(clean_title(rest.trim()));
        }
    }
    None
}

/// "Design System Inspired by Airbnb" → "Airbnb" — strip the boilerplate
/// prefix the corpus uses so the UI shows a clean brand label.
fn clean_title(raw: &str) -> String {
    let lower = raw.to_lowercase();
    for prefix in [
        "design system inspired by ",
        "design system for ",
        "design system: ",
        "design system - ",
    ] {
        if let Some(idx) = lower.find(prefix) {
            return raw[idx + prefix.len()..].trim().to_owned();
        }
    }
    raw.trim().to_owned()
}

fn extract_category(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix('>') else { continue };
        let rest = rest.trim_start();
        // Accept "Category:" with any casing.
        let lower = rest.to_lowercase();
        if let Some(suffix) = lower.strip_prefix("category:") {
            // Map the lowercase prefix length back into the original slice
            // so we preserve the author's casing for the value itself.
            let original_start = rest.len() - suffix.len();
            return Some(rest[original_start..].trim().to_owned());
        }
    }
    None
}

fn summarize(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let first_h1 = lines.iter().position(|l| {
        let t = l.trim_start();
        t.starts_with("# ") && !t.starts_with("##")
    });
    let Some(start) = first_h1 else { return String::new() };
    let after = &lines[start + 1..];
    let next_heading = after.iter().position(|l| {
        let t = l.trim_start();
        t.starts_with('#') && t.contains(' ')
    });
    let window: Vec<&str> = match next_heading {
        Some(end) => after[..end].to_vec(),
        None => after.to_vec(),
    };
    let mut buf = String::new();
    for line in window {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('>') {
            let rest_lower = rest.trim_start().to_lowercase();
            if rest_lower.starts_with("category:") {
                // Drop the metadata line — surfaced separately.
                continue;
            }
            // Strip the leading `>` but keep the content (other blockquotes).
            buf.push_str(rest.trim_start());
        } else {
            buf.push_str(line);
        }
        buf.push('\n');
    }
    buf.trim().to_owned()
}

fn extract_swatches(raw: &str) -> Vec<String> {
    // Match `#RGB` / `#RRGGBB` / `#RRGGBBAA` hex literals appearing in code
    // spans or prose. We do not use a regex crate to keep deps minimal —
    // a single linear scan is plenty for ≤64 KB inputs.
    let mut out = Vec::new();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'#' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
            j += 1;
        }
        let len = j - i - 1;
        if matches!(len, 3 | 6 | 8) {
            let token = &raw[i..j];
            let lower = token.to_ascii_lowercase();
            if !out.contains(&lower) {
                out.push(lower);
                if out.len() >= 16 {
                    break;
                }
            }
        }
        i = j.max(i + 1);
    }
    out.truncate(8);
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(dir: &Path, name: &str, body: &str) {
        let file = dir.join(name);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, body).unwrap();
    }

    fn fake_systems_dir() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        // 1. Markdown-driven brand with full prose, tokens, and components.
        let airbnb = dir.path().join("airbnb");
        write(
            &airbnb,
            "DESIGN.md",
            "# Design System Inspired by Airbnb\n\n\
             > Category: E-Commerce & Retail\n\
             > Travel marketplace. Warm coral accent.\n\n\
             Airbnb's 2026 design feels like a travel magazine. Rausch coral `#ff385c` is\n\
             used sparingly but unmistakably. Everything else is `#222222` grayscale.\n\n\
             ## 2. Layout\n\nMore words.",
        );
        write(&airbnb, "tokens.css", ":root { --rausch: #ff385c; }");
        write(&airbnb, "components.html", "<button>Book</button>");
        // 2. Manifest-driven brand — no DESIGN.md at all.
        let acme = dir.path().join("acme");
        write(
            &acme,
            "manifest.json",
            r##"{"id":"acme","title":"Acme","category":"Tools","summary":"Hammers and anvils.","palette":["#000000","#ff00ff"],"surface":"web"}"##,
        );
        // 3. Hybrid: DESIGN.md + manifest override of category.
        let stripe = dir.path().join("stripe");
        write(
            &stripe,
            "DESIGN.md",
            "# Stripe\n\n> Category: Fintech\n\nPayments brand color is `#635bff`.",
        );
        write(
            &stripe,
            "manifest.json",
            r##"{"category":"Payments Infrastructure"}"##,
        );
        // 4. Skipped: meta dir.
        write(
            &dir.path().join("_schema"),
            "AGENTS.md",
            "this is a schema, not a brand",
        );
        // 5. Skipped: empty dir.
        fs::create_dir_all(dir.path().join("empty")).unwrap();
        dir
    }

    #[test]
    fn registry_loads_three_fake_systems_from_tmp_dir() {
        let tmp = fake_systems_dir();
        let reg = DesignSystemRegistry::load_from_dir(tmp.path()).unwrap();
        assert_eq!(reg.count(), 3, "expected 3 brands, got {}", reg.count());
    }

    #[test]
    fn registry_get_by_id_returns_record() {
        let tmp = fake_systems_dir();
        let reg = DesignSystemRegistry::load_from_dir(tmp.path()).unwrap();
        let airbnb = reg.get("airbnb").expect("airbnb present");
        assert_eq!(airbnb.title, "Airbnb");
        assert_eq!(airbnb.category, "E-Commerce & Retail");
        assert!(airbnb.summary.contains("travel magazine"));
        assert!(airbnb.swatches.contains(&"#ff385c".to_owned()));
        assert!(airbnb.swatches.contains(&"#222222".to_owned()));
        assert_eq!(
            airbnb.tokens_css.as_deref(),
            Some(":root { --rausch: #ff385c; }")
        );
        assert_eq!(airbnb.fixture_html.as_deref(), Some("<button>Book</button>"));
        assert_eq!(airbnb.surface, DesignSystemSurface::Web);
    }

    #[test]
    fn manifest_only_brand_loads_correctly() {
        let tmp = fake_systems_dir();
        let reg = DesignSystemRegistry::load_from_dir(tmp.path()).unwrap();
        let acme = reg.get("acme").expect("acme present");
        assert_eq!(acme.title, "Acme");
        assert_eq!(acme.category, "Tools");
        assert_eq!(acme.summary, "Hammers and anvils.");
        assert_eq!(acme.swatches, vec!["#000000", "#ff00ff"]);
        assert!(acme.tokens_css.is_none());
        assert!(acme.fixture_html.is_none());
    }

    #[test]
    fn manifest_overrides_markdown_category() {
        let tmp = fake_systems_dir();
        let reg = DesignSystemRegistry::load_from_dir(tmp.path()).unwrap();
        let stripe = reg.get("stripe").expect("stripe present");
        // Title was not in manifest → comes from DESIGN.md H1.
        assert_eq!(stripe.title, "Stripe");
        // Category WAS in manifest → manifest wins.
        assert_eq!(stripe.category, "Payments Infrastructure");
        // Swatch still extracted from DESIGN.md.
        assert!(stripe.swatches.contains(&"#635bff".to_owned()));
    }

    #[test]
    fn registry_list_sorted_by_id() {
        let tmp = fake_systems_dir();
        let reg = DesignSystemRegistry::load_from_dir(tmp.path()).unwrap();
        let ids: Vec<&str> = reg.list().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["acme", "airbnb", "stripe"]);
    }

    #[test]
    fn loading_missing_dir_returns_error() {
        let tmp = tempdir().unwrap();
        let missing = tmp.path().join("nope");
        assert!(DesignSystemRegistry::load_from_dir(&missing).is_err());
    }

    #[test]
    fn extract_title_strips_prefix() {
        assert_eq!(
            extract_title("# Design System Inspired by Airbnb\n\nMore"),
            Some("Airbnb".to_owned())
        );
        assert_eq!(
            extract_title("# Stripe\n\nMore"),
            Some("Stripe".to_owned())
        );
        assert_eq!(extract_title("no h1 here"), None);
    }

    #[test]
    fn extract_category_handles_blockquote() {
        let raw = "# Brand\n\n> Category: Fintech\n\nBody";
        assert_eq!(extract_category(raw), Some("Fintech".to_owned()));
        // Case-insensitive prefix detection.
        let raw2 = "# Brand\n\n> category: Lower Case\n\n";
        assert_eq!(extract_category(raw2), Some("Lower Case".to_owned()));
        // Missing category.
        assert_eq!(extract_category("# Brand\n\nBody"), None);
    }

    #[test]
    fn extract_swatches_dedupes_and_caps() {
        let raw = "# B\n\nUse `#ff385c` and `#FF385C` and `#222222` and `#abc` and `#fff` and `#ff385c`.";
        let s = extract_swatches(raw);
        assert!(s.contains(&"#ff385c".to_owned()));
        assert!(s.contains(&"#222222".to_owned()));
        assert!(s.contains(&"#abc".to_owned()));
        // Dedup: `#ff385c` only counted once even though it appears twice
        // (and uppercase variant normalised to lowercase).
        let count = s.iter().filter(|x| x.as_str() == "#ff385c").count();
        assert_eq!(count, 1);
    }
}
