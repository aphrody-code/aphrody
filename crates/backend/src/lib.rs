// SPDX-License-Identifier: Apache-2.0
use std::path::Path;

use anyhow::Result;
use base::Vfs;
use reqwest::Client;
use tokio::fs;
use tracing::{error, info};

#[cfg(target_os = "windows")] pub mod chromium;
pub mod dns;
pub struct Md3Mirror {
    client: Client,
    vfs: Vfs,
}

impl Md3Mirror {
    pub fn new() -> Result<Self> {
        let vfs = Vfs::new()?;
        vfs.initialize_physical_layout()?;

        Ok(Self { client: Client::new(), vfs })
    }

    pub async fn start_mirroring(&self) -> Result<()> {
        let unix_target = "/var/mirror";
        let physical_path = self.vfs.resolve(unix_target)?;

        info!(
            "🚀 Démarrage du Mirroring MD3 vers {} (VFS: {})",
            unix_target,
            physical_path.display()
        );

        // Liste des composants/assets à synchroniser
        let assets = [
            (
                "https://raw.githubusercontent.com/material-components/material-web/main/README.md",
                "README.md",
            ),
            (
                "https://fonts.googleapis.com/css2?family=Google+Sans:wght@400;500;700&display=swap",
                "google_sans.css",
            ),
        ];

        for (url, filename) in assets.iter() {
            match self.download_asset(url, filename, &physical_path).await {
                Ok(_) => info!("✅ Synchronisé : {}", filename),
                Err(e) => error!("❌ Échec pour {} : {}", filename, e),
            }
        }

        Ok(())
    }

    async fn download_asset(&self, url: &str, filename: &str, target_dir: &Path) -> Result<()> {
        let response = self.client.get(url).send().await?;
        let content = response.bytes().await?;
        let dest_path = target_dir.join(filename);
        fs::write(dest_path, content).await?;
        Ok(())
    }
}
