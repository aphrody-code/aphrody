// SPDX-License-Identifier: Apache-2.0
use backend::Md3Mirror;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialisation du logging Google OS
    tracing_subscriber::fmt::init();

    println!("🔥 Google OS - Terminal Backend Initialisé 🔥");

    // MD3Mirror gère désormais son propre VFS
    let mirror = Md3Mirror::new()?;
    mirror.start_mirroring().await?;

    println!("✨ Mirroring MD3 terminé avec succès.");
    Ok(())
}
