//! M3 elevation — 6 levels mapped to shadow parameters + surface tint.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElevationLevel {
    Level0 = 0,
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
    Level5 = 5,
}

#[derive(Debug, Clone, Copy)]
pub struct ShadowParams {
    pub blur: f32,
    pub spread: f32,
    pub y_offset: f32,
    pub surface_tint_opacity: f32,
}

impl ElevationLevel {
    pub fn shadow_params(self) -> ShadowParams {
        match self {
            Self::Level0 => ShadowParams { blur: 0.0,  spread: 0.0, y_offset: 0.0, surface_tint_opacity: 0.0 },
            Self::Level1 => ShadowParams { blur: 2.0,  spread: 0.0, y_offset: 1.0, surface_tint_opacity: 0.05 },
            Self::Level2 => ShadowParams { blur: 4.0,  spread: 0.0, y_offset: 2.0, surface_tint_opacity: 0.08 },
            Self::Level3 => ShadowParams { blur: 6.0,  spread: 0.0, y_offset: 4.0, surface_tint_opacity: 0.11 },
            Self::Level4 => ShadowParams { blur: 8.0,  spread: 0.0, y_offset: 6.0, surface_tint_opacity: 0.12 },
            Self::Level5 => ShadowParams { blur: 12.0, spread: 0.0, y_offset: 8.0, surface_tint_opacity: 0.14 },
        }
    }
}
