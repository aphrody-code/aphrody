// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Commande `n2b rules` — liste toutes les règles connues.
use std::process::ExitCode;

use anyhow::Result;
use colored::Colorize;
use serde_json::json;

use n2b_core::types::Report;

pub fn run_rules(report: Report) -> Result<ExitCode> {
    // Liste statique des IDs de règles. Plus simple et stable qu'une
    // introspection runtime, et reflète ce que l'agent peut vraiment voir.
    let rules: &[(&str, &str)] = &[
        ("shebang/node", "shebang `node` → `bun`"),
        (
            "lock/rival",
            "lockfile concurrent (pnpm-lock / yarn.lock / package-lock)",
        ),
        ("cli/npm-install", "npm install → bun install"),
        ("cli/npm-ci", "npm ci → bun install --frozen-lockfile"),
        ("cli/npm-run", "npm run → bun run"),
        ("cli/npx", "npx → bunx"),
        ("cli/pnpm-install", "pnpm install → bun install"),
        ("cli/pnpm-add", "pnpm add → bun add"),
        ("cli/pnpm-dlx", "pnpm dlx → bunx"),
        ("cli/yarn-add", "yarn add → bun add"),
        ("pkg/package-manager", "packageManager → bun@<version>"),
        ("pkg/engines-pm", "engines.{npm,pnpm,yarn} → engines.bun"),
        (
            "pkg/redundant-dep",
            "dépendance redondante avec API Bun native",
        ),
        ("imports/node-prefix", "préfixer `fs` → `node:fs` etc."),
        (
            "imports/bun-native",
            "package npm → API Bun native (bun:sqlite, Bun.sql, etc.)",
        ),
        (
            "api/fs-readFileSync",
            "fs.readFileSync(path, 'utf8') → await Bun.file(path).text()",
        ),
        (
            "api/fs-writeFileSync",
            "fs.writeFileSync(path, data) → await Bun.write(path, data)",
        ),
        (
            "api/fs-existsSync",
            "fs.existsSync(path) → await Bun.file(path).exists()",
        ),
        (
            "api/fs-readFile-promise",
            "fsPromises.readFile → Bun.file(path).text()",
        ),
        (
            "api/json-parse-readFileSync",
            "JSON.parse(fs.readFileSync) → Bun.file(path).json()",
        ),
        ("api/dirname-esm", "__dirname ESM → import.meta.dir"),
        ("api/filename-esm", "__filename ESM → import.meta.path"),
        (
            "api/fileURLToPath",
            "fileURLToPath → Bun.fileURLToPath / import.meta.dir",
        ),
        (
            "api/new-url-import-meta",
            "new URL(..., import.meta.url) → import.meta.dir",
        ),
        (
            "api/path-join-dirname",
            "path.join(__dirname, ...) → path.join(import.meta.dir, ...)",
        ),
        ("api/buffer-alloc", "Buffer.alloc(n) → new Uint8Array(n)"),
        (
            "api/buffer-concat",
            "Buffer.concat → concaténation Uint8Array",
        ),
        (
            "api/buffer-from-string",
            "Buffer.from(str, 'utf8') → new TextEncoder().encode(str)",
        ),
        (
            "api/buffer-from-base64",
            "Buffer.from(x, 'base64') → atob/btoa",
        ),
        (
            "api/buffer-byteLength",
            "Buffer.byteLength → new TextEncoder().encode().length",
        ),
        ("api/http-createServer", "http.createServer → Bun.serve()"),
        (
            "api/https-createServer",
            "https.createServer → Bun.serve({ tls })",
        ),
        ("api/execSync", "child_process.execSync → shell Bun $`cmd`"),
        ("api/exec", "child_process.exec → Bun.spawn"),
        ("api/child-process-spawn", "spawn → Bun.spawn"),
        (
            "api/crypto-createHash",
            "crypto.createHash → Bun.hash / Bun.CryptoHasher",
        ),
        ("api/util-inspect", "util.inspect → Bun.inspect"),
        (
            "api/util-promisify",
            "util.promisify (préférer APIs async natives)",
        ),
        ("api/sleep-promise", "setTimeout Promise → Bun.sleep"),
        (
            "api/uuid-v4",
            "uuidv4() → crypto.randomUUID() / Bun.randomUUIDv7",
        ),
        ("api/express-server", "express() → Bun.serve"),
        ("api/toml-parse", "TOML.parse → Bun.TOML.parse"),
        ("api/semver", "semver.* → Bun.semver.*"),
        ("api/require-resolve", "require.resolve → Bun.resolveSync"),
        (
            "api/set-immediate",
            "setImmediate → queueMicrotask / setTimeout(fn, 0)",
        ),
        ("api/os-platform", "os.platform() → process.platform (info)"),
        ("api/os-homedir", "os.homedir() → Bun.env.HOME (info)"),
        (
            "api/process-env",
            "process.env.X → Bun.env.X (info, stylistique)",
        ),
        (
            "api/process-stdout-write",
            "process.stdout.write → Bun.stdout.write",
        ),
        (
            "api/process-stderr-write",
            "process.stderr.write → Bun.stderr.write",
        ),
        (
            "api/performance-now",
            "performance.now → Bun.nanoseconds (info)",
        ),
        ("ci/setup-node", "actions/setup-node → oven-sh/setup-bun@v2"),
        ("ci/node-version", "node-version → bun-version: latest"),
        ("docker/node-base", "FROM node:<tag> → FROM oven/bun:<tag>"),
        ("env/nvmrc", ".nvmrc / .node-version (info)"),
        (
            "tsconfig/bun-types",
            "compilerOptions.types : ajouter 'bun'",
        ),
        (
            "tsconfig/module-resolution",
            "moduleResolution → bundler/nodenext",
        ),
        (
            "workspace/pnpm-yaml",
            "pnpm-workspace.yaml → \"workspaces\" dans package.json racine",
        ),
        (
            "workspace/only-built-deps",
            "onlyBuiltDependencies → trustedDependencies",
        ),
        (
            "workspace/root-missing",
            "package.json racine sans \"workspaces\" alors que pnpm-workspace.yaml existe",
        ),
        (
            "workspace/trusted-deps-missing",
            "trustedDependencies manquant (onlyBuiltDependencies de pnpm non porté)",
        ),
        ("husky/pnpm-command", "hook husky 'pnpm X' → 'bun run X'"),
        ("husky/npm-command", "hook husky 'npm X' → 'bun run X'"),
        ("husky/yarn-command", "hook husky 'yarn X' → 'bun run X'"),
        ("husky/npx-command", "hook husky 'npx' → 'bunx --bun'"),
        ("husky/pnpm-dlx", "hook husky 'pnpm dlx' → 'bunx --bun'"),
        ("pkg/jest-script", "script 'jest' → 'bun test'"),
        (
            "pkg/tsup-bun-external",
            "script tsup + import('bun') → ajouter '--external bun'",
        ),
        // --- Bun namespace APIs ---
        ("api/bcrypt-hash", "bcrypt.hash → Bun.password.hash"),
        ("api/bcrypt-compare", "bcrypt.compare → Bun.password.verify"),
        ("api/argon2-hash", "argon2.hash/verify → Bun.password"),
        ("api/yaml-parse", "yaml.load/parse → Bun.YAML.parse"),
        (
            "api/yaml-stringify",
            "yaml.dump/stringify → Bun.YAML.stringify",
        ),
        ("api/json5-parse", "JSON5.parse → Bun.JSON5.parse"),
        (
            "api/json5-stringify",
            "JSON5.stringify → Bun.JSON5.stringify",
        ),
        ("api/marked-call", "marked() → Bun.markdown.html"),
        ("api/marked-parse", "marked.parse → Bun.markdown.html"),
        ("api/escape-html", "escapeHtml / he.encode → Bun.escapeHTML"),
        ("api/strip-ansi", "stripAnsi → Bun.stripANSI"),
        ("api/string-width", "stringWidth → Bun.stringWidth"),
        ("api/slice-ansi", "sliceAnsi → Bun.sliceAnsi"),
        ("api/which-call", "which(pkg) → Bun.which(pkg)"),
        ("api/cron-schedule", "cron.schedule → Bun.cron"),
        ("api/cronjob-new", "new CronJob → Bun.cron"),
        ("api/fast-deep-equal", "fastDeepEqual → Bun.deepEquals"),
        (
            "api/pako-gzip",
            "pako.gzip/deflate → Bun.gzipSync / deflateSync",
        ),
        (
            "api/pako-gunzip",
            "pako.ungzip/inflate → Bun.gunzipSync / inflateSync",
        ),
        ("api/express-app", "express() → Bun.serve (info)"),
        ("api/fastify-app", "fastify() → Bun.serve (info)"),
        ("api/koa-new", "new Koa() → Bun.serve (info)"),
        ("api/http-request", "http.request → fetch"),
        ("api/https-request", "https.request → fetch"),
        (
            "api/crypto-randomBytes",
            "crypto.randomBytes → crypto.getRandomValues",
        ),
        (
            "api/eventsource-new",
            "new EventSource → Bun.EventSource (déjà global, info)",
        ),
        ("api/cookie-parse", "cookie.parse → new Bun.CookieMap"),
        (
            "api/cookie-serialize",
            "cookie.serialize → Bun.Cookie.toString",
        ),
        ("api/aws-sdk-s3-client", "new S3Client → Bun.S3Client"),
        (
            "api/file-based-routing",
            "next-router → Bun.FileSystemRouter (info)",
        ),
        (
            "api/chalk-call",
            "chalk.<color> → Bun.color / ANSI natif (info)",
        ),
        ("api/zlib-gzipSync", "zlib.gzipSync → Bun.gzipSync (info)"),
        (
            "api/process-hrtime-bigint",
            "process.hrtime.bigint → Bun.nanoseconds (info)",
        ),
        ("api/execa-call", "execa → Bun.$ / Bun.spawn"),
        // --- bunfig.toml ---
        (
            "bunfig/registry-npmjs",
            "registry npmjs par défaut (redondant)",
        ),
        (
            "bunfig/option-note",
            "note sur une option bunfig (isolated, saveTextLockfile)",
        ),
        ("bunfig/unknown-option", "option bunfig inconnue (legacy)"),
        // --- tsconfig étendu ---
        (
            "tsconfig/module-legacy",
            "module=CommonJS/AMD/UMD → ESNext/Preserve",
        ),
        (
            "tsconfig/target-legacy",
            "target=ES2021 ou moins → ES2022+/ESNext",
        ),
        (
            "tsconfig/module-detection",
            "moduleDetection absent → 'force'",
        ),
        (
            "tsconfig/verbatim-module-syntax",
            "moduleResolution=bundler + verbatimModuleSyntax=true",
        ),
        (
            "tsconfig/allow-ts-extensions",
            "Bun résout les .ts nativement → allowImportingTsExtensions",
        ),
        (
            "tsconfig/no-emit",
            "bundler + noEmit=true (Bun émet, tsc type-check)",
        ),
        (
            "tsconfig/duplicate-node-types",
            "types=['bun','node'] redondant — bun suffit",
        ),
        // --- Next.js ---
        (
            "next/output-standalone",
            "next.config output:'standalone' (info, reste OK en Node)",
        ),
        (
            "next/webpack-custom",
            "next.config webpack() custom (Turbopack est default Next 16)",
        ),
        (
            "next/server-external-packages",
            "experimental.serverComponentsExternalPackages (auditer)",
        ),
        (
            "next/turbopack-missing",
            "next.config : webpack custom sans turbopack: {}",
        ),
        ("next/images-custom-loader", "next images.loader: 'custom'"),
        (
            "next/script-runtime",
            "script 'next dev/start' → préfixer par bunx --bun",
        ),
        (
            "next/build-turbopack",
            "'next build' sans --turbopack (Next 16)",
        ),
        (
            "next/custom-server-next-app",
            "next({ dev }) custom server → Bun.serve({ fetch })",
        ),
        (
            "next/request-handler",
            "app.getRequestHandler() → wrapper pour Bun.serve",
        ),
        // --- Ecosystem (info, guides) ---
        ("ecosystem/nextjs", "next détecté → guide Bun + Next.js"),
        ("ecosystem/nuxt", "nuxt détecté → guide Bun + Nuxt"),
        ("ecosystem/astro", "astro détecté → guide Bun + Astro"),
        (
            "ecosystem/remix",
            "@remix-run/react détecté → guide Bun + Remix",
        ),
        (
            "ecosystem/sveltekit",
            "@sveltejs/kit détecté → guide Bun + SvelteKit",
        ),
        ("ecosystem/tanstack-start", "@tanstack/start → guide Bun"),
        ("ecosystem/solid-start", "solid-start → guide Bun"),
        ("ecosystem/qwik", "@builder.io/qwik → guide Bun"),
        ("ecosystem/hono", "hono → guide Bun"),
        ("ecosystem/elysia", "elysia → guide Bun"),
        ("ecosystem/fastify", "fastify → guide Bun"),
        ("ecosystem/express", "express → guide Bun"),
        ("ecosystem/stric", "stric → guide Bun"),
        ("ecosystem/vite", "vite → guide Bun"),
        ("ecosystem/prisma", "prisma / @prisma/client → guide Bun"),
        ("ecosystem/drizzle", "drizzle-orm → guide Bun"),
        ("ecosystem/mongoose", "mongoose → guide Bun"),
        ("ecosystem/gel", "@edgedb/driver (Gel) → guide Bun"),
        ("ecosystem/pm2", "pm2 → guide daemon Bun"),
        ("ecosystem/sentry", "@sentry/node → guide Sentry + Bun"),
        (
            "ecosystem/discord-bot",
            "discord.js → guide Discord bot Bun",
        ),
        // --- Top 10 awesome-bun packages ---
        ("ecosystem/graphql-yoga", "graphql-yoga → intégration Bun"),
        ("ecosystem/orama", "@orama/orama → search engine"),
        ("ecosystem/brisa", "brisa → full-stack framework"),
        ("ecosystem/kysely", "kysely → SQL query builder"),
        (
            "ecosystem/kysely-bun",
            "kysely-bun-sqlite → Kysely + bun:sqlite",
        ),
        ("ecosystem/hattip", "@hattip/core → HTTP cross-runtime"),
        ("ecosystem/primate", "primate → web framework"),
        ("ecosystem/vixeny", "vixeny → functional web framework"),
        ("ecosystem/nbit", "nbit → zero-dep web framework"),
        (
            "ecosystem/bun-utilities",
            "bun-utilities → FS/shell helpers",
        ),
        ("ecosystem/electrobun", "electrobun → desktop apps Bun+Zig"),
        (
            "ecosystem/electron-alt",
            "electron → envisager Electrobun (Bun+Zig)",
        ),
        ("ecosystem/tauri", "tauri → compatible Bun frontend"),
        ("ecosystem/ink", "ink (React CLI) → tourne sous Bun"),
        ("ecosystem/bunli", "bunli → CLI framework Bun-native"),
        ("ecosystem/commander", "commander → compatible, ou Bunli"),
        (
            "ecosystem/yargs",
            "yargs → compatible, ou util.parseArgs/Bunli",
        ),
        // --- Rspack / Rstack (bundlers Rust) ---
        ("ecosystem/rspack", "@rspack/core → Rspack (Rust bundler)"),
        ("ecosystem/next-rspack", "next-rspack → Next.js + Rspack"),
        ("ecosystem/rsbuild", "@rsbuild/core → Rsbuild"),
        ("ecosystem/rslib", "@rslib/core → Rslib (build libs)"),
        ("ecosystem/rsdoctor", "rsdoctor → build analyzer"),
        ("ecosystem/rspress", "rspress → static site generator Rust"),
        (
            "next/rspack-wrapper",
            "withRspack() détecté dans next.config",
        ),
        // --- Cargo.toml — frameworks Rust → WASM ---
        ("ecosystem/yew", "yew (Cargo.toml) → React-like Rust"),
        (
            "ecosystem/leptos",
            "leptos (Cargo.toml) → fine-grained reactivity",
        ),
        (
            "ecosystem/dioxus",
            "dioxus (Cargo.toml) → cross-platform Rust GUI",
        ),
        (
            "ecosystem/sycamore",
            "sycamore (Cargo.toml) → Solid-like Rust",
        ),
        ("ecosystem/seed", "seed (Cargo.toml) → Elm-like SPA"),
        ("ecosystem/wgpu", "wgpu (Cargo.toml) → WebGPU"),
        ("ecosystem/naga", "naga (Cargo.toml) → shader translator"),
        ("ecosystem/wasm-bindgen", "wasm-bindgen (Cargo.toml)"),
        ("ecosystem/js-sys", "js-sys (Cargo.toml)"),
        ("ecosystem/web-sys", "web-sys (Cargo.toml)"),
        ("ecosystem/wasm-bindgen-futures", "wasm-bindgen-futures"),
        ("ecosystem/panic-hook", "console_error_panic_hook"),
        ("ecosystem/wee-alloc", "wee_alloc (small WASM allocator)"),
        ("ecosystem/napi-rs", "napi / napi-derive → napi-rs"),
        (
            "ecosystem/bun-native-plugin",
            "bun-native-plugin (Cargo.toml)",
        ),
        ("ecosystem/serde-wasm", "serde-wasm-bindgen"),
        ("ecosystem/gloo", "gloo (Rust+WASM toolkit)"),
        ("ecosystem/tauri-rs", "tauri (Cargo.toml)"),
        ("ecosystem/bevy", "bevy (Cargo.toml) → game engine"),
        ("ecosystem/mdxjs-rs", "mdxjs-rs (Cargo.toml)"),
        (
            "ecosystem/windows-rs",
            "windows / windows-sys (Cargo.toml) → Win32",
        ),
        ("ecosystem/libc", "libc (Cargo.toml) → POSIX + CRT"),
        ("ecosystem/nix-rs", "nix (Cargo.toml) → POSIX idiomatic"),
        (
            "ecosystem/lightningcss",
            "lightningcss (Cargo.toml) → CSS bundler",
        ),
        (
            "ecosystem/uutils",
            "uutils coreutils/findutils/diffutils/procps (cross-platform CLI)",
        ),
        (
            "ecosystem/util-linux-rs",
            "uutils/util-linux (mount/fdisk/lscpu/dmesg Rust, Linux-only)",
        ),
        // --- GNU → Rust rewrites ---
        ("ecosystem/ripgrep", "ripgrep (grep successor)"),
        ("ecosystem/fd-find", "fd (find successor)"),
        ("ecosystem/bat", "bat (cat + syntax)"),
        ("ecosystem/tokei", "tokei (cloc successor)"),
        ("ecosystem/hyperfine", "hyperfine (benchmark)"),
        ("ecosystem/du-dust", "dust (du successor)"),
        ("ecosystem/ouch", "ouch (universal de/compress)"),
        ("ecosystem/zoxide", "zoxide (cd successor)"),
        ("ecosystem/eza", "eza (ls successor)"),
        ("ecosystem/sd", "sd (sed successor)"),
        ("ecosystem/bottom", "bottom (top/htop successor)"),
        ("ecosystem/delta", "delta (git diff viewer)"),
        ("ecosystem/just", "just (make successor)"),
        ("ecosystem/watchexec", "watchexec (watch+exec)"),
        ("ecosystem/xh", "xh (curl/httpie successor)"),
        ("ecosystem/miniserve", "miniserve (HTTP serveur minimal)"),
        ("ecosystem/duf", "duf (df successor)"),
        // --- SWC stack ---
        ("ecosystem/swc", "SWC (@swc/core ou swc_core Cargo.toml)"),
        ("ecosystem/swc-node", "@swc-node/register (TS loader Node)"),
        // --- TypeScript type generation from Rust ---
        (
            "ecosystem/ts-rs",
            "ts-rs (Cargo.toml) → génère .ts depuis Rust",
        ),
        (
            "ecosystem/specta",
            "specta (Cargo.toml) → alternative ts-rs",
        ),
        // --- Turbopack internals ---
        (
            "ecosystem/turbopack",
            "turbopack / turbopack-core (Cargo.toml)",
        ),
        ("ecosystem/turbo-tasks", "turbo-tasks (Cargo.toml)"),
        // --- Rust backend stack ---
        ("ecosystem/serde", "serde (Cargo.toml)"),
        ("ecosystem/serde-json", "serde_json (Cargo.toml)"),
        ("ecosystem/tokio", "tokio (Cargo.toml) → async runtime"),
        ("ecosystem/reqwest", "reqwest (Cargo.toml) → HTTP client"),
        ("ecosystem/axum", "axum (Cargo.toml) → HTTP framework"),
        ("ecosystem/clap", "clap (Cargo.toml) → CLI parser"),
        ("ecosystem/anyhow", "anyhow (Cargo.toml)"),
        ("ecosystem/thiserror", "thiserror (Cargo.toml)"),
        // --- Rstack awesome (package.json) ---
        ("ecosystem/rstest", "@rstest/core → test runner Rust-based"),
        ("ecosystem/rslint", "@rslint/core → linter Rust-based"),
        ("ecosystem/storybook-rsbuild", "storybook-rsbuild"),
        ("ecosystem/nx-rspack", "@nx/rspack"),
        ("ecosystem/nx-rsbuild", "@nx/rsbuild"),
        ("ecosystem/nuxt-rspack", "@nuxt/rspack-builder"),
        ("ecosystem/repack", "Re.Pack (React Native)"),
        ("ecosystem/modernjs", "Modern.js"),
        ("ecosystem/esmx", "Esmx (micro-frontend)"),
        ("ecosystem/extension-js", "Extension.js"),
        // --- Next.js Turbopack config ---
        ("next/turbopack-rules", "turbopack.rules dans next.config"),
        ("next/turbopack-alias", "turbopack.resolveAlias"),
        ("next/transpile-packages", "transpilePackages (config Next)"),
        ("next/compiler-styled", "compiler.styledComponents (SWC)"),
        ("next/compiler-emotion", "compiler.emotion (SWC)"),
        ("next/compiler-remove-console", "compiler.removeConsole"),
        (
            "next/compiler-react-remove-props",
            "compiler.reactRemoveProperties",
        ),
        ("next/compiler-relay", "compiler.relay"),
        (
            "next/compiler-define",
            "compiler.define / defineServer (Next 15+)",
        ),
        ("next/swc-plugins", "experimental.swcPlugins"),
        ("next/swc-trace", "experimental.swcTraceProfiling"),
        // --- Tauri v2 ---
        ("ecosystem/tauri-v2", "@tauri-apps/api / cli"),
        ("ecosystem/tauri-v2-plugin", "@tauri-apps/plugin-*"),
        (
            "tauri/before-cmd-pm",
            "tauri.conf beforeDevCommand utilise npm/pnpm/yarn",
        ),
        (
            "tauri/frontend-dist-next-export",
            "frontendDist='out' → Next static export",
        ),
        // --- Rust web frameworks (flosse/rust-web-framework-comparison) ---
        ("ecosystem/actix-web", "actix-web (Cargo.toml)"),
        ("ecosystem/rocket", "rocket (Cargo.toml)"),
        ("ecosystem/salvo", "salvo (Cargo.toml)"),
        ("ecosystem/warp", "warp (Cargo.toml)"),
        ("ecosystem/tide", "tide (Cargo.toml)"),
        ("ecosystem/poem", "poem (Cargo.toml)"),
        ("ecosystem/gotham", "gotham (Cargo.toml)"),
        ("ecosystem/iron", "iron (Cargo.toml, legacy)"),
        ("ecosystem/nickel", "nickel (Cargo.toml)"),
        ("ecosystem/cot", "cot (Cargo.toml)"),
        ("ecosystem/pavex", "pavex (Cargo.toml)"),
        // --- Rust WASM frontend (étendu) ---
        ("ecosystem/egui", "egui (immediate-mode GUI)"),
        ("ecosystem/iced", "iced (Elm-inspired GUI)"),
        ("ecosystem/silkenweb", "silkenweb"),
        ("ecosystem/vizia", "vizia"),
        ("ecosystem/xilem", "xilem (experimental)"),
        ("ecosystem/floem", "floem"),
        // --- Templating ---
        ("ecosystem/askama", "askama (Jinja, compile-time)"),
        ("ecosystem/handlebars", "handlebars (runtime)"),
        ("ecosystem/tera", "tera (Jinja/Django)"),
        ("ecosystem/maud", "maud (HTML DSL macro)"),
        ("ecosystem/sailfish", "sailfish (compile-time fast)"),
        // --- WebSocket/HTTP ---
        (
            "ecosystem/tokio-tungstenite",
            "tokio-tungstenite (WS async)",
        ),
        ("ecosystem/tungstenite", "tungstenite (WS blocking)"),
        ("ecosystem/hyper", "hyper (low-level HTTP)"),
        ("ecosystem/ureq", "ureq (sync HTTP)"),
        ("ecosystem/isahc", "isahc (libcurl)"),
        // --- UI : shadcn + Radix ---
        ("ecosystem/shadcn", "shadcn/ui (components.json ou CLI)"),
        (
            "ecosystem/radix-ui",
            "@radix-ui/react-* (primitives headless)",
        ),
        (
            "ecosystem/cva",
            "class-variance-authority (variants Tailwind)",
        ),
        ("ecosystem/clsx", "clsx (className concat)"),
        (
            "ecosystem/tailwind-merge",
            "tailwind-merge (dédupe classes)",
        ),
        ("ecosystem/lucide", "lucide-react (icons default shadcn)"),
        // --- UI : Material Design 3 ---
        (
            "ecosystem/material-web",
            "@material/web (Web Components M3)",
        ),
        (
            "ecosystem/material-tailwind",
            "@material-tailwind/* (M3 + Tailwind)",
        ),
        ("ecosystem/emotion", "@emotion/* (CSS-in-JS, MUI default)"),
        // --- UI : icons & fonts ---
        (
            "ecosystem/material-symbols",
            "material-symbols (variable icons)",
        ),
        (
            "ecosystem/fontsource",
            "@fontsource/* (self-host Google Fonts)",
        ),
        // --- UI : utility libs ---
        ("ecosystem/sonner", "sonner (toasts)"),
        ("ecosystem/vaul", "vaul (drawer)"),
        ("ecosystem/cmdk", "cmdk (command menu)"),
        ("ecosystem/react-hook-form", "react-hook-form"),
        ("ecosystem/zod", "zod (schema validation)"),
        ("ecosystem/hookform-resolvers", "@hookform/resolvers"),
        ("ecosystem/skills", "skills (AI skills CLI, context packs)"),
        (
            "ecosystem/biome",
            "@biomejs/biome (linter+formatter Rust, remplace ESLint+Prettier)",
        ),
        ("ecosystem/oxc", "oxc (JS/TS Rust parser, Cargo.toml)"),
        ("ecosystem/oxlint", "oxlint (linter Rust OXC)"),
        // --- shadcn config issues ---
        (
            "shadcn/tailwind-css",
            "components.json tailwind.css invalide",
        ),
        ("shadcn/style-unknown", "components.json style invalide"),
        (
            "shadcn/custom-registry",
            "components.json registries custom",
        ),
        // --- Turborepo ---
        ("ecosystem/turbo", "turbo (turborepo, Rust)"),
        ("ecosystem/turbo-gen", "@turbo/gen"),
        (
            "turbo/global-deps-bun-lock",
            "globalDependencies : ajouter bun.lock",
        ),
        (
            "turbo/task-inputs-lock",
            "task inputs mentionne lockfile non-Bun",
        ),
        (
            "ecosystem/react-wasm",
            "react-wasm → charge .wasm comme composants React",
        ),
        (
            "ecosystem/wasm-react",
            "wasm-react (Cargo.toml) → composants React Rust→WASM",
        ),
        // --- .npmrc / .yarnrc / .pnpmrc → bunfig.toml ---
        (
            "npmrc/registry",
            ".npmrc registry → bunfig.toml [install].registry",
        ),
        (
            "npmrc/auth-token",
            ".npmrc _authToken → bunfig.toml [install.scopes]",
        ),
        (
            "npmrc/scoped-registry",
            "@scope:registry → bunfig.toml [install.scopes]",
        ),
        ("npmrc/always-auth", "'always-auth' spécifique npm"),
        ("npmrc/save-prefix", "save-exact/save-prefix → bunfig.toml"),
        (
            "npmrc/node-linker",
            "node-linker → bunfig.toml [install].linker",
        ),
        ("npmrc/engine-strict", "engine-strict (Bun lit engines.bun)"),
        (
            "npmrc/lockfile-flag",
            "options lockfile obsolètes avec bun.lock",
        ),
        // Phase 4 — nouvelles règles
        (
            "env/node-only-var",
            "vars Node-only dans .env (NODE_OPTIONS, NODE_PATH, etc.)",
        ),
        ("docker-compose/node-image", "image: node:* → oven/bun:*"),
        (
            "js-config/jest",
            "jest.config → bun:test (compatible Jest API)",
        ),
        ("js-config/vitest", "vitest.config → bun:test"),
        ("js-config/webpack", "webpack.config → Bun.build"),
        ("js-config/rollup", "rollup.config → Bun.build"),
        (
            "js-config/babel",
            "babel.config / .babelrc → transpile natif Bun",
        ),
        ("js-config/mocha", ".mocharc → bun:test"),
        ("globals/__dirname", "__dirname → import.meta.dir (ESM)"),
        ("globals/__filename", "__filename → import.meta.path (ESM)"),
        ("globals/process-cwd", "process.cwd() — supporté nativement"),
        ("globals/process-platform", "process.platform — natif"),
        ("globals/process-argv", "process.argv → Bun.argv (alias)"),
        ("globals/process-env", "process.env → Bun.env (alias)"),
        (
            "globals/module-exports",
            "module.exports → export default (CJS → ESM)",
        ),
        ("globals/exports", "exports.X → export const X (CJS → ESM)"),
        (
            "globals/require-dynamic",
            "require(<expr>) → await import(<expr>)",
        ),
    ];
    match report {
        Report::Json | Report::Jsonl => {
            let arr: Vec<_> = rules
                .iter()
                .map(|(id, desc)| {
                    json!({
                        "id": id,
                        "category": n2b_core::ai::category(id),
                        "description": desc,
                        "docs_url": n2b_core::ai::docs_url(id),
                        "confidence": n2b_core::ai::confidence(id, true),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&arr)?);
        }
        _ => {
            println!(
                "{} {}",
                "n2b".bold(),
                format!("({} rules)", rules.len()).dimmed()
            );
            let mut by_cat: std::collections::BTreeMap<&str, Vec<&(&str, &str)>> =
                Default::default();
            for r in rules {
                by_cat
                    .entry(n2b_core::ai::category(r.0))
                    .or_default()
                    .push(r);
            }
            for (cat, rs) in by_cat {
                println!("\n{}", cat.cyan().bold());
                for (id, desc) in rs {
                    println!("  {}  {}", id.yellow(), desc);
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
