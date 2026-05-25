# Architecture de voicebox

**Depot source explore :** `var/discord/voicebox/` (clone de `github.com/jamiepine/voicebox`, v0.5.0)

---

## 1. Vue d'ensemble : trois couches

voicebox est un studio vocal de bureau construit sur trois couches independantes qui communiquent exclusivement via HTTP local.

```
+-----------------------------------------------+
|  Frontend React (Vite / Bun)                  |
|  app/  — React 18, TanStack Router/Query      |
|  tauri/ — coquille Vite qui embed l'app        |
+-----------------------+-----------------------+
                        |
           fetch() sur http://127.0.0.1:17493
                        |
+-----------------------------------------------+
|  Coquille Tauri 2 (Rust)                      |
|  tauri/src-tauri/  — lance et supervise       |
|  le sidecar Python, gere les hotkeys,         |
|  l'audio natif (CPAL, WASAPI, ScreenCap)      |
+-----------------------+-----------------------+
                        |
       app.shell().sidecar("voicebox-server")
       args: --data-dir <path> --port 17493 --parent-pid <pid>
                        |
+-----------------------------------------------+
|  Backend Python FastAPI (sidecar)             |
|  backend/  — uvicorn, SQLite, HuggingFace     |
|  TTS (Qwen/Kokoro/Chatterbox/LuxTTS/TADA)     |
|  STT Whisper, LLM Qwen3                       |
+-----------------------------------------------+
```

Le Rust Tauri ne fait **jamais** d'inference IA. Toute la logique metier vocale vit dans le processus Python ; Tauri se contente de le spawner, superviser et tuer.

---

## 2. Comment le backend est lance

### 2.1. En developpement

Le script `justfile` (cible `dev`) commence par demarrer le backend Python dans un sous-processus, puis lance `bun run tauri dev` :

```
# justfile:113
{{ venv_bin }}/uvicorn backend.main:app --reload --port 17493 &
...
cd {{ tauri_dir }} && bun run tauri dev
```

Sur Windows (justfile:124-132), le pattern est identique avec `Start-Process -NoNewWindow`.

Le `package.json` racine expose les memes commandes (package.json:15-16) :

```json
"dev":        "bun run setup:dev && cd tauri && bun run tauri dev",
"dev:server": "uvicorn backend.main:app --reload --port 17493"
```

`setup:dev` genere des binaires sidecar factices (scripts/setup-dev-sidecar.js) pour que Tauri puisse compiler sans le vrai binaire PyInstaller. Le Rust detecte ces placeholders et bascule en mode "detecter un serveur deja en ecoute sur :17493" (main.rs:424-448).

**Commande exacte pour demarrer le backend seul en developpement :**

```bash
# Unix (depuis la racine du depot voicebox)
cd backend
python -m venv venv && source venv/bin/activate
pip install -r requirements.txt
uvicorn backend.main:app --reload --port 17493
```

```powershell
# Windows
cd backend
python -m venv venv ; .\venv\Scripts\Activate.ps1
pip install -r requirements.txt
python -m uvicorn backend.main:app --reload --port 17493
```

Ou via just (apres `just setup`) :

```bash
just dev-backend   # lance uvicorn --reload --port 17493 dans le venv
```

### 2.2. En production (sidecar PyInstaller)

Le build produit deux binaires via `backend/build_binary.py` :

- `voicebox-server` — CPU uniquement, packaging `--onefile`, ~400 Mo
- `voicebox-server-cuda` — CUDA, packaging `--onedir`, binaire + DLLs NVIDIA (~2-3 Go)

Ces binaires sont copies dans `tauri/src-tauri/binaries/` avec le suffixe de la triple rustc (ex. `voicebox-server-x86_64-pc-windows-msvc.exe`). Le `tauri.conf.json` les declare comme `externalBin` (tauri.conf.json:16) :

```json
"externalBin": ["binaries/voicebox-server", "binaries/voicebox-mcp"]
```

Au demarrage de l'application, le frontend React appelle la commande Tauri `start_server`. Celle-ci (main.rs:206-689) :

1. Verifie si le port 17493 est deja occupe (reutilise si c'est un processus voicebox, refuse sinon).
2. Detecte la presence du binaire CUDA dans `$APP_DATA_DIR/backends/cuda/` et verifie sa version.
3. Utilise `app.shell().sidecar("voicebox-server")` (via `tauri-plugin-shell`) pour obtenir le handle du binaire bundle.
4. Passe les arguments : `--data-dir <app_data_dir> --port 17493 --parent-pid <pid_tauri>`.
5. Appelle `.spawn()` qui retourne `(receiver, CommandChild)`.
6. Lit les lignes stdout/stderr jusqu'a detecter `"Uvicorn running"` ou `"Application startup complete"` (timeout 120 s).
7. Apres demarrage confirme, un `tokio::spawn` continue de relayer les logs vers le frontend via `app.emit("server-log", ...)`.

### 2.3. Arret et watchdog

Le binaire `backend/server.py` (point d'entree PyInstaller) demarre un thread watchdog (server.py:102-223) qui surveille le PID du parent Tauri toutes les 2 secondes. Si le parent meurt, le serveur s'arrete proprement sauf si :

- Tauri a envoye `POST /watchdog/disable` avant de quitter (mode "garder le serveur actif apres fermeture"), ou
- Tauri a ecrit le fichier sentinelle `$APP_DATA_DIR/.keep-running`.

Cote Rust, `RunEvent::Exit` gere les deux cas (main.rs:1431-1490).

---

## 3. L'API locale sur :17493

Le frontend ne passe **jamais** par une IPC Tauri pour les donnees vocales. Il fait des `fetch()` HTTP standard vers `http://127.0.0.1:17493` depuis le code React. L'URL de base est stockee dans un store Zustand (`serverStore.ts:31`). En mode distant, l'utilisateur peut pointer vers un autre hote.

Les CORS authorises incluent `tauri://localhost` et `https://tauri.localhost` (app.py:119-137) pour que la webview Tauri puisse contacter le serveur sans erreur de politique.

### Routes principales

Toutes les routes sont enregistrees dans `backend/routes/__init__.py:register_routers`. Le tableau ci-dessous liste les prefixes observes dans le code source :

| Endpoint                              | Description                                              |
|---------------------------------------|----------------------------------------------------------|
| `GET /health`                         | Etat du serveur (champ `status`, `model_loaded`, `gpu_available`) |
| `POST /transcribe`                    | STT : upload audio, retourne texte (routes/transcription.py) |
| `POST /generate`                      | TTS : demande de generation asynchrone                   |
| `GET /generate/{id}/status`           | SSE de progression de generation                          |
| `POST /generate/{id}/retry`           | Relancer une generation echouee                           |
| `POST /speak`                         | Parler un texte via profil (wrapper MCP, routes/speak.py) |
| `GET/POST /profiles`                  | Gestion des profils vocaux (clonage)                     |
| `POST /profiles/{id}/samples`         | Ajout d'un echantillon de voix                           |
| `GET/POST /history`                   | Historique des generations                                |
| `GET /audio/{id}`                     | Flux audio d'une generation                              |
| `GET/POST /models/status`             | Etat des modeles (telecharges ou non)                    |
| `POST /models/download`              | Declenchement du telechargement HuggingFace              |
| `GET/POST /settings/captures`         | Parametres de capture audio                              |
| `GET/POST /settings/generation`       | Parametres de generation TTS                             |
| `GET/POST /channels`                  | Canaux audio multi-sortie                                |
| `GET/POST /effects`                   | Chaine d'effets audio (Pedalboard)                       |
| `GET/POST /stories`                   | Timeline audio multi-voix                                |
| `GET/POST /captures`                  | Dictee vocale (STT + edition)                            |
| `GET /events/speak`                   | SSE : debut/fin de synthese (pour pill dictee)           |
| `POST /watchdog/disable`             | Desactiver le watchdog avant fermeture Tauri             |
| `POST /shutdown`                      | Arret propre du serveur (Windows)                        |
| `/mcp` (monte)                        | Point d'entree FastMCP Streamable HTTP                   |
| `PUT /mcp/bindings`                   | Liaison profil/moteur par client MCP                     |
| `GET /docs`                           | Documentation Swagger auto-generee (FastAPI)             |

Le client React (`app/src/lib/api/client.ts`) encapsule tous ces appels dans une classe `ApiClient` qui lit `serverStore.serverUrl` pour construire l'URL complete.

---

## 4. Moteurs TTS et STT

### 4.1. Moteurs TTS declares

Chaque moteur est declare comme `ModelConfig` dans `backend/backends/__init__.py` et instantie a la demande via `get_tts_backend_for_engine()`. Les modeles sont telecharges depuis HuggingFace Hub au premier usage.

| Moteur (`engine`)     | Fichier implementation             | Modele HuggingFace                                | Notes                          |
|-----------------------|------------------------------------|---------------------------------------------------|--------------------------------|
| `qwen`                | `pytorch_backend.py` (classe `PyTorchTTSBackend`) | `Qwen/Qwen3-TTS-12Hz-{1.7B,0.6B}-Base`     | Moteur principal, voice cloning zero-shot |
| `qwen_custom_voice`   | `qwen_custom_voice_backend.py`     | `Qwen/Qwen3-TTS-12Hz-{1.7B,0.6B}-CustomVoice`   | Variante fine-tuning voix personnalisee |
| `luxtts`              | `luxtts_backend.py` (classe `LuxTTSBackend`) | `YatharthS/LuxTTS`                          | Rapide, compatible CPU          |
| `chatterbox`          | `chatterbox_backend.py` (classe `ChatterboxTTSBackend`) | `ResembleAI/chatterbox`               | Multilingual, avec watermark Perth |
| `chatterbox_turbo`    | `chatterbox_turbo_backend.py` (classe `ChatterboxTurboTTSBackend`) | `ResembleAI/chatterbox-turbo` | Anglais uniquement, balises paralinguistiques |
| `tada`                | `hume_backend.py` (classe `HumeTadaBackend`) | `HumeAI/tada-{1b,3b-ml}`               | Llama + flow matching, 1B ou 3B multilingual |
| `kokoro`              | `kokoro_backend.py` (classe `KokoroTTSBackend`) | `hexgrad/Kokoro-82M`                     | 82M parametres, tres leger, G2P misaki |
| `mlx` (Apple Silicon) | `mlx_backend.py` (classe `MLXTTSBackend`) | via `mlx-audio`                             | Acceleration Metal uniquement   |

Le backend actif est detecte automatiquement par `utils/platform_detect.py:get_backend_type()` : `"mlx"` sur Apple Silicon, `"pytorch"` ailleurs. La selection CUDA/CPU est transmise au sidecar par le nom du binaire (`voicebox-server-cuda` → variable d'environnement `VOICEBOX_BACKEND_VARIANT=cuda`).

### 4.2. STT Whisper

Le moteur STT est Whisper (OpenAI) via HuggingFace Transformers. Cinq tailles sont disponibles, registrees comme `ModelConfig` avec `engine="whisper"` :

- `whisper-base` (`openai/whisper-base`)
- `whisper-small` (`openai/whisper-small`)
- `whisper-medium` (`openai/whisper-medium`)
- `whisper-large` (`openai/whisper-large-v3`)
- `whisper-turbo` (`openai/whisper-large-v3-turbo`)

Le modele charge est accessible via `services/transcribe.py:get_whisper_model()`.

### 4.3. LLM local pour la personnalite

Un backend LLM optionnel (`qwen_llm_backend.py`) utilise Qwen3 (0.6B, 1.7B, 4B) pour recrire les textes a voix haute selon la personnalite d'un profil. Sur Apple Silicon : `MLXQwenLLMBackend` via `mlx-lm`. Sur les autres plateformes : `PyTorchQwenLLMBackend` via transformers.

### 4.4. Telechargement des modeles

Tous les modeles passent par `huggingface_hub`. Le cache est le repertoire standard HuggingFace (`HF_HUB_CACHE`). A la premiere demande de generation, le backend verifie si le depot est dans le cache via `backends/base.py:is_model_cached()` ; s'il manque, il declenche un telechargement en tache de fond avec rapport de progression via SSE (`/models/migrate/progress`). L'interface propose aussi `POST /models/download` pour un telechargement anticipe.

### 4.5. Shim MCP

Un deuxieme binaire (`voicebox-mcp`, genere par `build_binary.py:build_shim()`) sert de proxy stdio-to-HTTP pour les clients MCP qui ne parlent pas HTTP. Il se connecte a `/mcp` sur le serveur principal et redirige le JSON-RPC. Il est declare dans `externalBin` au meme titre que `voicebox-server`.

---

## 5. Frontend React

Le workspace Bun contient quatre packages (package.json:5-10) :

| Package   | Role                                                        |
|-----------|-------------------------------------------------------------|
| `app/`    | Composants React partages (App.tsx, hooks, stores, client API) |
| `tauri/`  | Coquille Vite qui embed `app/` et se compile en WebView Tauri |
| `web/`    | Variante web standalone (meme logique, sans Tauri)          |
| `landing/`| Page marketing statique                                     |

Le frontend de l'application de bureau est dans `tauri/` (Vite + React via `@vitejs/plugin-react`). Il importe les sources de `app/` via des alias Vite (tauri/vite.config.ts:9-21). Le framework UI est **React 18** avec **TanStack Router** (routing file-based) et **TanStack Query** (cache serveur). Les composants primitifs proviennent de **Radix UI** habilles avec **Tailwind CSS 4**. L'etat global est gere par **Zustand**.

Biome (`@biomejs/biome 2.3.12`) remplace ESLint + Prettier pour le lint et le formatage.

---

## 6. Comparaison avec l'app desktop aphrody

| Critere                    | voicebox                                       | aphrody `apps/desktop`                                   |
|----------------------------|------------------------------------------------|----------------------------------------------------------|
| Coquille desktop           | Tauri 2 (Rust)                                 | Tauri 2 (Rust)                                           |
| Frontend                   | React 18 + Radix UI + Tailwind 4               | Angular 21.2 + Angular Material 21.2                     |
| Backend AI                 | Sidecar Python FastAPI sur :17493 (processus separe, HTTP) | Rust in-process via `aphrody::run_captured` (FFI directe, pas de reseau) |
| Communication frontend-backend | `fetch()` HTTP sur 127.0.0.1:17493       | Commandes Tauri IPC + appels Rust in-process             |
| Gestion de processus       | Rust spawne/supervise le sidecar Python, watchdog PID bidirectionnel | Tout reste dans le meme binaire Rust      |
| Moteurs vocaux             | Qwen3-TTS, Kokoro, Chatterbox, LuxTTS, TADA (Python/PyTorch) | `aphrody-voice` (crate Rust)             |
| STT                        | Whisper (HuggingFace / Python)                 | `aphrody-voice` integre, eventuellement cloud            |
| Duplication d'etat         | Aucune : le Python est la seule source de verite | Aucune : le Rust est la seule source de verite          |
| Isolation de crash         | Un crash Python ne tue pas la WebView          | Un panic Rust dans le thread worker ne tue pas le runtime Tauri |
| Cold start                 | Lent (import PyTorch ~5-30 s sur CPU)          | Rapide (Rust, pas de VM)                                 |

La difference fondamentale est architecturale : voicebox fait le choix du **sidecar HTTP** pour beneficier de l'ecosysteme Python ML (PyTorch, HuggingFace, Chatterbox, etc.) au prix d'un processus supplementaire et d'une latence de demarrage elevee. aphrody fait le choix de l'**in-process Rust** pour eliminer la latence reseau et le cout de demarrage, mais doit porter les modeles en Rust natif (via ONNX/bindings ou appels cloud).

---

## Appendice : demarrage rapide du backend seul pour brancher un bot

Pour consommer l'API vocale sans lancer Tauri :

```bash
# 1. Creer l'environnement Python (Python 3.12 recommande)
cd var/discord/voicebox
python3.12 -m venv backend/venv
source backend/venv/bin/activate          # Windows: backend\venv\Scripts\activate

# 2. Installer les dependances
pip install -r backend/requirements.txt
pip install --no-deps chatterbox-tts hume-tada
pip install git+https://github.com/QwenLM/Qwen3-TTS.git

# 3. Demarrer le serveur
uvicorn backend.main:app --reload --port 17493
# ou via just (apres `just setup`) :
just dev-backend

# 4. Verifier
curl http://127.0.0.1:17493/health
# {"status":"healthy","model_loaded":false,"gpu_available":false,...}

# 5. Consulter les routes
open http://127.0.0.1:17493/docs
```

Le premier appel `POST /generate` (ou `/speak`) declenchera le telechargement du modele choisi depuis HuggingFace. Les modeles les plus legers pour un bot vocal sont :

- `kokoro` (82M, ~300 Mo, CPU-friendly)
- `luxtts` (CPU-friendly, zero-shot cloning)
- `chatterbox-turbo` (anglais, avec balises d'emotion)
