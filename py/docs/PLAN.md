# Aphrody Continuous Improvement Plan

This document lists features, refactorings, optimizations, and validation checks planned for the `aphrody` Python package. The autonomous autopilot loop reads this list, selects a pending item (`⏳`), implements it, runs validation checks, and marks it completed (`✅`).

## Active Plan Items

- `⏳` [WEB] Reverse engineer the Scotty/Boq upload endpoint to support `--attach <path>` for image/document input in keyless web chat.

## Completed Items

- `✅` [WEB] Implement `aphrody web scrape` using a Scrapy asynchronous crawler with process-isolated engine and HTTPX fallback.
- `✅` [WEB] Implement `aphrody web auto-upgrade` using a keyless Gemini Vertex LLM (and programmatic mapping lookup fallback) to autonomously update client action hashes with rollback protection.
- `✅` [MEDIA] Implement `aphrody video gen` calling Vertex Veo video models with Pillow animated fallbacks.
- `✅` [MEDIA] Implement `aphrody music gen` calling audio/speech generation with NumPy/SciPy melody synthesis fallbacks.
- `✅` [REASONING] Implement `aphrody think` (and `--think` in chat) exposing thinking budgets and parsing thoughts from response parts.
- `✅` [AGENT] Implement `aphrody research` executing iterative Deep Research search loops using Google Search grounding.
- `✅` [WEB] Implement `aphrody web conversations` to list recent chats, and `aphrody web resume <cid>` to resume an existing conversation.
- `✅` [CLI/WEB] Add a `--thread` option and a REPL interactive loop to `aphrody web` (using threaded conversation context).
- `✅` [WEB] Parse and display the auto-generated conversation title from `StreamGenerate` responses (`{"11": [...]}`).
- `✅` [WEB] Add `--model` flag to `aphrody web` to allow selecting between model keys (e.g. Flash-Lite, 2.5 Pro) mapped from the bootstrap `WIZ_global_data`.
- `✅` [ROBUSTNESS] Implement automatic token refresh retry logic in `aphrody/auth/oauth.py` with exponential backoff on HTTP 429/503.
- `✅` [CLI] Add export command `aphrody cookies export --format csv` for legacy compatibility.
- `✅` [OPTIMIZATION] Integrate an async execution pipeline wrapper for the keyless Google Translate and Book search APIs using `httpx.AsyncClient` under `aphrody/google_keyless.py`.
- `✅` [EVALUATION] Add local semantic text similarity scoring in `aphrody/evaluation.py` using `fastembed` if available (falling back gracefully to word overlap if not installed).
- `✅` [CLI] Add an `--interactive` prompt interface for the top-level `aphrody setup` command when running in terminal interactive mode.
- `✅` [DOCS] Write a comprehensive developer onboarding guide explaining the token extraction path and credential manager integration in `docs/auth-architecture.md`.
- `✅` [VOICE] Add a dark-mode theme option with fluid OKLCH gradient transitions in the local voice server HTML interface (`aphrody/voice_server.py`).
- `✅` Implement robust file permissions enforcement (`0600`) when writing credentials files on Unix/macOS and strict ACLs on Windows in `aphrody/auth/credential_store.py`.
- `✅` Create modular CLI package package structure.
- `✅` Normalise forensic target paths cross-platform.
- `✅` Automate service account activation, IAM check, and API enablement.
- `✅` Add setup command to the main CLI entry point.
