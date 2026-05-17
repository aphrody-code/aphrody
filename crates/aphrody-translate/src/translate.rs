// SPDX-License-Identifier: Apache-2.0
//! Traduction EN -> FR via MyMemory (API publique gratuite, sans clé).
//!
//! Sur les cibles natives (Linux / Windows / macOS) :
//! - Quota anonyme : 5000 mots/jour. Avec email noreply : 50000 mots/jour.
//! - Tout résultat est mis en cache disque dans `.aphrody-translate-cache.json`.
//!   Les ré-exécutions sont alors instantanées.
//! - Si l'API renvoie une erreur ou est saturée, le texte source est retourné
//!   tel quel et un avertissement est tracé.
//!
//! Sur les cibles WebAssembly (`wasm32-unknown-unknown`, `wasm32-wasip1`) :
//! - La traduction réseau n'est pas disponible : `translate()` retourne le
//!   texte source inchangé. L'appelant doit passer `--no-translate` ou utiliser
//!   la lib pour les étapes scrub/aphrodify uniquement.

use std::path::{Path, PathBuf};

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use tracing::warn;

#[cfg(not(target_arch = "wasm32"))]
const ENDPOINT: &str = "https://api.mymemory.translated.net/get";

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: MyMemoryData,
    #[serde(rename = "responseStatus")]
    response_status: u32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Deserialize)]
struct MyMemoryData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CacheFile {
    pub entries: std::collections::BTreeMap<String, String>,
}

pub struct Translator {
    #[cfg(not(target_arch = "wasm32"))]
    client: reqwest::Client,
    cache: Arc<DashMap<String, String>>,
    /// Disk path for persistent cache. Empty path means cache is in-memory only.
    /// Field absent on `wasm32` where no file-system write is performed.
    #[cfg(not(target_arch = "wasm32"))]
    cache_path: PathBuf,
    /// Optional contact email forwarded to MyMemory for higher quota.
    /// Field absent on `wasm32` where no HTTP call is made.
    #[cfg(not(target_arch = "wasm32"))]
    contact_email: Option<String>,
}

impl Translator {
    /// Construct a [`Translator`] loading the on-disk cache when available.
    ///
    /// On `wasm32` targets the cache is in-memory only and no HTTP client is
    /// built (network translation is unavailable without a JS runtime or WASI
    /// networking).
    pub fn new(cache_path: PathBuf, contact_email: Option<String>) -> Result<Self> {
        let cache: Arc<DashMap<String, String>> = Arc::new(DashMap::new());

        #[cfg(not(target_arch = "wasm32"))]
        {
            use anyhow::Context as _;

            let client = reqwest::Client::builder()
                .user_agent("aphrody-translate/1.0")
                .timeout(std::time::Duration::from_secs(15))
                .build()?;

            if cache_path.exists() {
                let raw = std::fs::read_to_string(&cache_path)
                    .with_context(|| format!("read cache {:?}", cache_path))?;
                let file: CacheFile = serde_json::from_str(&raw).unwrap_or_default();
                for (k, v) in file.entries {
                    cache.insert(k, v);
                }
            }

            return Ok(Self { client, cache, cache_path, contact_email });
        }

        // wasm32: in-memory only, no HTTP client and no disk fields.
        #[cfg(target_arch = "wasm32")]
        {
            let _ = cache_path;
            let _ = contact_email;
            Ok(Self { cache })
        }
    }

    fn key(text: &str) -> String {
        let mut h = Sha256::new();
        h.update(text.as_bytes());
        hex::encode(h.finalize())
    }

    /// Translate `text` from English to French.
    ///
    /// On native targets this calls MyMemory and caches the result on disk.
    /// On wasm32 targets the input is returned unchanged (no network backend
    /// is available).
    pub async fn translate(&self, text: &str) -> Result<String> {
        let stripped = text.trim();
        if stripped.is_empty() {
            return Ok(String::new());
        }

        // In-memory cache hit — works on all targets.
        let key = Self::key(stripped);
        if let Some(hit) = self.cache.get(&key) {
            return Ok(hit.clone());
        }

        // wasm32: no network, return source text unchanged.
        #[cfg(target_arch = "wasm32")]
        {
            return Ok(stripped.to_string());
        }

        // Native: call MyMemory.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut url =
                format!("{ENDPOINT}?q={}&langpair=en|fr", urlencoding::encode(stripped));
            if let Some(mail) = &self.contact_email {
                url.push_str(&format!("&de={}", urlencoding::encode(mail)));
            }

            match self.client.get(&url).send().await {
                Ok(resp) => match resp.json::<MyMemoryResponse>().await {
                    Ok(parsed) if parsed.response_status == 200 => {
                        let fr = parsed.response_data.translated_text;
                        self.cache.insert(key, fr.clone());
                        Ok(fr)
                    },
                    Ok(other) => {
                        warn!(status = other.response_status, "mymemory non-200 status");
                        Ok(stripped.to_string())
                    },
                    Err(e) => {
                        warn!(error = %e, "mymemory parse error");
                        Ok(stripped.to_string())
                    },
                },
                Err(e) => {
                    warn!(error = %e, "mymemory network error");
                    Ok(stripped.to_string())
                },
            }
        }
    }

    /// Persist the in-memory cache to disk.
    ///
    /// This is a no-op on `wasm32` targets where no file system is available.
    pub fn persist(&self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            return Ok(());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use anyhow::Context as _;

            let mut file = CacheFile::default();
            for r in self.cache.iter() {
                file.entries.insert(r.key().clone(), r.value().clone());
            }
            let raw = serde_json::to_string_pretty(&file)?;
            if let Some(parent) = self.cache_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&self.cache_path, raw).context("persist cache")?;
            Ok(())
        }
    }
}

pub fn default_cache_path(root: &Path) -> PathBuf {
    root.join(".aphrody-translate-cache.json")
}
