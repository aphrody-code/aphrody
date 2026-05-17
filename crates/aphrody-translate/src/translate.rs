//! Traduction EN → FR via MyMemory (API publique gratuite, sans clé).
//!
//! - Quota anonyme : 5000 mots/jour. Avec email noreply : 50000 mots/jour.
//! - Tout résultat est mis en cache disque dans `.aphrody-translate-cache.json` à la racine du
//!   projet. Les ré-exécutions sont alors instantanées.
//! - Si l'API renvoie une erreur ou est saturée, le texte source est laissé tel quel et un
//!   avertissement est tracé.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::warn;

const ENDPOINT: &str = "https://api.mymemory.translated.net/get";

#[derive(Debug, Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: MyMemoryData,
    #[serde(rename = "responseStatus")]
    response_status: u32,
}

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
    client: reqwest::Client,
    cache: Arc<DashMap<String, String>>,
    cache_path: PathBuf,
    contact_email: Option<String>,
}

impl Translator {
    pub fn new(cache_path: PathBuf, contact_email: Option<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("aphrody-translate/1.0")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let cache: Arc<DashMap<String, String>> = Arc::new(DashMap::new());
        if cache_path.exists() {
            let raw = std::fs::read_to_string(&cache_path)
                .with_context(|| format!("read cache {:?}", cache_path))?;
            let file: CacheFile = serde_json::from_str(&raw).unwrap_or_default();
            for (k, v) in file.entries {
                cache.insert(k, v);
            }
        }
        Ok(Self { client, cache, cache_path, contact_email })
    }

    fn key(text: &str) -> String {
        let mut h = Sha256::new();
        h.update(text.as_bytes());
        hex::encode(h.finalize())
    }

    pub async fn translate(&self, text: &str) -> Result<String> {
        let stripped = text.trim();
        if stripped.is_empty() {
            return Ok(String::new());
        }
        let key = Self::key(stripped);
        if let Some(hit) = self.cache.get(&key) {
            return Ok(hit.clone());
        }

        let mut url = format!("{ENDPOINT}?q={}&langpair=en|fr", urlencoding::encode(stripped));
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

    pub fn persist(&self) -> Result<()> {
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

pub fn default_cache_path(root: &Path) -> PathBuf {
    root.join(".aphrody-translate-cache.json")
}
