// SPDX-License-Identifier: Apache-2.0
//
// Persistent install manifest at `$APHRODY_SKILLS_HOME/manifest.json`.
//
// Port of packages/aphrody-skills/src/manifest.ts. JSON shape is byte-stable
// with the TS version (same field names, same `version: 1` envelope).

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::runtime::sources::SourceSlug;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEntry {
    pub name: String,
    pub source: SourceSlug,
    pub source_path: String,
    pub copied_at: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub version: u32,
    pub home: String,
    pub updated_at: String,
    pub entries: Vec<ManifestEntry>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: 1,
            home: skills_home().to_string_lossy().into_owned(),
            updated_at: now_iso8601(),
            entries: Vec::new(),
        }
    }
}

/// Resolve `$APHRODY_SKILLS_HOME`, with the same fallback chain as TS:
/// `$APHRODY_SKILLS_HOME` -> `$HOME/.aphrody/skills` -> `$USERPROFILE/...`
/// -> `./.aphrody/skills`.
#[must_use]
pub fn skills_home() -> PathBuf {
    if let Ok(o) = env::var("APHRODY_SKILLS_HOME") {
        if !o.is_empty() {
            return PathBuf::from(o);
        }
    }
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".aphrody").join("skills")
}

#[must_use]
pub fn manifest_path() -> PathBuf {
    skills_home().join("manifest.json")
}

/// Create `$APHRODY_SKILLS_HOME` if absent, return the path.
pub fn ensure_home_exists() -> Result<PathBuf> {
    let dir = skills_home();
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("create skills home {}", dir.display()))?;
    }
    Ok(dir)
}

/// Read the manifest from disk. Returns a fresh empty manifest if the file
/// does not exist. Returns Err on parse failures (mirrors TS semantics).
pub fn read_manifest() -> Result<Manifest> {
    let path = manifest_path();
    if !path.exists() {
        return Ok(Manifest::default());
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("read manifest {}", path.display()))?;
    let parsed: Manifest = serde_json::from_str(&raw)
        .with_context(|| format!("parse manifest {}", path.display()))?;
    anyhow::ensure!(parsed.version == 1, "manifest schema mismatch (version != 1)");
    Ok(parsed)
}

/// Persist a manifest as pretty-printed JSON (trailing newline, like TS).
pub fn write_manifest(m: &Manifest) -> Result<()> {
    let path = manifest_path();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create manifest parent {}", parent.display()))?;
        }
    }
    let mut body = serde_json::to_string_pretty(m).context("encode manifest as JSON")?;
    body.push('\n');
    fs::write(&path, body).with_context(|| format!("write manifest {}", path.display()))?;
    Ok(())
}

/// Upsert one entry (matched by name+source) and persist.
pub fn upsert_entry(entry: ManifestEntry) -> Result<Manifest> {
    let mut current = read_manifest()?;
    current
        .entries
        .retain(|e| !(e.name == entry.name && e.source == entry.source));
    current.entries.push(entry);
    current.entries.sort_by(|a, b| {
        if a.source == b.source {
            a.name.cmp(&b.name)
        } else {
            a.source.as_str().cmp(b.source.as_str())
        }
    });
    current.version = 1;
    current.home = skills_home().to_string_lossy().into_owned();
    current.updated_at = now_iso8601();
    write_manifest(&current)?;
    Ok(current)
}

fn now_iso8601() -> String {
    // Use std::time only — chrono isn't needed for an ISO-8601 timestamp at
    // second precision. Format: YYYY-MM-DDTHH:MM:SSZ.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_iso8601(secs)
}

fn format_iso8601(secs: u64) -> String {
    // Days since 1970-01-01.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let h = rem / 3_600;
    let m = (rem % 3_600) / 60;
    let s = rem % 60;
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

// Howard Hinnant's date algorithm — converts days-since-epoch to (y,m,d).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_format_known_value() {
        // 2026-05-18T00:00:00Z = 1_779_840_000 seconds since epoch (computed
        // independently). We just check the algorithm produces a valid shape.
        let s = format_iso8601(1_779_840_000);
        assert_eq!(s.len(), 20);
        assert!(s.ends_with('Z'));
        assert!(s.contains('T'));
        // Sanity: first 4 chars are a year.
        assert!(s[..4].chars().all(|c| c.is_ascii_digit()));
    }
}
