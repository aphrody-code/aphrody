# LLM local rpbey — llama.cpp, OpenAI-compatible (le seam aphrody)

Le chat IA « Rpbey » (cf. [`chat.md`](./chat.md)) synthétise ses réponses avec **un LLM
auto-hébergé sur le VPS**, gratuit et privé : `llama.cpp` exposé en HTTP loopback,
**API OpenAI-compatible**. C'est ici qu'aphrody se branchera : il suffit qu'aphrody expose
un endpoint OpenAI Chat Completions et de repointer une variable d'environnement.

> Historique : Vertex/Gemini ont été **retirés** (payants, abandonnés) au profit de ce LLM
> local (commit `19745c2`). Le code parle OpenAI Chat Completions justement pour rester
> agnostique du backend.

## Fichiers & emplacements

| Élément | Chemin absolu |
|---------|---------------|
| Client OpenAI-compat (web) | `/home/ubuntu/rpbey/apps/web/src/server/services/llm.ts` |
| Service systemd | `/etc/systemd/system/rpbey-llm.service` |
| Binaire llama.cpp | `/home/ubuntu/llm/llama.cpp/build/bin/llama-server` |
| Modèle servi | `/home/ubuntu/llm/models/Llama-3.2-3B-Instruct-Q4_K_M.gguf` (~2.0 Go) |
| Modèle alternatif (présent, non servi) | `/home/ubuntu/llm/models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf` (~986 Mo) |
| THP (perf inférence) | `/etc/systemd/system/llm-thp.service` |

> La note CLAUDE.md mentionne `/home/ubuntu/llm/llama-server` — c'est un **raccourci** ;
> le binaire réel est sous `llama.cpp/build/bin/`. Les modèles sont **hors-repo**
> (`/home/ubuntu/llm/models/`, non versionnés). Ne pas lire les poids `.gguf`.

---

## 1. Le client web — `services/llm.ts`

`import "server-only"`. Client minimal OpenAI Chat Completions (juste un `fetch`, aucune
dépendance SDK → rien n'entre dans le bundle Next).

### Configuration (env vars)

| Variable | Défaut | Rôle |
|----------|--------|------|
| `RPBEY_LLM_URL` | `http://127.0.0.1:8080/v1/chat/completions` | **LE SEAM** : endpoint OpenAI Chat Completions |
| `RPBEY_LLM_MODEL` | `rpbey-local` | champ `model` de la requête (llama-server l'ignore, sert un seul modèle) |
| `RPBEY_LLM_TIMEOUT_MS` | `60000` | timeout d'un appel (au-delà → abort → repli extractif) |
| `RPBEY_CHAT_LLM` | (unset = actif) | **kill switch** : `RPBEY_CHAT_LLM=0` désactive le LLM (repli extractif déterministe) |

> Note : `RPBEY_CHAT_MODEL` apparaît dans `apps/web/.env` mais le code lit `RPBEY_LLM_MODEL`
> (champ `model`, sans effet sur llama-server mono-modèle). Le `model` réel = le `.gguf` chargé par le service.

### Forme de la requête (`body()`)

```jsonc
{
  "model": "<RPBEY_LLM_MODEL>",
  "messages": [ { "role": "system|user|assistant", "content": "..." } ],
  "stream": true | false,
  "temperature": 0.35,        // opts.temperature
  "max_tokens": 768,          // opts.maxTokens
  "top_p": 0.9,               // bride l'invention des petits modèles
  "repeat_penalty": 1.1       // pénalise la répétition
}
```

### API exportée

| Fonction | Signature | Comportement |
|----------|-----------|--------------|
| `isLlmEnabled()` | `(): boolean` | `true` sauf `RPBEY_CHAT_LLM === "0"` |
| `generate(messages, opts?)` | `Promise<string \| null>` | réponse complète (non-stream). Renvoie **`null` (jamais throw)** si désactivé/indisponible/timeout/vide. Log `console.warn` sur erreur. |
| `generateStream(messages, opts?)` | `AsyncGenerator<string>` | yield chaque fragment de texte du SSE OpenAI (`choices[].delta.content`) dès qu'il arrive. Indispensable sur CPU pour éviter de figer. Ne yield rien si LLM indisponible. |

`ChatTurn = { role: "system"|"user"|"assistant", content: string }`.

**Parsing SSE** (`generateStream`) : découpe le flux par lignes ; chaque event utile est
`data: {json}` ; `data: [DONE]` termine ; un fragment JSON incomplet est ignoré. Exactement
le format émis par `llama-server` (et par tout serveur OpenAI-compatible).

**Robustesse** : `AbortController` + timer ; sur `AbortError` (timeout) silencieux, sur autre
erreur `console.warn`. Jamais d'exception remontée à l'appelant → garantit le repli.

---

## 2. Le service — `rpbey-llm.service`

```ini
[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/llm
ExecStart=/home/ubuntu/llm/llama.cpp/build/bin/llama-server \
  -m /home/ubuntu/llm/models/Llama-3.2-3B-Instruct-Q4_K_M.gguf \
  --host 127.0.0.1 --port 8080 \
  -c 8192 -t 10 --parallel 1 -fa on --no-webui \
  --cache-reuse 256
Restart=always
RestartSec=3
Nice=5
MemoryMax=6G
MemoryHigh=5G
```

| Paramètre | Valeur | Sens |
|-----------|--------|------|
| Bind | `127.0.0.1:8080` | **loopback uniquement** (pas exposé publiquement) |
| Modèle | Llama-3.2-3B-Instruct **Q4_K_M** (GGUF) | quantisé 4-bit, ~2 Go sur disque |
| `-c 8192` | contexte | 8192 tokens (cap le prompt + réponse) |
| `-t 10` | threads | 10 threads CPU (**CPU-only**, pas de GPU sur le VPS) |
| `--parallel 1` | concurrence | 1 requête à la fois |
| `-fa on` | flash attention | activée |
| `--cache-reuse 256` | cache prompt | réutilise le préfixe commun entre tours |
| `--no-webui` | — | pas d'UI web, API seule |
| `MemoryMax=6G` | cgroup | borne mémoire dure |

- **Build** : `version: 1 (d749821)`, GNU 15.2.0, Linux x86_64.
- **THP** : `llm-thp.service` (oneshot) met `transparent_hugepage=always` +
  `defrag=defer+madvise` → meilleure perf d'inférence.

### Caractéristiques de perf (CPU-only)

- **~6 s avant le 1er token**, puis **~11 tok/s**. → le **streaming est obligatoire** côté
  chat (sinon l'UI fige plusieurs secondes). C'est pourquoi `generateStream` existe et que la
  route `/api/chat` stream en SSE.

### Endpoints (OpenAI-compatible, loopback)

llama-server expose l'API OpenAI standard ; rpbey n'utilise que :
- `POST /v1/chat/completions` (stream et non-stream).

Vérification rapide (ne renvoie pas de secret) :
```bash
curl -s http://127.0.0.1:8080/v1/models | head
systemctl status rpbey-llm.service --no-pager
```

---

## 3. Cible long terme : le daemon aphrody

D'après le CLAUDE.md rpbey, ce seam OpenAI-compat pointera à terme vers le **daemon aphrody**
(backend IA souverain du VPS : inférence candle + mémoire `aphrody-memory` + multi-persona
`aphrody-agent-home`). La persona Beyblade = profil agent-home `rpbey`
(`~/.aphrody/workspace-rpbey`). Plan référencé : `aphrody/docs/plans/aphrody-ai-backend.md`.

### Ce qu'aphrody doit fournir pour remplacer llama.cpp sans toucher rpbey

1. **Un endpoint OpenAI Chat Completions** (`POST .../v1/chat/completions`) qui accepte
   `{model, messages, stream, temperature, max_tokens, top_p, repeat_penalty}` et répond :
   - non-stream : `{ choices: [ { message: { content } } ] }`
   - stream : SSE `data: { choices: [ { delta: { content } } ] }` … `data: [DONE]`.
2. Respecter le **system prompt** rpbey (français, grounding strict, pas d'URL) — il est passé
   tel quel dans `messages[0]`, donc rien à coder côté aphrody, juste à honorer.
3. Idéalement, supporter le **multi-tour** (mémoire) — déjà fourni dans `messages` par rpbey
   (système + historique + tour courant). aphrody peut superposer sa propre mémoire
   (`aphrody-memory`) par-dessus, mais ce n'est pas requis pour la parité.

### Bascule

Changer une seule variable : `RPBEY_LLM_URL=http://<endpoint-aphrody>/v1/chat/completions`
(via `apps/web/.env`, lu par `rpbey-web.service` en `EnvironmentFile`), puis
`systemctl restart rpbey-web`. Le kill switch `RPBEY_CHAT_LLM=0` permet de désactiver
proprement le LLM (repli extractif) pendant une migration.

> Tant que l'endpoint aphrody répond plus lentement que ~quelques secondes au 1er token,
> garder le streaming est essentiel (déjà le cas). Le timeout `RPBEY_LLM_TIMEOUT_MS=60000`
> peut être ajusté selon la latence d'aphrody.
