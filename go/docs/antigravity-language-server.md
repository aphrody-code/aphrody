<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->

# The Go `language_server.exe` (Antigravity / Codeium engine)

> RE intel. No secrets: only public RPC/field names, launch flags, and the
> public self-signed-cert fingerprint. Personal paths anonymised to `<user>`.

## 1. What it is

Google's **Antigravity** desktop IDE is Codeium / Windsurf's agentic IDE
re-skinned for Google. Its compute engine is **not** a light Gemini client — it
is Codeium's **Go** `language_server.exe` (~136 MB), wrapped in a thin Electron
shell (`app.asar` v2.0.1, author = Google) plus a VSCode-fork workbench. The Go
binary owns all cloud auth and all upstream traffic; the Electron shell merely
spawns and supervises it.

Evidence it is Go:

- Embedded symbol namespace `codeium_common_go_proto` and the `exa.*_pb`
  protobuf package suffix (`docs/research/antigravity-sdk-analysis.md` §0.0;
  `var/data/antigravity-extract/REPORT.md` §TL;DR — local, gitignored).
- Publicly, Codeium/Windsurf's language server is a Go binary speaking gRPC +
  protobuf — Exafunction/codeium issues
  <https://github.com/Exafunction/codeium/issues/285>,
  <https://github.com/Exafunction/codeium/issues/286>; Windsurf plugin docs
  <https://docs.windsurf.com/plugins/getting-started>.
- The `_pb` package convention and `protoc-gen-go-grpc`-style service stubs are
  the standard Go gRPC toolchain (`google.golang.org/grpc`,
  `google.golang.org/protobuf`) — <https://grpc.io/docs/languages/go/quickstart/>,
  <https://pkg.go.dev/google.golang.org/grpc/cmd/protoc-gen-go-grpc>.

## 2. Transport & trust model

The workbench (or aphrody) talks to the Go server over **gRPC on self-signed
HTTPS bound to `127.0.0.1`**, on an **OS-assigned port** chosen at launch.

- **Port discovery**: the server prints its bound port to stdout/log, matched by
  the regex `listening on \w+ port at (\d+) for HTTP(S)?` (case-insensitive).
  Source: shipped `languageServer.js` `PORT_PATTERN`
  (`var/data/antigravity-extract/src/app/dist/languageServer.js:106-107`),
  reproduced in [`crates/antigravity-sdk/src/local_ls.rs`](../../crates/antigravity-sdk/src/local_ls.rs)
  (`read_port`, regex at the `DEFAULT_PORT_TIMEOUT` site).
- **Certificate**: self-signed, so WebPKI chain validation cannot apply. The
  client pins the certificate's `SubjectPublicKeyInfo` SHA-256 (Chromium HPKP
  `sha256/<base64>` form). The public pin recovered from Antigravity 2.0.1
  `constants.js` is the constant `PINNED_SPKI_SHA256_B64` in
  [`local_ls.rs`](../../crates/antigravity-sdk/src/local_ls.rs) — a public
  fingerprint, **not** a secret.
- **CSRF**: a per-launch CSRF token is supplied via `--csrf_token` and echoed on
  every gRPC call in the `x-csrf-token` header.

## 3. Launch contract

Recovered verbatim from the shipped launcher
`var/data/antigravity-extract/src/app/dist/languageServer.js` (function
`startLanguageServer`, lines ~187-210; the flag set was confirmed by listing all
`--*` tokens in that file). aphrody reproduces it exactly in
[`local_ls.rs`](../../crates/antigravity-sdk/src/local_ls.rs) (`LanguageServer::launch`).

```text
language_server --standalone
  --override_ide_name <name>
  --subclient_type hub
  --override_ide_version <version>
  --override_user_agent_name <name>
  --https_server_port <port|0>      # 0 = OS picks a free port
  --csrf_token <csrf>
  --app_data_dir <dir>
  --api_server_url https://generativelanguage.googleapis.com
  --cloud_code_endpoint https://daily-cloudcode-pa.googleapis.com
  --enable_sidecars
  [--headless]
```

Full set of flags observed in `languageServer.js`: `--standalone`,
`--subclient_type`, `--override_ide_name`, `--override_ide_version`,
`--override_user_agent_name`, `--https_server_port`, `--csrf_token`,
`--app_data_dir`, `--api_server_url`, `--cloud_code_endpoint`,
`--enable_sidecars`, `--headless`, and `--stamp` (build-CL probe; the launcher
runs `execFile(LS_BINARY, ['--stamp'], ...)`).

Defaults baked into the Rust bridge
([`local_ls.rs`](../../crates/antigravity-sdk/src/local_ls.rs)):

| Flag | Default constant | Value |
|---|---|---|
| `--api_server_url` | `DEFAULT_API_SERVER_URL` | `https://generativelanguage.googleapis.com` |
| `--cloud_code_endpoint` | `DEFAULT_CLOUD_CODE_ENDPOINT` | `https://daily-cloudcode-pa.googleapis.com` (the shell's runtime default = the `daily` ring) |
| `--https_server_port` | `LaunchConfig::https_server_port` | `0` (OS-chosen) |
| `--subclient_type` | hard-coded | `hub` |

### Binary resolution order

`LanguageServer::resolve_binary` ([`local_ls.rs`](../../crates/antigravity-sdk/src/local_ls.rs)):

1. `$ANTIGRAVITY_HARNESS_PATH` (file, or directory + known binary name
   `language_server[.exe]` / `localharness[.exe]`).
2. `$CODEIUM_LANGUAGE_SERVER_BIN` (legacy Codeium).
3. `resources/bin/language_server[.exe]` next to the current executable.
4. `language_server[.exe]` on `PATH` (resolved by the OS at spawn time).

aphrody never embeds the binary; it must already be present on the host.

## 4. gRPC service surface — `exa.language_server_pb`

The full RPC name set was recovered by RE of the shipped `languageServer.js`
launcher and the Go binary's embedded symbols (`codeium_common_go_proto`).
Recovered RPCs on `exa.language_server_pb.LanguageServerService`
(`var/data/antigravity-extract/REPORT.md` §3; proto header comment in
[`crates/antigravity-sdk/proto/exa_language_server.proto`](../../crates/antigravity-sdk/proto/exa_language_server.proto)):

- `FetchUserInfo`
- `GetAuthStatus`
- `GetLocalUserInfo`
- `GetGrantedScopes`
- `GetAvailableModels`
- `GetModelResponse`
- `AcceptTermsOfService`
- `AuthLogout`
- plus Cascade / MCP / Worktree / Plugin agent RPCs (the full agentic surface).

Sibling services seen in the binary: `exa.api_server_pb.ApiServerService`,
`exa.chat_client_server_pb`, `exa.analytics_pb`.

### What aphrody actually implements

The Rust `.proto` is a **deliberately small, hand-written, INFERRED subset** —
enough to compile a usable, extensible tonic client. It is **not** a faithful
descriptor dump (a full `google.protobuf.FileDescriptorSet` extraction via
GoReSym is documented as optional / not-done; see the proto header comment and
`var/data/antigravity-extract/REPORT.md` §Completion → "NON_FAIT: deep Go-binary
protobuf descriptor extraction").

Declared in [`exa_language_server.proto`](../../crates/antigravity-sdk/proto/exa_language_server.proto):

| RPC | Request | Response | Purpose |
|---|---|---|---|
| `FetchUserInfo` | `FetchUserInfoRequest{ Metadata, bool force_refresh }` | `FetchUserInfoResponse{ UserInfo }` | Signed-in cloud profile (email, name, tier, subject, email_verified) |
| `GetAuthStatus` | `GetAuthStatusRequest{ Metadata }` | `GetAuthStatusResponse{ AuthState state, string message, int64 expiry_unix_seconds }` | Whether a valid session is held |
| `GetAvailableModels` | `GetAvailableModelsRequest{ Metadata }` | `GetAvailableModelsResponse{ repeated Model, string default_model }` | Models for the user's account/tier |

Shared message `Metadata` carries IDE identity / session correlation:
`ide_name`, `ide_version`, `extension_name`, `extension_version`, `session_id`,
`request_id`, `locale` (all optional in practice). `enum AuthState` =
`{ UNSPECIFIED=0, UNAUTHENTICATED=1, AUTHENTICATED=2, EXPIRED=3 }`. `Model` =
`{ name, display_name, provider, bool available, int64 max_context_tokens }`.

The Rust `.proto` is intentionally extensible: add RPCs/messages as they are
reverse-engineered.

## 5. The Rust bridge (`local_ls.rs`)

[`crates/antigravity-sdk/src/local_ls.rs`](../../crates/antigravity-sdk/src/local_ls.rs)
reproduces the launch contract and speaks gRPC to the Go server directly,
without the proprietary Electron shell.

- **Feature-gated, host-only**: behind the non-default `local-ls` cargo feature
  ([`crates/antigravity-sdk/Cargo.toml`](../../crates/antigravity-sdk/Cargo.toml),
  feature `local-ls` pulls `dep:prost`, `dep:tonic`, `dep:tonic-prost`,
  `dep:tonic-prost-build`). Never compiled in the default build, CI, or `wasm32`.
- **Codegen**: `build.rs` compiles the `.proto` into `OUT_DIR`; the module
  includes it via `tonic::include_proto!("exa.language_server_pb")`.
- **Pinned TLS**: `SpkiPinVerifier` is a custom
  `rustls::client::danger::ServerCertVerifier`. It parses the end-entity X.509
  (`x509-parser`), SHA-256-hashes the `SubjectPublicKeyInfo`, and compares it to
  the decoded `PINNED_SPKI_SHA256_B64`; all other chain checks are bypassed
  because the endpoint is a local self-signed `127.0.0.1` listener. TLS 1.2/1.3
  signature verification is delegated to the active rustls `CryptoProvider`.
- **CSRF interceptor**: `CsrfInterceptor` injects the `x-csrf-token` header on
  every request via a tonic `Interceptor`.
- **Channel**: tonic `Channel` over `hyper-rustls` HTTPS connector (HTTP/2,
  `https_only`) to `https://127.0.0.1:{port}`.
- **Lifecycle**: `Child` spawned with `kill_on_drop(true)`; `stdout` is read
  line-by-line until the port regex matches (`DEFAULT_PORT_TIMEOUT` = 20 s),
  then the pinned channel is built. `shutdown()` awaits an explicit kill.
- **Typed calls**: `fetch_user_info`, `get_auth_status`, `get_available_models`,
  plus `stamp()` (runs `--stamp` to print the build CL).

### Two interop paths (aphrody implements #1, bridge enables #2)

1. **Direct cloud (default, recommended)**: Google OAuth2 token →
   `POST cloudcode-pa.googleapis.com/v1internal:loadCodeAssist` (and
   `:fetchAvailableModels`, `:onboardUser`) with `Authorization: Bearer …`.
   Needs no proprietary binary; this is the crate's primary surface
   (`AntigravityClient`). See
   [`docs/research/antigravity-sdk-analysis.md`](../research/antigravity-sdk-analysis.md) §0.0.
2. **Local gRPC LS (this module)**: spawn the Go `language_server.exe`, discover
   its port, pin its cert, send CSRF, call `exa.language_server_pb` — full agent
   surface (Cascade / MCP / worktrees) but requires the proprietary binary.

Auth note (anti-hallucination): the live OAuth client_ids, scopes, and token
live in the **desktop app** (Windows Credential Manager target
`gemini:antigravity`), **not** in the Go binary's wire surface and **not** in
the `google-antigravity` Python SDK. No token/cookie values are reproduced here.
See [`docs/research/antigravity-sdk-analysis.md`](../research/antigravity-sdk-analysis.md) §2.

## Sources

- In-repo: [`crates/antigravity-sdk/proto/exa_language_server.proto`](../../crates/antigravity-sdk/proto/exa_language_server.proto),
  [`crates/antigravity-sdk/src/local_ls.rs`](../../crates/antigravity-sdk/src/local_ls.rs),
  [`crates/antigravity-sdk/Cargo.toml`](../../crates/antigravity-sdk/Cargo.toml),
  [`docs/research/antigravity-sdk-analysis.md`](../research/antigravity-sdk-analysis.md),
  [`docs/research/vscode-fork-re-intel.md`](../research/vscode-fork-re-intel.md),
  [`docs/research/electron-re-intel.md`](../research/electron-re-intel.md).
  Local (gitignored): `var/data/antigravity-extract/REPORT.md`,
  `var/data/antigravity-extract/src/app/dist/languageServer.js`.
- External: <https://github.com/Exafunction/codeium/issues/285>,
  <https://github.com/Exafunction/codeium/issues/286>,
  <https://docs.windsurf.com/plugins/getting-started>,
  <https://grpc.io/docs/languages/go/quickstart/>,
  <https://pkg.go.dev/google.golang.org/grpc/cmd/protoc-gen-go-grpc>,
  <https://github.com/grpc/grpc-go>.
