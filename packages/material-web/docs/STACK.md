# Stack — modern, Bun-native, future-proof (2026)

This monorepo is a reference for a **2026 web component toolchain**: maximally
Bun-native, with each remaining tool kept only where it is provably the best
option, and every choice measured rather than assumed.

## At a glance

| Concern                   | Tool                                                          | Why / measured                                                                                             |
| ------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Package manager / runtime | **Bun 1.3**                                                   | one runtime for install, scripts, bundling, tests; runs `.ts` directly (no precompile)                     |
| Monorepo orchestration    | **Turborepo** + workspace **catalog**                         | cached task graph; single source of truth for shared dep versions                                          |
| Bundling                  | **`Bun.build`**                                               | self-contained component bundle + the `examples/showcase` app (`bun index.html`) — no Webpack/Rollup/Vite  |
| Sass                      | **`sass-embedded`** (native dart-sass)                        | one reused compiler + parallelism: **70 s → 4.5 s** (~15×) vs the dart2js `sass` CLI                       |
| Type-check                | **`tsgo`** (TypeScript 7, Go) + **`tsc`**                     | `tsgo` 4–10× faster for checking (`bun run typecheck:fast`); `tsc` stays canonical for the shipped `.d.ts` |
| Lint / format             | **oxlint** + **oxfmt** (Rust)                                 | 755 files in ~50 ms                                                                                        |
| Headless tests            | **`bun test`** + **happy-dom** + `element-internals-polyfill` | real Lit components (forms included) without a browser                                                     |
| Browser gate              | **bxc** (real Chromium)                                       | `Bun.build` → `Bun.serve` → Chrome; asserts every component upgrades + renders, 0 console errors           |
| CSS-in-JS transpile       | **`Bun.build` CSS** (LightningCSS port)                       | vendor prefixes / color fallbacks, opt-in                                                                  |

Full clean build of the component library: **~85 s → ~16 s**.

## Principles

1. **Bun-first, measured.** A tool is replaced by Bun only when Bun is faster
   _and_ correct. `hyperfine`/timed builds back every "X is faster" claim.
2. **Keep what only its owner can do.** `tsc` is irreplaceable today for `.d.ts`
   emission (Bun transpiles but does not type-check; `tsgo` declaration emit is
   still incomplete; `isolatedDeclarations` would need ~900 explicit annotations)
   — so it stays for that one job, while `tsgo` takes over fast checking.
3. **No dead weight.** Vendored forks (Lit, gts), the heavy browser-test stack
   (Playwright, web-test-runner, jasmine), and ~14 MB of upstream doc images were
   removed; the package now ships only code + CSS.
4. **Future-aligned.** TypeScript 6 today, `tsgo` (TS 7 native) wired in for the
   migration; `engines` pinned (Bun ≥ 1.3, Node ≥ 22); CI on the same versions.

## Rust tooling

Rust powers the parts of the chain where it is the proven fastest, **already in use**:

| Role          | Tool             | Note                                                             |
| ------------- | ---------------- | ---------------------------------------------------------------- |
| Lint          | **oxlint** (oxc) | 50–100× ESLint; 755 files in ~50 ms; pinned devDep, runs in CI   |
| Format        | **oxfmt** (oxc)  | Prettier-compatible; via the editor/commit hook                  |
| Task graph    | **Turborepo**    | Rust core; cached `build`/`typecheck`/`test`                     |
| CSS transpile | **LightningCSS** | Rust→Zig port, inside `Bun.build`'s CSS pipeline                 |
| Task runner   | **just**         | ergonomic recipes in `justfile` (`just check` = full local gate) |

Local dev ergonomics (the Rust CLI toolbelt — see the repo's `justfile stats`):
`ripgrep` (rg), `fd`, `dust`/`dua`, `tokei`, `hyperfine`, `sd`, `delta`/`difftastic`, `ouch`, `bat`, `eza`.

**Deliberately not adopted**: Rolldown/Rspack (Bun.build already bundles), biome/dprint
(oxc is the chosen oxidized chain), grass (Sass needs full dart-sass — `sass-embedded`).

## Package management (Bun)

The whole dependency graph is driven by Bun's package manager (`bun@1.3.14`,
`packageManager` pinned). Conventions, verified against the Bun docs:

| Concern           | How the repo does it                                                                                                                                                                                                                                                                                                                               |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workspaces        | `workspaces.packages = ["packages/*", "examples/*"]` in the root `package.json`. `plugins/` is intentionally **not** a workspace (the bundled `mui-mcp` is standalone).                                                                                                                                                                            |
| Shared versions   | A single **catalog** (`workspaces.catalog`) is the source of truth for shared deps (TypeScript, React, lit, three, turbo, …); packages reference it with `"catalog:"`. `bun publish` inlines the resolved version.                                                                                                                                 |
| Internal deps     | Always the **`workspace:*`** protocol (resolves from disk, never the registry) — never `file:`.                                                                                                                                                                                                                                                    |
| Lockfile          | Text **`bun.lock`** (`saveTextLockfile`, the 1.3 default). The binary `bun.lockb` is never committed.                                                                                                                                                                                                                                              |
| Patches           | `patchedDependencies` (`"<pkg>@<version>": "patches/<pkg>@<version>.patch"`) via `bun patch` — applied automatically on install (MCU 0.4.0 ESM fix, `@webgpu/types`).                                                                                                                                                                              |
| Lifecycle scripts | Bun's **default-secure** policy: postinstall is blocked unless a package is in `trustedDependencies`. We add **nothing** — e.g. `@parcel/watcher`'s source-build script stays blocked because the prebuilt `@parcel/watcher-linux-*` binaries cover it.                                                                                            |
| Private registry  | `@aphrody-code` → GitHub Packages, configured once in the root **`.npmrc`** (npm-standard, CI-compatible). Auth is an **env-var reference** (`_authToken=${…}`), never a literal token — so the file is safe to commit. Optional/dev-only consumers (e.g. the bxc QA harness) use `optionalDependencies` so a tokenless `bun install` never fails. |

No per-package `bunfig.toml` for registry config (the root `.npmrc` is the single
source). `npm`/`pnpm`/`npx` are forbidden in every script.

## Commands

```bash
bun install
bun run build          # turbo + Bun.build / sass-embedded / tsc (.d.ts)
bun run typecheck      # tsc — canonical (+ shipped .d.ts)
bun run typecheck:fast # tsgo (TS7 native) — ~4× faster, parity check
bun run lint           # oxlint
bun run test           # bun test — headless components (happy-dom) + units
bun run test:browser   # bxc gate — real Chromium
bun run example        # examples/showcase — bun-native dev server
```

## Cross-platform (responsive + native)

The system is **adaptive across platforms** without per-target rewrites:

| Layer                       | Package                                                        | Platform                  | Shares                                                            |
| --------------------------- | -------------------------------------------------------------- | ------------------------- | ----------------------------------------------------------------- |
| Design tokens / breakpoints | `@aphrody-code/m3-tokens` (`./breakpoints`, `./dynamic-color`) | **any** (pure TS, no DOM) | values + logic — usable from React Native too                     |
| Adaptive React hooks        | `@aphrody-code/m3-react/adaptive`                              | web                       | re-exports the `m3-tokens` primitives, adds `matchMedia`/`resize` |
| Web components              | `@material/web` + `@aphrody-code/m3-react`                     | web / webview             | the full `<md-*>` UI                                              |

The M3 window size classes (600 / 840 / 1200 / 1600 dp) are defined **once** in
`m3-tokens/breakpoints` and consumed identically everywhere — so the same app
classifies viewports the same way on every platform. The pure-TS token +
breakpoint layer is also consumable from React Native (native views share the
tokens, not the Lit DOM components).

See `CLAUDE.md` for the full build/test internals and verified pitfalls.
