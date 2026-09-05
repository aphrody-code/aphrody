<!-- SPDX-License-Identifier: Apache-2.0 -->

# PLAN-MOONSHOT — 30 jours pour maximiser les stars d'aphrody

> **NOTE (2026-05-21) — planning doc daté, partiellement périmé.** Cette
> stratégie de lancement a été écrite le 2026-05-17 et s'appuie sur des
> primitives depuis **supprimées** : les repos `n2b` / `bxc`, le manifest
> file-based `ai.json` (+ `schemas/ai.json/v1.json`, `docs/posts/2026-05-ai-json.md`),
> et le layout de worktrees `C:/worktree/` (l'ancien `docs/WORKTREES.md` a été
> supprimé le 2026-05-21). Le positionnement « owns ai.json » et les angles
> marketing n2b/bxc ne sont plus valides ; l'A2A passe par gRPC, et le **cœur**
> est 100 % Rust (monorepo polyglotte depuis 2026-05-21 : UI Bun/TS, ML Python —
> cf. [`../CLAUDE.md`](../CLAUDE.md) §2). À ré-évaluer avant tout lancement. Le reste (cadence,
> canaux, risques génériques) demeure utile.
>
> Sibling document to `docs/PLAN.md`. The orchestrator reconciles the two.
> Revision originale : **2026-05-17**.

> **Calibration honnête.** The literal "most-starred repo of all history" is
> openclaw at 372 670 stars (observed 2026-05-17 via `gh repo view`). Surpassing
> that in 30 days is not a deliverable; it is a north star that forces every
> decision through "is this thing actually exceptional?". The realistic 30-day
> outcome of an excellent launch on a hard-systems project of aphrody's size is
> **800 – 5 000 stars** (per `playbook.md`), with a long tail. This plan is
> built to maximise that band — and to leave the door open for the lucky 10x.

---

## 0. Real star-counts (observed 2026-05-17 via `gh repo view --json stargazerCount`)

| Slug                    | Upstream                                                              | Stars      | Forks  | Created     | Last push    |
|-------------------------|-----------------------------------------------------------------------|-----------:|-------:|-------------|--------------|
| `openclaw`              | openclaw/openclaw                                                     | 372 670    | 77 261 | 2025-11-24  | 2026-05-17   |
| `gemini-cli`            | google-gemini/gemini-cli                                              | 104 193    | 13 689 | 2025-04-17  | 2026-05-17   |
| `whisper`               | openai/whisper                                                        |  99 644    | 12 201 | 2022-09-16  | 2026-04-15   |
| `open-design`           | nexu-io/open-design                                                   |  43 566    |  4 979 | 2026-04-28  | 2026-05-17   |
| `agent-browser`         | vercel-labs/agent-browser                                             |  33 246    |  2 065 | 2026-01-11  | 2026-05-13   |
| `vercel-agent-skills`   | vercel-labs/agent-skills                                              |  26 706    |  2 436 | 2025-12-08  | 2026-05-16   |
| `components`            | angular/components                                                    |  25 027    |  6 838 | 2016-01-04  | 2026-05-16   |
| `vercel-skills`         | vercel-labs/skills                                                    |  18 939    |  1 529 | 2026-01-14  | 2026-05-16   |
| `design.md`             | google-labs-code/design.md                                            |  14 162    |  1 338 | 2026-04-10  | 2026-05-08   |
| `open-agents`           | vercel-labs/open-agents                                               |   5 464    |    694 | 2025-12-26  | 2026-05-15   |
| `live-api-web-console`  | google-gemini/live-api-web-console                                    |   2 545    |    728 | 2024-12-09  | 2026-05-17   |
| `n2b`                   | aphrody-code/n2b                                                      |       0    |      0 | 2026-04-17  | 2026-05-17   |
| `bxc`                   | aphrody-code/bxc                                                      |       0    |      0 | 2026-05-10  | 2026-05-17   |

Three observations from the table:

- **Velocity beats age.** `open-design` reached 43 566 stars in ~ 20 days,
  `design.md` 14 162 in ~ 37, `gemini-cli` 104 193 in ~ 13 months. The
  established-and-old `angular/components` (10 years) lands at 25 k.
  Aphrody therefore competes on velocity, not lineage.
- **Tribe-fit > raw quality.** `whisper` (99 644) and `openclaw`
  (372 670) both ride strong narratives ("OpenAI open-source the
  ASR SOTA", "personal AI assistant the lobster way"). Engineering
  excellence alone is insufficient.
- **Aphrody-code's own existing repos (`bxc`, `n2b`) sit at 0 stars** —
  meaning aphrody cannot rely on org reputation. It must launch on
  the strength of the artifact itself.

---

## 1. Per-worktree teardown — what drove the stars, what the first 90 s feels like

Each entry: **(a)** stars + the ONE thing that drove them, **(b)** the
first-90-second README experience, **(c)** the growth artefact worth
copying.

### 1.1 `openclaw/openclaw` — 372 670 stars

- **(a) Driver.** Personality + breadth-of-channel + named sponsors. The
  README opens with `EXFOLIATE! EXFOLIATE!` and the lobster mascot, then
  lists 22 messaging channels (WhatsApp · Telegram · Slack · Discord …)
  before any technical claim. Sponsor row (OpenAI, GitHub, NVIDIA,
  Vercel, Blacksmith, Convex) lands above the install command. Star
  History chart is embedded inline.
- **(b) 90 s.** Mascot SVG (line 4) → tagline (line 11) → 22-channel list
  (line 26) → sponsor strip (lines 36-89) → `npm install -g openclaw` +
  `openclaw onboard --install-daemon` (lines 100-106). A new visitor sees
  "personal AI assistant on every channel I already use" in < 10 s.
- **(c) Growth artefact.** The clawtributors avatar grid (lines 300-318)
  showing ~ 100 contributors as small clickable faces. Social proof on
  steroids.

### 1.2 `google-gemini/gemini-cli` — 104 193 stars

- **(a) Driver.** Google brand + free-tier hook ("60 req/min, 1000
  req/day with personal Google account") as the first bullet
  (`README.md:19`). Apache-2.0 + npm one-liner closes the deal.
- **(b) 90 s.** Five badges (CI / E2E / npm / license / codewiki) →
  hero screenshot (line 9) → tagline (lines 11-13) → free-tier
  bullet → `npx @google/gemini-cli` (line 42). Free + one-liner is the
  punch.
- **(c) Growth artefact.** `star-history.com` embed (lines 398-406)
  + multi-channel release lanes (`@latest` / `@preview` / `@nightly`)
  in lines 78-106 signal "alive, maintained, professional".

### 1.3 `openai/whisper` — 99 644 stars

- **(a) Driver.** Cited paper + benchmark table on languages
  (`README.md:78`). Pure model + 4-line install (`pip install
  -U openai-whisper`). Whisper "owns the noun" — say "transcribe an
  audio" and most engineers think Whisper.
- **(b) 90 s.** Paper + Blog + Model card + Colab links (line 2-5) →
  one-paragraph capability claim (line 8) → architecture diagram
  (line 13) → `pip install -U openai-whisper` (line 22). The
  arXiv paper link earns instant credibility.
- **(c) Growth artefact.** The model-size table (line 64-72) with
  parameters / VRAM / speed columns. The reader self-selects "I'll try
  the `turbo` model" before clicking anywhere else.

### 1.4 `nexu-io/open-design` — 43 566 stars

- **(a) Driver.** Sharp positioning against a closed competitor:
  *"The open-source alternative to Claude Design"* (`README.md:12`) +
  bold one-line claim *"40k stars in two weeks got us this far"*
  (`README.md:6-10`). Bundles **149 design systems × 131 skills** out
  of the box.
- **(b) 90 s.** Editorial banner image (line 14) → 7 shields-row
  (lines 18-26) → multi-locale README links (line 40, 12 languages) →
  positioning sentence "16 coding-agent CLIs auto-detected on your
  PATH" (line 12) → 8-screenshot demo grid (lines 86-128). Visual
  density screams "real product".
- **(c) Growth artefact.** Multi-locale README (12 translations:
  ar/de/es/fr/ja/ko/pt-BR/ru/tr/uk/zh-CN/zh-TW) — captures
  non-English crowds the launch-week post never reaches.

### 1.5 `vercel-labs/agent-browser` — 33 246 stars

- **(a) Driver.** Vercel org backing + comprehensive command surface
  (~ 200 CLI subcommands documented in one README). Owns the noun
  "agent browser" — the verb form (`agent-browser open <url>`) reads
  itself like a tutorial.
- **(b) 90 s.** `npm install -g agent-browser` (line 13) →
  `agent-browser install` to download Chrome for Testing (line 14) →
  9-line quick-start showing snapshot/click/fill/screenshot
  (lines 80-87). The reader runs the first 3 commands within 60 s.
- **(c) Growth artefact.** Dual-channel install (npm + brew + cargo)
  on the same line + 6-platform binary table (line 1220) → no excuse
  not to try. Plus `skills.sh/b/...` badge baked into header.

### 1.6 `vercel-labs/agent-skills` — 26 706 stars

- **(a) Driver.** Vercel-curated content: 6 large skills (react-best-
  practices, web-design-guidelines, react-native, view-transitions,
  composition-patterns, vercel-deploy-claimable) with explicit
  "use-when" triggers. No CLI to learn — just `npx skills add
  vercel-labs/agent-skills`.
- **(b) 90 s.** Skill list (lines 13-150) → install one-liner
  (line 157). The reader sees concrete value before scrolling.
- **(c) Growth artefact.** "Use when:" trigger list per skill (lines
  35-40 for `react-best-practices`) — gives agents and humans a
  decision rule.

### 1.7 `angular/components` — 25 027 stars

- **(a) Driver.** Google + Material Design + 10-year-old enterprise
  inertia. Five npm packages (`@angular/aria`, `cdk`, `material`,
  `google-maps`, `youtube-player`) covered by one repo.
- **(b) 90 s.** npm badge → CI badge → Gitter chat (line 4) →
  package table (line 9-15) → "Getting Started Guide" link
  (line 27). No screenshot, no demo, no positioning — pure
  reference-doc tone.
- **(c) Growth artefact.** Multi-package npm presence + the support
  policy section (line 57) tying it to Angular's release cadence.
  Trust signal for enterprise readers.

### 1.8 `vercel-labs/skills` — 18 939 stars

- **(a) Driver.** Owns the noun: `npx skills add <repo>` is the
  install verb for a growing ecosystem. **51-agent compatibility
  table** (lines 230-283) is the killer feature — every agent
  user finds their own row.
- **(b) 90 s.** `npx skills add vercel-labs/agent-skills` (line 16) →
  source-format examples (GitHub shorthand / URL / GitLab / git@ /
  local path, lines 22-39) → option flags table. The reader skim-reads
  "yes, my agent is supported" and clicks star.
- **(c) Growth artefact.** The "Supported Agents" table with 51 rows
  (AiderDesk, Amp, Antigravity, Augment, IBM Bob, Claude Code,
  OpenClaw, Cline, … Zencoder) — each row is a small inbound search
  channel.

### 1.9 `google-labs-code/design.md` — 14 162 stars

- **(a) Driver.** Concise spec for a freshly-named primitive.
  `DESIGN.md` is to design systems what `README.md` is to docs.
  Google-Labs banner + clear ABNF-like schema. The install is a
  single `npx @google/design.md lint DESIGN.md` (line 56).
- **(b) 90 s.** Spec example (lines 9-50) → lint command + JSON output
  (lines 56-71) → diff command (lines 73-86). The reader instantly
  understands the format and how to enforce it.
- **(c) Growth artefact.** "Token Types" table (lines 119-128) +
  "Section Order" table (lines 134-143) — opinionated structure with
  clear escape hatches ("Preserve; do not error" for unknown
  sections).

### 1.10 `vercel-labs/open-agents` — 5 464 stars

- **(a) Driver.** "Deploy with Vercel" button at line 3 + clear
  architectural take ("the agent is not the sandbox", line 22). The
  fork-and-adapt framing ("not treated as a black box", line 7)
  invites contribution.
- **(b) 90 s.** Vercel deploy button (line 3) → "What it is"
  three-layer ASCII (lines 11-20) → architectural decision section
  (lines 22-31) → env-vars summary (lines 56-90). Reader's
  immediate test: does my Postgres + GitHub App fit the env list?
- **(c) Growth artefact.** Deploy-button URL with pre-filled product
  integrations (Neon Postgres + Upstash KV) — one click to a live
  demo on their own Vercel account.

### 1.11 `google-gemini/live-api-web-console` — 2 545 stars

- **(a) Driver.** Demo first: YouTube thumbnail right under the title
  (line 5). "Free Gemini API key" lowers the barrier. Three demo
  branches (proactive-audio, GenExplainer, GenWeather, GenList)
  triple the surface area.
- **(b) 90 s.** YouTube thumbnail (line 5) → "create a free Gemini
  API key" (line 11) → `npm install && npm start` (line 14) → 50-line
  TypeScript example with vega-embed (lines 34-99). Tutorial-as-README.
- **(c) Growth artefact.** YouTube demo video embedded as a clickable
  thumbnail. Video-first beats text-first for "live API" demos.

### 1.12 `aphrody-code/n2b` — 0 stars

- **(a) Driver.** Not launched. README is technical-French, 178
  lines, no banner image, no demo gif. Solid tech (Rust+TS
  Turborepo, 68 rules, Bun plugin) but zero growth artefacts.
- **(b) 90 s.** Tagline (line 3) → migration patterns list
  (lines 5-9) → architecture tree (lines 12-37) → `cargo build`
  + `bun install`. Engineers like it; new visitors bounce.
- **(c) Growth artefact missing.** No badges row, no screenshot,
  no `npx n2b` one-liner before the technical depth. Migration
  before/after gif is the obvious unbuilt asset.

### 1.13 `aphrody-code/bxc` — 0 stars

- **(a) Driver.** Strong positioning ("Zero-Spawn Browser Engine for
  AI Agents") + benchmark table ("10× faster cold start, 100× faster
  DOM query"). 1-click curl install (line 33). Mermaid architecture
  diagram (lines 71-79). Tone is launch-ready.
- **(b) 90 s.** Title (line 1) → 4-badge row (lines 5-8) → positioning
  paragraph (line 11) → "Why Bxc" bullets (lines 16-26) → curl install
  (line 33) → benchmark table (line 56-60). Reader knows in 60 s.
- **(c) Growth artefact missing.** No Show HN yet, no Discord/Discussion
  link, no animated benchmark gif. README is ready; the launch is not.

---

## 2. Transferable patterns (13 distilled, each cited and mapped to aphrody)

### P1 — Editorial banner image above the first heading

- **Evidence.** `open-design/README.md:14` `<img src="docs/assets/banner.png"
  width="100%" />`. `openclaw/README.md:5-8` lobster mascot SVG, dark/light
  responsive.
- **Aphrody today.** `assets/aphrody-social-preview.svg` (1280×640) exists
  but is NOT referenced in `README.md`.
- **Action.** Embed `assets/aphrody-social-preview.svg` as line 4 of
  `README.md`, before badges. **Deliverable D+1.**

### P2 — Multi-shield row with consistent style (`for-the-badge`)

- **Evidence.** `open-design/README.md:18-26` seven shields with
  `style=for-the-badge`, fixed `labelColor=0d1117`, semantic colour
  per metric (stars=gold, forks=green, issues=red, PRs=purple,
  contributors=blue, commits=orange, last-commit=violet).
- **Aphrody today.** Six small flat-style badges (lines 12-18) using the
  default `for-the-badge` is absent.
- **Action.** Rewrite badge row with `style=for-the-badge` + consistent
  palette. **Deliverable D+2.**

### P3 — Multi-locale README (12 translations on `open-design`)

- **Evidence.** `C:/worktree/open-design/` contains README.{ar,de,es,fr,
  ja-JP,ko,pt-BR,ru,tr,uk,zh-CN,zh-TW}.md alongside README.md. Line 40
  is a centered language switcher.
- **Aphrody today.** README is bilingual EN/FR mixed in one file. No
  language switcher.
- **Action.** Split into `README.md` (canonical EN) + at minimum
  `README.fr.md` + `README.zh-CN.md` + `README.ja.md`. Wire the
  switcher as line 22. **Deliverable D+10.**

### P4 — Hero command demo in console block (cast or asciinema)

- **Evidence.** `bxc/README.md:42-49` shows the "God Mode" 4-line
  TypeScript usage inline. `agent-browser/README.md:80-87` shows
  the 7-command quick-start that produces a screenshot.
- **Aphrody today.** `assets/aphrody-demo.cast` + `aphrody-doctor-
  demo.cast` exist. Neither is rendered to gif yet.
- **Action.** Convert both casts to gifs via `agg --theme aphrody-dark`,
  embed `aphrody-demo.gif` as line 24 of README.md. **Deliverable D+4.**

### P5 — Bundle a large catalogue out of the box

- **Evidence.** `open-design/README.md:67` "129 design systems +
  31 skills". `vercel-skills/README.md:230` 51-agent compatibility
  table.
- **Aphrody today.** `docs/audits/aphrody-completeness.md:33` reports
  16 skills, 10 design-systems mirrored, target 152.
- **Action.** Close the design-systems gap (10 → 152). Document the
  number in the README hero ("152 brand-grade DESIGN.md systems +
  131 skills mirrored from open-design + openclaw"). **Deliverable
  D+8.**

### P6 — One-liner install path per platform on the first scroll

- **Evidence.** `gemini-cli/README.md:36-72` shows npx + npm-global +
  brew + macports + conda one-liners stacked vertically.
- **Aphrody today.** `README.md:67-76` already has curl-sh + irm-ps1 +
  scoop + brew. Cargo + winget missing.
- **Action.** Add `cargo install aphrody`, `winget install
  aphrody-code.aphrody`, `npm install -g @aphrody-code/aphrody-wasm`
  to the install block. **Deliverable D+6.**

### P7 — Star-history.com embedded chart

- **Evidence.** `gemini-cli/README.md:398-406` + `openclaw/README.md:278`.
- **Aphrody today.** Absent.
- **Action.** Add chart as the penultimate README section once star
  count crosses 50 (so the chart is not empty). **Deliverable D+15.**

### P8 — Named sponsor strip

- **Evidence.** `openclaw/README.md:36-89` 6 named sponsors
  (OpenAI, GitHub, NVIDIA, Vercel, Blacksmith, Convex) with
  light/dark logo `<picture>` blocks.
- **Aphrody today.** No sponsor strip. `FUNDING.yml` exists but is
  not surfaced in README.
- **Action.** Apply to GitHub Sponsors + thanks.dev. Surface
  whatever lands (even a single sponsor) under
  `README.md#sponsors`. **Deliverable D+18 (organic).**

### P9 — Discord / Discussions / Matrix prominent link

- **Evidence.** `open-design/README.md:35` Discord shield with
  invite link. `openclaw/README.md:17` discord badge at the top.
- **Aphrody today.** `docs/COMMUNITY.md` mentions future Discord;
  no invite created yet.
- **Action.** Create Discord server `aphrody`, generate vanity
  invite `discord.gg/aphrody`, badge it in the shield row.
  Enable GitHub Discussions. **Deliverable D+5.**

### P10 — "Use when" agent trigger list per skill

- **Evidence.** `vercel-agent-skills/README.md:35-40` (react-best-
  practices "use when" bullets). `open-design/README.md:138-180`
  (skill table with platform / scenario / output).
- **Aphrody today.** `.claude/skills/<name>/SKILL.md` files have
  frontmatter; the README never enumerates them.
- **Action.** Add a `## Skills shipped (16)` table in README listing
  name + trigger phrase + output for each of the 16 SKILL.md files
  under `.claude/skills/`. **Deliverable D+7.**

### P11 — Specific testable benchmark in the README hero

- **Evidence.** `bxc/README.md:55-60` "10x cold start, 6x memory,
  100x DOM query". `aphrody/README.md:26-55` already has the
  `mrx scan` 55 ms wall-clock + 14 ms internal walk.
- **Action.** Promote the `mrx` numbers from line 26 to a single
  shield in the badge row: `mrx scan 19k files | 1.4 s warm`.
  **Deliverable D+3.**

### P12 — YouTube/Loom demo video embedded as thumbnail

- **Evidence.** `live-api-web-console/README.md:5` clickable
  thumbnail → YouTube. `openclaw` has no video but compensates
  with the mascot.
- **Aphrody today.** No video. asciinema casts exist.
- **Action.** Record 90 s screencast of `aphrody doctor` + `mrx scan`
  + cross-platform `aphrody --version` on Linux/Win/wasm. Upload
  unlisted YouTube + Loom. Embed thumbnail above demo console
  block. **Deliverable D+14.**

### P13 — Spec-first repos own a noun

- **Evidence.** `google-labs-code/design.md` (14 162 stars)
  owns "DESIGN.md". `vercel-labs/skills` (18 939) owns the `skills`
  verb (`npx skills add`).
- **Aphrody today.** `ai.json` is a real candidate primitive
  (Claude-to-Claude A2A manifest), already published as
  `schemas/ai.json/v1.json` + `docs/posts/2026-05-ai-json.md`.
- **Action.** Promote `ai.json` to its own positioning bullet in the
  README hero: "Owns `ai.json` — the file-based A2A manifest two
  agents use to coordinate". Submit `ai.json` schema to
  schemastore.org. **Deliverable D+9.**

---

## 3. Aphrody current-state diff vs the patterns above

### What aphrody already has that resonates

- **Cross-platform Rust binary that actually runs** on Linux, Windows,
  wasm32-unknown-unknown, wasm32-wasip1 (matrix in `docs/PLAN.md:69-101`).
  Real differentiator vs gemini-cli (Node) and open-design (Next.js).
- **`mrx` Monorepo Real-time X-platform Mapper** — 19 213 files /
  482 MB scanned in 1.4 s warm (`README.md:177-186`). Specific
  testable benchmark already published.
- **`ai.json` A2A protocol** — the only repo in the catalogue with
  a file-based 3-deep handshake demonstrated across two Claude Code
  instances (`docs/posts/2026-05-ai-json.md`, 2 076 words).
- **Supply-chain Google-grade** — `cargo-vet` feeds from Google /
  Mozilla / Fuchsia / ChromeOS / Bytecode Alliance / Embark / Zcash
  (`README.md:425-429`). No other repo in the catalogue makes this
  claim.
- **`387/387` nextest green** on Windows (`docs/PLAN.md:65`). Real
  green CI, not a façade.
- **8-worktree dep catalogue** + `gemini-live-aphrody/` 1 000-line
  fork — shows the project consumes upstream seriously.
- **209 SKILL.md** aggregated by `packages/aphrody-skills/`
  (16 first-party in `.claude/skills/` + 193 imported via
  `sources.ts` from open-design + openclaw + vercel-skills +
  gemini-cli + agent-browser + vercel-agent-skills + open-agents).

### What aphrody is missing vs the high-star upstreams

| Gap                                                | Pattern | Worktree precedent                |
|----------------------------------------------------|---------|-----------------------------------|
| Banner image in README                             | P1      | open-design, openclaw             |
| `for-the-badge` style consistency                  | P2      | open-design                       |
| Multi-locale README (zh-CN, ja, fr)                | P3      | open-design                       |
| Demo gif embedded above install block              | P4      | open-design (8-shot grid)         |
| 152-system DESIGN.md mirror finished               | P5      | open-design (149 systems)         |
| `cargo install aphrody` one-liner                  | P6      | gemini-cli (`npx ...`)            |
| Star-history chart                                 | P7      | gemini-cli, openclaw              |
| Sponsor strip                                      | P8      | openclaw (6 sponsors)             |
| Active Discord                                     | P9      | open-design, openclaw             |
| Per-skill `use-when` trigger surfaced              | P10     | vercel-agent-skills               |
| YouTube/Loom demo video                            | P12     | live-api-web-console              |
| "Owns a noun" positioning (`ai.json`)              | P13     | design.md                         |
| Clawtributors avatar grid                          | (P14)   | openclaw (~ 100 avatars)          |

The gaps are all README / community-plumbing work. The engineering
substrate is in place.

---

## 4. The 30-day moonshot arc

### Positioning sentence (proposed)

> **aphrody — one Rust binary, three operating systems, one ai.json:
> the cross-platform CLI two Claude Codes use to coordinate, and the
> file-based agent manifest other agents can adopt.**

Calibrated against `open-design`'s hero
("The open-source alternative to Claude Design. Local-first,
web-deployable, BYOK at every layer"): same shape — one phrase, one
concrete differentiator, one verb.

### The single hero demo (15-30 s)

**Subject.** Split-screen terminal: left pane Linux (Ubuntu 26.04 via
WSL), right pane Windows 11 PowerShell. Both run, in lockstep:

1. `aphrody --version` → `aphrody 1.0.0-canary` (both panes).
2. `mrx scan --root .` → 1.4 s warm, JSON output (both panes).
3. `aphrody a2a send --to winclean "ping"` → envelope flushed into
   `.coord/inbox-from-aphrody.jsonl` → peer Claude on winclean side
   acknowledges via `inbox-from-winclean.jsonl`.

**Command that produces it.**

```bash
bun run scripts/demo-record.ts --output assets/aphrody-hero.gif \
  --duration 25 \
  --split-screen linux:wsl,windows:pwsh7 \
  --commands "aphrody --version,mrx scan --root .,aphrody a2a send --to winclean ping"
```

(Script `scripts/demo-record.ts` is **D+4** deliverable. Falls back
to two recorded asciinema casts stitched with `ffmpeg -filter_complex
hstack` if Bun-driven recording is unavailable.)

### HN / Reddit / Lobste.rs launch sequence

Per `playbook.md` rules (Show HN first, 5-7 days gap, technical post
before submission).

| When | Where | Exact title |
|------|-------|-------------|
| D+15, Tue 13:00 UTC | Show HN | **`Show HN: aphrody — one Rust binary on Linux, Windows, and wasm32 — 19,213 files mapped in 1.4 s`** |
| D+15 | r/rust | **`aphrody: cross-platform Rust CLI with hermetic Google-grade supply chain (cargo-vet + 7 feeds)`** |
| D+20, Tue 13:00 UTC | Lobste.rs (submit the post, not the repo) | **`File-based A2A manifest: how two Claude Codes coordinated across two repos using ai.json + JSONL mailbox`** |
| D+22 | r/programming | **`Two Claude Code instances coordinated for 24h via a 4-channel file-based protocol — here is the schema`** |
| D+24 | Dev.to (mirror of the technical post for SEO) | `Designing ai.json — a file-based A2A manifest` |
| D+28 | YouTube demo upload + tweet thread | (untitled — just publish) |

Modeled after Show HN templates that actually shipped:
- **whisper**'s arXiv-link pattern → we mirror with the
  `docs/posts/2026-05-ai-json.md` (existing 2 076-word piece) as
  the depth-link.
- **bxc**'s benchmark-table pattern → we mirror with `mrx scan`
  19 213-files-in-1.4 s number.
- **open-design**'s "open-source alternative to X" pattern → we
  mirror with "cross-platform alternative to platform-specific
  CLIs (gemini-cli is Node-only; aphrody is one Rust binary)".

### Repos / influencers to ping for boosts (5-10)

- **`steipete` (openclaw maintainer)** — built openclaw to 372 670
  stars; the A2A `ai.json` directly mirrors openclaw's channel
  architecture, plausible mutual-interest mention.
- **`burntsushi` (ripgrep)** — `mrx` is built on his `ignore` crate
  (`README.md:179`). Genuine credit + a Tweet ack would be
  high-value.
- **`mitsuhiko` (Armin Ronacher, `uv`)** — Rust-CLI-distribution
  patterns; `uv` and aphrody share the curl-bash + cargo +
  winget + brew install matrix.
- **`pablovirgo` (Vercel DevRel)** — aphrody consumes
  `vercel-labs/{agent-browser,agent-skills,skills,open-agents}`
  via `packages/aphrody-skills/src/sources.ts`; reciprocal
  acknowledgement plausible.
- **`tj` (TJ Holowaychuk)** — collects interesting CLIs; aphrody
  fits the "well-crafted CLI" bucket.
- **`indragiek` (Indragie Karunaratne)** — Mac/Windows tooling; the
  cross-platform-from-Rust angle resonates.
- **HN user `nikolayvk`** — operates `simonwillison/llm`-adjacent
  posts; a thoughtful comment from him on the Show HN moves needles.
- **`Marak` / `darccio` / `epage` (clap maintainer)** — aphrody uses
  clap + clap_complete (`docs/PLAN.md:201`); credit them and they may
  amplify.
- **`zh.r/rust` mod team** — for a translated cross-post of the
  technical thread, post-launch.

(Pinging means: open a thoughtful issue or send a courteous DM,
linking the specific upstream feature aphrody uses. Do NOT mass-DM.)

### 1-click install per platform (curated)

| Platform | Command | Status |
|----------|---------|--------|
| Linux / macOS | `curl -sSf https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.sh | sh` | ✅ shipped (`packaging/install.sh`) |
| Windows (PowerShell 7+) | `irm https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.ps1 | iex` | ✅ shipped |
| Scoop (Windows) | `scoop bucket add aphrody https://github.com/aphrody-code/scoop-bucket && scoop install aphrody` | ⏳ bucket needs publishing |
| Homebrew | `brew install aphrody-code/tap/aphrody` | ⏳ tap needs publishing |
| WinGet | `winget install aphrody-code.aphrody` | ⏳ manifest exists, PR to `winget-pkgs` open |
| Cargo | `cargo install aphrody` | ⏳ blocked on publishing `base`, `backend`, `a2a-*` to crates.io first |
| npm (wasm) | `npm install @aphrody-code/aphrody-wasm` | ⏳ scaffolded, `wasm-pack publish --access public` awaits `npm login` |
| Snap | `snap install aphrody` | ✅ manifest in `packaging/snap/`, awaits store publish |
| AUR | `yay -S aphrody-bin` | ✅ PKGBUILD in `packaging/aur-bin/`, awaits AUR push |
| Nix | `nix run github:aphrody-code/aphrody` | ✅ `flake.nix` ready |
| Chocolatey | `choco install aphrody` | ✅ `aphrody.nuspec` ready |
| RPM (Fedora) | `dnf install aphrody` | ✅ `aphrody.spec` ready |
| Flatpak | `flatpak install com.aphrody.aphrody` | ✅ manifest ready |

Goal at D+15: every row in the table is ✅.

### Community plumbing

- **Discord.** `discord.gg/aphrody` vanity — channels `#general`,
  `#mrx`, `#a2a`, `#contrib`, `#help-linux`, `#help-win11`,
  `#help-wasm`, `#design-systems`. **D+5.**
- **GitHub Discussions.** Enable the 6 default categories.
  Pre-seed 3 questions ("Why one Rust binary across Linux/Win/wasm?",
  "What is `ai.json` and how do two agents handshake?", "How does
  `mrx scan` differ from `tokei` or `scc`?"). **D+5.**
- **Multi-locale `CONTRIBUTING`.** EN canonical +
  `CONTRIBUTING.fr.md` + `CONTRIBUTING.zh-CN.md` + `CONTRIBUTING.ja.md`.
  **D+12.**
- **`good first issue` label.** Pre-tag 15 issues
  (sample: "Add `aphrody --color=never` flag", "Translate
  `docs/POST-LAUNCH.md` to French", "Wire `mrx scan --watch` debounce
  flag to env var", …). **D+11.**
- **`docs/HALL-OF-FAME.md`.** Mirror openclaw's clawtributors grid
  pattern. Auto-generate via GitHub Action on every merge. **D+19.**

---

## 5. Risk register (10 specific risks + mitigations)

| # | Risk | Concrete mitigation |
|---|------|---------------------|
| R1 | Buying stars / using star-bots → GitHub unlists or shadow-deletes the repo (`playbook.md:124`). Permanent project death. | Never. Rejected at the policy level. `docs/COMMUNITY.md` to add explicit "we do not accept star-trading" clause. |
| R2 | Premature Show HN before the demo gif lands → top reply "no screenshot, looks like vapor" kills the thread. | Gate the Show HN behind `[ -f assets/aphrody-hero.gif ]`. **Do not** submit before D+15. |
| R3 | Bundling 152 brand DESIGN.md inflates repo to > 1 GB clone size → bootstrap times become a meme. | Mirror under `assets/design-systems/` via Git LFS OR keep them in `C:/worktree/open-design/` and reference paths only (current pattern in `docs/WORKTREES.md`). |
| R4 | "aphrody" name collision risk (likely fine — `gh search repos aphrody` returns < 20 distinct repos, none in the CLI space). | Pre-launch: lock npm `@aphrody-code/*` scope, PyPI `aphrody-cli`, Docker Hub `aphrody/cli`. Done by D+13. |
| R5 | French/English code-switch in README repels English-only HN crowd → bounce rate spikes. | D+10 split into `README.md` (pure EN) + `README.fr.md`. Pattern from `open-design`. |
| R6 | `cargo install aphrody` fails because `base`, `backend`, `a2a-*` are not yet on crates.io → reader's first command errors. | D+11 ship full publish ladder per `docs/cargo/PUBLISH-LADDER.md`. Verify with `scripts/verify-publish.sh`. |
| R7 | Demo gif looks staged or fake → "show me the real terminal" comments. | Record live, no edits, no terminal-themed overlays. Use real WSL + real Win11 PowerShell. Sign the gif's SHA-256 into `assets/aphrody-hero.gif.sha256`. |
| R8 | Aphrody is wrongly perceived as "another LLM CLI" instead of "cross-platform Rust CLI + A2A protocol". The 30-second test fails on positioning, not on tech. | D+1 positioning sentence revision + D+9 owns-a-noun framing around `ai.json`. |
| R9 | Discord opens; nobody joins → empty server is a trust signal worse than no server (`playbook.md` anti-pattern: "dead community is hostile"). | Soft-launch Discord D+5 with 3-5 invited friendlies (Claude AI eng team, ripgrep maintainer, peer winclean Claude). Public link only D+13. |
| R10 | A2A `ai.json` peer (winclean) goes silent during launch week → live demo of "two Claude Codes coordinating" reads as fiction. | Pin a `winclean` snapshot tag `aphrody-launch-d0`. Render the 6-channel handshake as a static `docs/posts/2026-05-ai-json.md` figure (already in place: 2 076 words + traces). The recording is reproducible from the tag. |

---

## 6. Top-50 punch list (sorted by leverage, highest first)

Each item is `[ ] D+N — <one-line task> (filename or command)`. The
deliverable is concrete; the date is the latest acceptable land
window.

- [ ] D+1 — Rewrite README first 30 lines around new positioning sentence (`README.md` lines 1-30).
- [ ] D+1 — Embed `assets/aphrody-social-preview.svg` as line 4 of README (P1).
- [ ] D+2 — Replace badge row with `for-the-badge` style + consistent palette (`README.md:12-18`, P2).
- [ ] D+3 — Promote `mrx scan 19k files | 1.4 s warm` to a top-row shield (`README.md`, P11).
- [ ] D+4 — Record + embed `assets/aphrody-hero.gif` (split-screen Linux+Win, 25 s; `scripts/demo-record.ts` new file, P4).
- [ ] D+4 — Convert existing asciinema casts to gif as fallback (`agg --theme aphrody-dark assets/aphrody-demo.cast assets/aphrody-demo.gif`).
- [ ] D+5 — Create Discord server, vanity invite `discord.gg/aphrody`, badge in shields row (P9).
- [ ] D+5 — Enable GitHub Discussions; pre-seed 3 Q&A topics.
- [ ] D+6 — Add `cargo install aphrody` + `winget install aphrody-code.aphrody` + `npm install -g @aphrody-code/aphrody-wasm` rows to install block (`README.md:67-76`, P6).
- [ ] D+7 — Add `## Skills shipped (16)` section listing each `.claude/skills/<name>/SKILL.md` with use-when trigger (`README.md`, P10).
- [ ] D+8 — Drive `assets/design-systems/manifest.json` from 10 → 152 entries via `bun run scripts/design-systems-import.ts --batch=100` (audit cite: `docs/audits/aphrody-completeness.md:33`, P5).
- [ ] D+9 — Add `## ai.json — the A2A manifest` section to README; submit schema to schemastore.org PR (`schemas/ai.json/v1.json`, P13).
- [ ] D+10 — Split bilingual README → `README.md` (EN) + `README.fr.md` + `README.zh-CN.md` + `README.ja.md`; wire switcher (P3).
- [ ] D+11 — Publish `base`, `backend`, `a2a-pb`, `a2a-client`, `a2a-server`, `a2a-grpc`, `a2a`, `aphrody` to crates.io in topological order per `docs/cargo/PUBLISH-LADDER.md`; verify via `scripts/verify-publish.sh` (R6).
- [ ] D+11 — Pre-tag 15 `good first issue`s in GitHub Issues.
- [ ] D+12 — Translate `CONTRIBUTING.md` → `.fr.md` + `.zh-CN.md` + `.ja.md`.
- [ ] D+12 — Push Homebrew formula to `aphrody-code/homebrew-tap`; verify `brew install aphrody-code/tap/aphrody`.
- [ ] D+12 — Push Scoop bucket to `aphrody-code/scoop-bucket`; verify `scoop install aphrody`.
- [ ] D+13 — Open WinGet PR `microsoft/winget-pkgs` for `aphrody-code.aphrody` v1.0.0.
- [ ] D+13 — Lock external name slots: `@aphrody-code/*` (npm), `aphrody-cli` (PyPI), `aphrody/cli` (Docker Hub), `aphrody` (AUR, snap, Flathub), `aphrody.dev` (DNS), `discord.gg/aphrody` (Discord vanity) (R4).
- [ ] D+13 — Promote Discord invite from soft to public.
- [ ] D+14 — Record YouTube screencast (90 s, cross-platform `aphrody doctor`), upload unlisted, embed thumbnail in README (P12).
- [ ] D+14 — Tag `v1.0.0` from `main`. Trigger `release.yml` workflow (already wired). Verify SHA-256 on each artefact via `scripts/sbom-extract.sh`.
- [ ] D+15 — Submit Show HN at Tue 13:00 UTC: title `Show HN: aphrody — one Rust binary on Linux, Windows, and wasm32 — 19,213 files mapped in 1.4 s` (per `docs/launch/SHOW-HN.md`).
- [ ] D+15 — Submit r/rust post titled `aphrody: cross-platform Rust CLI with hermetic Google-grade supply chain (cargo-vet + 7 feeds)`.
- [ ] D+15 — Begin 2-hour comment response cadence per `docs/POST-LAUNCH.md` (already drafted).
- [ ] D+16 — Triage every new issue within 4 h. Label + acknowledge. Pattern: `playbook.md:104`.
- [ ] D+17 — Ship one user-requested feature within 24 h of the first concrete ask. Post `Show HN follow-up: heard you, shipped X`.
- [ ] D+18 — Open GitHub Sponsors + thanks.dev profile, surface in README `## Sponsors` (P8).
- [ ] D+19 — Generate `docs/HALL-OF-FAME.md` avatar grid via GH action; reference from README (P14 / openclaw pattern).
- [ ] D+20 — Submit Lobste.rs (the post, not the repo): title `File-based A2A manifest: how two Claude Codes coordinated across two repos using ai.json + JSONL mailbox`.
- [ ] D+21 — Diagnostic gate per `playbook.md:135`: if star count < 500, double down on ENGINEERING value (ship feature) not posting. If > 5000, deploy the most-requested-feature push.
- [ ] D+22 — r/programming cross-post: `Two Claude Code instances coordinated for 24h via a 4-channel file-based protocol — here is the schema`.
- [ ] D+23 — Push Snap to store + AUR `aphrody-bin` package (`packaging/snap/`, `packaging/aur-bin/`).
- [ ] D+24 — Dev.to mirror of the `2026-05-ai-json.md` technical post for long-tail SEO.
- [ ] D+25 — Submit `aphrody` to `awesome-rust`, `awesome-cli`, `awesome-cross-platform`, `awesome-wasm` (PRs to upstream awesome-X lists).
- [ ] D+26 — Open issues on 5 friendly upstreams (`ripgrep`, `uv`, `clap`, `tokio`, `wasm-bindgen`) thanking them for the dependency aphrody uses. Net-positive PR welcome.
- [ ] D+26 — Publish Flatpak to Flathub via `packaging/flatpak/com.aphrody.aphrody.json`.
- [ ] D+27 — Re-record `aphrody-hero.gif` with v1.0.1 numbers if any update shipped post-launch. Refresh README.
- [ ] D+28 — YouTube demo public, accompany with Twitter/X thread (5 tweets, technical, with the asciinema link).
- [ ] D+29 — Star-history.com chart added to README penultimate section once count crosses 50 (P7).
- [ ] D+30 — Retrospective post `docs/posts/2026-06-aphrody-30-days.md`: real star count, what worked, what flopped, next 60-day plan.
- [ ] D+30 — Update `docs/PLAN.md` with the Phase R section (post-moonshot maintenance arc).
- [ ] D+8 — Wire `bun run scripts/skills-harvest-vercel.ts` so all 51 `vercel-skills` agents resolve to a row in `packages/aphrody-skills/src/sources.ts` (P5 inversion).
- [ ] D+11 — Generate `docs/COMPATIBILITY.md` listing every CLI / agent that resolves an aphrody skill (analogue of vercel-skills 51-agent table).
- [ ] D+16 — Publish `crates/m3-tokens` to crates.io independently; pitch on `r/webdev` as "M3 baseline tokens for any Rust project".
- [ ] D+17 — Open PR to `google-labs-code/design.md` adding `aphrody` to the "Built on DESIGN.md" section if such a section exists; otherwise propose creating one.
- [ ] D+12 — Publish `crates/mrx-cli` standalone on crates.io as `mrx` (high-leverage standalone — bench numbers already public).
- [ ] D+19 — Add aphrody `Action` to GitHub Marketplace (`action.yml` calling `cargo install aphrody && aphrody mrx scan --root .`).
- [ ] D+24 — Submit `mrx` to `r/datahoarder` + `r/programming` separately as a standalone tool (small tool, sharp claim).
- [ ] D+28 — Pin a public GitHub Project board with the next 30 days' roadmap to signal "alive, planned".

---

## 7. Source-of-truth back-references

- `docs/WORKTREES.md` — 13-worktree catalogue + bootstrap one-liner.
- `.claude/skills/start/references/playbook.md` — the 30-second test, channel
  ranking, Show HN templates, anti-patterns. **Do not contradict.**
- `docs/launch/SHOW-HN.md` — Show HN title candidates + comment templates.
- `docs/POST-LAUNCH.md` — D+0 / +24h / +72h / +7d engagement protocol.
- `docs/audits/aphrody-completeness.md` — coverage matrix vs open-design +
  openclaw; tracks the 152-system gap, the 16-skill count, the 26-crate
  workspace.
- `docs/audits/2026-05-17-open-design-openclaw-harvest.md` — original
  harvest map; the prompt queue at the end of that file feeds items D+8
  and D+12 of this plan.
- `docs/PLAN.md` — operational engineering plan; this moonshot doc does NOT
  duplicate engineering tasks (`P-Linux`, `P-Win11`, `P-Wasm`, `P-A2A`),
  it focuses on README / launch / distribution / community plumbing.

---

## 8. Honest closing note

The 30-day target is **800 - 5 000 stars** with a long-tail trajectory.
The 372 670-star openclaw bar is unreachable in 30 days. The
moonshot framing exists to force the question *"is this thing
actually exceptional?"* through every decision — and the answer to
that question is the only durable lever.

The two scenarios at D+21 (per `playbook.md:135`):

- **< 500 stars** → the answer is more engineering value + sharper
  positioning, NOT more posting.
- **> 5 000 stars** → double down on the angle that landed; ship
  the most-requested feature in 24 h; re-launch the hero gif with
  the post-launch v1.0.1 numbers.

Either way the work after D+30 is the same — continue shipping the
cross-platform Rust binary on the same hermetic supply chain, keep
the A2A `ai.json` protocol moving with a real peer, and let the
long-tail of awesome-X listings + dev.to SEO compound.

*Reconciliation between this doc and `docs/PLAN.md` is the
orchestrator's job, not aphrody's.*
