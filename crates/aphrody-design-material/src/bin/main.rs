//! `aphrody-design-material` CLI — emit M3 schemes, tokens, and direction
//! metadata as JSON.

use anyhow::{Context, Result};
use aphrody_design_material::{
    directions, format_hex_rgb, parse_hex_argb,
    registry::DesignSystemRegistry,
    Elevation, Scheme, SchemeVariant, ShapeFamily, TonalPalette, TypeRole,
};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(
    name = "aphrody-design-material",
    about = "Material Design 3 toolkit: schemes, tokens, palettes, directions.",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Scheme operations.
    Scheme {
        #[command(subcommand)]
        op: SchemeCmd,
    },
    /// Tokens dump (all M3 design tokens for a seed).
    Tokens {
        #[command(subcommand)]
        op: TokensCmd,
    },
    /// Direction registry operations.
    Direction {
        #[command(subcommand)]
        op: DirectionCmd,
    },
    /// Tonal palette operations.
    Palette {
        #[command(subcommand)]
        op: PaletteCmd,
    },
}

#[derive(Debug, Subcommand)]
enum SchemeCmd {
    /// Generate an M3 scheme from a seed color.
    Generate {
        #[arg(long)]
        seed: String,
        #[arg(long, value_enum, default_value_t = ModeArg::Light)]
        mode: ModeArg,
    },
}

#[derive(Debug, Subcommand)]
enum TokensCmd {
    /// Dump all M3 tokens (scheme + shape + elevation + motion + typography).
    Dump {
        #[arg(long)]
        seed: String,
    },
}

#[derive(Debug, Subcommand)]
enum DirectionCmd {
    /// List all built-in directions.
    List,
    /// Show one direction by id.
    Show { id: String },
}

#[derive(Debug, Subcommand)]
enum PaletteCmd {
    /// Sample a palette at a specific tone.
    Tone {
        #[arg(long)]
        seed: String,
        #[arg(long)]
        tone: u8,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ModeArg {
    Light,
    Dark,
    HcLight,
    HcDark,
}

impl From<ModeArg> for SchemeVariant {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Light => Self::Light,
            ModeArg::Dark => Self::Dark,
            ModeArg::HcLight => Self::HighContrastLight,
            ModeArg::HcDark => Self::HighContrastDark,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Scheme { op } => cmd_scheme(op),
        Cmd::Tokens { op } => cmd_tokens(op),
        Cmd::Direction { op } => cmd_direction(op),
        Cmd::Palette { op } => cmd_palette(op),
    }
}

fn cmd_scheme(op: SchemeCmd) -> Result<()> {
    let SchemeCmd::Generate { seed, mode } = op;
    let seed_argb = parse_hex_argb(&seed).context("invalid --seed hex")?;
    let scheme = Scheme::for_palettes(
        &aphrody_design_material::CorePalettes::from_seed(seed_argb),
        SchemeVariant::from(mode),
    );
    let json = scheme_to_json(&scheme);
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

fn cmd_tokens(op: TokensCmd) -> Result<()> {
    let TokensCmd::Dump { seed } = op;
    let seed_argb = parse_hex_argb(&seed)?;
    let palettes = aphrody_design_material::CorePalettes::from_seed(seed_argb);

    let schemes = json!({
        "light": scheme_to_json(&Scheme::for_palettes(&palettes, SchemeVariant::Light)),
        "dark": scheme_to_json(&Scheme::for_palettes(&palettes, SchemeVariant::Dark)),
        "highContrastLight": scheme_to_json(&Scheme::for_palettes(&palettes, SchemeVariant::HighContrastLight)),
        "highContrastDark": scheme_to_json(&Scheme::for_palettes(&palettes, SchemeVariant::HighContrastDark)),
    });

    let shapes: serde_json::Value = ShapeFamily::all()
        .iter()
        .map(|s| {
            (
                format!("{s:?}"),
                json!({ "radius_dp": s.radius_dp(), "css": s.css_border_radius() }),
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into();

    let elevations: serde_json::Value = Elevation::all()
        .iter()
        .map(|e| {
            (
                format!("{e:?}"),
                json!({
                    "dp": e.dp(),
                    "tint_alpha": e.surface_tint_alpha(),
                    "box_shadow": e.box_shadow_css(),
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into();

    let easings: serde_json::Value = aphrody_design_material::Easing::all()
        .iter()
        .map(|e| {
            let (a, b, c, d) = e.bezier();
            (
                format!("{e:?}"),
                json!({ "bezier": [a, b, c, d], "css": e.css() }),
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into();

    let types: serde_json::Value = TypeRole::all()
        .iter()
        .map(|r| {
            let s = r.style();
            (
                format!("{r:?}"),
                json!({
                    "family": s.font_family,
                    "weight": s.font_weight,
                    "size_dp": s.font_size_dp,
                    "line_height_dp": s.line_height_dp,
                    "letter_spacing_dp": s.letter_spacing_dp,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into();

    let state_layers = json!({
        "hover": aphrody_design_material::HOVER,
        "focus": aphrody_design_material::FOCUS,
        "pressed": aphrody_design_material::PRESSED,
        "dragged": aphrody_design_material::DRAGGED,
    });

    let out = json!({
        "seed": format_hex_rgb(seed_argb),
        "schemes": schemes,
        "shape": shapes,
        "elevation": elevations,
        "easings": easings,
        "durations_ms": aphrody_design_material::DURATIONS_MS,
        "typography": types,
        "state_layers": state_layers,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn cmd_direction(op: DirectionCmd) -> Result<()> {
    match op {
        DirectionCmd::List => {
            let reg = DesignSystemRegistry::builtin();
            println!("{}", serde_json::to_string_pretty(&reg.directions)?);
        }
        DirectionCmd::Show { id } => {
            let d = directions::find(&id)
                .with_context(|| format!("no direction with id '{id}'"))?;
            println!("{}", serde_json::to_string_pretty(d)?);
        }
    }
    Ok(())
}

fn cmd_palette(op: PaletteCmd) -> Result<()> {
    let PaletteCmd::Tone { seed, tone } = op;
    let seed_argb = parse_hex_argb(&seed)?;
    let palette = TonalPalette::from_seed_argb(seed_argb);
    let argb = palette.tone(tone);
    let out = json!({
        "seed": format_hex_rgb(seed_argb),
        "tone": tone,
        "argb": format!("0x{argb:08X}"),
        "hex": format_hex_rgb(argb),
        "hue": palette.hue,
        "chroma": palette.chroma,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn scheme_to_json(scheme: &Scheme) -> serde_json::Value {
    // Use the derived Serialize impl on Scheme, then post-process u32 fields
    // to hex strings for human readability.
    let raw = serde_json::to_value(scheme).expect("scheme serialize");
    let serde_json::Value::Object(map) = raw else {
        return raw;
    };
    let converted = map
        .into_iter()
        .map(|(k, v)| {
            let v2 = if let Some(n) = v.as_u64() {
                serde_json::Value::String(format_hex_rgb(n as u32))
            } else {
                v
            };
            (k, v2)
        })
        .collect::<serde_json::Map<_, _>>();
    serde_json::Value::Object(converted)
}
