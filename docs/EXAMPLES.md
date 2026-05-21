<!-- SPDX-License-Identifier: Apache-2.0 -->
# Examples

Recipe collection for `aphrody`. Each section is a copy-paste shell snippet
plus the expected output, so you can verify the recipe end-to-end before
adapting it to your project. For background reading see
[`INSTALL.md`](./INSTALL.md), [`PROTOCOL.md`](./PROTOCOL.md), and
[`PERFORMANCE.md`](./PERFORMANCE.md). Every command was cross-checked against
the real clap surface in `crates/cli/src/main.rs`.

---

## 1. First-time install + verify

POSIX one-liner (Linux + macOS):

```bash
curl -sSf https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.sh | sh
aphrody --version
# aphrody 1.0.0-canary
# commit:    <git-sha>
# built:     <epoch> (unix epoch)
# target:    x86_64-unknown-linux-gnu
# profile:   release

aphrody doctor
# aphrody doctor — environment + integration diagnostics
#
# [runtime]
#   binary version: 1.0.0-canary (commit <sha>, built <date>, target ...)
#   rustls CryptoProvider: installed (ring)
#   reqwest TLS backend: rustls
#   mimalloc allocator: active (native)
#   tokio runtime: multi-thread, full
# ...
```

The `doctor` command exits non-zero on `UNHEALTHY`, so it doubles as a
CI gate (see recipe 11).

---

## 2. Diagnostic JSON output (for scripts)

```bash
aphrody doctor --json | jq '.verdict'
# "HEALTHY"
```

The full JSON shape is documented in
[`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md). Pipe it into your monitoring
stack: every probe has a stable `status` field (`OK` / `DEGRADED` / `MISSING`).

---

## 3. Generate shell completions

```bash
aphrody completions bash > ~/.local/share/bash-completion/completions/aphrody
aphrody completions zsh  > ~/.zsh/completions/_aphrody
aphrody completions fish > ~/.config/fish/completions/aphrody.fish
```

PowerShell + Elvish are also supported via the `clap_complete::Shell` enum:

```powershell
aphrody completions power-shell > $PROFILE.CurrentUserAllHosts
```

Restart your shell, then `aphrody <TAB>` lists every subcommand.

---

## 4. Cross-Claude A2A coordination

Peer agents coordinate over the typed **gRPC A2A transport** (crates `a2a-pb`
/ `a2a` / `a2a-client` / `a2a-server`). The former file-based mailbox
(`ai.json` manifest + `.coord/*.jsonl`) was removed; the only remaining
file mirror is the winclean compatibility inbox
`C:\winclean\.coord\inbox-from-aphrody.jsonl`. See the `a2a-*` crate docs
for the envelope schema and handshake.

---

## 5. Web fetch / recon via the MCP server

Forensic web fetch and recon are exposed by the native `aphrody-mcp` server
(`universal_web_fetch`, `advanced_recon`, `dns_recon`) — there is no longer a
separate BXC scraping engine. Call them through Claude Code / any MCP client.

---

## 6. DNS OSINT reconnaissance

```bash
aphrody dns example.com
# A / AAAA / MX / TXT / NS records via system resolver
# crt.sh subdomain enumeration
# hackertarget passive DNS
# Aggregated JSON written to stdout
```

Pipe through `jq` to extract a specific record set:

```bash
aphrody dns example.com | jq '.records.A'
```

---

## 7. Run a shell command via `aphrody auto`

`auto` is a clap `external_subcommand` — anything after it is forwarded to
the platform shell (`sh -c` on Unix, `cmd /C` on Windows):

```bash
aphrody auto cargo build --release
aphrody auto cargo nextest run --workspace
aphrody auto uv pip install pandas
```

Useful when you want `aphrody`'s logging, mimalloc allocator, and rustls
provider already wired before invoking a downstream tool.

---

## 8. mrx monorepo scan

`mrx` is the in-repo monorepo mapper. It writes two artifacts to the cwd
unless overridden via `--out` / `--map`:

```bash
mrx scan
ls -la path.json monorepo-map.json
# Both auto-written to cwd (see CLAUDE.md §7 — gitignored at repo root).

# Scan a different tree
mrx --root /path/to/another/monorepo scan

# CI gate — exits 1 on findings
mrx check
```

Long-running daemon mode debounces FS events (default 1500 ms):

```bash
mrx watch --debounce-ms 500
```

---

## 9. WASM library in a browser

The `aphrody-wasm` crate exposes the base-crate cryptography surface via
`wasm-bindgen`. Build with `wasm-pack build --target web`, then:

```javascript
import init, { version, platform_short_name, decrypt_aes_gcm } from './aphrody_wasm.js';

await init();
console.log(version());              // "1.0.0-canary"
console.log(platform_short_name());  // "wasm32-unknown-unknown"

const plaintext = decrypt_aes_gcm(ciphertext, key); // Uint8Array
```

See `crates/aphrody-wasm/examples/browser-playground.html` for a runnable
demo and [`WASM/`](./WASM) for the build matrix.

---

## 10. Embedded as a library (Rust)

Add the `base` crate to your workspace and call the cryptography primitives
directly — same path used by both the native CLI and the WASM bridge:

```rust
use base::Crypto;

let plaintext = Crypto::decrypt_aes_gcm(&ciphertext, &key)
    .map_err(|e| anyhow::anyhow!("decrypt failed: {e}"))?;
```

`base` is `no_std`-compatible and pulls zero platform-specific code — safe
to embed in WASM, embedded Linux, or a hot-path Tokio task.

---

## 11. CI integration

Drop-in GitHub Actions step that installs the binary, runs diagnostics,
and fails the build on a non-`HEALTHY` verdict:

```yaml
jobs:
  diagnostic:
    runs-on: ubuntu-latest
    steps:
      - name: Install aphrody
        run: |
          curl -sSf https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.sh | sh
          echo "$HOME/.local/bin" >> "$GITHUB_PATH"
      - name: Run doctor
        run: aphrody doctor --json | tee diag.json
      - name: Verify verdict
        run: jq -re '.verdict == "HEALTHY"' diag.json
```

Mirror on Windows runners via the PowerShell installer
(`packaging/install.ps1`), documented in
[`INSTALL.md`](./INSTALL.md) §2.

---

## 12. Python SDK: WebView2 History Analysis with Magika & LangExtract

Verify and extract structured timeline data from the WebView2 default profile directory using `magika` to classify file formats and `langextract` to format search queries and visited websites:

```bash
# Verify environment and run the WebView2 deep dive example script
uv run libs/antigravity-sdk-python/examples/deep_dives/webview2_magika_langextract.py
# Reading 'gemini:antigravity' from Windows Credential Manager...
# Successfully loaded Gemini access token for Vertex AI.
# Found WebView2 user profile: C:\Users\<user>\AppData\Local\Google\Google\latest\default\WebView2\EBWebView\Default
#
# Classifying WebView2 profile files using Magika:
#   - File: History      -> Magika: sqlite (MIME: application/x-sqlite3)
#   - File: Preferences  -> Magika: json (MIME: application/json)
#   - File: Web Data     -> Magika: sqlite (MIME: application/x-sqlite3)
#
# Reading recent Visited URLs from History database...
# Retrieved 15 history items.
#
# Running LangExtract structure synthesis using Gemini model...
# Extracted 15 grounded entities:
#   1. [SearchQuery] text='google chrome download'
#      Attributes: {'query': 'google chrome download', 'engine': 'Google Search', 'topic': 'browser download'}
#   ...
# Saved structured JSONL output to: webview2_analysis_results.jsonl
# Saved interactive HTML visualization to: webview2_visualization.html
```

It queries the `History` SQLite database (via a safe temporary copy to bypass file locks) and structures URLs/titles using the `gemini-2.5-flash` model.

---

## 13. Native Google AI Ultra / Gemini client (`aphrody antigravity`)

`aphrody antigravity` is the scriptable, non-interactive port of the Antigravity
(`agy`) cloud surface. It reads the user's Google OAuth token **at runtime** from
the platform credential store (Windows Credential Manager entry
`gemini:antigravity`) — no secret is ever embedded in the binary. Native-only
(builds a `reqwest`/rustls client + reads the credential store; not on wasm32).
Every variant prints JSON on stdout, so it pipes straight into `jq`:

```bash
# Who am I? (Google OpenID userinfo: email + name)
aphrody antigravity whoami --json | jq '.email'
# "yohan@example.com"

# Models available to the signed-in account / tier
# (v1internal:fetchAvailableModels)
aphrody antigravity models --json | jq '.models[].name'

# Bootstrap the Code Assist session — project / tier / entitlements
# (v1internal:loadCodeAssist)
aphrody antigravity load --json | jq '.cloudaicompanionProject, .currentTier.id'

# Single-turn Gemini prompt (generativelanguage v1beta generateContent).
# --model defaults to gemini-2.0-flash; output is always pretty JSON.
aphrody antigravity chat --prompt "Explain io_uring in one sentence."
aphrody antigravity chat --model gemini-2.5-flash --prompt "Refactor this loop" \
  | jq -r '.candidates[0].content.parts[0].text'
```

On a platform without a credential store (Linux/macOS/wasm) the underlying SDK
returns `SdkError::Unsupported`, surfaced verbatim as a `miette` report — supply
your own `OAuthToken` via the `antigravity-sdk` crate instead (recipe 10 pattern).

---

## 14. Magika file classification (`aphrody re classify`)

Native, in-process Google Magika (deep-learning content classifier) — the
replacement for the Python `magika` shell-out. Requires building with
`--features magika` (links the ONNX Runtime; host-only, not hermetic-offline,
not wasm). Without the feature the command errors with a rebuild hint rather
than silently degrading:

```bash
cargo build -p aphrody --features magika --release

aphrody re classify state.vscdb --pretty | jq '.label, .score'
# "sqlite"
# 0.99996...

# Output shape: { label, mime_type, group, description, extensions,
#                 is_text, score, kind, overwrite_reason }
aphrody re classify ./app.asar | jq -r '"\(.label) (\(.mime_type))"'
# unknown (application/octet-stream)   # asar has no Magika signature → triage instead
```

Pair it with the `re` reverse-engineering family (`triage`, `strings`,
`sections`, `disasm`, `google`) for full binary analysis of Google/Electron
artifacts.

---

## 15. Reproducible forensic extraction (`aphrody forensics`)

`aphrody forensics` maps and inspects local artifacts (Chromium / Electron /
`state.vscdb`) **without ever emitting secret values**. Requires
`--features forensics` (links `rusqlite` with bundled libsqlite3; host-only).
Two contracts, both secret-safe by construction:

```bash
cargo build -p aphrody --features forensics --release

# `map` — parallel directory walk, emits { path, size, ext } per file.
# Reads directory metadata + filenames ONLY; file CONTENTS are never opened.
aphrody forensics map --target ~/.gemini --out var/data/gemini-map
jq '.file_count, .files[0]' var/data/gemini-map/map.json

# `sqlite` — schema dump, opened SQLITE_OPEN_READ_ONLY. Reads
# `SELECT name, sql FROM sqlite_master` (table/index NAMES + CREATE statements
# only). Value columns of ItemTable / cookies / secret-style tables are NEVER
# selected, so no secret bytes leak. JSON on stdout.
aphrody forensics sqlite --db state.vscdb | jq '.tables[].name'
```

This is the reproducible replacement for ad-hoc forensic one-liners: the
security contract (no content reads on `map`, no value columns on `sqlite`) is
enforced in code, not by convention.

---

## 16. Where to find more

- [`PROTOCOL.md`](./PROTOCOL.md) — historical file-based A2A spec (the live
  transport is gRPC; see the `a2a-*` crates).
- [`PERFORMANCE.md`](./PERFORMANCE.md) — bench recipes, headline claims,
  reproduction matrix.
- [`MIGRATION.md`](./MIGRATION.md) — switching from competing tools
  (gemini-cli, claude-cli, gh, mrx legacy).
- [`TROUBLESHOOTING.md`](./TROUBLESHOOTING.md) — known pitfalls (rustls
  CryptoProvider, `--icf=all`, GTK3 CVEs, etc.).
- [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md) — consolidated architecture
  view, platform matrix, deliverables.
- `crates/aphrody-wasm/examples/browser-playground.html` — runnable WASM demo.

Have a recipe worth adding? Open a PR against this file — the bar is
copy-paste reproducibility plus the actual expected output.
