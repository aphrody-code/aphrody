<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody plugin — refocus on Google design (proposal)

Goal: pivot the `aphrody` plugin's centre of gravity to **Google / Material 3
design** — **additively**. The Google-design surface is elevated to the front;
nothing is removed.

> **Decision (2026-05-22): Photoshop and Blender STAY in the MCP surface.**
> The `aphrody-adobe` split below is **declined / not executed** — the Adobe
> (12 cloud + 3 live Photoshop), Firefly, and Blender tools remain first-class
> MCP tools in the default `aphrody` plugin. The refocus is purely additive
> (the `google-design` skill, `google-design-researcher` agent and the
> `docs/design/` package lead the design story); the table below is retained
> only as historical rationale, not a to-do.

Already shipped this session (the design core): the `google-design` skill, the
`google-design-researcher` Google-sources-only sub-agent, the full
`docs/design/` package (M3 styles/foundations/layout/components/glossary,
Gemini theme, design.google references), `crates/m3-tokens` (10-step shape,
GEMINI_DARK) and `crates/mui-rs-components` (complete M3 component set).

## Keep / elevate — Google-design aligned

- **Skills**: `google-design` (new, primary), `design-google-ingest`, `design-md`,
  `design-review`, `design-brief`, `design-consultation`, `color-expert`,
  `creative-director`, `brand-guidelines`, `best-stack-2026`, `context7-mcp`,
  `microsoft-docs` (cross-ref), `start`, `skill-creator`.
- **Agents**: `google-design-researcher` (new), `design-google-curator`,
  `rust-engineer`, `rust-architect`, `code-review`, `cargo-auditor`,
  `cross-platform-validator`, `build`.
- **Tools**: `gemini_*`, `universal_web_fetch`, `docs_auto_search`,
  `context7_*`, `microsoft_docs_*`, `screen_capture` (design verification),
  `coding_style_guide`.

## Deprioritize / candidate for removal — off the design focus

| Surface | What | Recommendation |
|---|---|---|
| **Cloud Photoshop** (12 MCP tools) | `photoshop_*`, `firefly_*`, `firefly_to_photoshop` | Move to a **separate optional plugin** (`aphrody-adobe`) rather than delete — keeps the work, removes it from the design-focused default surface. |
| **Live Photoshop UXP** (3 tools) | `photoshop_live_*`, `apps/photoshop-uxp`, `apps/photoshop-remote` | Same — fold into `aphrody-adobe`. |
| **Blender bridge** | `apps/aphrody` Python `blender.py`/`bpy_runner.py` + socket | Out of design scope; gate behind an opt-in feature/plugin, don't ship by default. |
| **`aphrody-firefly` crate** | Adobe Firefly Services client | Keep in-tree (real, tested) but exclude from the plugin's advertised tool set. |
| **Misc skills** | `competitive-ads-extractor`, `dream`, `autopilot`, `apple-hig` | `apple-hig` contradicts a Google-only focus → drop or clearly mark "contrast only". The others are off-topic for a design plugin → move to a general plugin. |

## Concrete steps (when approved)

1. Split the manifest: a lean `aphrody` (design + core dev) + an optional
   `aphrody-adobe` plugin carrying the Photoshop/Firefly tools.
2. Update `.claude-plugin/plugin.json` description/keywords to lead with
   Material 3 / Gemini design (currently leads with Adobe/Photoshop).
3. Trim the MCP tool registration so the default server advertises the
   design + core tools; Photoshop tools register only when `aphrody-adobe`
   is enabled.
4. Keep all Rust crates compiling (no code deleted) — this is a packaging
   refocus, not a capability deletion.

> Nothing here is executed automatically. Photoshop/Blender removal touches a
> shipping MCP surface and code you just built — confirm the split before any
> tool is unregistered or any file removed.
