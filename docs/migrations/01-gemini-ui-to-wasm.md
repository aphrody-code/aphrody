# Migration 01 : `packages/gemini` vers `crates/aphrody-wgpu-material`

**Priorité :** 1 (Critique)
**Statut :** Planifié
**Cible :** `wasm32-unknown-unknown` + WebGPU

## 1. État des Lieux (TS/Next.js)
Le package `packages/gemini` contient ~4 500 lignes de code React (TSX) reproduisant l'interface web Gemini au pixel près.
- **Routage :** App Router Next.js (`app/api/*`, `app/page.tsx`).
- **Composants :** 41 composants UI (`MessageBubble`, `VoiceWaveform`, `PromptBar`).
- **Core Logic :** Intégration Whisper, MCP, Gateway asynchrone (`core/`).
- **Assets :** Polices Google Sans Flex (~41k lignes binaire), M3 Tokens.

## 2. Problématique
Ce package contrevient à la politique 100% Rust (`CLAUDE.md §2`). L'exécution dépend de Node.js/Next.js et embarque un runtime JS lourd qui est incompatible avec la distribution via un binaire CLI natif cross-platform.

## 3. Plan de Migration Rust
Le portage s'appuiera sur `crates/aphrody-react-reconciler` (déjà implémenté) et `crates/aphrody-wgpu-material`.

### Étape A : Core & API (Backend)
- Migrer `core/auth.ts`, `core/mcp.ts` vers des handlers HTTP dans `crates/backend/src/routes/`.
- Migrer la logique Whisper (`core/whisper.ts`) vers `crates/aphrody-voice-stt`.
- Remplacer les routes API Next.js par des endpoints Axum ou un canal WebSocket direct.

### Étape B : Composants & Rendu (Frontend WASM)
- Transcrire les hooks `useChat.ts`, `useVoiceInput.ts` en structs Rust gérant leur state interne (via `yew` ou le reconciler maison `aphrody-react-reconciler`).
- Convertir la surcouche CSS complexe (`globals.css`) et les dégradés WebGPU (`lib/webgpu-gradient.ts`) en shaders WGSL natifs dans `crates/aphrody-wgpu-material`.
- Remplacer les primitives UI React (`PromptBar.tsx`, `MessageBubble.tsx`) par des composants Rust générant du DOM via `web-sys` et `wasm-bindgen`.

### Étape C : Intégration
- Compiler le client en `aphrody-terminal-wasm.wasm`.
- L'injecter via le daemon HTTP Rust.

## 4. Critères de Succès
- [ ] L'interface s'affiche dans un navigateur sans charger un seul script `.js` (hormis le glue code `wasm-bindgen`).
- [ ] La communication se fait en WebSocket vers le binaire Rust local, sans serveur Node.
- [ ] Suppression complète du dossier `packages/gemini/`.
