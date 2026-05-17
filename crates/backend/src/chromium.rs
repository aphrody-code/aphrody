#![cfg(target_os = "windows")]

use std::{fs, path::PathBuf};

use anyhow::{Result, anyhow};
use base::Crypto;
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ChromiumProfile {
    pub name: String,
    pub path: PathBuf,
}

pub struct ChromiumParser {
    user_data_path: PathBuf,
    master_key: Option<Vec<u8>>,
}

impl ChromiumParser {
    pub fn new(user_data_path: PathBuf) -> Self {
        Self { user_data_path, master_key: None }
    }

    pub fn get_profiles(&self) -> Vec<String> {
        let local_state_path = self.user_data_path.join("Local State");
        let content = fs::read_to_string(local_state_path).unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

        let mut profiles = Vec::new();
        if let Some(info_cache) = v.get("profile").and_then(|p| p.get("info_cache"))
            && let Some(obj) = info_cache.as_object()
        {
            for key in obj.keys() {
                profiles.push(key.clone());
            }
        }

        if profiles.is_empty() {
            profiles.push("Default".to_string());
        }
        profiles
    }

    /// Charge et déchiffre la Master Key de Chromium via DPAPI
    pub fn load_master_key(&mut self) -> Result<()> {
        let local_state_path = self.user_data_path.join("Local State");
        let content = fs::read_to_string(local_state_path)?;
        let v: serde_json::Value = serde_json::from_str(&content)?;

        let b64_key = v["os_crypt"]["encrypted_key"]
            .as_str()
            .ok_or_else(|| anyhow!("Clé maître introuvable dans Local State"))?;

        // Décodage Base64 (Chromium utilise un format spécifique)
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64_key)
            .map_err(|e| anyhow!("Erreur de décodage Base64 : {}", e))?;

        if &decoded[0..5] != b"DPAPI" {
            return Err(anyhow!("Format de clé invalide (DPAPI attendu)"));
        }

        // Les 5 premiers octets sont "DPAPI", le reste est la clé chiffrée
        let encrypted_key = &decoded[5..];
        let master_key = Crypto::decrypt_dpapi(encrypted_key)?;

        self.master_key = Some(master_key);
        Ok(())
    }

    pub fn get_master_key(&self) -> Option<&Vec<u8>> {
        self.master_key.as_ref()
    }

    /// Extrait les cookies pour un domaine spécifique
    pub fn get_cookies(&self, profile: &str, host_key: &str) -> Result<Vec<(String, String)>> {
        let master_key =
            self.master_key.as_ref().ok_or_else(|| anyhow!("Master Key non chargée"))?;

        let cookies_path = self.user_data_path.join(profile).join("Network/Cookies");
        if !cookies_path.exists() {
            return Err(anyhow!("Fichier Cookies introuvable : {}", cookies_path.display()));
        }

        // On copie le fichier pour éviter de bloquer la base SQLite si Chrome est ouvert
        let temp_cookies = std::env::temp_dir().join("google_cli_cookies.db");
        fs::copy(&cookies_path, &temp_cookies)?;

        let conn = rusqlite::Connection::open(&temp_cookies)?;
        let mut stmt =
            conn.prepare("SELECT name, encrypted_value FROM cookies WHERE host_key LIKE ?")?;

        let cookie_iter = stmt.query_map([format!("%{}%", host_key)], |row| {
            let name: String = row.get(0)?;
            let encrypted_value: Vec<u8> = row.get(1)?;
            Ok((name, encrypted_value))
        })?;

        let mut results = Vec::new();
        for cookie in cookie_iter {
            let (name, encrypted_value) = cookie?;
            if let Ok(decrypted) = Crypto::decrypt_aes_gcm(&encrypted_value, master_key)
                && let Ok(value) = String::from_utf8(decrypted)
            {
                results.push((name, value));
            }
        }

        let _ = fs::remove_file(temp_cookies);
        Ok(results)
    }
}
