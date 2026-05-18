//! Loader for an external token manifest (compatible with the scrape format
//! at `assets/scrape/m3-material/tokens/`).
//!
//! The manifest format is intentionally minimal:
//!
//! ```json
//! {
//!   "schema": "m3-design-system/v1",
//!   "directions": [
//!     { "id": "...", "name": "...",
//!       "seed_primary_argb": 4288585636,
//!       "seed_secondary_argb": 4288585636,
//!       "seed_tertiary_argb": 4288585636,
//!       "density": "Comfortable",
//!       "mood": "..." }
//!   ]
//! }
//! ```

use crate::directions::Direction;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Errors surfaced by registry loading.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported schema: {0}")]
    UnsupportedSchema(String),
}

/// Owned form of `Direction` (so it can be deserialized from JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedDirection {
    pub id: String,
    pub name: String,
    pub seed_primary_argb: u32,
    pub seed_secondary_argb: u32,
    pub seed_tertiary_argb: u32,
    pub density: String,
    pub mood: String,
}

/// Top-level manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: String,
    pub directions: Vec<OwnedDirection>,
}

/// Registry of loaded design directions.
#[derive(Debug, Default, Clone)]
pub struct DesignSystemRegistry {
    pub directions: Vec<OwnedDirection>,
}

impl DesignSystemRegistry {
    /// Load an M3 manifest from a directory containing `manifest.json`.
    pub fn load_m3(dir: &Path) -> Result<Self, RegistryError> {
        let path = dir.join("manifest.json");
        let bytes = std::fs::read(&path)?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        if !manifest.schema.starts_with("m3-design-system/") {
            return Err(RegistryError::UnsupportedSchema(manifest.schema));
        }
        Ok(Self {
            directions: manifest.directions,
        })
    }

    /// Built-in registry seeded from `directions::all()`.
    #[must_use]
    pub fn builtin() -> Self {
        let directions = crate::directions::all()
            .iter()
            .map(|d: &Direction| OwnedDirection {
                id: d.id.to_owned(),
                name: d.name.to_owned(),
                seed_primary_argb: d.seed_primary_argb,
                seed_secondary_argb: d.seed_secondary_argb,
                seed_tertiary_argb: d.seed_tertiary_argb,
                density: format!("{:?}", d.density),
                mood: d.mood.to_owned(),
            })
            .collect();
        Self { directions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn registry_load_from_tmp_json() {
        let tmp = std::env::temp_dir().join(format!(
            "aphrody-design-material-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("manifest.json");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            let m = Manifest {
                schema: "m3-design-system/v1".to_owned(),
                directions: vec![OwnedDirection {
                    id: "test".to_owned(),
                    name: "Test".to_owned(),
                    seed_primary_argb: 0xFF67_50A4,
                    seed_secondary_argb: 0xFF62_5B71,
                    seed_tertiary_argb: 0xFF7D_5260,
                    density: "Comfortable".to_owned(),
                    mood: "test".to_owned(),
                }],
            };
            f.write_all(&serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        }
        let reg = DesignSystemRegistry::load_m3(&tmp).unwrap();
        assert_eq!(reg.directions.len(), 1);
        assert_eq!(reg.directions[0].id, "test");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn registry_builtin_has_six() {
        let reg = DesignSystemRegistry::builtin();
        assert_eq!(reg.directions.len(), 6);
    }

    #[test]
    fn registry_rejects_bad_schema() {
        let tmp = std::env::temp_dir().join(format!(
            "aphrody-design-material-badschema-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("manifest.json");
        std::fs::write(&path, br#"{"schema":"wrong/v1","directions":[]}"#).unwrap();
        let err = DesignSystemRegistry::load_m3(&tmp).unwrap_err();
        assert!(matches!(err, RegistryError::UnsupportedSchema(_)));
        std::fs::remove_dir_all(&tmp).ok();
    }
}
