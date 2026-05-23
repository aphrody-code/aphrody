---
name: aphrody-cmd-ai-creative
description: Contexte direct des commandes IA / génératif / créatif du CLI aphrody — chat, gemini, agy, agy-loop, antigravity, hermes, notebooklm, image, firefly, memory, design, logo. Use quand il faut savoir ce que fait une de ces commandes, ses flags, ses variables d'env et son feature-gate avant de l'invoquer.
version: "1.0.0"
metadata:
  source: aphrody native — crates/cli/src/main.rs (enum Commands)
  since: "2026-05-23"
---

# aphrody — commandes IA / génératif / créatif

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant complétion.
Cross-platform (Linux #1, Windows, macOS). Vérité de référence : `aphrody <cmd> --help`.

## LLM & chat

- **`aphrody chat --prompt <txt>`** — agent turn-loop unifié, one-shot (gemini-runtime + tools + memory + session + router + cost + context). Sans `--prompt` ⇒ erreur structurée (REPL ratatui = phase 2). Flags : `--model gemini/default|anthropic/claude-opus-4-7`, `--system`, `--stub` (backend déterministe CI), `--web` (transport Gemini web app keyless cookie au lieu du token agy). Défaut backend = token **agy**.
- **`aphrody gemini …`** — lance le binaire natif Gemini CLI bundlé (bun --compile). Args forwardés.
- **`aphrody agy …`** — forward vers le binaire natif Antigravity CLI (`agy`). Résolution : `$APHRODY_AGY_BIN` > `%LOCALAPPDATA%\agy\bin\agy.exe` > PATH.
- **`aphrody agy-loop <start|stop|status|hook>`** — boucle de codage autonome pour `agy` via le hook `AfterAgent`. `start --goal "…" [--max N]` arme la boucle ; `stop` désarme ; `status` affiche l'état ; `hook` est le driver (stdin/stdout JSON) câblé par le plugin `extensions/aphrody-agy`. Force `agy` à recoder jusqu'au jeton `APHRODY_LOOP_DONE`. État : `.agents/aphrody-loop.json`.
- **`aphrody antigravity chat --model … --prompt …`** — surface API Antigravity pure (token Credential Manager `gemini:antigravity` + RPC `cloudcode-pa`), sans binaire. Sortie JSON (`| jq '.candidates[0].content'`).
- **`aphrody hermes --channel <discord|x>`** — agent multi-canaux voice-to-voice (Gemini 3.5 Flash, token agy keyless). `--voice` (STT Whisper entrant + TTS ElevenLabs sortant), `--trigger @aphrody`, `--web`, `--stub`, `--simulate` (msg synthétique headless). Env : `DISCORD_BOT_TOKEN`/`X_HANDLE`, `OPENAI_API_KEY`, `ELEVENLABS_API_KEY`.
- **`aphrody notebooklm …`** — opérations NotebookLM (list/get). Env : `NOTEBOOKLM_AT_TOKEN` + `NOTEBOOKLM_BL_TOKEN` (XSRF + bootstrap exportés par le web UI).
- **`aphrody memory <migrate|audit|list>`** — providers mémoire tier-1 (Mem0 / Honcho / SqliteLocal) via le trait `MemoryProvider`. Env HTTP : `MEM0_API_KEY` / `HONCHO_API_KEY`.

## Image / créatif

- **`aphrody image generate "<prompt>" --out <dir>`** — génération Gemini (Nano Banana) → disque, via cookie jar Google (no API key). Feature-gate `--features images`.
- **`aphrody firefly generate "<prompt>" --out <dir>`** — Adobe Firefly Services (v3 async). OAuth S2S IMS. Feature-gate `--features firefly`. Env : `FIREFLY_CLIENT_ID` / `FIREFLY_CLIENT_SECRET`.
- **`aphrody design <…>`** — outillage tokens Material 3 (export CSS couleur + feuille de fusion UI). Adossé à `crates/m3-tokens`.
- **`aphrody logo [--ico|--svg|--maskable] [--cols N] [--size N]`** — assets logo dérivés : `.ico` multi-résolution, `.svg` scalable, ou PNG adaptatif maskable M3.

## Garde-fous

- Keyless par défaut (cookies/token agy) ; jamais committer une clé en clair.
- Feature-gates host-only (`images`/`firefly`/`forensics`/`index`) : absents d'un build par défaut et du wasm.
