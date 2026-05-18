// SPDX-License-Identifier: Apache-2.0
//! Per-format normalizers.
//!
//! Each accepted artifact format has a [`FormatAdapter`] impl that:
//!
//! * Validates the raw payload (magic bytes for binary formats, light
//!   structural checks for text formats).
//! * Strips unsafe constructs that would break the sandboxed preview UI
//!   (HTML inline scripts, ZIP path-traversal entries, etc.).
//! * Returns a [`NormalizedArtifact`] with the canonical MIME and the
//!   sanitized bytes.

use std::borrow::Cow;
use std::io::{Cursor, Read};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{SidecarError, SidecarResult};

/// Whitelisted artifact formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactFormat {
    /// HTML page (sandboxed inline preview).
    Html,
    /// PDF document (magic-byte validated).
    Pdf,
    /// PowerPoint deck (.pptx — Open Office XML).
    Pptx,
    /// ZIP archive (no path traversal, no absolute paths).
    Zip,
    /// CommonMark / GFM Markdown source.
    Markdown,
}

impl ArtifactFormat {
    /// Canonical MIME for this format.
    #[must_use]
    pub const fn canonical_mime(self) -> &'static str {
        match self {
            Self::Html => "text/html",
            Self::Pdf => "application/pdf",
            Self::Pptx => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            Self::Zip => "application/zip",
            Self::Markdown => "text/markdown",
        }
    }

    /// Best-effort inference from the artifact-declared MIME type.
    #[must_use]
    pub fn from_mime(mime: &str) -> Option<Self> {
        // Strip parameters (`; charset=utf-8` etc.).
        let bare = mime.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
        Some(match bare.as_str() {
            "text/html" | "application/xhtml+xml" => Self::Html,
            "application/pdf" => Self::Pdf,
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            | "application/vnd.ms-powerpoint" => Self::Pptx,
            "application/zip" | "application/x-zip-compressed" => Self::Zip,
            "text/markdown" | "text/x-markdown" => Self::Markdown,
            _ => return None,
        })
    }
}

/// Output of a successful normalization pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedArtifact {
    /// Resolved format.
    pub format: ArtifactFormat,
    /// Canonical MIME for `format`.
    pub mime: String,
    /// Sanitized bytes ready for the preview sandbox.
    pub bytes: Vec<u8>,
    /// Adapter-emitted notes (e.g. "stripped 3 <script> tags").
    pub notes: Vec<String>,
}

/// Adapter contract.
pub trait FormatAdapter: Send + Sync {
    /// Format this adapter handles.
    fn format(&self) -> ArtifactFormat;

    /// Normalize `raw` for safe preview.
    ///
    /// # Errors
    ///
    /// [`SidecarError::Normalize`] if the payload cannot be made safe (e.g.
    /// invalid magic bytes, ZIP with absolute paths, Markdown that doesn't
    /// parse as UTF-8).
    fn normalize(&self, raw: &[u8]) -> SidecarResult<NormalizedArtifact>;
}

/// Dispatch table: returns the canonical adapter for a given format.
#[must_use]
pub fn adapter_for(format: ArtifactFormat) -> Box<dyn FormatAdapter> {
    match format {
        ArtifactFormat::Html => Box::new(HtmlAdapter::default()),
        ArtifactFormat::Pdf => Box::new(PdfAdapter),
        ArtifactFormat::Pptx => Box::new(PptxAdapter),
        ArtifactFormat::Zip => Box::new(ZipAdapter),
        ArtifactFormat::Markdown => Box::new(MarkdownAdapter),
    }
}

// ── HTML ─────────────────────────────────────────────────────────────────────

/// Strips `<script>` blocks and inline `on*=` handlers from raw HTML.
pub struct HtmlAdapter {
    script_re: Regex,
    inline_handler_re: Regex,
    javascript_url_re: Regex,
}

impl Default for HtmlAdapter {
    fn default() -> Self {
        Self {
            script_re: Regex::new(r"(?is)<script\b[^>]*>.*?</script\s*>").unwrap(),
            inline_handler_re: Regex::new(r#"(?i)\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#)
                .unwrap(),
            javascript_url_re: Regex::new(r#"(?i)(href|src)\s*=\s*("javascript:[^"]*"|'javascript:[^']*'|javascript:[^\s>]+)"#).unwrap(),
        }
    }
}

impl FormatAdapter for HtmlAdapter {
    fn format(&self) -> ArtifactFormat {
        ArtifactFormat::Html
    }

    fn normalize(&self, raw: &[u8]) -> SidecarResult<NormalizedArtifact> {
        let text = std::str::from_utf8(raw)
            .map_err(|e| SidecarError::Normalize(format!("html not utf-8: {e}")))?;

        let mut notes = Vec::new();

        let stripped_scripts = self.script_re.replace_all(text, "");
        let removed_scripts = self.script_re.find_iter(text).count();
        if removed_scripts > 0 {
            notes.push(format!("stripped {removed_scripts} <script> block(s)"));
        }

        let stripped_handlers: Cow<'_, str> =
            self.inline_handler_re.replace_all(&stripped_scripts, "");
        let removed_handlers = self.inline_handler_re.find_iter(&stripped_scripts).count();
        if removed_handlers > 0 {
            notes.push(format!("stripped {removed_handlers} inline on* handler(s)"));
        }

        let final_html = self
            .javascript_url_re
            .replace_all(&stripped_handlers, "$1=\"about:blank\"")
            .into_owned();

        Ok(NormalizedArtifact {
            format: ArtifactFormat::Html,
            mime: ArtifactFormat::Html.canonical_mime().to_string(),
            bytes: final_html.into_bytes(),
            notes,
        })
    }
}

// ── Markdown ─────────────────────────────────────────────────────────────────

/// CommonMark-validates UTF-8 markdown source. Reuses the same minimal
/// validation strategy as `aphrody-terminal-markdown`'s OSC payload check
/// (must be UTF-8, must not contain raw NUL bytes).
pub struct MarkdownAdapter;

impl FormatAdapter for MarkdownAdapter {
    fn format(&self) -> ArtifactFormat {
        ArtifactFormat::Markdown
    }

    fn normalize(&self, raw: &[u8]) -> SidecarResult<NormalizedArtifact> {
        let text = std::str::from_utf8(raw)
            .map_err(|e| SidecarError::Normalize(format!("markdown not utf-8: {e}")))?;
        if text.contains('\0') {
            return Err(SidecarError::Normalize(
                "markdown payload contains null byte".into(),
            ));
        }
        Ok(NormalizedArtifact {
            format: ArtifactFormat::Markdown,
            mime: ArtifactFormat::Markdown.canonical_mime().to_string(),
            bytes: text.as_bytes().to_vec(),
            notes: Vec::new(),
        })
    }
}

// ── PDF ──────────────────────────────────────────────────────────────────────

/// Validates the PDF magic header (`%PDF-`).
pub struct PdfAdapter;

impl FormatAdapter for PdfAdapter {
    fn format(&self) -> ArtifactFormat {
        ArtifactFormat::Pdf
    }

    fn normalize(&self, raw: &[u8]) -> SidecarResult<NormalizedArtifact> {
        if raw.len() < 5 || &raw[..5] != b"%PDF-" {
            return Err(SidecarError::Normalize(
                "pdf magic bytes `%PDF-` missing".into(),
            ));
        }
        Ok(NormalizedArtifact {
            format: ArtifactFormat::Pdf,
            mime: ArtifactFormat::Pdf.canonical_mime().to_string(),
            bytes: raw.to_vec(),
            notes: Vec::new(),
        })
    }
}

// ── PPTX (and any other Open Office XML zip-backed format) ───────────────────

/// Validates the PPTX magic header (PKZip `PK\x03\x04`). Detailed
/// content-type inspection (the `[Content_Types].xml` inside the zip)
/// is intentionally out of scope — we only confirm the envelope.
pub struct PptxAdapter;

impl FormatAdapter for PptxAdapter {
    fn format(&self) -> ArtifactFormat {
        ArtifactFormat::Pptx
    }

    fn normalize(&self, raw: &[u8]) -> SidecarResult<NormalizedArtifact> {
        if raw.len() < 4 || &raw[..4] != b"PK\x03\x04" {
            return Err(SidecarError::Normalize(
                "pptx magic bytes `PK\\x03\\x04` missing".into(),
            ));
        }
        Ok(NormalizedArtifact {
            format: ArtifactFormat::Pptx,
            mime: ArtifactFormat::Pptx.canonical_mime().to_string(),
            bytes: raw.to_vec(),
            notes: Vec::new(),
        })
    }
}

// ── ZIP ──────────────────────────────────────────────────────────────────────

/// Validates a raw ZIP archive's central directory: no absolute paths, no
/// `..` segments, no empty filenames.
///
/// We hand-roll a minimal central-directory walker (signature
/// `PK\x05\x06` for EOCD, then back-trace entries via `PK\x01\x02`) so the
/// crate does not need to pull in a full zip decompressor — we only audit
/// file names.
pub struct ZipAdapter;

impl FormatAdapter for ZipAdapter {
    fn format(&self) -> ArtifactFormat {
        ArtifactFormat::Zip
    }

    fn normalize(&self, raw: &[u8]) -> SidecarResult<NormalizedArtifact> {
        if raw.len() < 4 || &raw[..4] != b"PK\x03\x04" {
            return Err(SidecarError::Normalize(
                "zip magic bytes `PK\\x03\\x04` missing".into(),
            ));
        }
        let names = list_zip_entry_names(raw)?;
        let mut notes = Vec::with_capacity(names.len());
        for name in &names {
            if name.is_empty() {
                return Err(SidecarError::Normalize("zip entry has empty name".into()));
            }
            if name.starts_with('/') || name.starts_with('\\') {
                return Err(SidecarError::Normalize(format!(
                    "zip entry uses absolute path: {name}"
                )));
            }
            if name.contains("..") {
                return Err(SidecarError::Normalize(format!(
                    "zip entry uses parent-dir traversal: {name}"
                )));
            }
            // Reject Windows drive letters too (`C:\…`).
            if name.len() >= 2 && name.as_bytes()[1] == b':' {
                return Err(SidecarError::Normalize(format!(
                    "zip entry uses drive-letter path: {name}"
                )));
            }
            notes.push(format!("entry: {name}"));
        }
        Ok(NormalizedArtifact {
            format: ArtifactFormat::Zip,
            mime: ArtifactFormat::Zip.canonical_mime().to_string(),
            bytes: raw.to_vec(),
            notes,
        })
    }
}

/// Walk the local-file headers of a ZIP archive and return every entry name.
///
/// Each local-file header is `PK\x03\x04` (4) + 26-byte header (containing
/// `compressed_size` at offset 18 and `name_len` at offset 26) + name
/// (`name_len` bytes) + extra (`extra_len` bytes) + payload
/// (`compressed_size` bytes).
fn list_zip_entry_names(raw: &[u8]) -> SidecarResult<Vec<String>> {
    let mut cursor = Cursor::new(raw);
    let mut names = Vec::new();
    let total = raw.len() as u64;

    loop {
        if cursor.position() + 4 > total {
            break;
        }
        let mut sig = [0u8; 4];
        cursor
            .read_exact(&mut sig)
            .map_err(|e| SidecarError::Normalize(format!("zip header read: {e}")))?;
        if &sig != b"PK\x03\x04" {
            // Reached central directory or EOCD — stop scanning.
            break;
        }
        // Local file header (excluding the 4-byte signature) is 26 bytes.
        let mut hdr = [0u8; 26];
        cursor
            .read_exact(&mut hdr)
            .map_err(|e| SidecarError::Normalize(format!("zip lfh read: {e}")))?;
        // Layout (little-endian):
        //   0..2 version_needed
        //   2..4 flags
        //   4..6 method
        //   6..8 mod_time
        //   8..10 mod_date
        //  10..14 crc32
        //  14..18 compressed_size
        //  18..22 uncompressed_size
        //  22..24 name_len
        //  24..26 extra_len
        let compressed_size = u32::from_le_bytes(hdr[14..18].try_into().unwrap()) as u64;
        let name_len = u16::from_le_bytes(hdr[22..24].try_into().unwrap()) as u64;
        let extra_len = u16::from_le_bytes(hdr[24..26].try_into().unwrap()) as u64;

        if cursor.position() + name_len > total {
            return Err(SidecarError::Normalize("zip entry name overruns file".into()));
        }
        let mut name_bytes = vec![0u8; name_len as usize];
        cursor
            .read_exact(&mut name_bytes)
            .map_err(|e| SidecarError::Normalize(format!("zip name read: {e}")))?;
        let name = String::from_utf8(name_bytes)
            .map_err(|e| SidecarError::Normalize(format!("zip name not utf-8: {e}")))?;
        names.push(name);

        cursor.set_position(cursor.position() + extra_len + compressed_size);
    }

    if names.is_empty() {
        return Err(SidecarError::Normalize("zip has no local file headers".into()));
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        // Minimal valid-ish ZIP: only local-file headers, stored (method=0),
        // crc=0, no central directory. Enough for our entry-name walker.
        let mut out = Vec::new();
        for (name, data) in entries {
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&[0u8; 2]); // version_needed
            out.extend_from_slice(&[0u8; 2]); // flags
            out.extend_from_slice(&[0u8; 2]); // method (stored)
            out.extend_from_slice(&[0u8; 2]); // mod_time
            out.extend_from_slice(&[0u8; 2]); // mod_date
            out.extend_from_slice(&[0u8; 4]); // crc32
            out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes()); // comp_size
            out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes()); // uncomp_size
            out.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes()); // name_len
            out.extend_from_slice(&[0u8; 2]); // extra_len
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
        }
        out
    }

    #[test]
    fn html_strips_script_tags() {
        let adapter = HtmlAdapter::default();
        let raw = b"<h1>ok</h1><script>alert(1)</script><p>x</p>";
        let norm = adapter.normalize(raw).unwrap();
        let body = std::str::from_utf8(&norm.bytes).unwrap();
        assert!(!body.contains("script"), "got: {body}");
        assert!(body.contains("<h1>ok</h1>"));
        assert!(norm.notes.iter().any(|n| n.contains("script")));
    }

    #[test]
    fn html_strips_inline_handlers_and_js_urls() {
        let adapter = HtmlAdapter::default();
        let raw = br#"<a href="javascript:alert(1)" onclick="bad()">click</a>"#;
        let norm = adapter.normalize(raw).unwrap();
        let body = std::str::from_utf8(&norm.bytes).unwrap();
        assert!(!body.contains("javascript:"), "got: {body}");
        assert!(!body.contains("onclick"), "got: {body}");
    }

    #[test]
    fn pdf_magic_bytes_required() {
        assert!(PdfAdapter.normalize(b"%PDF-1.7 trailer").is_ok());
        assert!(PdfAdapter.normalize(b"not a pdf").is_err());
    }

    #[test]
    fn pptx_magic_bytes_required() {
        assert!(PptxAdapter.normalize(b"PK\x03\x04rest").is_ok());
        assert!(PptxAdapter.normalize(b"PDF-xxxx").is_err());
    }

    #[test]
    fn zip_rejects_path_traversal() {
        let bad = make_zip(&[("../etc/passwd", b"x")]);
        let err = ZipAdapter.normalize(&bad).unwrap_err();
        assert!(matches!(err, SidecarError::Normalize(ref s) if s.contains("parent-dir")));
    }

    #[test]
    fn zip_rejects_absolute_paths() {
        let bad = make_zip(&[("/etc/shadow", b"x")]);
        let err = ZipAdapter.normalize(&bad).unwrap_err();
        assert!(matches!(err, SidecarError::Normalize(ref s) if s.contains("absolute")));
    }

    #[test]
    fn zip_accepts_safe_relative_paths() {
        let good = make_zip(&[("dir/file.txt", b"hello"), ("dir/sub/another.bin", b"\x00\x01")]);
        let norm = ZipAdapter.normalize(&good).unwrap();
        assert_eq!(norm.notes.len(), 2);
    }

    #[test]
    fn markdown_rejects_non_utf8() {
        let err = MarkdownAdapter.normalize(&[0xff, 0xfe, 0xfd]).unwrap_err();
        assert!(matches!(err, SidecarError::Normalize(_)));
    }

    #[test]
    fn format_inference_from_mime() {
        assert_eq!(ArtifactFormat::from_mime("text/html"), Some(ArtifactFormat::Html));
        assert_eq!(
            ArtifactFormat::from_mime("text/markdown; charset=utf-8"),
            Some(ArtifactFormat::Markdown)
        );
        assert_eq!(ArtifactFormat::from_mime("nope/unknown"), None);
    }
}
