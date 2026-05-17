use std::sync::Arc;

use async_trait::async_trait;
use backend::Md3Mirror;
use base::Vfs;
use miette::Result;

/// GoogleContext : L'état global injecté dans chaque commande (Pattern Starship/Cargo)
pub(crate) struct GoogleContext {
    pub vfs: Arc<Vfs>,
    pub mirror: Arc<Md3Mirror>,
    pub version: &'static str,
}

impl GoogleContext {
    pub(crate) fn new() -> Result<Self> {
        let vfs = Arc::new(Vfs::new().map_err(|e| miette::miette!(e.to_string()))?);
        vfs.initialize_physical_layout().map_err(|e| miette::miette!(e.to_string()))?;

        let mirror = Arc::new(Md3Mirror::new().map_err(|e| miette::miette!(e.to_string()))?);

        Ok(Self { vfs, mirror, version: "1.0.0-canary" })
    }
}

/// TerminalCommand : Trait d'exécution modulaire pour les sous-commandes
#[async_trait]
pub(crate) trait TerminalCommand {
    async fn execute(&self, ctx: &GoogleContext) -> Result<()>;
}
