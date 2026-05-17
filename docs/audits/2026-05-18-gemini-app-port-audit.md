<!-- SPDX-License-Identifier: Apache-2.0 -->
# Audit — gemini.google.com/app port to Next.js + WASM + M3 + WebGPU + voice-first

Date: 2026-05-18
Driver: aphrody mission item — "verifie que tout https://gemini.google.com/app est recree
en next js native aphrody wasm mD3 webpu, voice first".
Auditor: Claude Opus 4.7 (sub-agent).

## 1. Reference surface map (gemini.google.com/app, 2026-05 build)

Surfaces enumerated from the public gemini.google.com/app shell and from the
existing pixel-perfect single-file clone
`crates/aphrody-wasm/examples/gemini-clone-pixel-perfect.html` (734 lines, already
mirrors every visible affordance):

| # | Surface piece | Description |
|---|---|---|
| S1 | Top app bar | Hamburger -> brand wordmark (spectrum-shift gradient text) -> model picker pill -> spacer -> share icon -> settings icon -> avatar ring (gradient border, initial inside) |
| S2 | Left navigation rail | 4 icon buttons: New, Recent, Gems, Settings. Active state = brand-blue 18% tint over surface-container |
| S3 | Empty-state hero | 64px sparkle SVG (4-color radial) + 56px "Hello, <name>" greeting in spectrum-shift gradient text + 22px subtitle + 4 suggestion chips (1 featured with gradient border) |
| S4 | Conversation pane | Alternating user/assistant bubbles. User bubble right-aligned, surface-container-high. Assistant bubble left-aligned, sparkle avatar pseudo-element, optional streaming shimmer (spectrum-shift moving gradient bar) |
| S5 | Prompt bar | 56px tall, pill radius 28px. Attach (+) -> textarea -> mic icon -> send button. On focus the border becomes a spectrum-shift gradient border. Send button fills with spectrum-shift when enabled |
| S6 | Disclaimer foot | "Gemini may display inaccurate info..." centered label-medium |
| S7 | Drawer/Settings panel | Slide-in panel for theme/voice/data controls (modal in upstream) |
| S8 | Model picker dropdown | Triggered by app-bar pill; lists 2.5 Flash, 2.5 Pro, 2.0 Flash Exp, etc. |
| S9 | Share dialog | Modal with conversation URL + copy + revoke |
| S10 | Gems panel | Catalog of user-defined system-prompt gems |
| S11 | Recent history list | Slide-in panel listing previous conversations, click to load |
| S12 | Voice input affordance | Mic button in prompt bar. Push-to-hold OR click-to-toggle. Visible audio waveform during recording. Cancel-on-escape. Transcript fills textarea on stop |
| S13 | Voice output (TTS) | Per-assistant-bubble speaker button + auto-play toggle in settings (default OFF for privacy) |
| S14 | WebGPU hero gradient | Animated brand-color gradient backing the empty-state hero. Should attempt `navigator.gpu`, gracefully fall back to CSS spectrum-shift |

## 2. Existing aphrody assets (reuse without re-implementing)

| Asset | Path | Status |
|---|---|---|
| Backend SDK (ask/stt/speak) | `packages/gemini-live-aphrody/src/{index,gateway,whisper,voice,auth}.ts` | FAIT — used as the API route handler engine |
| Pixel-perfect reference clone | `crates/aphrody-wasm/examples/gemini-clone-pixel-perfect.html` | FAIT — design ground truth |
| M3 brand tokens | `crates/m3-tokens/src/gemini_brand.rs` | FAIT — colors, gradients, corners |
| Google Sans Flex font + exporter | `crates/m3-tokens/src/google_sans_flex.rs` + `assets/fonts/google-sans-flex/*.ttf` | FAIT — variable font shipped (9.8 MB) |
| shadcn-bridge composables (Sparkle / PromptBar / MessageBubble / SuggestionChip / AvatarRing) | `crates/shadcn-bridge/src/gemini.rs` | FAIT — Rust reference, mirrored in React port |
| Material Web components | `packages/ui/components/` (button) + `@material/web` ^2.4.1 dep | INCOMPLET — only button shipped, more wrappers possible but not blocking |
| WASM gradient module | `crates/aphrody-wasm/` | INCOMPLET — placeholder for WebGPU hero shader (task #115) |
| Workspace `next-*` / `turbopack-*` deps | root `Cargo.toml` workspace.dependencies | FAIT — declared from git source `aphrody-code/next.js#aphrody` |

## 3. Surface-to-asset mapping with verdict

| # | Surface | Mapped to (before this tick) | Verdict pre-tick | Action this tick |
|---|---|---|---|---|
| S1 | App bar | reference HTML only | NON_FAIT (React) | New `AppBar.tsx` |
| S2 | Left rail | reference HTML only | NON_FAIT (React) | New `LeftRail.tsx` |
| S3 | Hero + suggestions | shadcn-bridge `Sparkle`, reference HTML | NON_FAIT (React) | New `Sparkle.tsx`, `SuggestionChip.tsx`, `GradientHero.tsx` |
| S4 | Conversation | shadcn-bridge `MessageBubble` | NON_FAIT (React) | New `MessageBubble.tsx` |
| S5 | Prompt bar | shadcn-bridge `PromptBar` | NON_FAIT (React) | New `PromptBar.tsx` (voice-first sizing) |
| S6 | Disclaimer | trivial | n/a | inline in `app/page.tsx` |
| S7 | Settings drawer | not started | NON_FAIT | INCOMPLET this tick — `app/page.tsx` exposes settings toggle for auto-TTS only (deferred panel) |
| S8 | Model picker | reference HTML only | NON_FAIT | INCOMPLET this tick — static pill in `AppBar.tsx`, dropdown deferred |
| S9 | Share dialog | not started | NON_FAIT | INCOMPLET this tick — deferred |
| S10 | Gems panel | not started | NON_FAIT | NON_FAIT this tick — separate scope, requires server-side gem CRUD |
| S11 | Recent history | not started | NON_FAIT | NON_FAIT this tick — requires server-side conversation persistence |
| S12 | Voice input | gemini-live-aphrody `transcribe()` | NON_FAIT (UI) | New `useVoiceInput.ts` + `VoiceWaveform.tsx` + `/api/stt/route.ts` |
| S13 | Voice output | gemini-live-aphrody `synthesize()` | NON_FAIT (UI) | New `useVoiceOutput.ts` + `/api/tts/route.ts` + bubble speaker button |
| S14 | WebGPU hero | placeholder | INCOMPLET | `GradientHero.tsx` attempts `navigator.gpu`, falls back to CSS spectrum-shift; WGSL shader path tagged `TODO_FROM_WGPU` for task #115 |

## 4. Voice-first verification points

| Point | Required by mission | Delivered |
|---|---|---|
| Mic button is bigger than send button | YES | `PromptBar.tsx` — mic 56px, send 40px |
| Push-to-hold AND click-to-toggle | YES | `useVoiceInput.ts` supports both via `start()` / `stop()` + pointer-down/pointer-up handlers |
| Visible waveform during recording | YES | `VoiceWaveform.tsx` — WebAudio `AnalyserNode` -> canvas, 60 fps |
| Auto-TTS toggle in settings, default OFF | YES | `useVoiceOutput.ts` reads `autoPlay` flag from localStorage, default false |
| Barge-in cancel on new mic activity | YES | `useVoiceOutput.ts` exposes `cancel()`, `useVoiceInput.ts` calls it before `start()` |

## 5. Scaffold result

Package created at `packages/gemini-app-aphrody/`:

- 1x `package.json`, 1x `next.config.ts`, 1x `tsconfig.json`, 1x `README.md`
- 1x `app/layout.tsx`, 1x `app/page.tsx`, 1x `app/styles/globals.css`
- 3x API routes: `app/api/chat/route.ts`, `app/api/stt/route.ts`, `app/api/tts/route.ts`
- 8x components: `AppBar`, `LeftRail`, `Sparkle`, `SuggestionChip`, `GradientHero`,
  `MessageBubble`, `PromptBar`, `VoiceWaveform`
- 2x hooks: `useVoiceInput`, `useVoiceOutput`
- 1x WASM bridge placeholder: `app/wasm/index.ts` (re-exports a CSS fallback,
  `TODO_FROM_WGPU` marker for task #115 to fill in `wgpu`-compiled module)
- 1x smoke test: `tests/smoke.test.ts`

Verify gate (per mission): `cd packages/gemini-app-aphrody && bun --bun tsc --noEmit`.

## 6. Out of scope (explicitly deferred)

- Gems panel (S10) -- requires server-side gem storage; future tick.
- Recent history (S11) -- requires conversation persistence + auth; future tick.
- WebGPU shader compile (S14) -- task #115 owns wgpu pipeline + WGSL.
- Server-side OAuth flow -- `app/api/chat/route.ts` reads env directly, OAuth dance
  remains the gemini-live-aphrody backend's responsibility.
