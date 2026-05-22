<!-- SPDX-License-Identifier: Apache-2.0 -->
# antigravity-langserver-re

**A reverse-engineering (RE) reconstruction — NOT official Google source.**

Structural Go reconstruction of the sidecar shipped with the **Antigravity IDE**:

```
…\Antigravity IDE\resources\app\extensions\antigravity\bin\language_server_windows_x64.exe
```

- 133 MB, PE64, Go binary.
- Toolchain (internal Google / blaze, not go modules):
  `go1.27-20260427-RC04 cl/906595525 +boringcrypto,simd`.
- `redress info` → main root **`third_party/jetski/cmd/language_server`**,
  1 main / 218 std / 1300 vendor packages.

Antigravity is a Google fork of **Windsurf / Codeium**: codename **Jetski**
(`google3/third_party/jetski/`), agent runtime **cortex** (Go), execution engine
**Cascade**, proto namespace `exa.*`.

## Tools used (installed for this RE) + versions

| Tool | Version | Role | Result on this binary |
|------|---------|------|------------------------|
| `goretk/redress` | **v1.2.67** (gore v0.13.28) | pclntab parse, package tree, per-file/per-type method recovery (`source` projection) | **WORKED** — drove the whole reconstruction |
| `mandiant/GoReSym` | **v1.7.1** | pclntab/types/paths JSON | **FAILED** — "failed to locate pclntab" (its magic table does not recognise the unreleased internal go1.27 build) |
| Sysinternals `strings` | **v2.54** | host / scope / model / clientID constants | WORKED |
| `go` toolchain | go1.26.3 windows/amd64 | build + vet of this module | — |

`redress types` also FAILED (moduledata typelink layout of this internal go1.27
differs from what gore knows → overflow). So **field _types_ are inferred**;
field _names_ are real (recovered from proto `Get<Field>` accessor methods).

## What is FAITHFUL vs INFERRED

**Faithful (recovered verbatim from symbols):**
- Package tree under `third_party/jetski/` (209 packages — see
  `var/data/antigravity-ide-re/redress/jetski-packages.txt`).
- Service names + RPC method names:
  - `LanguageServerService` — **237 methods** (`pkg/langserver/methods.go`),
  - v1internal `JetskiService` — **14 methods** (`pkg/cloudcode/jetski_service.go`),
  - v1internal `PredictionService` — **5 methods** (`pkg/cloudcode/prediction_service.go`).
- Proto message **type names** and **field names** (from `Get<Field>` methods).
- Auth providers (IDE / Standalone / CLI / AntigravityHub) + `AuthClient` method set.
- `CascadeManager` method set (the agent run loop) + `EndBattleModeError`.
- Agent tool set (26 `*ToolConverter` symbols → `pkg/cortex/tools.go`).
- Hosts, OAuth scopes, OAuth client IDs (byte-for-byte), model IDs.

**Inferred (RE could not recover; reconstructed plausibly):**
- Field _types_, field ordering, proto field numbers.
- Method parameter/return _types_ (only names live in pclntab for this build) —
  service interfaces use reconstructed request/response structs.
- All function bodies (impossible from a stripped Go binary).

Every reconstructed declaration carries a `// reconstructed from: <symbol>` comment.

## Package layout

| Package | Mirrors | Content |
|---------|---------|---------|
| `pkg/cloudcode` | `google3/google/internal/cloud/code/v1internal/*` | v1internal `JetskiService` + `PredictionService`, message types, hosts, the 37 `v1internal:*` methods |
| `pkg/langserver` | `third_party/jetski/language_server_pb/*` | local `LanguageServerService` (237 RPCs, Connect transport), typed core subset |
| `pkg/cortex` | `third_party/jetski/cortex/*` | agent runtime: trajectory model, agent-state, 26 agent tools |
| `pkg/cascade` | `cortex/cortex/cascade_manager.go` | `CascadeManager` agentic run loop |
| `pkg/auth` | `language_server/{auth_client,code_assist_client}` | OAuth client + 4 auth providers, scopes, client IDs |
| `pkg/jetski` | `third_party/jetski/` (umbrella) | codenames, model IDs, Unleash hosts |

## Verify

```sh
cd go/antigravity-langserver-re
go build ./...   # exit 0
go vet ./...     # exit 0
```

## Raw artefacts

`C:\src\aphrody\var\data\antigravity-ide-re\`:
`redress/source-all.txt` (131,910-line source projection),
`redress/packages-full.txt`, `redress/jetski-packages.txt`,
`ls-strings.txt`, `ls-service-methods.txt`, `models.txt`,
`v1internal-methods.txt`, `goresym/goresym.json` (the failure record).
