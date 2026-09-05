<!-- SPDX-License-Identifier: Apache-2.0 -->
# Bun inside a Tauri v2 app — real-world usage, patterns, pitfalls, and the aphrody verdict

Research-only document (read-only investigation; no `cargo` run). Surveys how
real projects wire **Bun** into a **Tauri v2** app, separates the patterns that
actually work from the footguns, and validates aphrody's planned wiring
(static `frontendDist` built by `bun run build`, Rust backend calling
`aphrody::run_captured` in-process — see
[`aphrody-integration.md`](aphrody-integration.md),
[`README.md`](README.md), and `docs/plans/tauri-app.md`).

Author: aphrody-code. Last updated: 2026-05-24. Tauri v2 line, Bun 1.3.x.

> One-line framing that the whole document defends: **in a Tauri app, Bun is the
> frontend toolchain (package manager + bundler + dev server), never the app's
> backend runtime.** The Tauri backend is Rust; for aphrody it calls the `cli`
> library in-process. Bun's only job is to turn the TS/Lit/Material-Web frontend
> into a static `dist/` and (optionally) serve it with HMR during dev.

---

## 0. The two things people mean by "Bun in Tauri" — keep them apart

The single most common source of confusion (and of every pitfall in §3) is
conflating two completely different roles:

| Role | What Bun does | Is this aphrody's plan? |
|---|---|---|
| **A. Frontend toolchain** (the norm) | `bun install` (PM), `bun run build` → static assets, `bun run dev` → dev server/HMR. Bun produces `frontendDist`; Tauri embeds it. The backend stays pure Rust. | **Yes.** |
| **B. Backend/sidecar runtime** (a minority) | `bun build --compile` a Bun server to a standalone exe, ship it as a Tauri **sidecar** (`externalBin`), webview talks to it over stdio/HTTP/RPC. Business logic lives in TS, not Rust. | **No — explicitly rejected** ([`aphrody-integration.md`](aphrody-integration.md) §2, path b2). |

Role A is what `create-tauri-app` scaffolds and what every high-star app below
uses. Role B is a legitimate but niche topology for teams that want their logic
in TypeScript; it adds a whole Bun runtime per OS and a process hop. aphrody's
logic is already a Rust `cdylib`/lib, so role B would be pure overhead. This
document is therefore mostly about role A, and treats role B as the
clearly-labelled anti-pattern for our case.

The Tauri maintainers state the same boundary outright. In the official
"Bun?" discussion, JonasKruckenberg: *"If we do enable JavaScript in the Core
expect it to be Deno rather than bun … because it's written in Rust"* — i.e.
Bun is welcome as PM/frontend tooling, but the **core/runtime is and stays
Rust** ([tauri-apps/tauri discussion #5837](https://github.com/tauri-apps/tauri/discussions/5837)).

---

## 1. How Bun attaches to Tauri v2 — the `build.*` contract

Tauri does **not** integrate with any bundler specially. At dev/build time it
just (1) optionally runs a shell command you give it, then (2) loads either a
dev-server URL or a static directory. That entire contract is four fields in
`tauri.conf.json` `build` (definitions quoted from the v2 config reference,
[v2.tauri.app/reference/config](https://v2.tauri.app/reference/config/)):

- **`beforeDevCommand`** — *"A shell command to run before `tauri dev` kicks
  in."* Can be a string, or an object `{ script, cwd, wait }` (`wait` defaults
  to `false`). This is where you put `bun run dev`.
- **`beforeBuildCommand`** — *"A shell command to run before `tauri build`
  kicks in."* This is where you put `bun run build`.
- **`devUrl`** — *"The URL to load in development … usually a dev server …
  with hot-reload and HMR."* e.g. `http://localhost:1420`. Omit it if you have
  no dev server and rely on `frontendDist`.
- **`frontendDist`** — *"The path to the application assets (usually the `dist`
  folder of your javascript bundler) …"*. Relative paths are **recursively
  embedded into the binary** at compile time by `tauri-codegen`
  (see [`architecture.md`](architecture.md) §codegen). **Path is resolved
  relative to the `src-tauri/` (Tauri crate) directory**, so the canonical value
  is `"../dist"`.

The crucial, repeatedly-misunderstood fact: **Tauri runs `beforeBuildCommand`
and then simply reads `frontendDist`. It does no validation, no "did the build
emit files" check, no bundler-aware logic.** A maintainer spells this out in
[tauri-apps discussion #11474](https://github.com/orgs/tauri-apps/discussions/11474):
Tauri "simply executes the `beforeBuildCommand` without special handling"; if
`dist/` is empty or wrong, you get a missing-entry-module error from the embed
step, not a helpful one. The contract is therefore entirely on the build script:
**`bun run build` must deterministically produce the `frontendDist` directory.**

### Scaffolding it with Bun

`create-tauri-app` treats Bun as a first-class package manager. Both entry
points work and are documented:

- `bun create tauri-app` (interactive; Bun appears in the PM picker next to
  npm/pnpm/yarn) — [v2.tauri.app/start/create-project](https://v2.tauri.app/start/create-project/).
- `bunx create-tauri-app` — listed alongside npm/pnpm/yarn/deno/cargo in the
  [create-tauri-app README](https://github.com/tauri-apps/create-tauri-app/blob/dev/README.md)
  (repo: [tauri-apps/create-tauri-app](https://github.com/tauri-apps/create-tauri-app),
  1.6k stars, pushed 2026-05-22).

The generated `tauri.conf.json` for a JS/TS template is exactly the four-field
block above with `bun run dev` / `bun run build`. Note: aphrody does **not**
scaffold the frontend here — the frontend lives in the sibling `aphrody-ts`
repo (`apps/desktop-ui`) and emits a static `dist/`; aphrody only authors the
`crates/aphrody-app` Tauri crate and its `tauri.conf.json`. Scaffolding is
relevant only as evidence of the canonical wiring.

### Vite-on-Bun vs native `bun build` — the dev-server fork

Within role A there are two sub-styles, and they differ on **what serves the
dev URL**:

- **A1 — Bun as PM/launcher, Vite as bundler/dev server** (the dominant
  real-world style). `package.json` has `"dev": "vite"`, `"build": "tsc &&
  vite build"`; `tauri.conf.json` calls `bun run dev` / `bun run build`. Bun
  installs deps and runs scripts; **Vite** owns HMR and the production bundle.
- **A2 — native Bun bundler/dev server, no Vite** (aphrody's plan). `"build":
  "bun build ./src/index.html --outdir dist --minify"`, dev via `bun ./index.html`
  (the Bun fullstack dev server, since Bun 1.3) or `Bun.serve`. No Vite, no
  `node_modules` bundler binding. This is what `docs/research/bun-vs-vite-2026.md`
  recommends for `aphrody-ts` (Bun ~5.4x faster prod build, zero extra deps).

Both satisfy the same `build.*` contract — Tauri cannot tell them apart; it just
runs the script and reads `dist/`. The choice only affects the dev loop and the
build's dependency footprint, not the Tauri side. Bun's own docs even nudge
toward A2: *"You can use Vite with Bun, but many projects get faster builds &
drop hundreds of dependencies by switching to HTML imports"*
([bun.com/docs/guides/ecosystem/vite](https://bun.com/docs/guides/ecosystem/vite)).

---

## 2. Real repos using Bun + Tauri (verified via GitHub API, 2026-05-24)

Star counts and `tauri.conf.json` `build` blocks below were read directly from
each repo via the GitHub API on 2026-05-24 (not from a blog's paraphrase). The
first group is role A (frontend toolchain — **aphrody's pattern**); the second
is role B (sidecar runtime — the anti-pattern for us).

### Role A — Bun as the frontend toolchain (static `frontendDist`)

| Repo | Stars | Last push | What it is | How Bun is wired |
|---|---:|---|---|---|
| [surrealdb/surrealist](https://github.com/surrealdb/surrealist) | 1281 | 2026-05-22 | Official SurrealDB GUI (Tauri v2). | `build`=`{ beforeDevCommand:"bun run dev", beforeBuildCommand:"bun run build", frontendDist:"../dist", devUrl:"http://localhost:1420" }`. `packageManager:"bun@1.2.8"`, `bun.lock` present. Scripts: `"dev":"vite"`, `"build":"tsc && vite build"` → **A1 (Vite-on-Bun)**. |
| [atuinsh/desktop](https://github.com/atuinsh/desktop) | 2412 | 2026-04-29 | Atuin "Runbooks that run" desktop app. | Identical block: `beforeBuildCommand:"bun run build"`, `beforeDevCommand:"bun run dev"`, `devUrl:"http://localhost:1420"`, `frontendDist:"../dist"`. Tauri crate under `backend/`. |
| [triwinds/ns-emu-tools](https://github.com/triwinds/ns-emu-tools) | 4886 | 2026-05-20 | NS-emulator installer/updater (Tauri). | Appears in the `"bun run dev"` + `frontendDist` code-search set; Bun drives the JS build. |
| [irokaru/pixel-scaler](https://github.com/irokaru/pixel-scaler) | 242 | 2026-05-17 | Pixel-art upscaling tool. | Same `bun run dev`/`frontendDist` wiring. |

These four are independent, actively-maintained (all pushed within the last ~5
weeks as of writing), span 242–4886 stars, and all use **exactly the
`build.*` block aphrody plans**. The dominant style is **A1** (Bun as
PM + task-runner, Vite as the dev server/bundler) — `surrealist` is the
clearest, fully-inspected example.

Scale check: GitHub code search on 2026-05-24 returned **~1208** repos with
`"beforeBuildCommand"`+`"bun run build"` in a `tauri.conf.json`, and **~1412**
with `"bun run dev"`+`"frontendDist"`. Bun + Tauri (role A) is a well-trodden,
mainstream combination, not an exotic one.

### Role B — Bun as a backend/sidecar runtime (the anti-pattern for aphrody)

| Repo | Stars | Last push | What it is | How Bun is wired |
|---|---:|---|---|---|
| [niraj-khatiwada/tauri-bun](https://github.com/niraj-khatiwada/tauri-bun) | 27 | 2026-05-23 | Template: Tauri client + **Bun web-server sidecar**, bidirectional RPC via kkrpc over stdio. | Bun server `bun build --compile`d to a standalone exe, embedded as a Tauri **sidecar**; "almost zero extra Rust aside from sidecar config." Logic in TS, not Rust. |
| [lott-ai/tauribun](https://github.com/lott-ai/tauribun) | 4 | 2026-01-24 | "Tauri + Bun powered desktop apps"; Bun sidecar via oRPC over stdio. | `beforeBuildCommand:"bun build --compile ./src-bun/main.ts"`, `beforeDevCommand:"bun build --compile --watch ./src-bun/main.ts"`, `frontendDist:"../dist"`, `devUrl:"http://localhost:3000"`. Here `beforeBuildCommand` compiles a Bun **backend**, not the frontend. |

These exist and work, but they deliberately move business logic into a Bun
process — the opposite of aphrody, whose logic is already Rust one `use` away.
Note how role B even *reuses the same `build.*` field names for a different
purpose* (`beforeBuildCommand` compiles a server binary, not the UI) — more
evidence that the `build.*` contract is just "run a shell command", and that the
fields' meaning is entirely up to you. Their low star counts (4, 27) vs the
role-A apps (1281, 2412, 4886) also indicate role A is the mainstream choice.

---

## 3. Pitfalls (the parts that actually bite — emphasis on Windows + dev server)

### 3.1 The `--bun` flag: do NOT force the Bun runtime for the dev/build command

This is the single most important Bun-specific gotcha, and it is exactly the
"Bun = toolchain, not runtime" line in practice:

- By default, `bun run <script>` that ends up invoking Vite **respects Vite's
  `#!/usr/bin/env node` shebang and runs Vite on Node**, not on Bun
  ([bun.com/docs/guides/ecosystem/vite](https://bun.com/docs/guides/ecosystem/vite)).
- `bun --bun run …` (or `bunx --bun vite`) **forces** Vite's CLI onto the Bun
  runtime. Doing this for the Tauri dev command has caused the **Tauri process
  to exit immediately**: `bun --bun run tauri dev` fails while `bun run tauri
  dev` works ([oven-sh/bun #7731](https://github.com/oven-sh/bun/issues/7731),
  closed; macOS-reported but it is a runtime-substitution bug, not OS-specific).
- The official "Bun?" discussion repeats the warning: forcing `bun --bun run
  tauri` can yield strict-ESM errors; **avoid forcing Bun mode** for the
  orchestrating command ([discussion #5837](https://github.com/tauri-apps/tauri/discussions/5837)).

**Takeaway:** if you use Vite-on-Bun (A1), keep `beforeDevCommand` as plain
`bun run dev` and let the shebang run Vite on Node. Reserve `--bun` for code you
*want* on the Bun runtime. aphrody avoids the whole class by going **A2**: a
native `bun build` (no Vite CLI, no Node-vs-Bun shebang ambiguity for the
production build).

### 3.2 Windows: `beforeDevCommand`/`beforeBuildCommand` is spawned through the system shell

Tauri runs these as *shell commands* (the field is literally documented as "a
shell command"), so on Windows they go through `cmd`-style spawning, and the
`bun`/`bunx` executable plus any script-resolved binary must be reachable and
spawnable there. Practical consequences seen in the wild:

- **`bun: not found` / exit code 127 in CI and on fresh machines** when `bun`
  isn't on `PATH` for the shell that Tauri (or `tauri-action`) spawns. In CI
  this is the most common failure:
  [tauri-apps/tauri-action #986](https://github.com/tauri-apps/tauri-action/issues/986)
  — *"sh: 1: bun: not found … beforeBuildCommand 'bun run build' failed with
  exit code 127"* because the workflow installed Node but never installed Bun.
  **Fix: install Bun first** (the `oven-sh/setup-bun` action), before
  `tauri-action`/`tauri build` runs. The same applies locally: the shell Tauri
  spawns must resolve `bun`.
- **Shebang resolution on Windows.** `#!/usr/bin/env node` shebangs are a Unix
  mechanism; on Windows, script→interpreter resolution depends on the launcher.
  Under A1 (Vite-on-Bun), `bun run dev` resolving Vite-on-Node behaves
  differently than on Linux/macOS, and historically Vite-via-Bun spawn issues
  (e.g. esbuild EACCES/spawn) surfaced through this path
  ([oven-sh/bun #3237](https://github.com/oven-sh/bun/issues/3237), closed;
  reported on macOS, root-caused to bundler-binary spawn under Bun). A2 (native
  `bun build`, no esbuild/Vite child process) does not have this failure mode.
- **`cwd` matters.** `beforeDevCommand`/`beforeBuildCommand` run from the
  project root, but `frontendDist` is resolved **relative to `src-tauri/`** — so
  `"../dist"` is correct only when the build script writes `dist/` at the repo
  root (the parent of `src-tauri/`). Mismatched `cwd` is a frequent
  "frontendDist not found"/"missing entry module" cause
  ([discussion #11474](https://github.com/orgs/tauri-apps/discussions/11474): the
  fix is structural — make the build actually emit the dir the path points at).
  If the build script lives elsewhere, use the object form
  `beforeBuildCommand: { script: "bun run build", cwd: "..." }`.

### 3.3 Dev-server / HMR pitfalls

- **`devUrl` must match the dev server's real address/port.** Tauri loads
  `devUrl` verbatim in dev; if Vite/Bun serves on a different host/port (or
  binds `127.0.0.1` vs `localhost`, or picks a fallback port when the configured
  one is busy), the webview shows a blank/err page. The de-facto convention in
  the surveyed apps is `http://localhost:1420` (Tauri's default scaffold port);
  pin the dev server to it.
- **`beforeDevCommand` does not block by default.** `wait` defaults to `false`,
  so Tauri can start compiling/loading before the dev server is actually
  listening, producing a transient blank window or connection-refused on the
  first load. Historically this was a hard race
  ([tauri #4740](https://github.com/tauri-apps/tauri/issues/4740), closed —
  "App compilation starts before beforeDevCommand complete"). Modern Tauri polls
  `devUrl` before showing content, but for a slow first `bun`/Vite start you can
  set `beforeDevCommand: { script: "bun run dev", wait: true }` to be explicit.
- **Custom-element (Lit/Material Web) HMR is imperfect on every JS tool.**
  Neither Vite nor Bun cleanly hot-swaps a re-registered `customElements.define`
  — the Lit half tends to full-reload regardless
  (`docs/research/bun-vs-vite-2026.md` §3). This is a property of the frontend,
  not of the Bun↔Tauri seam, but it tempers any "Vite HMR is better" argument
  for aphrody's Material-Web-heavy UI.

### 3.4 Sidecar (role B) pitfalls — listed so we knowingly avoid them

If one ever shipped a Bun sidecar (we won't): a full Bun standalone exe per OS
(tens of MB) must be `bun build --compile`d for each target and declared as
`externalBin`; cross-OS compile + sidecar-naming-by-target-triple is fiddly; and
it adds a process/stdio/RPC hop for logic aphrody already has in-process in Rust.
This is precisely why [`aphrody-integration.md`](aphrody-integration.md) §3 rates
the Bun sidecar (path b2) the worst on hops, footprint, and latency.

---

## 4. Validation for aphrody + recommended config

### 4.1 Does aphrody's plan match what works in production? — Yes.

aphrody's plan (per `docs/plans/tauri-app.md` T1.2 / T2.1 and
[`aphrody-integration.md`](aphrody-integration.md) §5):

1. **Static `frontendDist` built by `bun run build`** — this is *exactly* the
   role-A pattern of `surrealdb/surrealist` (1281 stars) and `atuinsh/desktop`
   (2412 stars). Verified: their `build` blocks are character-for-character what
   aphrody plans (`beforeBuildCommand:"bun run build"`, `frontendDist:"../dist"`).
2. **No Bun dev server in production** — correct and matches everyone: in
   release, `frontendDist` is embedded into the binary by `tauri-codegen`; no
   dev server, no Bun, runs at runtime. The dev server (if any) is a dev-only
   convenience behind `devUrl`.
3. **Backend = Rust calling `aphrody::run_captured` in-process** — aligned with
   the maintainers' own statement that the core is Rust and Bun is not the
   runtime ([#5837](https://github.com/tauri-apps/tauri/discussions/5837)), and
   strictly better than the role-B sidecar that the low-star `tauri-bun`/
   `tauribun` repos use.
4. **Native `bun build` (A2) rather than Vite-on-Bun (A1)** — a *stronger*
   choice than the surveyed apps on two axes: it sidesteps the `--bun`/shebang
   Node-vs-Bun runtime hazard (§3.1) and the Vite/esbuild child-spawn issues
   (§3.2), and it carries zero bundler `node_modules` (per
   `bun-vs-vite-2026.md`). The only thing aphrody gives up is Vite's
   per-module React Fast Refresh — largely moot for a Material-Web/Lit UI whose
   custom-element HMR is imperfect on every tool anyway (§3.3).

**Verdict: the plan holds. It is the mainstream, production-proven topology,
and aphrody's A2 refinement removes the two biggest Bun-specific footguns.** No
change of direction is warranted.

### 4.2 Recommended `tauri.conf.json` `build` block for `crates/aphrody-app`

Concrete, drop-in (the frontend lives in `aphrody-ts/apps/desktop-ui`, emitting
a static `dist/` that `crates/aphrody-app/` consumes as `frontendDist`):

```json
{
  "build": {
    "beforeDevCommand": { "script": "bun run dev", "wait": true },
    "beforeBuildCommand": "bun run build",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../ui/dist"
  }
}
```

Notes on each field, for aphrody specifically:

- **`beforeBuildCommand: "bun run build"`** — the `aphrody-ts` `desktop-ui`
  `build` script must run native Bun (A2) **with `NODE_ENV=production`** so the
  React/Lit layer isn't shipped in dev mode (the §4.5 footgun in
  `bun-vs-vite-2026.md` nearly doubled the bundle). e.g. the script behind it:
  `NODE_ENV=production bun build ./src/index.html --outdir dist --minify
  --target browser`. It MUST deterministically emit the `frontendDist` dir.
- **`frontendDist`** — point it at wherever the Bun build writes `dist/`,
  **expressed relative to `src-tauri`/the `aphrody-app` crate dir**. If the UI
  build lives in the sibling repo, vendor/copy its `dist/` into a path under the
  crate (e.g. `../ui/dist`) or use an absolute build step; do not assume the
  sibling repo's layout at embed time.
- **`devUrl` + `beforeDevCommand`** — only needed for the dev loop. Pin the Bun
  dev server (`bun ./index.html` / `Bun.serve`) to `1420` to match `devUrl`, and
  keep `wait: true` so Tauri doesn't race a cold Bun start (§3.3). If aphrody
  ships **no** dev server, both fields may be omitted and Tauri will load
  `frontendDist` directly in dev too.

### 4.3 Windows pitfalls to pre-empt (aphrody is cross-platform, Win is #2)

1. **`bun` must be on `PATH` for the shell Tauri spawns** — locally and in CI.
   In the `tauri-action`/GitHub-Actions Windows job, add `oven-sh/setup-bun`
   **before** the Tauri build step, or `beforeBuildCommand` dies with
   `bun: not found` / exit 127 ([#986](https://github.com/tauri-apps/tauri-action/issues/986)).
2. **Stay on A2 native `bun build`; never `bun --bun run` the dev/build command**
   — avoids the Windows shebang/Node-vs-Bun substitution failures and the
   immediate-exit class (§3.1, [#7731](https://github.com/oven-sh/bun/issues/7731);
   esbuild-spawn [#3237](https://github.com/oven-sh/bun/issues/3237)).
3. **Watch `cwd` vs `frontendDist`** — `frontendDist` is relative to the Tauri
   crate dir; ensure the Windows build writes `dist/` where the path points
   (use the `{ script, cwd }` object form if the build runs from elsewhere).
4. **Decouple the frontend build from the Rust build** — keep Bun out of the
   core `cargo` workflow; `crates/aphrody-app` is build-excluded (per the plan),
   so only the GUI build invokes `bun`, and the Linux #1 / Windows #2 / wasm #3
   `cargo check` gates for the CLI binary stay Bun-free.

---

## 5. Bottom line

Bun-as-frontend-toolchain + Tauri-v2-Rust-backend is a mainstream, well-sourced
combination: ~1.2k+ public `tauri.conf.json` files wire `bun run build`, and
flagship apps (surrealist 1281 stars, atuin desktop 2412 stars, ns-emu-tools 4886 stars) use
the exact `beforeBuildCommand:"bun run build"` + `frontendDist:"../dist"` block
aphrody plans. The maintainers confirm the boundary (core is Rust; Bun is not
the runtime). aphrody's plan is correct as written, and its choice of **native
`bun build` (A2)** over Vite-on-Bun is a refinement that removes the two biggest
Bun footguns (the `--bun`/shebang runtime hazard and Vite/esbuild child-spawn
issues). The sidecar topology (role B) seen in the low-star `tauri-bun`/
`tauribun` repos is rightly rejected — aphrody's logic is already Rust
in-process. **Keep the plan; just enforce `NODE_ENV=production` in the Bun build,
pin `devUrl` to the dev server's real port with `wait:true`, and install Bun
before the Tauri step on every (Windows included) machine and CI runner.**

---

## Sources

All accessed 2026-05-23/24. Repo metadata (stars, last push) and the quoted
`tauri.conf.json` `build`/`package.json` `scripts` blocks were read directly via
the GitHub API on 2026-05-24.

Official Tauri / Bun docs:
- Tauri v2 config reference (`build.beforeDevCommand`/`beforeBuildCommand`/
  `devUrl`/`frontendDist`) — `https://v2.tauri.app/reference/config/`
- Tauri v2 "Create a Project" (Bun as PM, `bun create tauri-app`) —
  `https://v2.tauri.app/start/create-project/`
- create-tauri-app README (Bun among npm/pnpm/yarn/deno/cargo) —
  `https://github.com/tauri-apps/create-tauri-app/blob/dev/README.md`
- Bun official Vite guide (shebang/`--bun`, "HTML imports" recommendation) —
  `https://bun.com/docs/guides/ecosystem/vite`

Maintainer stance / discussions / issues:
- tauri-apps/tauri discussion #5837 "Bun?" (core stays Rust; Bun = frontend
  tooling; avoid forcing `--bun`) — `https://github.com/tauri-apps/tauri/discussions/5837`
- tauri-apps discussion #11474 (Tauri runs `beforeBuildCommand` without special
  handling; `frontendDist` relative to `src-tauri`) —
  `https://github.com/orgs/tauri-apps/discussions/11474`
- oven-sh/bun #7731 "Tauri process immediate exit" (`bun --bun run tauri dev`
  breaks; plain `bun run` works) — `https://github.com/oven-sh/bun/issues/7731`
- oven-sh/bun #3237 "Vite error when using bun with Tauri" (esbuild spawn
  EACCES under Bun) — `https://github.com/oven-sh/bun/issues/3237`
- tauri-apps/tauri-action #986 "bun: not found" (install Bun via
  `oven-sh/setup-bun` before the build) — `https://github.com/tauri-apps/tauri-action/issues/986`
- tauri-apps/tauri #4740 ("App compilation starts before beforeDevCommand
  complete" — the `wait` race) — `https://github.com/tauri-apps/tauri/issues/4740`

Real repos (role A — frontend toolchain):
- surrealdb/surrealist (1281 stars, 2026-05-22) — `https://github.com/surrealdb/surrealist`
- atuinsh/desktop (2412 stars, 2026-04-29) — `https://github.com/atuinsh/desktop`
- triwinds/ns-emu-tools (4886 stars, 2026-05-20) — `https://github.com/triwinds/ns-emu-tools`
- irokaru/pixel-scaler (242 stars, 2026-05-17) — `https://github.com/irokaru/pixel-scaler`

Real repos (role B — Bun sidecar runtime, the anti-pattern for aphrody):
- niraj-khatiwada/tauri-bun (27 stars, 2026-05-23) — `https://github.com/niraj-khatiwada/tauri-bun`
- lott-ai/tauribun (4 stars, 2026-01-24) — `https://github.com/lott-ai/tauribun`

aphrody in-repo basis:
- `docs/plans/tauri-app.md`, `docs/tauri/README.md`,
  `docs/tauri/aphrody-integration.md`, `docs/tauri/architecture.md`,
  `docs/research/bun-vs-vite-2026.md`.
