// SPDX-License-Identifier: Apache-2.0
//! Aphrody extension marketplace index.
//!
//! A small, async-friendly, dependency-light catalog of installable extensions
//! (skills, MCP servers, hooks, themes). The crate ships three pluggable
//! loaders implementing the [`IndexLoader`] trait:
//!
//! * [`EmbeddedIndexLoader`] — compile-time baked entries used for offline
//!   bootstrapping (CI, hermetic builds, first-run with no network).
//! * [`FileIndexLoader`] — reads a marketplace index from a local JSON file.
//! * [`HttpIndexLoader`] — fetches a marketplace index over HTTPS, with
//!   `rustls-tls` and a strict 10s timeout.
//!
//! The on-disk schema is intentionally narrower than the lint-engine
//! frontmatter shape: the marketplace concerns itself with *discoverability +
//! integrity*, not with the source layout of any given extension. See
//! [`ExtensionEntry`] for the canonical record shape.
//!
//! ## Integrity
//!
//! Every entry advertises a SHA-256 of the artifact it points to. The
//! [`verify_sha256`] helper is exposed so installers can verify downloads
//! against the index before persisting them to disk.
//!
//! ## Example — load the embedded index and pick the latest skill
//!
//! ```no_run
//! use aphrody_marketplace::{EmbeddedIndexLoader, ExtensionKind, IndexLoader};
//!
//! # async fn run() -> Result<(), aphrody_marketplace::MarketplaceError> {
//! let loader = EmbeddedIndexLoader::default();
//! let index  = loader.load_index().await?;
//! let skills = index.find_by_kind(ExtensionKind::Skill);
//! assert!(!skills.is_empty());
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

pub mod awesome;

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Kind of artifact an [`ExtensionEntry`] describes.
///
/// The marketplace deliberately limits the namespace to the four extension
/// surfaces aphrody actually consumes — skills (`SKILL.md`), MCP servers,
/// editor hooks, and visual themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionKind {
    /// A `SKILL.md`-shaped agent skill (cf. `crates/aphrody-skills-forge`).
    Skill,
    /// A Model Context Protocol stdio / HTTP server.
    Mcp,
    /// A repository hook (pre-commit, post-merge…) delivered as a Rust binary.
    Hook,
    /// A terminal / UI theme bundle.
    Theme,
}

impl ExtensionKind {
    /// Canonical lowercase identifier used in JSON / CLI surfaces.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Mcp => "mcp",
            Self::Hook => "hook",
            Self::Theme => "theme",
        }
    }
}

/// One catalog entry — a single installable artifact at a specific version.
///
/// Multiple versions of the same logical extension are represented as
/// distinct [`ExtensionEntry`] rows sharing the same `id`. Callers use
/// [`MarketplaceIndex::latest_version`] /
/// [`MarketplaceIndex::compatible_with`] to pick one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionEntry {
    /// Stable kebab-case identifier (e.g. `aphrody-terminal-vt`).
    pub id: String,
    /// Display name shown in UI lists.
    pub name: String,
    /// SemVer version of *this* row. Round-trips via [`semver::Version`].
    pub version: Version,
    /// One-line marketing description.
    pub description: String,
    /// Artifact kind (`Skill`, `Mcp`, `Hook`, `Theme`).
    pub kind: ExtensionKind,
    /// Where to fetch the artifact from (HTTPS or `file://` URL).
    pub source_url: String,
    /// SHA-256 of the artifact body, hex-encoded (64 lowercase hex chars).
    pub sha256: String,
    /// Free-form author label (org or handle).
    pub author: String,
    /// Tags used by [`MarketplaceIndex::find_by_tag`].
    #[serde(default)]
    pub tags: Vec<String>,
    /// Runtime requirement tag (e.g. `rust`, `wasm`, `mcp-stdio`).
    pub runtime: String,
    /// IDs of other extensions this one needs at install-time.
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Top-level index document. Round-trips via JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceIndex {
    /// Catalog rows. Order is preserved but not significant.
    pub entries: Vec<ExtensionEntry>,
    /// UTC timestamp the index was published. Used by clients to decide
    /// whether to refresh.
    pub updated_at: DateTime<Utc>,
}

impl MarketplaceIndex {
    /// Build an index from `entries`, stamping `updated_at` to *now*.
    #[must_use]
    pub fn new(entries: Vec<ExtensionEntry>) -> Self {
        Self {
            entries,
            updated_at: Utc::now(),
        }
    }

    /// All rows sharing the requested `id` (one per published version).
    #[must_use]
    pub fn find_by_id<'a>(&'a self, id: &str) -> Vec<&'a ExtensionEntry> {
        self.entries.iter().filter(|e| e.id == id).collect()
    }

    /// Filter rows by [`ExtensionKind`].
    #[must_use]
    pub fn find_by_kind(&self, kind: ExtensionKind) -> Vec<&ExtensionEntry> {
        self.entries.iter().filter(|e| e.kind == kind).collect()
    }

    /// Rows whose [`ExtensionEntry::tags`] contain `tag` (exact, case-sensitive).
    #[must_use]
    pub fn find_by_tag<'a>(&'a self, tag: &str) -> Vec<&'a ExtensionEntry> {
        self.entries
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Latest semver-sorted row matching `id`, or `None` if absent.
    #[must_use]
    pub fn latest_version(&self, id: &str) -> Option<&ExtensionEntry> {
        self.find_by_id(id).into_iter().max_by(|a, b| a.version.cmp(&b.version))
    }

    /// All rows of `id` whose [`ExtensionEntry::version`] satisfies `req`,
    /// sorted ascending.
    #[must_use]
    pub fn compatible_with(&self, id: &str, req: &VersionReq) -> Vec<&ExtensionEntry> {
        let mut hits: Vec<&ExtensionEntry> = self
            .find_by_id(id)
            .into_iter()
            .filter(|e| req.matches(&e.version))
            .collect();
        hits.sort_by(|a, b| a.version.cmp(&b.version));
        hits
    }

    /// Distinct extension ids in the catalog.
    #[must_use]
    pub fn ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.entries.iter().map(|e| e.id.clone()).collect();
        v.sort();
        v.dedup();
        v
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure surface for every marketplace operation.
#[derive(Debug, Error)]
pub enum MarketplaceError {
    /// HTTP request failed (DNS, connect, TLS, decode, non-2xx).
    #[error("http transport error: {0}")]
    Http(String),

    /// Local filesystem I/O failure.
    #[error("io error on {path}: {source}")]
    Io {
        /// Path that caused the failure.
        path: PathBuf,
        /// Underlying OS-level error.
        #[source]
        source: std::io::Error,
    },

    /// JSON (de)serialisation failure.
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML (de)serialisation failure (only used by optional ingestion paths).
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Invalid semver string when projecting raw input.
    #[error("semver parse error: {0}")]
    Semver(#[from] semver::Error),

    /// The requested id / version / tag is absent from the index.
    #[error("not found: {0}")]
    NotFound(String),

    /// Artifact bytes did not match the expected SHA-256.
    #[error("integrity mismatch: expected {expected}, got {got}")]
    IntegrityMismatch {
        /// SHA-256 advertised by the index.
        expected: String,
        /// SHA-256 actually computed over the bytes.
        got: String,
    },
}

impl From<reqwest::Error> for MarketplaceError {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err.to_string())
    }
}

/// Result alias used by all loader entry points.
pub type MarketplaceResult<T> = Result<T, MarketplaceError>;

// ---------------------------------------------------------------------------
// Loader trait + impls
// ---------------------------------------------------------------------------

/// Pluggable index source. Implementors fetch / decode an index from
/// somewhere (file, network, in-memory).
#[async_trait]
pub trait IndexLoader: Send + Sync {
    /// Produce a fresh [`MarketplaceIndex`].
    async fn load_index(&self) -> MarketplaceResult<MarketplaceIndex>;
}

/// File-backed loader — reads JSON from `path`.
#[derive(Debug, Clone)]
pub struct FileIndexLoader {
    path: PathBuf,
}

impl FileIndexLoader {
    /// Wrap an on-disk path. The file is read lazily on each
    /// [`IndexLoader::load_index`] call.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl IndexLoader for FileIndexLoader {
    async fn load_index(&self) -> MarketplaceResult<MarketplaceIndex> {
        let path = self.path.clone();
        let bytes = tokio::fs::read(&path).await.map_err(|source| MarketplaceError::Io {
            path: path.clone(),
            source,
        })?;
        let idx: MarketplaceIndex = serde_json::from_slice(&bytes)?;
        tracing::debug!(
            target: "aphrody_marketplace::file",
            entries = idx.entries.len(),
            path = %path.display(),
            "loaded marketplace index from file",
        );
        Ok(idx)
    }
}

/// HTTPS-backed loader — fetches JSON from `url`.
#[derive(Debug, Clone)]
pub struct HttpIndexLoader {
    url: String,
    client: reqwest::Client,
}

impl HttpIndexLoader {
    /// Build a loader with a strict 10s connect+read timeout. The supplied URL
    /// must be reachable via HTTPS in production.
    ///
    /// # Errors
    ///
    /// Returns [`MarketplaceError::Http`] if reqwest cannot build a client
    /// (TLS backend unavailable, malformed default headers…).
    pub fn new(url: impl Into<String>) -> MarketplaceResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("aphrody-marketplace/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| MarketplaceError::Http(e.to_string()))?;
        Ok(Self {
            url: url.into(),
            client,
        })
    }

    /// Inject a pre-built [`reqwest::Client`] — useful in tests where the
    /// caller wants a non-default transport (e.g. wiremock).
    pub fn with_client(url: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            url: url.into(),
            client,
        }
    }
}

#[async_trait]
impl IndexLoader for HttpIndexLoader {
    async fn load_index(&self) -> MarketplaceResult<MarketplaceIndex> {
        let resp = self.client.get(&self.url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(MarketplaceError::Http(format!(
                "GET {} returned HTTP {}",
                self.url, status
            )));
        }
        let bytes = resp.bytes().await?;
        let idx: MarketplaceIndex = serde_json::from_slice(&bytes)?;
        tracing::debug!(
            target: "aphrody_marketplace::http",
            entries = idx.entries.len(),
            url = %self.url,
            "loaded marketplace index over http",
        );
        Ok(idx)
    }
}

/// Compile-time, network-free loader.
///
/// Always succeeds. Use this for hermetic builds and as a sane first-run
/// catalog before the agent has a chance to fetch the live index.
#[derive(Debug, Clone, Default)]
pub struct EmbeddedIndexLoader;

#[async_trait]
impl IndexLoader for EmbeddedIndexLoader {
    async fn load_index(&self) -> MarketplaceResult<MarketplaceIndex> {
        Ok(embedded_index())
    }
}

// ---------------------------------------------------------------------------
// Embedded baseline catalog
// ---------------------------------------------------------------------------

/// The baked-in seed index. Mutating these entries is a workspace-wide event:
/// every consumer ships them as the first-run default. Real provider names
/// follow the aphrody whitelist (anthropic / gemini / antigravity only — no
/// openai / mistral / cohere references).
#[must_use]
pub fn embedded_index() -> MarketplaceIndex {
    let entries = vec![
        ExtensionEntry {
            id: "aphrody-terminal-vt".to_string(),
            name: "aphrody Terminal VT".to_string(),
            version: Version::new(0, 1, 0),
            description: "LLM-first VT/ANSI parser with alt-screen, SGR-1006 mouse, OSC 52 clipboard support.".to_string(),
            kind: ExtensionKind::Skill,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/aphrody-terminal-vt".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["terminal".to_string(), "vt".to_string(), "ansi".to_string()],
            runtime: "rust".to_string(),
            dependencies: vec![],
        },
        ExtensionEntry {
            id: "aphrody-skills-forge".to_string(),
            name: "aphrody Skills Forge".to_string(),
            version: Version::new(0, 1, 0),
            description: "Programmatic SKILL.md scaffolding, registry loader, and lint engine.".to_string(),
            kind: ExtensionKind::Skill,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/aphrody-skills-forge".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["skill".to_string(), "scaffold".to_string(), "lint".to_string()],
            runtime: "rust".to_string(),
            dependencies: vec![],
        },
        ExtensionEntry {
            id: "aphrody-cron".to_string(),
            name: "aphrody Cron".to_string(),
            version: Version::new(0, 1, 0),
            description: "Async-friendly interval/daily/cron scheduler with pluggable JSON-backed job store.".to_string(),
            kind: ExtensionKind::Skill,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/aphrody-cron".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000003".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["scheduler".to_string(), "cron".to_string()],
            runtime: "rust".to_string(),
            dependencies: vec![],
        },
        ExtensionEntry {
            id: "aphrody-cron".to_string(),
            name: "aphrody Cron".to_string(),
            version: Version::new(0, 2, 0),
            description: "Async-friendly interval/daily/cron scheduler — adds JSONL persistence + SIGINT trap.".to_string(),
            kind: ExtensionKind::Skill,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/aphrody-cron".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000004".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["scheduler".to_string(), "cron".to_string()],
            runtime: "rust".to_string(),
            dependencies: vec![],
        },
        ExtensionEntry {
            id: "anthropic-mcp".to_string(),
            name: "Anthropic MCP Bridge".to_string(),
            version: Version::new(0, 1, 0),
            description: "Native Rust MCP stdio adapter for the Anthropic provider channel.".to_string(),
            kind: ExtensionKind::Mcp,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/aphrody-mcp".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000005".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["mcp".to_string(), "anthropic".to_string()],
            runtime: "mcp-stdio".to_string(),
            dependencies: vec![],
        },
        ExtensionEntry {
            id: "gemini-mcp".to_string(),
            name: "Gemini MCP Bridge".to_string(),
            version: Version::new(0, 1, 0),
            description: "Native Rust MCP stdio adapter for the Gemini provider channel.".to_string(),
            kind: ExtensionKind::Mcp,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/aphrody-mcp".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000006".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["mcp".to_string(), "gemini".to_string()],
            runtime: "mcp-stdio".to_string(),
            dependencies: vec![],
        },
        ExtensionEntry {
            id: "antigravity-mcp".to_string(),
            name: "Antigravity MCP Bridge".to_string(),
            version: Version::new(0, 1, 0),
            description: "Native Rust MCP stdio adapter for the Antigravity provider channel.".to_string(),
            kind: ExtensionKind::Mcp,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/aphrody-mcp".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000007".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["mcp".to_string(), "antigravity".to_string()],
            runtime: "mcp-stdio".to_string(),
            dependencies: vec![],
        },
        ExtensionEntry {
            id: "m3-tokens-theme".to_string(),
            name: "Material Design 3 Tokens".to_string(),
            version: Version::new(1, 0, 0),
            description: "Baseline M3 design tokens — colour, typography, elevation, motion.".to_string(),
            kind: ExtensionKind::Theme,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/m3-tokens".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000008".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["theme".to_string(), "material".to_string(), "m3".to_string()],
            runtime: "wasm".to_string(),
            dependencies: vec![],
        },
        ExtensionEntry {
            id: "oxclint-hook".to_string(),
            name: "oxclint pre-commit hook".to_string(),
            version: Version::new(0, 3, 0),
            description: "Pre-commit hook running oxclint over staged JS/TS migration candidates.".to_string(),
            kind: ExtensionKind::Hook,
            source_url: "https://github.com/aphrody-code/aphrody/tree/main/crates/aphrody-xtask".to_string(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000009".to_string(),
            author: "aphrody-code".to_string(),
            tags: vec!["hook".to_string(), "lint".to_string(), "pre-commit".to_string()],
            runtime: "rust".to_string(),
            dependencies: vec!["aphrody-skills-forge".to_string()],
        },
    ];
    // Fixed updated_at so tests + cache validators are deterministic. Real
    // remote indices override this on publish.
    let updated_at = Utc
        .with_ymd_and_hms(2026, 5, 19, 0, 0, 0)
        .single()
        .unwrap_or_else(Utc::now);
    MarketplaceIndex {
        entries,
        updated_at,
    }
}

// ---------------------------------------------------------------------------
// Integrity helper
// ---------------------------------------------------------------------------

/// Verify `bytes` hashes to `expected` (lowercase hex, 64 chars).
///
/// # Errors
///
/// Returns [`MarketplaceError::IntegrityMismatch`] when the computed and
/// expected digests differ.
pub fn verify_sha256(bytes: &[u8], expected: &str) -> MarketplaceResult<()> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let got = format!("{:x}", hasher.finalize());
    if got.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(MarketplaceError::IntegrityMismatch {
            expected: expected.to_ascii_lowercase(),
            got,
        })
    }
}

/// Convenience: re-export a [`FileIndexLoader`] pointing at the conventional
/// `~/.config/aphrody/marketplace.json` location.
#[must_use]
pub fn default_user_index_path() -> PathBuf {
    dirs_home().join(".config").join("aphrody").join("marketplace.json")
}

fn dirs_home() -> PathBuf {
    // Avoid pulling the `dirs` crate just for one call; honour XDG /
    // platform envs by hand. Defaults to the current dir if neither resolves.
    if let Some(h) = std::env::var_os("HOME") {
        return PathBuf::from(h);
    }
    if let (Some(drive), Some(path)) = (
        std::env::var_os("HOMEDRIVE"),
        std::env::var_os("HOMEPATH"),
    ) {
        let mut p = PathBuf::from(drive);
        p.push(path);
        return p;
    }
    PathBuf::from(".")
}

/// Convenience: validate that `path` looks like a JSON file we can later read.
///
/// Pure path-shape check — does not touch the filesystem. Useful for CLI
/// argument validation.
#[must_use]
pub fn looks_like_json_path(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase) == Some("json".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_round_trips_through_json() {
        let idx = embedded_index();
        let s = serde_json::to_string(&idx).expect("ser");
        let back: MarketplaceIndex = serde_json::from_str(&s).expect("de");
        assert_eq!(idx, back);
    }

    #[test]
    fn kind_str_is_lowercase() {
        assert_eq!(ExtensionKind::Skill.as_str(), "skill");
        assert_eq!(ExtensionKind::Mcp.as_str(), "mcp");
        assert_eq!(ExtensionKind::Hook.as_str(), "hook");
        assert_eq!(ExtensionKind::Theme.as_str(), "theme");
    }

    #[test]
    fn verify_sha256_matches_known_vector() {
        // SHA-256("abc") = ba7816bf...f20015ad
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        verify_sha256(b"abc", expected).expect("match");
    }

    #[test]
    fn verify_sha256_rejects_wrong_digest() {
        let err = verify_sha256(b"abc", &"0".repeat(64)).unwrap_err();
        assert!(matches!(err, MarketplaceError::IntegrityMismatch { .. }));
    }

    #[test]
    fn looks_like_json_path_accepts_capitalised() {
        assert!(looks_like_json_path(Path::new("foo.JSON")));
        assert!(!looks_like_json_path(Path::new("foo.yaml")));
    }
}
