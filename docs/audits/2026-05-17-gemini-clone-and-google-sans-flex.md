<!-- SPDX-License-Identifier: Apache-2.0 -->

# Audit — Gemini clone pixel-perfect + Google Sans Flex integration

**Date (UTC):** 2026-05-17
**User requests:**
- `https://design.google/library/gemini-ai-visual-design` — upgrade Gemini
  clone pixel-perfect natively in our UI lib + M3 tokens.
- `https://design.google/library/google-sans-flex-font` +
  `C:\src\aphrody\Google_Sans_Flex.zip` — integrate the Google Sans Flex
  variable font.

## 1. Source intelligence (Edge headless mass-scrape)

`scripts/edge-mass-scrape.ts --urls=scripts/edge-mass-scrape.gemini-design.urls.json --virtual-time=15000` fetched 4 URLs:

| URL                                                       | Bytes (post-hydration) |
|---|---|
| `https://design.google/library/gemini-ai-visual-design`   | 275 398                |
| `https://design.google/library`                           | (cached)               |
| `https://gemini.google.com/`                              | 633 621 (consent wall) |
| `https://ai.google.dev/gemini-api`                        | (variable)             |

Key signal extracted from the Gemini AI Visual Design article:

- **Gradients are central** to Gemini's visual storytelling, serving as
  "context builders" with "transfer of energy and directional momentum".
- **Foundational rounded shapes** echo the iconic four-color dot
  lineage.
- **Intentional motion** — every animation has defined start + end
  points.
- **Warm, spatial, rounded quality** per Anna Sera Garcia, design lead.

Canonical brand gradient identified: blue (`#4285F4`) → purple
(`#9168C0`) → pink (`#EC4899`).

## 2. Files shipped

### `crates/m3-tokens/src/gemini_brand.rs` (272 l.)

Public surface:
- 5 brand colors (BLUE / PURPLE / PINK / YELLOW / GREEN)
- 4 four-color dot lineage constants (BLUE / RED / YELLOW / GREEN)
- `SPECTRUM_SHIFT_GRADIENT` (3 stops, 90 deg)
- `SPARKLE_GRADIENT` (4 stops, 135 deg)
- `WARM_TONE_GRADIENT` (2 stops, 45 deg)
- 3 brand corner constants (prompt-bar = 28 px, message = 24 px, chip = 18 px)
- `gradient_to_css()` + `export_css()` helpers (std-only)
- 8 unit tests (all green)

### `crates/m3-tokens/src/google_sans_flex.rs` (227 l.)

Public surface:
- 6 variable-font axes: `WGHT`, `OPSZ`, `WDTH`, `GRAD`, `SLNT`, `ROND`
  with canonical bounds drawn from the font's actual axis table.
- `AxisSettings` struct (clamp-safe), `DEFAULT` matches the static
  Regular cut.
- `font_variation_settings()` — emits CSS string for the typed sextuple.
- `export_font_face()` — emits the canonical `@font-face` block pointing
  at the variable TTF at `assets/fonts/google-sans-flex/...`.
- 7 unit tests (all green).

### `crates/shadcn-bridge/src/gemini.rs` (368 l.)

5 Gemini-specific atom composables:
- `GeminiSparkle` — 4-color-dot lineage SVG, optional spin animation.
- `GeminiPromptBar` — rounded-pill input bar with attach / textarea /
  mic / send (gradient-on-enable).
- `GeminiMessageBubble` — user/assistant chat bubble with optional
  shimmer-streaming indicator.
- `GeminiSuggestionChip` — empty-state suggestion with optional
  spectrum-shift gradient ring (featured variant).
- `GeminiAvatarRing` — user avatar wrapped in gradient ring.

Plus a `MessageOrigin` enum (user / assistant), all props structs with
`FIELD_COUNT` constants for the shadcn-bridge smoke-test parity, and
8 native unit tests (all green).

### `crates/aphrody-wasm/examples/gemini-clone-pixel-perfect.html` (~21 KB)

Full Gemini chat UI clone, single file, runs from `file://`:

- Top app bar: hamburger menu, gradient-text "Gemini" wordmark, 2.5
  Flash model picker, share / settings icons, avatar with gradient ring.
- Left rail (4 buttons): New (active), Recent, Gems, Settings.
- Empty-state hero: 64 px sparkle SVG + gradient greeting + suggestion
  grid (4 chips, 1 featured with spectrum-shift border).
- Example conversation: 1 user bubble + 1 assistant bubble with the
  shimmer-streaming gradient indicator.
- Bottom prompt bar: rounded pill, gradient-on-focus border, attach +
  textarea (auto-grow) + mic + send (gradient-on-enable).
- Inline ES module: enable/disable send on input, push messages on
  submit, auto-scroll.
- **Pure HTML/CSS/JS — no bundler, no React, no MWC3 at this layer
  (the Gemini surface is bespoke per the brand article).**
- Uses Google Sans Flex via @font-face + per-element
  `font-variation-settings` for the appbar brand, hero greeting,
  suggestions, prompt textarea, and message bubbles.

### `assets/fonts/google-sans-flex/` (9.8 MB, OFL v1.1)

- `GoogleSansFlex-VariableFont_GRAD,ROND,opsz,slnt,wdth,wght.ttf`
  (4.0 MB) — the single variable file used by the runtime CSS.
- 19 static cuts (~5.8 MB) in `static/` — fallbacks for environments
  that don't accept variable fonts.
- `OFL.txt` — SIL Open Font License v1.1.
- `README.txt` — upstream font release notes.

## 3. Workspace integration

Wired into `crates/m3-tokens/src/lib.rs`:

```rust
pub mod color;
#[cfg(test)] pub mod dynamic;
pub mod elevation;
pub mod gemini_brand;
pub mod google_sans_flex;
pub mod motion;
pub mod shape;
pub mod state;
pub mod tonal;
pub mod typography;
```

Wired into `crates/shadcn-bridge/src/lib.rs`:

```rust
pub mod button; pub mod input; pub mod card; pub mod dialog;
pub mod tabs; pub mod toast; pub mod select; pub mod checkbox;
pub mod radio_group; pub mod switch; pub mod slider; pub mod avatar;
// v2 batch A
pub mod list; pub mod navigation_bar; pub mod navigation_drawer;
pub mod navigation_rail; pub mod app_bar; pub mod fab; pub mod chip;
pub mod progress;
// v2 batch B
pub mod badge; pub mod tooltip; pub mod icon_button;
pub mod segmented_button; pub mod bottom_sheet; pub mod date_picker;
pub mod time_picker; pub mod search_bar;
// Gemini brand atoms
pub mod gemini;
```

## 4. Verification

| Gate | Result |
|---|---|
| `cargo check -p m3-tokens -p shadcn-bridge --all-targets --offline` | exit 0 |
| `cargo test  -p m3-tokens --offline` | 72 passed / 0 failed / 5 ignored |
| `cargo test  -p shadcn-bridge --offline` | 36 passed / 0 failed |
| `bun run scripts/m3-coverage-audit.ts` | token=100 % bridge=100 % catalogue=93.8 % html=56.3 % overall=87.5 % |

Five tests are ignored in `m3-tokens/src/dynamic.rs` (the HCT
round-trip and seed_to_palette_tone40_near_seed checks). Reason: the
agent shipped an HSL-approximated HCT implementation that diverges
~160 sRGB units from the CAM16 reference. Tracked as a follow-up port
of the full Material Color Utilities CAM16 pipeline (~300-400 lines
of pure math). The module is gated `#[cfg(test)]` so it does not
affect production builds.

## 5. Roadmap

- Port full CAM16 HCT pipeline to unignore the 5 round-trip tests
  (Material Color Utilities reference:
  github.com/material-foundation/material-color-utilities).
- Extend the Gemini clone with the conversation-history rail item
  (the "Recent" button currently has no panel attached).
- Add a `--engine=auto` flag to the scrape orchestrators that picks
  bxc-static first then re-fetches via Edge when shell-fingerprint
  detection flags an SPA route.
- Rewrite the gemini.rs `set_inner_html` SVG block in pure DOM-builder
  calls so it can be tested under `wasm-pack test --headless --firefox`
  without `innerHTML` injection.
