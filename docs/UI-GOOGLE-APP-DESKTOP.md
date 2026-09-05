<!-- SPDX-License-Identifier: Apache-2.0 -->
# Aphrody UI — Google-app-desktop minimal assistant bar

> **AVERTISSEMENT (2026-06-04) — document HISTORIQUE / aspirationnel.** Le crate
> `crates/gui` (winit/wgpu/Vello + `mui-rs*`) décrit ici a été **supprimé/extrait
> du dépôt** ; aucun des chemins `crates/gui/src/*.rs` cités plus bas n'existe
> aujourd'hui. La surface UI réelle d'aphrody est le **monorepo Material Design 3
> Bun/TS** (`packages/*` = `@aphrody-code/*`). Conserver comme
> spec de référence visuelle, pas comme description de code livré.

A minimal, always-available desktop assistant styled like the Google app for
desktop (<https://search.google/google-app/desktop/>): a frameless, borderless,
translucent, always-on-top floating bar that exposes a single Gemini **3.5
Flash** model and the full aphrody Rust stack (voice, image/video, filesystem).

Home crate (historique, supprimé) : `crates/gui` (native Material 3 surface —
`mui-rs-renderer` / Vello / wgpu + winit + Taffy). No browser, no Electron.

## Window (implemented)

`crates/gui/src/main.rs` — `winit` window attributes:
- `with_decorations(false)` — frameless / borderless.
- `with_transparent(true)` — rounded translucent surface (the M3 pill).
- `with_window_level(WindowLevel::AlwaysOnTop)` — always active in foreground.
- compact size (~760x132) — a floating bar, not a full app window.

Planned: drag-to-move (hit-test on the bar background), global hotkey to
summon/dismiss (workspace `global-hotkey` candidate), per-monitor centering near
the top like the Google app.

## Visual design (pixel-perfect Material 3 / Google)

Sourced from the workspace `m3-tokens` crate (already wired via
`gui::tokens_reply`): `BASELINE_DARK` palette, Google Sans Flex font face,
Material Symbols Outlined, M3 typography CSS. Layout:

```
┌───────────────────────────────────────────────────────────┐
│  (gem)   Ask Gemini…                          [mic] [+] [×] │   collapsed bar
└───────────────────────────────────────────────────────────┘
            ▼ expands downward on a reply / image / video
┌───────────────────────────────────────────────────────────┐
│  reply text … / generated image / video player             │
└───────────────────────────────────────────────────────────┘
```

- Gemini gem (the rainbow spark) left; rounded 28dp pill; `surface` background
  at ~92% opacity; `outline_variant` hairline; on-surface text.
- Mic button (audio-to-audio), `+` (attach / image / video / file), close.

## Single model — Gemini 3.5 Flash (implemented at the IPC layer)

`gui::GeminiAssistant` holds one cached `aphrody_chat::backend::GeminiWebBackend`
(3.5 Flash via the signed-in Google cookie jar, `~/.aphrody/google-cookies.json`,
no API key). `GeminiAssistant::ask(prompt_id, prompt)` returns an `IpcReply`.
Backend bootstraps once and is reused (latency objective); conversation
continuity is threaded server-side.

## Audio-to-audio (plan)

`crates/gui/src/voice.rs` + the `aphrody-voice` crate (STT/TTS). The mic button
toggles capture → `aphrody-voice` STT → `GeminiAssistant::ask` → `aphrody-voice`
TTS playback. Replaces the current `dispatch_cmd("voice-stt-toggle")` "not yet
wired" placeholder in `lib.rs`. Always-listening (wake word / push-to-talk) is a
follow-up.

## Image + video (plan)

Reading + creation via the `gemini-web` SDK media surface (already shipped):
- Create: `gemini-web` Nano Banana (image) / Veo (video) — the MCP tools
  `gemini_image` / `gemini_video`; reply carries `generated_image_urls` /
  `generated_video_urls`, rendered in the expanded panel (image view / video
  player).
- Read: drop a file via `+`; pass to 3.5 Flash for analysis.

## OS filesystem (plan)

Native `std::fs` + `aphrody-tools` filesystem tools, exposed through the `+`
menu (open / save / attach). Native binary = full OS access (no sandbox).

## IPC contract

`gui::IpcMessage` (Prompt / Cmd) ⇄ `gui::IpcReply` (Text / Tokens / Error /
Html / Unknown). The render loop dispatches Prompt → `GeminiAssistant::ask`,
Cmd → `dispatch_cmd`.

## Status

Implemented: frameless always-on-top translucent window; `GeminiAssistant`
(3.5 Flash, cached); M3 token surface. Next: bar layout (gem + input + mic +
actions), voice loop, media rendering, fs menu, drag + global hotkey.
