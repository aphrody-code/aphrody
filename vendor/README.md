# vendor/ — vendored upstream sources (excluded)

This directory is **intentionally gitignored** (see `.gitignore` §21).

## Why

`aphrody` is a max-light monorepo. We vendored upstream sources (Bun, uv, depot_tools, a2a-rs) once to validate integration, then dropped them from the git index:

| Source | Size | Status | Consumption |
|---|---|---|---|
| `vendor/bun/` | 105 MB | Re-clone on demand | Used as Bun runtime (CLI binary on PATH) |
| `vendor/uv/` | 29 MB | Re-clone on demand | Used as `uv` Python toolchain (CLI binary) |
| `vendor/depot_tools/` | 7.7 MB | Re-clone on demand | Chromium dev tooling, optional |
| `vendor/a2a-rs/` | 342 KB | Re-clone on demand | Reference A2A Rust impl, study only |

## Re-clone (when needed)

```bash
mkdir -p vendor && cd vendor
gh repo clone oven-sh/bun
gh repo clone astral-sh/uv
gh repo clone --depth 1 https://chromium.googlesource.com/chromium/tools/depot_tools.git
gh repo clone a2aproject/a2a-rs
```

## Production consumption

For production builds, we consume:
- **Bun** : install via [bun.sh](https://bun.sh) — `curl -fsSL https://bun.sh/install | bash`
- **uv** : install via [astral.sh/uv](https://astral.sh/uv) — `curl -LsSf https://astral.sh/uv/install.sh | sh`
- **a2a-rs** : our own forks `crates/a2a*` are derived from this reference
- **depot_tools** : only on Chromium contributor machines

No vendored sources are required for `cargo build -p cli`.
