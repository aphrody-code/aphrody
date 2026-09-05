<!-- SPDX-License-Identifier: Apache-2.0 -->

# Audit — open-design + openclaw extraction map (cross-repo harvest)

**Date (UTC):** 2026-05-17
**User request:** "fork both nexu-io/open-design + openclaw/openclaw,
extract their DESIGN.md system, SKILL.md, memory consolidation,
Gemini CLI integration, Antigravity native OAuth, voice-talk Discord
provider, AI gateway server, AI minimal env, MCP open-design core,
agui package, and all best resources for us."
**Clones landed at:**
- `C:\worktree\open-design\`  (nexu-io/open-design, 309 MB)
- `C:\worktree\openclaw\`     (openclaw/openclaw,  239 MB)
- `C:\worktree\design.md\`    (google-labs-code/design.md spec)
- `C:\worktree\google-labs-skills\` (upstream of design-md skill)

## 1. DESIGN.md system

| Target ask                | Where in open-design / openclaw                             | Status                                     |
|---|---|---|
| DESIGN.md spec            | `C:/worktree/design.md/docs/spec.md` (canonical)            | INTERNALISED — `aphrody/DESIGN.md` ships v1, lint exit 0 errors |
| 152 brand DESIGN.md files | `C:/worktree/open-design/design-systems/<slug>/DESIGN.md`   | available for reference (152 systems incl. agentic, airbnb, ant, apple, arc, atelier-zero, bento, canva, claude, cohere, coinbase, cisco, ...) |
| design-md skill           | `C:/worktree/open-design/skills/design-md/SKILL.md`         | mirrors `google-labs-code/skills` upstream |

**Ship plan:** add an aphrody-side `scripts/design-md-import.ts` that
lazily pulls a referenced upstream DESIGN.md when an agent asks for
`{brand=airbnb}` or `{brand=ant}`. Source of truth stays in open-design.

## 2. SKILL.md framework

| Target ask                  | Where                                                                                 |
|---|---|
| daemon-side skill loader    | `C:/worktree/open-design/apps/daemon/src/skills.ts` (+ `plugins/local-skill.ts`)      |
| skill manifest schema       | `C:/worktree/open-design/apps/daemon/tests/fixtures/plugin-fixtures/sample-plugin/SKILL.md` |
| 100+ canonical SKILL.md     | `C:/worktree/open-design/skills/*/SKILL.md`                                           |
| openclaw .agents/skills/    | `C:/worktree/openclaw/.agents/skills/{discord-clawd,parallels-discord-roundtrip}/`    |

**Ship plan:** align aphrody's `.claude/skills/*/SKILL.md` frontmatter
to the canonical `name/description/triggers/od` schema from
open-design's daemon loader so the same skills can run under either
runtime without rewrite.

## 3. Memory consolidation

| Target ask           | Where in openclaw                                          |
|---|---|
| Active memory orchestrator | `C:/worktree/openclaw/extensions/active-memory/`          |
| Memory core abstraction    | `C:/worktree/openclaw/extensions/memory-core/src/memory/` |
| Memory-host SDK           | `C:/worktree/openclaw/packages/memory-host-sdk/`           |
| LanceDB vector backend    | `C:/worktree/openclaw/extensions/memory-lancedb/`         |
| Wiki-flavoured backend    | `C:/worktree/openclaw/extensions/memory-wiki/`            |
| QA scenarios              | `C:/worktree/openclaw/qa/scenarios/memory/`               |

**Ship plan:** wire a `crates/aphrody-memory/` crate that owns the
trait surface mirroring `memory-host-sdk`, with two backends:
LanceDB (matches openclaw) and the file-based `.coord/*.jsonl` mailbox
(matches our existing A2A protocol). The crate becomes a Rust-side
substitute for the openclaw memory pipeline.

## 4. Gemini CLI integration

| Target ask           | Where in open-design                                            |
|---|---|
| Runtime adapter      | `C:/worktree/open-design/apps/daemon/src/runtimes/defs/gemini.ts`|
| Auto-detection       | same file — `bin: 'gemini'`, `versionArgs: ['--version']`        |
| Workspace trust env  | `GEMINI_CLI_TRUST_WORKSPACE: 'true'` (avoids `--skip-trust` hidden flag) |
| Stream-json output   | `buildArgs() -> ['--output-format', 'stream-json', '--yolo']`    |
| Fallback model list  | `gemini-3-pro-preview`, `gemini-3-flash-preview`, `gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.5-flash-lite` |

**Ship plan:** port `gemini.ts` to a `crates/gemini-runtime/` adapter
exposing the same surface (`bin`, `versionArgs`, `env`, `buildArgs`)
for use by aphrody's CLI when the user asks for a Gemini-backed turn.

## 5. Antigravity native OAuth provider

| Target ask                   | Where                                                          |
|---|---|
| MCP HTTP/SSE OAuth 2.1 client | `C:/worktree/open-design/apps/daemon/src/mcp-oauth.ts`        |
| Token persistence            | `C:/worktree/open-design/apps/daemon/src/mcp-tokens.ts` (implied) |
| RFC coverage                 | RFC 9728 (protected-resource discovery), RFC 8414 (auth-server discovery), RFC 7591 (DCR), RFC 7636 (PKCE) |
| Test suite                   | `C:/worktree/open-design/apps/daemon/tests/mcp-oauth.test.ts`  |

**Note:** "Antigravity" appears to be the user's name for the OAuth
flow class — the file is named `mcp-oauth.ts` upstream. The
implementation is RFC-clean (no `mcp-remote` subprocess listener
hack), suitable for daemon use.

**Ship plan:** port the OAuth state machine to `crates/aphrody-mcp/`
(Rust) using `reqwest` + `oauth2` crate. Cache DCR registrations in
`var/data/mcp-oauth-clients.json` matching the upstream format byte-
for-byte so tokens survive runtime switches.

## 6. Voice-talk Discord provider

| Target ask                  | Where in openclaw                                  |
|---|---|
| Discord extension           | `C:/worktree/openclaw/extensions/discord/`         |
| Discord voice               | `C:/worktree/openclaw/extensions/discord/src/voice/` |
| Talk-voice extension        | `C:/worktree/openclaw/extensions/talk-voice/`      |
| Voice-call extension        | `C:/worktree/openclaw/extensions/voice-call/`      |
| ElevenLabs voice provider   | `C:/worktree/open-design/apps/daemon/src/elevenlabs-voices.ts` |

**Ship plan:** maintain openclaw as the canonical Discord+voice
provider (it already supports 22 channels). Aphrody integrates via the
openclaw Gateway HTTP API. No need to port the Discord client; build
a thin `crates/aphrody-voice/` shim that POSTs to the openclaw
gateway.

## 7. AI gateway server

| Target ask                    | Where in openclaw                                        |
|---|---|
| Cloudflare AI Gateway adapter | `C:/worktree/openclaw/extensions/cloudflare-ai-gateway/` |
| Vercel AI Gateway adapter     | `C:/worktree/openclaw/extensions/vercel-ai-gateway/`     |
| Internal gateway (ios/android)| `C:/worktree/openclaw/apps/{ios,android}/.../Gateway/`   |
| QQ-bot engine gateway         | `C:/worktree/openclaw/extensions/qqbot/src/engine/gateway/` |

**Ship plan:** ship `crates/aphrody-gateway/` exposing a
provider-agnostic surface (OpenAI-compatible BYOK), with adapters
matching Cloudflare AI Gateway + Vercel AI Gateway envelope formats.

## 8. AI minimal env

| Target ask              | Where                                                                  |
|---|---|
| BYOK proxy fallback     | open-design README: "No CLI? An OpenAI-compatible BYOK proxy is the same loop minus the spawn." |
| 16 auto-detected CLIs   | open-design daemon — Claude Code, Codex, Devin, Cursor Agent, Gemini CLI, OpenCode, Qwen, Qoder, Copilot CLI, Hermes, Kimi, Pi, Kiro, Kilo, Mistral Vibe, DeepSeek TUI |

**Ship plan:** aphrody's `aphrody doctor` subcommand already detects
toolchains. Add an `aphrody runtimes` subcommand mirroring
open-design's auto-detect logic so contributors see what's available
on their machine.

## 9. MCP open-design core

| Target ask                 | Where in open-design                                    |
|---|---|
| MCP daemon plugin loader   | `C:/worktree/open-design/apps/daemon/src/plugins/`     |
| MCP test fixtures          | `C:/worktree/open-design/apps/daemon/tests/plugin-*.test.ts` |
| MCP route layer            | (covered by mcp-oauth + plugin loader)                  |

**Ship plan:** mirror the open-design MCP plugin contract surface in
our `crates/google_mcp/` so any open-design plugin can drop into
aphrody. Re-use the existing aphrody MCP server scaffolding.

## 10. agui package

| Target ask           | Where in open-design                                         |
|---|---|
| agui-adapter package | `C:/worktree/open-design/packages/agui-adapter/`             |
| agui route tests     | `C:/worktree/open-design/apps/daemon/tests/agui-route.test.ts` |

**Ship plan:** add `packages/agui-adapter` to aphrody as a path dep
(no fork yet — let open-design own the upstream). Aphrody CLI gets a
thin Rust binding via `crates/agui-bridge/` that exposes the same
WASM-side `<a-gui-*>` custom elements.

## 11. Bonus harvested (best resources)

| Asset                         | Path                                                                       |
|---|---|
| 152 production DESIGN.md      | `C:/worktree/open-design/design-systems/`                                  |
| 100+ design SKILL.md          | `C:/worktree/open-design/skills/`                                          |
| 30+ design templates          | `C:/worktree/open-design/design-templates/`                                |
| Apple HIG skill               | `C:/worktree/open-design/skills/apple-hig/`                                |
| Brand-guidelines skill        | `C:/worktree/open-design/skills/brand-guidelines/`                         |
| Color-expert skill            | `C:/worktree/open-design/skills/color-expert/`                             |
| Competitive-ads-extractor     | `C:/worktree/open-design/skills/competitive-ads-extractor/`                |
| Critique skill                | `C:/worktree/open-design/skills/critique/`                                 |
| Creative-director skill       | `C:/worktree/open-design/skills/creative-director/`                        |
| Doc skill                     | `C:/worktree/open-design/skills/doc/`                                      |
| Design-brief skill            | `C:/worktree/open-design/skills/design-brief/`                             |
| Design-consultation skill     | `C:/worktree/open-design/skills/design-consultation/`                      |
| 30+ openclaw extensions       | `C:/worktree/openclaw/extensions/`                                         |
| MCP-oauth (RFC-clean)         | `C:/worktree/open-design/apps/daemon/src/mcp-oauth.ts`                     |
| ElevenLabs voice provider     | `C:/worktree/open-design/apps/daemon/src/elevenlabs-voices.ts`             |
| LanceDB vector memory backend | `C:/worktree/openclaw/extensions/memory-lancedb/`                          |
| Cloudflare AI Gateway adapter | `C:/worktree/openclaw/extensions/cloudflare-ai-gateway/`                   |
| Vercel AI Gateway adapter     | `C:/worktree/openclaw/extensions/vercel-ai-gateway/`                       |

## 12. Roadmap (suggested integration order)

1. **DONE** — adopt DESIGN.md spec; aphrody/DESIGN.md ships at repo root.
2. **DONE** — design-google-ingest skill + curator agent generate `docs/DESIGN-GOOGLE.md`.
3. **Next-tick** — align `.claude/skills/*/SKILL.md` frontmatter to
   the open-design daemon schema (`name/description/triggers/od`).
4. **Next-tick** — port `apps/daemon/src/mcp-oauth.ts` to
   `crates/aphrody-mcp/` (Rust + `oauth2` + `reqwest`).
5. **Next-tick** — port `apps/daemon/src/runtimes/defs/gemini.ts` to
   `crates/gemini-runtime/`.
6. **Tick 38+** — scaffold `crates/aphrody-memory/` with the
   memory-host-sdk trait + LanceDB + file-JSONL backends.
7. **Tick 39+** — wire `crates/aphrody-gateway/` with Cloudflare +
   Vercel AI Gateway adapters.
8. **Tick 40+** — integrate the openclaw Discord+voice gateway via
   thin Rust shim.

## 13. Licences

- **open-design (nexu-io)**: check repository — multiple licences,
  some skills upstream'd from Google Labs.
- **openclaw**: MIT (per `C:/worktree/openclaw/LICENSE`).
- **design.md (google-labs-code)**: Apache-2.0.
- **angular/components**: MIT.

No licence conflicts for harvest-only inspection. Any code port must
preserve attribution + propagate the source licence into aphrody
where applicable (most are MIT or Apache-2.0, compatible with
aphrody's Apache-2.0 baseline).

---

_Audit generator: manual at 2026-05-17. Refresh via
`bash` re-clone + re-run this audit when upstream restructures._
