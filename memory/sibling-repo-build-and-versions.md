---
name: sibling-repo-build-and-versions
description: "CARGO_TARGET_DIR override sends sibling (n2b/bxc) builds into aphrody's target; bxc version is baked into the compiled standalone. Current published versions."
metadata: 
  node_type: memory
  type: reference
  originSessionId: e87d3ad8-df91-4692-835f-a6350089539d
---

**`CARGO_TARGET_DIR` is exported in the VPS shell** to
`/home/ubuntu/aphrody/target/linux-gnu`. So `cargo build` inside a sibling repo
(`/home/ubuntu/n2b`, `/home/ubuntu/bxc/rust-bridge`) writes its artifacts to
**aphrody's** target dir, NOT the sibling's local `target/`. Consequences:
- Installing a freshly built sibling binary: pull it from
  `/home/ubuntu/aphrody/target/linux-gnu/release/<bin>`, not `<repo>/target/release/`.
- For builds that must land in the repo's own `target/` (e.g. bxc's
  `scripts/build-standalone.ts` expects `rust-bridge/target/release/libbxc_rust_bridge.so`),
  prefix with `env -u CARGO_TARGET_DIR`.
- `shenron/scripts/bun-migrate.sh` hardcodes `/home/ubuntu/n2b/target/release/n2b`
  (with a `command -v n2b` PATH fallback) — that primary path is empty under the
  override, so keep `~/.local/bin/n2b` fresh.

**bxc CLI version is compile-time baked.** `bin/bxc` execs the compiled
`dist/standalone/bxc-linux-x64` when present; that binary embeds `BUILD_VERSION`
via `bun build --compile --define` (from `package.json` at build time). Bumping
`package.json` does NOT change `bxc --version` until you rebuild the standalone:
`env -u CARGO_TARGET_DIR bun scripts/build-standalone.ts` (needs the rust-bridge
`.so` built first). Cross-targets (win/mac/arm) fail locally on canary Bun
1.4.0 (no cross-download) — CI builds those on native runners; linux-x64 is the
one that matters on the VPS.

**Versions as of 2026-06-04 (manifests bumped + committed + pushed to main; NOT
yet published — tag/publish to GitHub Packages is human-gated):**
- n2b: crates + `@n2b/*` packages = `0.6.1`; `n2b-native` stays `0.1.0` (ABI v1);
  bun-agent Claude plugin + Gemini ext = `2.3.2`.
- bxc: `package.json` / `ai.json` / `gemini-extension.json` = `0.6.2`;
  `@aphrody-code/x` = `1.0.6` (separate line); rust-bridge `0.1.0`.
- Consumers (`rg`, `rpbey`) pin `@aphrody-code/bxc: ^0.6.0` (caret) → auto-pick a
  published 0.6.x; no consumer manifest edits needed. `yoyo` uses
  `file:../../../bxc/packages/x` (path is valid — `bxc/packages/x` exists,
  `x-client` does NOT; the CLAUDE.md x-client-extraction note is stale).

n2b has **no MCP server** (the "n2b MCP tool" roadmap item is unbuilt — don't
assume one exists). MCP registered in `~/.config/aphrody/mcp.json` = aphrody +
bxc only. See [[n2b-fix-and-agent-stack-bun]].

**n2b contract-test deadlock — FIXED 2026-06-04 (commit `4e6fb4a`).** Root cause:
`registry/packages.toml` had `ws` and `axios` under BOTH `imports/bun-native`
and `imports/rust-sdk-alt`; the `PACKAGES` Lazy dedup keyed on `package` alone
→ `panic!("package en doublon: ws")`. In a **debug** build that worker-thread
panic deadlocked the main thread, so `n2b test/fixture` hung forever and every
`cargo test` (contract.rs) piled up stuck procs (seen: ~25 procs, 75 min old,
load 28). Fix: dedup now keys on `(id, package)`; `BUN_REPLACEMENTS`
(node_imports.rs) now filters to `id == imports/bun-native` (the rust-sdk-alt
row was also wrongly telling TS `import ws` to use `tokio-tungstenite`).
Contract suite now 15/15 in 0.88s. If you ever see hung `n2b test/fixture`
procs again: reaping orphaned ones (parent=PID 1) is safe, but a **live**
`cargo test` from a peer claude session respawns them — trace ancestry first.
Builds land in aphrody's target dir (`CARGO_TARGET_DIR` override above), so the
`contract-*` binaries appear under `aphrody/target/linux-gnu/debug/deps/`.

**Stale installed binary (rebuilt 2026-06-04).** The `~/.local/bin/n2b` binary
predated `4e6fb4a`, so any **release** scan (`n2b .`) panicked with the OLD
message `package en doublon: ws` at `registry.rs:50` (abort, exit 134) — the
source was already fixed but the deployed bin wasn't. Rebuilt
(`env -u RUSTC_WRAPPER cargo build --release -p n2b --config "build.rustc-wrapper=''"`)
and reinstalled from `aphrody/target/linux-gnu/release/n2b` → `~/.local/bin/n2b`
(now `n2b 0.6.1`, scans clean). Lesson: that panic = stale bin, not a registry
regression; rebuild before debugging the TOML.
