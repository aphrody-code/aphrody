<!-- SPDX-License-Identifier: Apache-2.0 -->
# Audit : unification aphrody ↔ winclean (Windows-only specialization)

**Date** : 2026-05-19
**Repos audités** : `C:\src\aphrody\` (cross-platform Rust) + `C:\src\winclean\` (Windows-only NativeAOT C#)
**Déclencheur** : user clarification "winclean est la version windows only d'aphrody"

## Contexte

Avant cette session, la relation entre les deux repos n'était pas explicitement
documentée dans `aphrody/CLAUDE.md`. Les deux instances Claude (aphrody en
`C:\src\aphrody\`, winclean en `C:\src\winclean\`) communiquaient via
mailbox A2A bidirectionnelle (`ai/outbox.jsonl` ↔ `.coord/inbox-from-aphrody.jsonl`)
mais sans cadre conceptuel commun documenté.

User a clarifié : **winclean = aphrody Windows-only specialization**, même org
`aphrody-code`, scope OS-orthogonal. À partir de là, deux questions :

1. Y a-t-il du code/skills/docs côté winclean qui devrait logiquement vivre
   dans aphrody (cross-platform) ?
2. Quels sont les bons critères de séparation ?

## Critères de séparation appliqués

| Type d'item | Va dans aphrody (cross-platform Rust) | Reste dans winclean (Win11 NativeAOT) |
|---|---|---|
| Logique pure Rust portable | ✅ | ❌ |
| C#/.NET NativeAOT P/Invoke Win32 | ❌ | ✅ |
| C++20 deep (DWM, RAMMap, ConPTY) | ❌ | ✅ |
| Skills/agents génériques (RE, deep analysis, protocol) | ✅ | ✅ cache local OK |
| Skills/agents IEVR-spécifiques (port-c-to-rust, smoke-e2e) | ❌ | ✅ |
| Skills/agents Winclean.Mcp-spécifiques (auto-loop-coder, migration-reviewer) | ❌ | ✅ |
| MCP tools Win32 P/Invoke | ❌ | ✅ (146 dans Winclean.Mcp) |
| MCP tools cross-platform (re_triage, voice_*, bxc_*) | ✅ | n/a |
| Docs strategy IEVR | ❌ | ✅ |
| Mailbox A2A `.coord/*-aphrody*` | ❌ | ✅ (peer-side mailbox) |
| Git refs `aphrody-handshake-*` | ❌ | ✅ (history immutable) |

## Audit côté winclean

### Rust files trouvés (hors `_deps/` build artifacts)
- `apps/iecode-web/crates/ievr-engine/` (4 .rs + Cargo.toml)
  → **reste** ; ai.json `decisions_log` 2026-05-17 : "ievr-engine stays in
    C:\winclean. aphrody consumes via cargo path dep". Cohérent : IEVR scope.

### Cargo.toml trouvés (workspace candidates)
- `packages/n2b/Cargo.toml` (submodule `aphrody-code/n2b.git`)
  → **reste** ; aphrody a ses propres working copies `crates/n2b-*`, winclean
    a son clone légitime du même upstream.
- `packages/steamguard-cli/Cargo.toml` → **reste** ; Steam 2FA pour automation IEVR.
- `packages/iecode/cli/build/**/_deps/...` → **reste** ; build dependencies générées (tree_sitter et al.).

### Docs top-level winclean
- `AUTONOMOUS-AI-LOOP-PLAN.md`, `NATIVE-MCP-PLAN.md`, `STACK.md`, `MEMORY.md`,
  `COMMIT_CONVENTION.md` → **restent** ; tous mentionnent IEVR / Winclean.Mcp /
  Steam dans leur scope.

### Skills `.claude/skills/` winclean
- `a2a-duel-loop`, `cpk-verify`, `cs-test`, `port-c-to-rust`, `smoke-e2e`,
  `steam-bootstrap`, `winclean-pr` → **restent** ; tous référencent
  explicitement IEVR / Winclean.Mcp / nie.exe / Steam DB.
- `auto-loop-coder` → **reste** ; explicitement WinClean (`ai.json open tasks` + `.coord/STOP`).
- `aphrody-yolo-grind` → **reste comme cache** ; master vit déjà dans aphrody.

### Agents `.agents/` winclean
- `migration-reviewer.md` → **reste** ; "migrations IPC WinClean + NativeAOT + scripts/ipc/".
- `.agents/skills/cpp-coding-standards`, `csharp-async`, `dotnet-backend-patterns`
  → **restent** ; explicitement C++/C# bound, hors-scope aphrody Rust-only.

### Candidats vraiment génériques (move/copy)

| Skill | Verdict | Action prise |
|---|---|---|
| `.agents/skills/protocol-reverse-engineering` (527 L) | ✅ Generic network RE techniques (Wireshark, tshark, scapy, mitmproxy, JA3/JA3S, custom protocol docs) | **COPIÉ** vers `aphrody/.claude/plugins/aphrody/skills/protocol-reverse-engineering/` (footer "WinClean Integration Rules" stripped) |
| `.agents/skills/deep-analysis` (3 fichiers, 2067 L) | ✅ Generic reverse engineering deep dive — patterns + examples + iterative analysis | **COPIÉ** vers `aphrody/.claude/plugins/aphrody/skills/deep-analysis/` (footer stripped from SKILL.md) |
| `.agents/skills/wasm-compatibility` (220 L) | ⚠️ marimo-notebook spécifique, pas portable au scope aphrody-wasm | **SKIP** (hors-scope) |

## Décision finale : COPY, pas MV

Choix de **copier** (pas `mv` destructif) :

- **Pas de cassure côté peer winclean** — son setup reste fonctionnel.
- **Skills sont distribuables par nature** — chaque peer peut en posséder sa version.
- **Documentation provenance** — frontmatter `source:` indique l'origine et la date du sync.
- **Réversible** — si un peer met à jour son skill, on peut re-sync via `cp`.

Si user veut forcer cleanup côté winclean après confirmation que aphrody est
canonical pour ces 2 skills : `rm -rf C:\src\winclean\.agents\skills\{protocol-reverse-engineering,deep-analysis}`.

## Documentation aphrody mise à jour

- `CLAUDE.md` § 6.0 ajouté — "Relation aphrody ↔ winclean" avec matrice des
  axes (cible OS, langage, binaire, mission, MCP, A2A) + conséquences
  opérationnelles (où vivent les skills, pas de duplication).
- `ai.json` peers[0] (winclean) déjà documenté depuis 2026-05-19 commit `1c74a1056`.

## Skills aphrody après sync

| Catégorie | Skills (post-sync) | Total |
|---|---|---|
| Avant cette session | 34 skills + 1 nested namespace | 35 dirs |
| Après sync winclean | + `protocol-reverse-engineering` + `deep-analysis` | **37 dirs** |

## Validation

- ✅ Aucune mention `winclean` / `IEVR` / `iecode` / `nie.exe` dans les 4 fichiers copiés (grep clean).
- ✅ Frontmatter `name:` + `description:` + `version:` + `source:` présents.
- ✅ WinClean integration footer (toolchain un/unx, French, NativeAOT) stripped des 2 SKILL.md.
- ✅ Write access côté winclean confirmé (`touch /c/src/winclean/.test-aphrody-write` OK) — possible escalade vers `mv` strict si user demande.

## Suite

Aucune autre action winclean→aphrody requise. Le scope est cleanly orthogonal,
la communication A2A est en place, la documentation des conventions est mise à jour.
