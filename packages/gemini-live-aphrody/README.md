# @aphrody/gemini-live-aphrody

Headless aphrody fork of [`google-gemini/live-api-web-console`](https://github.com/google-gemini/live-api-web-console).

**No `GEMINI_API_KEY` required.** Two interchangeable backends:

| Backend            | Auth source                                           | Chat model                                              | STT                                                       | TTS               |
| ------------------ | ----------------------------------------------------- | ------------------------------------------------------- | --------------------------------------------------------- | ----------------- |
| `gemini-oauth`     | `~/.gemini/oauth_creds.json` written by the Gemini CLI (`gemini auth login`). Refreshed automatically via the OAuth refresh token. | Google `generativelanguage.googleapis.com` `generateContent` (`GEMINI_MODEL`, default `gemini-2.0-flash-exp`). | n/a (Gemini handles audio natively if you wire WebSocket; this headless build does text only). | ElevenLabs `xi-api-key` |
| `whisper-gateway`  | `APHRODY_GATEWAY_TOKEN` (Bearer) on `APHRODY_GATEWAY_URL`. | OpenAI-compatible chat completions endpoint (`APHRODY_GATEWAY_MODEL`). Works against Cloudflare AI Gateway, Vercel AI Gateway, OpenAI proxy, llama.cpp, vLLM, LM Studio. | Local `whisper` CLI by default; OpenAI Whisper HTTP API when `OPENAI_API_KEY` is set. | ElevenLabs `xi-api-key` |

Select the backend at runtime:

```bash
APHRODY_LIVE_BACKEND=gemini-oauth    bun run packages/gemini-live-aphrody/src/index.ts ask "hello"
APHRODY_LIVE_BACKEND=whisper-gateway bun run packages/gemini-live-aphrody/src/index.ts ask "hello"
```

The default (`gemini-oauth`) reuses the OAuth credentials a developer already has from logging in with the Gemini CLI, so a clean install needs zero extra secrets.

## Why this fork exists

Upstream `live-api-web-console` is a React demo wired against `process.env.REACT_APP_GEMINI_API_KEY`. That is unacceptable for the aphrody workflow because:

1. The dev machine already has a long-lived OAuth credential set from `gemini auth login`. Requiring an additional API key duplicates secrets.
2. Several deployment targets (locked-down Linux boxes, the wasm32 reference build) cannot acquire a Gemini API key at all.
3. We want a fully offline-capable path (`whisper-gateway` with local Whisper + local llama.cpp) so the CLI keeps working without network egress.

This package strips the React/SCSS/Create-React-App surface, exposes the same conversational primitives as a pure Bun TypeScript library + CLI, and routes auth through either the OAuth credential file or a configurable OpenAI-compatible gateway.

## Install

This package lives inside the aphrody workspace (`packages/*` glob in the root `package.json`). The workspace install installs it automatically:

```bash
bun install
```

Standalone use is possible by depending on the package directly:

```jsonc
{
  "dependencies": {
    "@aphrody/gemini-live-aphrody": "workspace:*"
  }
}
```

## CLI

```text
Usage: bun run packages/gemini-live-aphrody/src/index.ts <subcommand> [args]

Subcommands:
  doctor                        Print resolved backend + env diagnostics.
  ask    <prompt>               Single-turn chat against the active backend.
  stt    <audio-path>           Transcribe a file via Whisper.
  speak  <text> [output.mp3]    TTS via ElevenLabs.

Select backend with APHRODY_LIVE_BACKEND=gemini-oauth | whisper-gateway.
```

Example:

```bash
APHRODY_LIVE_BACKEND=whisper-gateway \
APHRODY_GATEWAY_URL=https://gateway.ai.cloudflare.com/v1/<acct>/<gw>/openai \
APHRODY_GATEWAY_TOKEN=$CF_TOKEN \
APHRODY_GATEWAY_MODEL=gpt-4o-mini \
ELEVENLABS_API_KEY=$ELEVEN_KEY \
ELEVENLABS_VOICE_ID=$VOICE_ID \
  bun run packages/gemini-live-aphrody/src/index.ts speak "bonjour" /tmp/out.mp3
```

## Library

Each module is independently consumable:

```ts
import { ask } from "@aphrody/gemini-live-aphrody";
import { authorizationHeader } from "@aphrody/gemini-live-aphrody/src/auth";
import { chatCompletion } from "@aphrody/gemini-live-aphrody/src/gateway";
import { transcribe } from "@aphrody/gemini-live-aphrody/src/whisper";
import { synthesize } from "@aphrody/gemini-live-aphrody/src/voice";

const reply = await ask("Quelle heure est-il ?");
const tokenHeader = await authorizationHeader();          // Bearer ya29...
const stt = await transcribe({ audioPath: "/tmp/in.wav" });
const tts = await synthesize({ text: "bonjour" });
const chat = await chatCompletion({ messages: [{ role: "user", content: "hi" }] });
```

## Environment variables

See `.env.example` for the full list. Summary:

- `APHRODY_LIVE_BACKEND` — `gemini-oauth` or `whisper-gateway`.
- `GEMINI_CREDENTIALS_PATH` — override the OAuth credentials path.
- `GEMINI_MODEL` — override the Gemini model id.
- `APHRODY_GATEWAY_URL` / `APHRODY_GATEWAY_TOKEN` / `APHRODY_GATEWAY_MODEL` — OpenAI-compatible endpoint.
- `OPENAI_API_KEY` / `WHISPER_BINARY` / `WHISPER_MODEL` — Whisper transport tuning.
- `ELEVENLABS_API_KEY` / `ELEVENLABS_VOICE_ID` / `ELEVENLABS_MODEL_ID` — ElevenLabs TTS.

`GEMINI_API_KEY` is intentionally not consulted anywhere in this package.

## Build

```bash
bun build packages/gemini-live-aphrody/src/index.ts --target=bun --outdir=dist
```

## Upstream attribution

Forked from [`google-gemini/live-api-web-console`](https://github.com/google-gemini/live-api-web-console) at commit `0a4542fe0e39d07956ea7af5de45d7c81fde8960`. Upstream is Apache-2.0; this fork retains the same license (see `LICENSE`).

The two Google OAuth constants reused in `src/auth.ts` (`OAUTH_CLIENT_ID`, `OAUTH_CLIENT_SECRET`) come from the public Gemini CLI source at `gemini-cli/packages/core/src/code_assist/oauth2.ts`. Per Google's [installed-application guidance](https://developers.google.com/identity/protocols/oauth2#installed) these values are not actually secret and may be embedded in source.

## License

Apache-2.0. See [`LICENSE`](./LICENSE).
