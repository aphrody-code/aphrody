// SPDX-License-Identifier: Apache-2.0
//! JSONL mailbox reader with mtime-based change detection.
//!
//! Reads `inbox-from-aphrody.jsonl` and `inbox-from-winclean.jsonl` from
//! the coord directory.  Files are re-parsed only when their mtime has
//! advanced, preserving cursor position across refreshes.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;

use crate::Envelope;

/// A parsed mailbox for one peer side.
#[derive(Debug, Clone)]
pub struct Mailbox {
    /// Path to the `.jsonl` file.
    pub path: PathBuf,
    /// All envelopes in insertion order.
    pub envelopes: Vec<Envelope>,
    /// Last observed mtime; `None` if the file has not been read yet.
    pub last_mtime: Option<SystemTime>,
}

impl Mailbox {
    /// Construct a [`Mailbox`] pointing at `path`.
    ///
    /// Does not read the file; call [`Self::refresh`] to load envelopes.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            envelopes: Vec::new(),
            last_mtime: None,
        }
    }

    /// Re-read the file if its mtime has advanced since the last read.
    ///
    /// Returns `true` if the mailbox was updated, `false` if unchanged.
    /// A missing file is treated as an empty mailbox (no error).
    ///
    /// # Errors
    ///
    /// Returns an error only if the file exists but cannot be read or parsed.
    pub fn refresh(&mut self) -> Result<bool> {
        let current_mtime = match std::fs::metadata(&self.path) {
            Ok(meta) => meta.modified().ok(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Treat a missing file as empty — coord dir may not yet have
                // a second peer.
                if !self.envelopes.is_empty() {
                    self.envelopes.clear();
                    self.last_mtime = None;
                    return Ok(true);
                }
                return Ok(false);
            }
            Err(e) => return Err(anyhow::anyhow!("stat {}: {e}", self.path.display())),
        };

        if self.last_mtime == current_mtime {
            return Ok(false);
        }

        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| anyhow::anyhow!("read {}: {e}", self.path.display()))?;

        // Silently skip lines that do not parse rather than aborting — a
        // concurrent writer may have flushed a partial line.
        let envelopes: Vec<Envelope> = content
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with("//")
            })
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();

        self.envelopes = envelopes;
        self.last_mtime = current_mtime;
        Ok(true)
    }
}

/// Snapshot of both peer mailboxes.
#[derive(Debug, Clone)]
pub struct MailboxPair {
    /// `inbox-from-aphrody.jsonl`
    pub from_aphrody: Mailbox,
    /// `inbox-from-winclean.jsonl`
    pub from_winclean: Mailbox,
}

impl MailboxPair {
    /// Construct a pair for `coord_dir`.
    pub fn new(coord_dir: &Path) -> Self {
        Self {
            from_aphrody: Mailbox::new(coord_dir.join("inbox-from-aphrody.jsonl")),
            from_winclean: Mailbox::new(coord_dir.join("inbox-from-winclean.jsonl")),
        }
    }

    /// Refresh both mailboxes.
    ///
    /// Returns `true` if either side changed.
    ///
    /// # Errors
    ///
    /// Returns the first error encountered.
    pub fn refresh(&mut self) -> Result<bool> {
        let a = self.from_aphrody.refresh()?;
        let b = self.from_winclean.refresh()?;
        Ok(a || b)
    }

    /// Mtime of a heartbeat file, or `None` if absent.
    pub fn heartbeat_mtime(coord_dir: &Path, side: &str) -> Option<SystemTime> {
        let path = coord_dir.join(format!("heartbeat-{side}.txt"));
        std::fs::metadata(path).ok()?.modified().ok()
    }
}

/// Return the envelope slice for the given peer `side`.
///
/// `side` must be `"aphrody"` or `"winclean"`; any other value returns the
/// aphrody mailbox as a safe default.
pub fn mailbox_pair_envelopes<'a>(pair: &'a MailboxPair, side: &str) -> &'a [crate::Envelope] {
    if side == "winclean" {
        &pair.from_winclean.envelopes
    } else {
        &pair.from_aphrody.envelopes
    }
}
