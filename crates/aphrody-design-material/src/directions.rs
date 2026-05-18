//! 6 visual directions — 5 ported from `open-design/apps/daemon/src/prompts/directions.ts`
//! plus a new **Material Expressive** flagship direction.

use serde::{Deserialize, Serialize};

/// A visual direction: seed colors + tone of voice + density.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Direction {
    pub id: &'static str,
    pub name: &'static str,
    pub seed_primary_argb: u32,
    pub seed_secondary_argb: u32,
    pub seed_tertiary_argb: u32,
    pub density: Density,
    pub mood: &'static str,
}

/// Density of the layout grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Density {
    Compact,
    Comfortable,
    Spacious,
}

/// All 6 directions.
#[must_use]
pub const fn all() -> &'static [Direction] {
    &[
        Direction {
            id: "material-expressive",
            name: "Material Expressive",
            seed_primary_argb: 0xFF67_50A4,
            seed_secondary_argb: 0xFF62_5B71,
            seed_tertiary_argb: 0xFF7D_5260,
            density: Density::Comfortable,
            mood: "Vibrant tonal purples; M3 default flagship.",
        },
        Direction {
            id: "calm-minimal",
            name: "Calm Minimal",
            seed_primary_argb: 0xFF44_6FB0,
            seed_secondary_argb: 0xFF55_5F71,
            seed_tertiary_argb: 0xFF6E_5676,
            density: Density::Spacious,
            mood: "Cool blue-greys; lots of whitespace, body-large copy.",
        },
        Direction {
            id: "warm-editorial",
            name: "Warm Editorial",
            seed_primary_argb: 0xFFB0_3D2E,
            seed_secondary_argb: 0xFF76_5849,
            seed_tertiary_argb: 0xFF66_5E40,
            density: Density::Comfortable,
            mood: "Warm terracotta; serif headlines, magazine cadence.",
        },
        Direction {
            id: "neon-cyber",
            name: "Neon Cyber",
            seed_primary_argb: 0xFF00_E5FF,
            seed_secondary_argb: 0xFFFF_006E,
            seed_tertiary_argb: 0xFF7B_61FF,
            density: Density::Compact,
            mood: "High-contrast cyan/magenta; tight grid, mono accents.",
        },
        Direction {
            id: "earth-organic",
            name: "Earth Organic",
            seed_primary_argb: 0xFF54_6E3A,
            seed_secondary_argb: 0xFF82_6F4C,
            seed_tertiary_argb: 0xFF35_6A60,
            density: Density::Spacious,
            mood: "Sage greens + clay; soft shapes, large radii.",
        },
        Direction {
            id: "mono-architect",
            name: "Mono Architect",
            seed_primary_argb: 0xFF1C_1C1C,
            seed_secondary_argb: 0xFF42_4242,
            seed_tertiary_argb: 0xFFB8_B8B8,
            density: Density::Compact,
            mood: "Monochrome greys; grid-led, geometric, low chroma.",
        },
    ]
}

/// Look up a direction by id.
#[must_use]
pub fn find(id: &str) -> Option<&'static Direction> {
    all().iter().find(|d| d.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_six_total() {
        assert_eq!(all().len(), 6);
    }

    #[test]
    fn directions_material_expressive_is_canonical_purple() {
        let d = find("material-expressive").expect("missing direction");
        assert_eq!(d.seed_primary_argb, 0xFF67_50A4);
        assert_eq!(d.seed_secondary_argb, 0xFF62_5B71);
        assert_eq!(d.seed_tertiary_argb, 0xFF7D_5260);
    }

    #[test]
    fn directions_unique_ids() {
        let ids: Vec<_> = all().iter().map(|d| d.id).collect();
        let mut uniq = ids.clone();
        uniq.sort_unstable();
        uniq.dedup();
        assert_eq!(uniq.len(), ids.len());
    }
}
