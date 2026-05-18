# @aphrody/gemini

Unified Gemini application: Pixel-perfect Next.js 16 App Router port + CLI entrypoint wired to OAuth Gemini CLI + Whisper STT + ElevenLabs TTS.

- App Router (Next.js 16, React 19)
- M3 baseline + Gemini brand tokens via `lib/m3-tokens.ts`
- WebGPU spectrum-shift gradient with CSS fallback
- Voice-first: mic > send (push-to-hold + click-toggle)
- OAuth path (no paid API key required)

## Fork wiring (mandatory)

This package consumes Aphrody's fork of Next.js — **not** the npm `next` —
declared as `"next": "link:next"` in `package.json`.

Before `bun install`, register the fork once on the machine:

```bash
cd ../../  # to aphrody root
cd ../worktree/next.js/packages/next
bun link
```

Then back in aphrody:

```bash
cd packages/gemini
bun install
bun link next      # resolves the link target
```

Verify the fork is the active dep:

```bash
bun pm ls next | head -3
cat node_modules/next/package.json | head -3
# expected: "version": "16.3.0-canary.2"
```

> Note: the fork must be **built** (`pnpm install && pnpm build` inside
> `/c/worktree/next.js`) before `next dev` is callable, because the
> `bin/next` entry compiles to `dist/bin/next`. Source-resolution via
> `bun link` works regardless of whether the fork has been built.

## Scripts

```bash
bun run dev          # next dev (no turbo) on :3000
bun run dev:turbo    # next dev --turbo (needs fork turbopack binary)
bun run build        # next build
bun run start        # next start :3000
bun run typecheck    # bun --bun tsc --noEmit
bun run test         # bun test tests/
bun run copy-fonts   # mirror assets/fonts/google-sans-flex/ -> public/fonts/
```

## Environment

| Variable | Used for |
|---|---|
| `APHRODY_LIVE_BACKEND` | `gemini-oauth` (default) or `whisper-gateway` |
| `GEMINI_CREDENTIALS_PATH` | Override OAuth credentials path (defaults to `~/.gemini/oauth_creds.json`) |
| `GEMINI_MODEL` | Model id (defaults to `gemini-2.0-flash-exp`) |
| `APHRODY_GATEWAY_URL` | OpenAI-compatible chat completions endpoint (whisper-gateway backend) |
| `APHRODY_GATEWAY_TOKEN` | Bearer token for the gateway |
| `OPENAI_API_KEY` | If set, /api/stt uses OpenAI's Whisper HTTP API |
| `WHISPER_BINARY` | Override the local whisper CLI path (default: `whisper`) |
| `ELEVENLABS_API_KEY` | Required for /api/tts |
| `ELEVENLABS_VOICE_ID` | Required for /api/tts |

If any required env var is missing the matching API route returns 503 with
an explicit error message — no silent fallback.

## Layout

```
app/
  layout.tsx, page.tsx, globals.css
  api/chat/route.ts, api/stt/route.ts, api/tts/route.ts
  components/AppBar.tsx, LeftRail.tsx, HeroEmptyState.tsx,
            Sparkle.tsx, SuggestionChip.tsx, ConversationPane.tsx,
            MessageBubble.tsx, PromptBar.tsx, MicButton.tsx,
            VoiceWaveform.tsx, ModelPicker.tsx
hooks/
  useChat.ts, useVoiceInput.ts, useVoiceOutput.ts
lib/
  m3-tokens.ts (mirrors crates/m3-tokens/src/gemini_brand.rs)
  webgpu-gradient.ts (WGSL shader + CSS fallback)
public/fonts/google-sans-flex/ (copied at dev/build via scripts/copy-fonts.ts)
tests/
  api-routes.test.ts, components.test.tsx
```
