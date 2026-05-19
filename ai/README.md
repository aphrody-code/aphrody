<!-- SPDX-License-Identifier: Apache-2.0 -->
# `ai/` — A2A coordination in-tree

> **Source de vérité A2A pour aphrody.** Cette directory est déclarée par
> [`ai.json`](../ai.json) racine via l'extension `file-transport/v1` et
> consommée par les peers (winclean, futurs) qui lisent `ai.json` pour
> savoir où nous écrire et où nous lire.

## Layout

| Path | Rôle | Source de vérité |
|---|---|---|
| `heartbeat.txt` | Proof-of-life ISO-8601 + résumé de session courante | aphrody (écriture exclusive) |
| `outbox.jsonl` | Messages que aphrody envoie aux peers (append-only NDJSON) | aphrody |
| `inbox.jsonl` | Messages reçus des peers (append-only NDJSON) | peers |
| `peers/winclean.ai.json` | Mirror local du `ai.json` de chaque peer | snapshot (refresh manuel ou auto) |

## Convention message

Chaque ligne de `inbox.jsonl` / `outbox.jsonl` = un objet JSON valide
respectant le schéma A2A v1 (cf. `crates/a2a-pb/proto/a2a.proto`). Format
recommandé :

```json
{
  "id": "<unique-id>",
  "ts": "<ISO-8601 UTC>",
  "from": "<sender@org/repo>",
  "to": "<receiver@org/repo>",
  "type": "<fact|ask|reply|heartbeat>",
  "re": "<optional reply-to-id>",
  "subject": "<short>",
  "body": "<markdown ou texte brut>",
  "channel_hint": ["file_jsonl", "http_jsonrpc", "git_tag"]
}
```

## Politique gitignore

- **Structure** (ce README, `peers/.gitkeep`) : **trackée**.
- **Contenu transient** (`heartbeat.txt`, `inbox.jsonl`, `outbox.jsonl`,
  `peers/*.ai.json` snapshots) : **gitignored** — c'est de l'état runtime
  qui change à chaque tick.

Voir [`.gitignore`](../.gitignore) section "A2A runtime".

## Cross-repo coordination

Aphrody coordonne avec le peer `winclean` (cf. `peers/winclean.ai.json`).
Pour bootstrap d'un nouveau peer :

1. Ajouter un objet dans `ai.json` racine sous `peers[]`.
2. Copier le `ai.json` du peer dans `ai/peers/<name>.ai.json`.
3. Échanger les premières envelopes via `outbox.jsonl` / `inbox.jsonl`.

## Compatibilité legacy

L'ancien dossier `C:\winclean\.coord\` reste la source de vérité **côté
peer winclean** — aphrody y écrit en miroir (back-compat transition)
mais le canonique est désormais ici.

Voir `CLAUDE.md` §6.1 pour le protocole complet.
