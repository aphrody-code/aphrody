# wasm-pack Bun-Native Patch

## Initial Inventory

All npm/node process invocations were confined to a single file:

| File | Line | Use-case | Before |
|------|------|----------|--------|
| `src/npm.rs` | 13 | pack | `child::new_command("npm")` + `.arg("pack")` |
| `src/npm.rs` | 21 | publish | `child::new_command("npm")` + `.arg("publish")` |
| `src/npm.rs` | 48 | login | `child::new_command("npm")` + `args(["login", ...])` |

No `npx`, `node`, `yarn`, or `pnpm` invocations existed in the Rust source. The `node` keyword appeared only as the `--node` test flag (wasm-bindgen-test-runner, not a process spawn).

Total spawned npm commands: **3** (pack, publish, login).

## Changes Made

### New file: `src/js_runtime.rs` (264 lines)

- `PackageManagerKind` enum: `Auto | Bun | Npm | Yarn | Pnpm` — implements `clap::ValueEnum`.
- `PackageManager` enum: `Bun { bin } | Npm { bin } | Yarn { bin } | Pnpm { bin }`.
- `PackageManager::detect(kind)`: reads `WASM_PACK_FORCE_NPM=1` first, then the explicit kind, then auto-detects via `which::which("bun")`.
- Methods: `.command()`, `.pack(path)`, `.publish(path, access, tag)`, `.login(registry, scope, auth_type)`, `.name()`, `.bin()`.
- 5 unit tests covering env var override and name resolution.

### Modified files

| File | Change |
|------|--------|
| `src/lib.rs` | Added `pub mod js_runtime`; added `--package-manager` field to `Cli` struct |
| `src/main.rs` | Calls `PackageManager::detect(args.package_manager)` and passes `pm` to `run_wasm_pack` |
| `src/command/mod.rs` | `run_wasm_pack(command, pm: PackageManager)` — routes pm to pack/publish/login |
| `src/npm.rs` | All three functions now accept `pm: &PackageManager` and delegate to it |
| `src/command/pack.rs` | `pack(path, pkg_directory, pm)` |
| `src/command/publish/mod.rs` | `publish(..., pm)` |
| `src/command/login.rs` | `login(..., pm)` |

## Before / After Examples

### pack
```
# Before
npm pack

# After — Bun detected
bun pm pack

# After — npm fallback
npm pack
```

### publish
```
# Before
npm publish --access public --tag latest

# After — Bun detected
bun publish --access public --tag latest

# After — npm fallback
npm publish --access public --tag latest
```

### login
```
# Before
npm login --registry=https://registry.npmjs.org/

# After — Bun (no bun login, warns and falls back)
[WARN] Bun does not support `bun login`. Falling back to `npm login` ...
npm login --registry=https://registry.npmjs.org/
```

## How to Test

```bash
cd /home/ubuntu/rsbun/wasm/wasm-pack

# Verify binary
./target/release/wasm-pack --version
# -> wasm-pack 0.14.0

# Check --package-manager appears in help
./target/release/wasm-pack --help | grep package-manager

# Force Bun (auto-detected on this VPS since bun is in PATH)
./target/release/wasm-pack --package-manager bun build --help

# Force npm regardless of bun presence
WASM_PACK_FORCE_NPM=1 ./target/release/wasm-pack --package-manager auto --version

# Run unit tests
cargo test js_runtime
# -> 5 passed
```

## Residual Risks / Design Choices

1. **`bun login` unsupported**: Bun has no `bun login` equivalent yet. The fallback to `npm login` requires bun installed. A `log::warn!` makes this transparent.

2. **`bun pm pack` requires Bun >= 1.1.6**: Older Bun installations will fail at runtime. No version check is performed — the error from Bun itself is surfaced to the user.

3. **`bun publish` requires Bun >= 1.1.37**: Same caveat.

4. **yarn/pnpm pack/publish**: Both fall back to npm with a warning because their flag semantics differ (e.g. yarn uses `yarn pack` with different flag names). This is the safe choice.

5. **`which` crate already a dependency**: No new Cargo.toml change needed.

6. **Auto-detection is PATH-order based**: On this VPS `bun` is always found first. Developers who want npm can set `WASM_PACK_FORCE_NPM=1` or pass `--package-manager npm`.

7. **`wasm-pack test --node`**: This flag controls `wasm-bindgen-test-runner` target selection, not a Node.js process spawn. It was not changed — it is independent of the package manager.
