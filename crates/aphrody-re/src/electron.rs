// SPDX-License-Identifier: Apache-2.0
//! Electron binary analyser — decodes the Electron **fuse wire** and surfaces
//! Electron / Node / Chromium versions, V8 snapshot / code-cache markers, and
//! `ELECTRON_RUN_AS_NODE` capability from raw binary bytes.
//!
//! All detection runs on the raw `&[u8]` via `memchr::memmem` needle search and
//! `regex` for structured version patterns. No additional crates beyond those
//! already in the workspace. Never panics on arbitrary input.
//!
//! # Fuse wire format (authoritative, `@electron/fuses`)
//!
//! Electron embeds a 32-byte ASCII **sentinel** in its main binary, immediately
//! followed by the fuse wire:
//!
//! ```text
//! "dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX" [version:u8][length:u8][state byte ; length]
//! ```
//!
//! Each state byte is **not** `0/1/2` — the real alphabet (from the upstream
//! `FuseState` enum) is:
//!
//! - `0x30` (`'0'`) → `disable`
//! - `0x31` (`'1'`) → `enable`
//! - `0x72` (`'r'`) → `removed`
//! - `0x90`         → `inherit`
//!
//! The V1 fuse order is fixed: `RunAsNode`, `EnableCookieEncryption`,
//! `EnableNodeOptionsEnvironmentVariable`, `EnableNodeCliInspectArguments`,
//! `EnableEmbeddedAsarIntegrityValidation`, `OnlyLoadAppFromAsar`,
//! `LoadBrowserProcessSpecificV8Snapshot`, `GrantFileProtocolExtraPrivileges`,
//! `WasmTrapHandlers`.
//!
//! # Example
//!
//! ```
//! use aphrody_re::electron::analyze_electron;
//!
//! // A buffer that merely names Electron, with no fuse sentinel.
//! let buf = b"... Electron/37.0.0 chrome_100_percent.pak ...";
//! let report = analyze_electron(buf);
//! assert_eq!(report.electron_version.as_deref(), Some("37.0.0"));
//! assert!(!report.fuse_wire_found);
//! ```

use memchr::memmem;
use regex::bytes::Regex;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Constants — fuse wire alphabet + order (from @electron/fuses upstream)
// ---------------------------------------------------------------------------

/// The 32-byte ASCII fuse sentinel embedded by Electron ≥ 12.
pub const FUSE_SENTINEL: &[u8] = b"dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX";

/// Wire byte for a disabled fuse (`FuseState.DISABLE`, ASCII `'0'`).
const FUSE_DISABLE: u8 = 0x30;
/// Wire byte for an enabled fuse (`FuseState.ENABLE`, ASCII `'1'`).
const FUSE_ENABLE: u8 = 0x31;
/// Wire byte for a removed fuse (`FuseState.REMOVED`, ASCII `'r'`).
const FUSE_REMOVED: u8 = 0x72;
/// Wire byte for an inherited/default fuse (`FuseState.INHERIT`).
const FUSE_INHERIT: u8 = 0x90;

/// V1 fuse names in wire order (index = position in the wire).
const FUSE_NAMES_V1: &[&str] = &[
    "RunAsNode",
    "EnableCookieEncryption",
    "EnableNodeOptionsEnvironmentVariable",
    "EnableNodeCliInspectArguments",
    "EnableEmbeddedAsarIntegrityValidation",
    "OnlyLoadAppFromAsar",
    "LoadBrowserProcessSpecificV8Snapshot",
    "GrantFileProtocolExtraPrivileges",
    "WasmTrapHandlers",
];

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Decoded state of a single fuse. `state` is one of `enable`, `disable`,
/// `removed`, `inherit`, or `unknown` (for an out-of-spec wire byte).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FuseState {
    /// Fuse name (e.g. `"RunAsNode"`). For indices past the known V1 set the
    /// name is `"FuseIndex<N>"`.
    pub name: String,
    /// Decoded state string.
    pub state: String,
}

impl FuseState {
    fn decode(index: usize, raw: u8) -> Self {
        let name = FUSE_NAMES_V1
            .get(index)
            .map_or_else(|| format!("FuseIndex{index}"), |n| (*n).to_owned());
        let state = match raw {
            FUSE_ENABLE => "enable",
            FUSE_DISABLE => "disable",
            FUSE_REMOVED => "removed",
            FUSE_INHERIT => "inherit",
            _ => "unknown",
        }
        .to_owned();
        Self { name, state }
    }

    fn is_enabled(&self) -> bool {
        self.state == "enable"
    }
}

/// Full Electron analysis report. All `Vec` fields are always present (never
/// `null`); `Option` fields are `null` when not found.
#[derive(Debug, Clone, Serialize)]
pub struct ElectronReport {
    /// Electron version (`"M.m.p"`), from `Electron/<v>` or `electron@<v>`.
    pub electron_version: Option<String>,
    /// Chromium version (`"M.m.b.p"`), from `Chrome/<v>`.
    pub chromium_version: Option<String>,
    /// Node version (`"M.m.p"`), from `node/<v>`, `Node.js v<v>`, or `node@<v>`.
    pub node_version: Option<String>,
    /// Decoded fuse states (empty if no sentinel/wire was found).
    pub fuses: Vec<FuseState>,
    /// `true` if the fuse sentinel + a readable wire header were located.
    pub fuse_wire_found: bool,
    /// `true` if `ELECTRON_RUN_AS_NODE` is present as a literal **or** the
    /// `RunAsNode` fuse is enabled.
    pub run_as_node: bool,
    /// `true` if the `EnableEmbeddedAsarIntegrityValidation` fuse is enabled.
    pub asar_integrity: bool,
    /// `true` if the `OnlyLoadAppFromAsar` fuse is enabled.
    pub only_load_app_from_asar: bool,
    /// `true` if a V8 snapshot marker was found
    /// (`v8_context_snapshot` / `snapshot_blob` / `browser_v8_context_snapshot`).
    pub v8_snapshot: bool,
    /// `true` if a V8 code-cache marker was found (`v8-compile-cache`,
    /// `cachedDataRejected`, `bytenode`, `.jsc`).
    pub v8_code_cache: bool,
    /// Human-readable list of detection signals.
    pub indicators: Vec<String>,
}

// ---------------------------------------------------------------------------
// Main public entry point
// ---------------------------------------------------------------------------

/// Analyse a raw binary blob for Electron fuse + version artefacts.
///
/// Never panics on arbitrary input; all needle/regex calls operate on bounded
/// ranges and reads past the slice end are truncated, never indexed unchecked.
///
/// # Example
///
/// ```
/// use aphrody_re::electron::{analyze_electron, FUSE_SENTINEL};
///
/// // Synthetic fuse wire: sentinel + version(1) + length(2) + [enable, disable].
/// let mut buf = FUSE_SENTINEL.to_vec();
/// buf.extend_from_slice(&[0x01, 0x02, 0x31, 0x30]); // RunAsNode=enable, CookieEnc=disable
/// let r = analyze_electron(&buf);
/// assert!(r.fuse_wire_found);
/// assert_eq!(r.fuses.len(), 2);
/// assert_eq!(r.fuses[0].name, "RunAsNode");
/// assert_eq!(r.fuses[0].state, "enable");
/// assert!(r.run_as_node); // RunAsNode fuse enabled
/// ```
#[must_use]
pub fn analyze_electron(bytes: &[u8]) -> ElectronReport {
    let mut indicators: Vec<String> = Vec::new();

    // --- 1. Fuse wire ------------------------------------------------------
    let (fuses, fuse_wire_found) = decode_fuse_wire(bytes, &mut indicators);

    // --- 2. Versions -------------------------------------------------------
    let electron_version = extract_electron_version(bytes, &mut indicators);
    let chromium_version = extract_chromium_version(bytes, &mut indicators);
    let node_version = extract_node_version(bytes, &mut indicators);

    // --- 3. ELECTRON_RUN_AS_NODE literal ----------------------------------
    let run_as_node_literal = memmem::find(bytes, b"ELECTRON_RUN_AS_NODE").is_some();
    if run_as_node_literal {
        indicators.push("Marker: ELECTRON_RUN_AS_NODE".to_owned());
    }

    // --- 4. Derive verdicts from decoded fuses ----------------------------
    let fuse_run_as_node = fuse_enabled(&fuses, "RunAsNode");
    let asar_integrity = fuse_enabled(&fuses, "EnableEmbeddedAsarIntegrityValidation");
    let only_load_app_from_asar = fuse_enabled(&fuses, "OnlyLoadAppFromAsar");
    let run_as_node = run_as_node_literal || fuse_run_as_node;

    // --- 5. V8 markers -----------------------------------------------------
    let v8_snapshot = detect_v8_snapshot(bytes, &mut indicators);
    let v8_code_cache = detect_v8_code_cache(bytes, &mut indicators);

    ElectronReport {
        electron_version,
        chromium_version,
        node_version,
        fuses,
        fuse_wire_found,
        run_as_node,
        asar_integrity,
        only_load_app_from_asar,
        v8_snapshot,
        v8_code_cache,
        indicators,
    }
}

// ---------------------------------------------------------------------------
// Fuse wire decode
// ---------------------------------------------------------------------------

/// Locate the fuse sentinel and decode the trailing wire. Returns the decoded
/// fuses and whether a sentinel + readable header was found.
///
/// Wire layout after the 32-byte sentinel: `[version:u8][length:u8][state ; length]`.
/// Reads are truncated at the slice end; a sentinel at the very tail with no
/// header bytes yields `fuse_wire_found = false`.
fn decode_fuse_wire(bytes: &[u8], indicators: &mut Vec<String>) -> (Vec<FuseState>, bool) {
    let Some(pos) = memmem::find(bytes, FUSE_SENTINEL) else {
        return (Vec::new(), false);
    };
    indicators.push(format!("Fuse sentinel @ offset {pos}"));

    let header = pos + FUSE_SENTINEL.len();
    // Need at least the 2-byte header (version + length) to call it a wire.
    let Some(&version) = bytes.get(header) else {
        return (Vec::new(), false);
    };
    let Some(&length) = bytes.get(header + 1) else {
        return (Vec::new(), false);
    };
    indicators.push(format!("Fuse wire version {version}, length {length}"));

    let wire_start = header + 2;
    // Clamp the requested length to what is actually present in the slice so a
    // truncated/corrupt binary cannot cause an out-of-range read.
    let available = bytes.len().saturating_sub(wire_start);
    let take = usize::from(length).min(available);

    let mut fuses = Vec::with_capacity(take);
    for i in 0..take {
        let raw = bytes[wire_start + i];
        let fuse = FuseState::decode(i, raw);
        if fuse.is_enabled() {
            indicators.push(format!("Fuse enabled: {}", fuse.name));
        }
        fuses.push(fuse);
    }

    (fuses, true)
}

fn fuse_enabled(fuses: &[FuseState], name: &str) -> bool {
    fuses.iter().any(|f| f.name == name && f.is_enabled())
}

// ---------------------------------------------------------------------------
// Version extraction
// ---------------------------------------------------------------------------

fn extract_first(bytes: &[u8], pattern: &str, label: &str, indicators: &mut Vec<String>) -> Option<String> {
    let re = Regex::new(pattern).expect("static regex");
    let cap = re.captures(bytes)?;
    let m = cap.get(1)?;
    let ver = std::str::from_utf8(m.as_bytes()).ok()?.to_owned();
    indicators.push(format!("{label}: {ver}"));
    Some(ver)
}

fn extract_electron_version(bytes: &[u8], indicators: &mut Vec<String>) -> Option<String> {
    // `Electron/37.0.0` (user-agent) or `electron@37.0.0` (package spec).
    extract_first(bytes, r"[Ee]lectron[/@](\d+\.\d+\.\d+(?:[-.\w]+)?)", "Electron version", indicators)
}

fn extract_chromium_version(bytes: &[u8], indicators: &mut Vec<String>) -> Option<String> {
    // `Chrome/M.m.b.p` (user-agent style).
    extract_first(bytes, r"Chrome/(\d+\.\d+\.\d+\.\d+)", "Chromium version", indicators)
}

fn extract_node_version(bytes: &[u8], indicators: &mut Vec<String>) -> Option<String> {
    // `node/22.16.0`, `node@22.16.0`, or `Node.js v22.16.0`.
    extract_first(
        bytes,
        r"(?:node[/@]|Node\.js v)(\d+\.\d+\.\d+(?:[-.\w]+)?)",
        "Node version",
        indicators,
    )
}

// ---------------------------------------------------------------------------
// V8 markers
// ---------------------------------------------------------------------------

fn detect_v8_snapshot(bytes: &[u8], indicators: &mut Vec<String>) -> bool {
    let needles: &[&[u8]] = &[
        b"browser_v8_context_snapshot",
        b"v8_context_snapshot",
        b"snapshot_blob",
    ];
    let mut found = false;
    for needle in needles {
        if memmem::find(bytes, needle).is_some() {
            indicators.push(format!(
                "V8 snapshot marker: {}",
                std::str::from_utf8(needle).unwrap_or("<binary>")
            ));
            found = true;
        }
    }
    found
}

fn detect_v8_code_cache(bytes: &[u8], indicators: &mut Vec<String>) -> bool {
    let needles: &[&[u8]] = &[
        b"v8-compile-cache",
        b"cachedDataRejected",
        b"produceCachedData",
        b"bytenode",
    ];
    let mut found = false;
    for needle in needles {
        if memmem::find(bytes, needle).is_some() {
            indicators.push(format!(
                "V8 code-cache marker: {}",
                std::str::from_utf8(needle).unwrap_or("<binary>")
            ));
            found = true;
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic fuse wire: sentinel + version + length + state bytes.
    fn make_wire(version: u8, states: &[u8]) -> Vec<u8> {
        let mut buf = b"PADDING_PREFIX_BYTES".to_vec();
        buf.extend_from_slice(FUSE_SENTINEL);
        buf.push(version);
        buf.push(states.len() as u8);
        buf.extend_from_slice(states);
        buf.extend_from_slice(b"TRAILING");
        buf
    }

    #[test]
    fn decodes_full_v1_wire() {
        // All 9 V1 fuses with a mix of states.
        let states = [
            FUSE_ENABLE,   // RunAsNode
            FUSE_DISABLE,  // EnableCookieEncryption
            FUSE_DISABLE,  // EnableNodeOptionsEnvironmentVariable
            FUSE_DISABLE,  // EnableNodeCliInspectArguments
            FUSE_ENABLE,   // EnableEmbeddedAsarIntegrityValidation
            FUSE_ENABLE,   // OnlyLoadAppFromAsar
            FUSE_DISABLE,  // LoadBrowserProcessSpecificV8Snapshot
            FUSE_ENABLE,   // GrantFileProtocolExtraPrivileges
            FUSE_ENABLE,   // WasmTrapHandlers
        ];
        let buf = make_wire(1, &states);
        let r = analyze_electron(&buf);

        assert!(r.fuse_wire_found, "sentinel + header must be found");
        assert_eq!(r.fuses.len(), 9);
        assert_eq!(r.fuses[0].name, "RunAsNode");
        assert_eq!(r.fuses[0].state, "enable");
        assert_eq!(r.fuses[4].name, "EnableEmbeddedAsarIntegrityValidation");
        assert_eq!(r.fuses[4].state, "enable");

        // Derived verdicts.
        assert!(r.run_as_node, "RunAsNode enabled => run_as_node true");
        assert!(r.asar_integrity, "integrity fuse enabled");
        assert!(r.only_load_app_from_asar, "OnlyLoadAppFromAsar enabled");
    }

    #[test]
    fn decodes_removed_and_inherit_states() {
        let states = [FUSE_REMOVED, FUSE_INHERIT, 0xFF];
        let buf = make_wire(1, &states);
        let r = analyze_electron(&buf);
        assert!(r.fuse_wire_found);
        assert_eq!(r.fuses[0].state, "removed");
        assert_eq!(r.fuses[1].state, "inherit");
        // Out-of-spec byte => "unknown", never a panic.
        assert_eq!(r.fuses[2].state, "unknown");
    }

    #[test]
    fn no_sentinel_means_no_wire() {
        let buf = b"this binary has no electron fuse sentinel at all".to_vec();
        let r = analyze_electron(&buf);
        assert!(!r.fuse_wire_found);
        assert!(r.fuses.is_empty());
        assert!(!r.run_as_node);
        assert!(!r.asar_integrity);
    }

    #[test]
    fn sentinel_at_tail_without_header_is_not_a_wire() {
        // Sentinel present but no version/length bytes follow.
        let buf = FUSE_SENTINEL.to_vec();
        let r = analyze_electron(&buf);
        assert!(!r.fuse_wire_found, "no header bytes => not a valid wire");
        assert!(r.fuses.is_empty());
    }

    #[test]
    fn sentinel_with_version_but_no_length_is_not_a_wire() {
        let mut buf = FUSE_SENTINEL.to_vec();
        buf.push(0x01); // version only
        let r = analyze_electron(&buf);
        assert!(!r.fuse_wire_found);
    }

    #[test]
    fn truncated_wire_is_clamped_not_panicked() {
        // length byte claims 9 fuses but only 2 state bytes are present.
        let mut buf = FUSE_SENTINEL.to_vec();
        buf.push(0x01); // version
        buf.push(0x09); // claims 9
        buf.extend_from_slice(&[FUSE_ENABLE, FUSE_DISABLE]); // only 2
        let r = analyze_electron(&buf);
        assert!(r.fuse_wire_found);
        assert_eq!(r.fuses.len(), 2, "must clamp to available bytes");
        assert!(r.run_as_node);
    }

    #[test]
    fn extracts_electron_version_ua_and_pkg() {
        let r1 = analyze_electron(b"Mozilla/5.0 Electron/37.2.4 Safari");
        assert_eq!(r1.electron_version.as_deref(), Some("37.2.4"));
        let r2 = analyze_electron(b"dep: electron@28.0.0-beta.1 installed");
        assert_eq!(r2.electron_version.as_deref(), Some("28.0.0-beta.1"));
    }

    #[test]
    fn extracts_chromium_and_node_versions() {
        let buf = b"Chrome/124.0.6367.243 node/22.16.0 misc Node.js v18.20.0";
        let r = analyze_electron(buf);
        assert_eq!(r.chromium_version.as_deref(), Some("124.0.6367.243"));
        // First node match wins (`node/22.16.0`).
        assert_eq!(r.node_version.as_deref(), Some("22.16.0"));
    }

    #[test]
    fn run_as_node_literal_sets_flag_without_fuse() {
        let buf = b"env check ELECTRON_RUN_AS_NODE present";
        let r = analyze_electron(buf);
        assert!(r.run_as_node, "literal alone sets run_as_node");
        assert!(!r.fuse_wire_found);
        assert!(r.indicators.iter().any(|s| s.contains("ELECTRON_RUN_AS_NODE")));
    }

    #[test]
    fn detects_v8_snapshot_and_code_cache() {
        let buf = b"v8_context_snapshot.bin browser_v8_context_snapshot.bin v8-compile-cache bytenode";
        let r = analyze_electron(buf);
        assert!(r.v8_snapshot);
        assert!(r.v8_code_cache);
        assert!(r.indicators.iter().any(|s| s.contains("V8 snapshot")));
        assert!(r.indicators.iter().any(|s| s.contains("V8 code-cache")));
    }

    #[test]
    fn empty_input_is_all_false_no_panic() {
        let r = analyze_electron(b"");
        assert!(!r.fuse_wire_found);
        assert!(r.fuses.is_empty());
        assert!(!r.run_as_node);
        assert!(!r.v8_snapshot);
        assert!(!r.v8_code_cache);
        assert!(r.electron_version.is_none());
        assert!(r.chromium_version.is_none());
        assert!(r.node_version.is_none());
    }

    #[test]
    fn report_serializes_stable_json_shape() {
        let buf = make_wire(1, &[FUSE_ENABLE, FUSE_DISABLE]);
        let r = analyze_electron(&buf);
        let json = serde_json::to_value(&r).expect("serialize");
        assert!(json["fuses"].is_array());
        assert!(json["fuse_wire_found"].is_boolean());
        assert!(json["run_as_node"].is_boolean());
        assert!(json["asar_integrity"].is_boolean());
        assert!(json["only_load_app_from_asar"].is_boolean());
        assert!(json["v8_snapshot"].is_boolean());
        assert!(json["v8_code_cache"].is_boolean());
        assert!(json["indicators"].is_array());
        assert_eq!(json["fuses"][0]["name"], "RunAsNode");
        assert_eq!(json["fuses"][0]["state"], "enable");
    }

    #[test]
    fn arbitrary_garbage_never_panics() {
        // Fuzz-ish: many short slices including partial sentinels.
        for len in 0..40usize {
            let buf: Vec<u8> = FUSE_SENTINEL.iter().copied().take(len).collect();
            let _ = analyze_electron(&buf);
        }
        let weird = vec![0xFFu8; 1000];
        let _ = analyze_electron(&weird);
    }
}
