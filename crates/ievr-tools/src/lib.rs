// SPDX-License-Identifier: Apache-2.0
//! ievr-tools — IEVR binary inventory analysis library.
//!
//! Consumes the JSON output of peer-Claude's inventory scan
//! (`var/data/ievr-binaries-inventory.json`) and provides typed access
//! plus aggregation helpers.  Extended with format-parser submodules for
//! PE/COFF Windows binaries, CRIWARE @UTF tables (CPK / USM / AWB), and
//! the `cpk_list.cfg.bin` master manifest.

pub mod cri;
pub mod manifest;
pub mod pe;

use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

/// Full inventory document as emitted by the peer-Claude scan script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// ISO-8601 timestamp of the scan.
    pub ts: String,
    /// Absolute path to the IEVR Steam installation root.
    pub install_root: String,
    /// Sum of all binary sizes in bytes (as reported by the scan).
    pub total_size_bytes: u64,
    /// Flat list of every binary discovered under `install_root`.
    pub binaries: Vec<Binary>,
}

/// A single binary entry within the inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binary {
    /// File size in bytes.
    pub size_bytes: u64,
    /// Path relative to `install_root`, using forward-slash separators.
    pub rel_path: String,
    /// File extension (without leading dot), e.g. `"exe"`, `"dll"`, `"pak"`.
    pub ext: String,
}

impl Inventory {
    /// Load an [`Inventory`] from a JSON file on disk.
    ///
    /// The file is read entirely into memory before deserialisation; the
    /// 60 GB IEVR install produces an inventory file that comfortably fits
    /// in RAM (empirically < 20 MB for ~1000 entries).
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if JSON deserialisation
    /// fails (malformed schema, truncated file, etc.).
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| {
            anyhow::anyhow!("failed to read inventory file `{}`: {e}", path.as_ref().display())
        })?;
        let inv: Self = serde_json::from_slice(&bytes).map_err(|e| {
            anyhow::anyhow!(
                "failed to deserialise inventory JSON from `{}`: {e}",
                path.as_ref().display()
            )
        })?;
        Ok(inv)
    }

    /// Total number of binary entries in the inventory.
    #[must_use]
    pub fn count(&self) -> usize {
        self.binaries.len()
    }

    /// Total size in bytes as declared in the inventory header.
    ///
    /// This is the pre-computed field from the JSON root, not a re-sum of
    /// individual entries, so it matches what the scan script recorded.
    #[must_use]
    pub fn total_size(&self) -> u64 {
        self.total_size_bytes
    }

    /// Group binaries by extension and return aggregated statistics.
    ///
    /// Returns a `Vec` of `(extension, file_count, total_bytes)` tuples,
    /// sorted descending by `total_bytes` so the largest groups appear first.
    ///
    /// Extensions are taken verbatim from [`Binary::ext`]; an empty string
    /// is used as the key for files with no extension.
    #[must_use]
    pub fn by_ext(&self) -> Vec<(String, usize, u64)> {
        let mut map: HashMap<String, (usize, u64)> = HashMap::new();
        for b in &self.binaries {
            let entry = map.entry(b.ext.clone()).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += b.size_bytes;
        }
        let mut v: Vec<_> =
            map.into_iter().map(|(ext, (count, bytes))| (ext, count, bytes)).collect();
        // Primary sort: largest total byte footprint first.
        v.sort_by(|a, b| b.2.cmp(&a.2));
        v
    }

    /// Return references to the `n` largest binaries by [`Binary::size_bytes`],
    /// sorted descending.
    ///
    /// If `n` exceeds the number of binaries the full list is returned.
    #[must_use]
    pub fn largest(&self, n: usize) -> Vec<&Binary> {
        let mut sorted: Vec<&Binary> = self.binaries.iter().collect();
        sorted.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        sorted.into_iter().take(n).collect()
    }
}
