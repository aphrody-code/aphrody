// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Parse le dossier markdown produit par siteone-crawler.
//!
//! siteone conserve la structure d'URL du site dans le dossier :
//!   out/_siteone_md/
//!     index.html.md              ← home
//!     components/
//!       button/index.html.md
//!     styles/
//!       color/index.html.md
//!
//! On scanne récursivement, extrait le premier H1 comme titre et la 1re
//! ligne non-vide comme description, déduit une section depuis le path.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct Page {
    /// Path relatif sous md_dir, e.g. "components/button/index.html.md".
    pub rel_path: String,
    /// URL reconstruite depuis le path + root URL.
    pub url: String,
    /// Premier `# ` du markdown (ou fallback dérivé du path).
    pub title: String,
    /// Description courte ≤ 150 chars (tirée du paragraphe d'intro).
    pub description: String,
    /// Section déduite (premier segment du path, ou "Home").
    pub section: String,
    /// Contenu markdown complet.
    pub content: String,
}

#[derive(Debug)]
pub struct Site {
    pub root_url: String,
    pub site_title: String,
    pub site_description: String,
    pub pages: Vec<Page>,
}

impl Site {
    /// Retourne les pages groupées par section, tri alphabétique.
    pub fn sections(&self) -> BTreeMap<String, Vec<&Page>> {
        let mut map: BTreeMap<String, Vec<&Page>> = BTreeMap::new();
        for p in &self.pages {
            map.entry(p.section.clone()).or_default().push(p);
        }
        for v in map.values_mut() {
            v.sort_by(|a, b| a.title.cmp(&b.title));
        }
        map
    }
}

pub fn scan_export(md_dir: &Path, root_url: &str) -> Result<Site> {
    let mut pages = Vec::new();
    let root = url::Url::parse(root_url).with_context(|| format!("parse {root_url}"))?;

    if !md_dir.exists() {
        anyhow::bail!("dossier export siteone introuvable : {}", md_dir.display());
    }

    for entry in WalkDir::new(md_dir).into_iter().flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext_ok = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| e.eq_ignore_ascii_case("md"))
            .unwrap_or(false);
        if !ext_ok {
            continue;
        }

        let content =
            std::fs::read_to_string(path).with_context(|| format!("lire {}", path.display()))?;
        let rel = path
            .strip_prefix(md_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();

        let title = extract_title(&content).unwrap_or_else(|| path_to_title(&rel));
        let description = extract_description(&content, &title);
        let section = derive_section(&rel);
        let url = reconstruct_url(&root, &rel);

        pages.push(Page {
            rel_path: rel,
            url,
            title,
            description,
            section,
            content,
        });
    }

    // Home page = path le plus court (index.html.md à la racine).
    pages.sort_by_key(|p| p.rel_path.matches('/').count());
    let home = pages.first().cloned();
    let (site_title, site_description) = match home {
        Some(h) => (h.title.clone(), h.description.clone()),
        None => (
            root.host_str().unwrap_or("Site").to_string(),
            format!("Content extracted from {root_url}"),
        ),
    };

    Ok(Site {
        root_url: root_url.to_string(),
        site_title,
        site_description,
        pages,
    })
}

fn extract_title(md: &str) -> Option<String> {
    for line in md.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("# ") {
            let cleaned = rest.trim().to_string();
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn extract_description(md: &str, title: &str) -> String {
    // Premier paragraphe non-vide qui n'est ni le titre ni un heading.
    let mut after_title = false;
    for line in md.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix('#') {
            if rest.trim() == title || after_title {
                after_title = true;
                continue;
            }
            // Skip autre heading.
            continue;
        }
        if after_title || title.is_empty() {
            return truncate(t, 150);
        }
    }
    // Fallback : 1re ligne non-vide.
    md.lines()
        .find(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .map(|l| truncate(l.trim(), 150))
        .unwrap_or_default()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

fn path_to_title(rel: &str) -> String {
    let trimmed = rel
        .trim_end_matches(".md")
        .trim_end_matches(".html")
        .trim_end_matches("/index");
    let last = trimmed.split('/').next_back().unwrap_or(trimmed);
    last.replace(['-', '_'], " ")
        .split_whitespace()
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(ch) => ch.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn derive_section(rel: &str) -> String {
    let segments: Vec<&str> = rel
        .split('/')
        .filter(|s| !s.is_empty() && !s.starts_with("index"))
        .collect();
    if segments.is_empty() || segments.len() == 1 {
        return "Home".to_string();
    }
    let seg = segments[0];
    seg.replace(['-', '_'], " ")
        .split_whitespace()
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

fn reconstruct_url(root: &url::Url, rel: &str) -> String {
    let trimmed = rel.trim_end_matches(".md").trim_end_matches(".html");
    let trimmed = trimmed.trim_start_matches('/');
    match root.join(trimmed) {
        Ok(u) => u.to_string(),
        Err(_) => format!("{root}{trimmed}"),
    }
}
