<!-- SPDX-License-Identifier: Apache-2.0 -->
# PLAN — `aphrody-agent-home` : Soul / Identity / Workspace de l'agent

> Plan d'exécution dédié. Création : **2026-05-23**.
> Cible : combler **tous** les gaps openclaw « soul » + « agent-workspace »
> identifiés dans [`docs/research/openclaw-vs-aphrody.md`](../research/openclaw-vs-aphrody.md),
> via **une crate dédiée** `crates/aphrody-agent-home`, en portant les patterns
> openclaw **puis en les poussant beaucoup plus loin grâce aux garanties mémoire
> de Rust** (mmap zero-copy, cache content-addressed, hot-reload atomique,
> `Send + Sync` par construction).
>
> Concepts amont audités :
> - <https://docs.openclaw.ai/concepts/soul>
> - <https://docs.openclaw.ai/concepts/agent-workspace>
> - Source clonée (gitignorée) : `var/openclaw/src/agents/{workspace,bootstrap-budget,identity,workspace-templates}.ts`
>
> Conventions PLAN : `⏳` actionable sans humain · `✅` clos · `🚧` bloqué ·
> `🚫` hors-scope. Autonomie totale (CLAUDE.md §0.1). Zéro stub (§1).

---

## 0. Pourquoi une crate dédiée

Aujourd'hui aphrody a la **plomberie** (`oc-onboard` crée `~/.aphrody/workspace`,
`aphrody-prompts` rend des templates minijinja avec scrubber PII) mais **pas
l'âme** : aucun `SOUL.md`, aucune file-map workspace seedée, aucun chargement
runtime dans le system prompt, aucun budget anti-bloat. Le tout est éparpillé
entre `cli/src/oc_cmd.rs` et `aphrody-prompts`.

`aphrody-agent-home` centralise **l'identité persistante de l'agent** : son
espace de travail, son âme (persona), son identité (nom/vibe/emoji), l'utilisateur
qu'il sert, ses outils locaux, sa mémoire, et l'assemblage de tout ça en un
system prompt borné. C'est le pendant Rust de `var/openclaw/src/agents/workspace.ts`
+ `bootstrap-budget.ts` + `identity.ts`, unifié et durci.

---

## 1. File-map canonique (port fidèle openclaw)

Verbatim depuis `workspace.ts:21-36` — namespace aphrody (`~/.aphrody/workspace`,
override `$APHRODY_WORKSPACE` puis profil `$APHRODY_PROFILE` →
`~/.aphrody/workspace-<profile>`).

| Fichier | Rôle | Trio onboarding |
|---------|------|:---:|
| `AGENTS.md` | Règles opératoires / comportement | |
| `SOUL.md` | Persona, ton, opinions, brièveté, humour, limites, franchise | ✓ |
| `IDENTITY.md` | Nom de l'agent, vibe, emoji | ✓ |
| `USER.md` | Identité utilisateur + comment l'adresser | ✓ |
| `TOOLS.md` | Conventions outils locaux | |
| `HEARTBEAT.md` | Checklist des runs heartbeat (optionnel) | |
| `BOOT.md` | Checklist de démarrage (optionnel) | |
| `BOOTSTRAP.md` | Rituel one-shot premier run (à supprimer après) | |
| `MEMORY.md` | Mémoire long-terme curée (optionnel) | |
| `memory/YYYY-MM-DD.md` | Logs mémoire quotidiens | |
| `skills/` | Skills propres au workspace (→ `aphrody-skills`) | |
| `canvas/` | Fichiers UI canvas (→ `a2a-ui`) | |
| `.aphrody/workspace-state.json` | État workspace versionné (v1) | |

---

## 2. Gap → solution (mapping exhaustif)

| # | Gap openclaw non comblé | Item PLAN | Statut |
|---|--------------------------|-----------|:---:|
| G1 | Pas de `SOUL.md` ni couche persona | AH-1 `Soul` typé + validation | ⏳ |
| G2 | `oc-onboard` seed un workspace **vide** | AH-4 seeding file-map + templates | ⏳ |
| G3 | Pas de chargement runtime dans le system prompt | AH-5 assembleur `SystemPromptView` | ⏳ |
| G4 | Pas de budget anti-bloat | AH-6 `BootstrapBudget` (per-file + total) | ⏳ |
| G5 | Pas d'`IDENTITY.md` (nom/vibe/emoji) | AH-2 `Identity` typé | ⏳ |
| G6 | Pas d'`USER.md` / `TOOLS.md` structurés | AH-3 `UserProfile` + `ToolsDoc` | ⏳ |
| G7 | Pas de `HEARTBEAT.md` / `BOOT.md` | AH-7 hooks heartbeat/boot | ⏳ |
| G8 | Pas de garde sandbox workspace | AH-8 `WorkspaceGuard` path-containment | ⏳ |
| G9 | Pas de workspace git-backed | AH-9 git-backup via `gix` | ⏳ |
| G10 | Pas de profils multi-agents | AH-10 résolution profil/multi-agent | ⏳ |

---

## 3. « Pousser plus loin » — l'avantage mémoire Rust

openclaw fait du `readFileSync` + un `Map<path, {content, identity}>` JS
caché par `dev:ino:size:mtime` (`workspace.ts:43-88`), recalculé par processus,
mono-thread, et recopie une `String` complète par session. aphrody peut faire
strictement mieux :

| Levier Rust | Gain vs openclaw | Item |
|-------------|------------------|:---:|
| **mmap zero-copy** (`memmap2::Mmap` derrière `Arc`) | Les fichiers bootstrap sont mappés une fois, partagés par toutes les sessions/threads, backed par le page-cache OS, récupérables sous pression — pas de N copies `String`. | AH-11 |
| **Cache content-addressed** (`blake3`) | Clé `(dev,ino,size,mtime)` **+** hash contenu → dédup fichiers identiques entre agents/profils, re-parse incrémental des seuls fichiers changés, index persistant sur disque (survit au restart). | AH-12 |
| **Troncature streaming O(budget)** | `BudgetWriter` tronque sur frontière de grappheme en **une passe**, sans allocation intermédiaire, en émettant les mêmes stats + signature que `bootstrap-budget.ts`. | AH-6 |
| **Hot-reload atomique** (`notify` + `arc-swap`) | Watch SOUL/IDENTITY/USER → swap `Arc<AgentHome>` sans downtime ni restart. openclaw re-lit par session. | AH-13 |
| **`Send + Sync` par construction** | `AgentHome` partageable sur tout le runtime tokio ; les caches openclaw sont des `Map` mono-process JS. | AH-5 |
| **Validation au parse** | `Soul`/`Identity` sont des structs typées serde, pas du markdown libre ; lints heuristiques rejettent les anti-patterns openclaw (life-stories, changelogs, policy-dumps, philosophie vague) **avant** injection. | AH-1 |
| **git pur-Rust** (`gix`) | Backup/restore workspace sans shell-out `git` → cross-platform, dégrade proprement sur wasm. | AH-9 |

---

## 4. API cible (esquisse, `#![forbid(unsafe_code)]` sauf module mmap gated)

```rust
// crates/aphrody-agent-home/src/lib.rs
pub struct AgentHome {            // Send + Sync, Clone (Arc interne)
    pub root: PathBuf,            // ~/.aphrody/workspace[-<profile>]
    pub soul: Option<Soul>,
    pub identity: Identity,
    pub user: Option<UserProfile>,
    pub tools: Option<ToolsDoc>,
    files: Arc<FileCache>,        // mmap + content-addressed (AH-11/12)
}

pub struct Soul {                 // SOUL.md → frontmatter typé (AH-1)
    pub tone: String,
    pub opinions: Vec<String>,
    pub brevity: Brevity,
    pub humor: Humor,
    pub boundaries: Vec<String>,
    pub default_bluntness: Bluntness,
    pub body: String,             // markdown libre résiduel
}

pub struct Identity {             // IDENTITY.md (AH-2)
    pub name: String,
    pub vibe: Option<String>,
    pub emoji: Option<String>,
}

pub struct BootstrapBudget {      // AH-6 — défauts openclaw
    pub max_chars: usize,         // 12_000
    pub total_max_chars: usize,   // 60_000
    pub near_limit_ratio: f32,    // 0.85
}

impl AgentHome {
    pub fn open(opts: HomeOptions) -> Result<Self, HomeError>;
    pub fn onboard(opts: OnboardOptions) -> Result<Self, HomeError>;  // seed file-map
    pub fn system_prompt(&self, budget: &BootstrapBudget) -> SystemPromptView<'_>; // borrowed, AH-5
    pub fn watch(self: &Arc<Self>) -> Result<HomeWatcher, HomeError>; // hot-reload, AH-13
    pub fn git_backup(&self, msg: &str) -> Result<(), HomeError>;     // AH-9
}

pub struct SystemPromptView<'a> { // Cow<'a, str> par fichier, stats budget
    pub sections: Vec<PromptSection<'a>>,
    pub truncation: Option<TruncationReport>,
}
```

Lints au parse (AH-1) : refus si `SOUL.md` contient des marqueurs de
changelog (`## v\d`), des dumps de policy (`SECURITY`, `LICENSE`), ou dépasse un
ratio de phrases « philosophiques vagues » — message actionnable, pas de panic.

---

## 5. Phases & items actionnables

### P0 — Scaffolding crate (⏳ AH-0)
- `crates/aphrody-agent-home/` : `Cargo.toml` (membre workspace), `lib.rs`,
  `#![forbid(unsafe_code)]` global + `unsafe` localisé/justifié dans `mmap.rs`.
- Deps : `serde`, `thiserror`, `blake3`, `memmap2` (host-only), `arc-swap`,
  `notify` (host-only), `gix` (host-only, feature `git`), `unicode-segmentation`.
- Gating wasm : mmap/notify/gix derrière `#[cfg(not(target_arch = "wasm32"))]`,
  fallback lecture mémoire sur wasm.
- Valider versions exactes via `mcp__aphrody__docs_auto_search` (§2.5 CLAUDE.md)
  avant d'épingler — `memmap2`, `arc-swap`, `notify`, `gix`, `blake3`.

### P1 — Modèle de données typé
- ⏳ AH-1 `soul.rs` : `Soul` + parse frontmatter + lints anti-pattern + 6 tests.
- ⏳ AH-2 `identity.rs` : `Identity` (name/vibe/emoji) + parse + tests.
- ⏳ AH-3 `user.rs` + `tools.rs` : `UserProfile`, `ToolsDoc` + tests.

### P2 — Workspace & cache
- ⏳ AH-11 `mmap.rs` : `FileCache` mmap zero-copy `Arc<Mmap>`, gated host-only.
- ⏳ AH-12 `cache.rs` : index content-addressed blake3 + identité `(dev,ino,size,mtime)`,
  persistant `.aphrody/workspace-state.json` (v1), re-parse incrémental.
- ⏳ AH-8 `guard.rs` : `WorkspaceGuard` (canonicalize + containment, réutilise le
  pattern `contained_in` de `aphrody-skills::plugin_manifest`), cap 2 MiB/fichier
  (`MAX_WORKSPACE_BOOTSTRAP_FILE_BYTES`).

### P3 — Assemblage system prompt
- ⏳ AH-6 `budget.rs` : `BootstrapBudget` + `BudgetWriter` streaming (per-file +
  total + near-limit 0.85), `TruncationReport` + signature stable + warning
  dedup (off/once/always), cas spécial `AGENTS.md` — parité `bootstrap-budget.ts`.
- ⏳ AH-5 `assemble.rs` : `system_prompt()` → `SystemPromptView<'_>` (Cow borrowed),
  ordre déterministe (cache-friendly, cf. CLAUDE.md prompt-cache), tests de parité.
- ⏳ AH-7 `heartbeat.rs` + `boot.rs` : injection conditionnelle `HEARTBEAT.md`/`BOOT.md`.

### P4 — Cycle de vie
- ⏳ AH-4 `onboard.rs` + templates `include_str!` : seed `SOUL/AGENTS/IDENTITY/USER/`
  `TOOLS/HEARTBEAT/BOOTSTRAP.md` ; flag `--skip-bootstrap`, refus d'écrasement
  sans `--force`, suppression auto `BOOTSTRAP.md` après premier run.
- ⏳ AH-13 `watch.rs` : `HomeWatcher` (notify + arc-swap), hot-reload SOUL/IDENTITY/USER.
- ⏳ AH-9 `git.rs` (feature `git`) : `git_backup()` via `gix` (init/add/commit),
  restore/clone multi-machine.
- ⏳ AH-10 `profile.rs` : résolution `$APHRODY_PROFILE` → `workspace-<profile>`,
  multi-agent (workspace par agentId).

### P5 — Intégration CLI & runtime
- ⏳ AH-14 brancher `oc_cmd.rs::OcOnboard` sur `AgentHome::onboard` (remplace le
  `create_dir_all` nu lignes 181-214) — seed la file-map, garde la compat
  `aphrody.json`.
- ⏳ AH-15 brancher `aphrody-chat` / `aphrody-prompts` : injecter
  `system_prompt(&budget)` à chaque session (le SOUL « a un vrai poids », parité
  openclaw). Consommé aussi par `agy-loop` et `hermes`.
- ⏳ AH-16 `aphrody doctor` : check workspace (taille bootstrap, troncatures,
  fichiers manquants) — port `doctor-workspace.ts` + `doctor-bootstrap-size.ts`.

### P6 — Durcissement
- ⏳ AH-17 cross-target : `cargo check -p aphrody-agent-home` sur les 3 cibles
  (linux-gnu #1, windows-msvc #2, wasm32 #3) — wasm via fallback no-mmap.
- ⏳ AH-18 bench criterion : cold-load workspace (mmap vs read) + assemblage prompt.
- ⏳ AH-19 mémoire institutionnelle : memory `agent-home-crate` + lien
  `[[vercel-skills-integration]]` (`skills/` du workspace).

---

## 6. Matrice cross-platform

| Module | Linux #1 | Windows #2 | wasm32 #3 |
|--------|:---:|:---:|:---:|
| modèle typé (soul/identity/user/tools) | ✅ | ✅ | ✅ |
| `FileCache` mmap | ✅ memmap2 | ✅ memmap2 | ↩ fallback read mémoire |
| budget / assemblage | ✅ | ✅ | ✅ |
| `notify` hot-reload | ✅ inotify | ✅ ReadDirectoryChangesW | 🚫 (no-op) |
| `gix` backup | ✅ | ✅ | 🚫 (feature off) |

`$APHRODY_HOME` / `USERPROFILE` / `HOME` résolus comme `oc_cmd.rs::home_dir`.

---

## 7. Validation (tolérance zéro, CLAUDE.md §3)

```bash
cargo ci-offline                                   # clippy -D warnings
cargo xt-offline                                   # nextest
cargo check -p aphrody-agent-home --target x86_64-unknown-linux-gnu --locked
cargo check -p aphrody-agent-home --target x86_64-pc-windows-msvc   --locked
cargo check -p aphrody-agent-home --target wasm32-unknown-unknown   --locked
cargo deny check && cargo vet
```
Tests de **parité** : le budget/troncature/signature doivent reproduire les
sorties de `bootstrap-budget.ts` (fixtures dérivées de `var/openclaw`).
Vérif réelle (§7 « Verify strictly ») : `aphrody oc-onboard` puis inspection de
la file-map seedée + `aphrody chat` exhibant le ton SOUL.

---

## 8. Hors-scope (par design, pas des manques)

- 🚫 Apps natives Apple / menu-bar (openclaw macOS) — aphrody est Google-centric.
- 🚫 `canvas/` UI : vit dans `a2a-ui` / `aphrody-ts` (repo frère).
- 🚫 Sandbox Docker/SSH/OpenShell (gap #2 du comparatif) — traité séparément
  dans `aphrody-sdk`, pas ici.
- 🚫 Profils d'auth / credentials : restent dans `~/.aphrody/` (hors workspace),
  jamais dans la crate home.

---

## 9. Ordre d'exécution recommandé

`AH-0 → AH-1/2/3 (parallélisable) → AH-8 → AH-11/12 → AH-6 → AH-5 → AH-4 →
AH-7/9/10/13 → AH-14/15/16 → AH-17/18/19`.

Premier livrable observable (vertical slice) : **AH-0+AH-1+AH-4+AH-5+AH-14** →
`aphrody oc-onboard` seede un `SOUL.md`/`IDENTITY.md` réel et `aphrody chat`
l'injecte. Le reste durcit et pousse au-delà d'openclaw.
