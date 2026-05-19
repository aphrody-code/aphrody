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

use n2b_rules::cli_commands::apply_cli_rules;
use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::make_finding;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

const REDUNDANT_DEPS: &[&str] = &[
    "node-fetch",
    "isomorphic-fetch",
    "cross-fetch",
    "dotenv",
    "dotenv-cli",
    "rimraf",
    "mkdirp",
    "better-sqlite3",
    "sqlite3",
    "uuid",
    "nanoid",
    "ts-node",
    "tsx",
    "ts-node-esm",
    "concurrently",
];

/// Mapping dep npm → guide Bun spécifique. Émis en Info pour pointer l'utilisateur
/// vers la doc de portage / intégration sans rien "corriger".
/// Tuples : (dep_name, rule_id suffix, guide_url, label).
const ECOSYSTEM_GUIDES: &[(&str, &str, &str, &str)] = &[
    // Frameworks full-stack
    (
        "next",
        "nextjs",
        "https://bun.sh/guides/ecosystem/nextjs",
        "Next.js",
    ),
    (
        "nuxt",
        "nuxt",
        "https://bun.sh/guides/ecosystem/nuxt",
        "Nuxt",
    ),
    (
        "astro",
        "astro",
        "https://bun.sh/guides/ecosystem/astro",
        "Astro",
    ),
    (
        "@remix-run/react",
        "remix",
        "https://bun.sh/guides/ecosystem/remix",
        "Remix",
    ),
    (
        "@sveltejs/kit",
        "sveltekit",
        "https://bun.sh/guides/ecosystem/sveltekit",
        "SvelteKit",
    ),
    (
        "@tanstack/start",
        "tanstack-start",
        "https://bun.sh/guides/ecosystem/tanstack-start",
        "TanStack Start",
    ),
    (
        "solid-start",
        "solid-start",
        "https://bun.sh/guides",
        "SolidStart",
    ),
    (
        "@builder.io/qwik",
        "qwik",
        "https://bun.sh/guides/ecosystem/qwik",
        "Qwik",
    ),
    // HTTP frameworks
    (
        "hono",
        "hono",
        "https://bun.sh/guides/ecosystem/hono",
        "Hono",
    ),
    (
        "elysia",
        "elysia",
        "https://bun.sh/guides/ecosystem/elysia",
        "Elysia",
    ),
    ("fastify", "fastify", "https://bun.sh/guides", "Fastify"),
    (
        "express",
        "express",
        "https://bun.sh/guides/ecosystem/express",
        "Express",
    ),
    (
        "stric",
        "stric",
        "https://bun.sh/guides/ecosystem/stric",
        "StricJS",
    ),
    // Build / frontend
    (
        "vite",
        "vite",
        "https://bun.sh/guides/ecosystem/vite",
        "Vite",
    ),
    // ORMs & DB clients
    (
        "prisma",
        "prisma",
        "https://bun.sh/guides/ecosystem/prisma",
        "Prisma",
    ),
    (
        "@prisma/client",
        "prisma",
        "https://bun.sh/guides/ecosystem/prisma",
        "Prisma",
    ),
    (
        "drizzle-orm",
        "drizzle",
        "https://bun.sh/guides/ecosystem/drizzle",
        "Drizzle",
    ),
    (
        "mongoose",
        "mongoose",
        "https://bun.sh/guides/ecosystem/mongoose",
        "Mongoose",
    ),
    (
        "@edgedb/driver",
        "gel",
        "https://bun.sh/guides/ecosystem/gel",
        "Gel",
    ),
    // Daemons / process managers
    ("pm2", "pm2", "https://bun.sh/guides/ecosystem/pm2", "PM2"),
    // Observability
    (
        "@sentry/node",
        "sentry",
        "https://bun.sh/guides/ecosystem/sentry",
        "Sentry",
    ),
    (
        "@sentry/bun",
        "sentry",
        "https://bun.sh/guides/ecosystem/sentry",
        "Sentry",
    ),
    // Bots
    (
        "discord.js",
        "discord-bot",
        "https://bun.sh/guides/ecosystem/discordjs",
        "discord.js",
    ),
    // --- Top 10 awesome-bun packages (https://github.com/oven-sh/awesome-bun) ---
    (
        "graphql-yoga",
        "graphql-yoga",
        "https://the-guild.dev/graphql/yoga-server/v3/integrations/integration-with-bun",
        "GraphQL Yoga",
    ),
    (
        "@orama/orama",
        "orama",
        "https://github.com/oramasearch/orama",
        "Orama (full-text search)",
    ),
    (
        "brisa",
        "brisa",
        "https://github.com/brisa-build/brisa",
        "Brisa (full-stack + Web Components)",
    ),
    (
        "kysely",
        "kysely",
        "https://github.com/kysely-org/kysely",
        "Kysely (SQL query builder)",
    ),
    (
        "kysely-bun-sqlite",
        "kysely-bun",
        "https://www.npmjs.com/package/kysely-bun-sqlite",
        "Kysely + bun:sqlite adapter",
    ),
    (
        "@hattip/core",
        "hattip",
        "https://github.com/hattipjs/hattip",
        "HatTip (cross-runtime HTTP)",
    ),
    (
        "primate",
        "primate",
        "https://github.com/primatejs/primate",
        "Primate web framework",
    ),
    (
        "vixeny",
        "vixeny",
        "https://github.com/mimiMonads/vixeny",
        "Vixeny (functional web framework)",
    ),
    (
        "nbit",
        "nbit",
        "https://github.com/sstur/nbit",
        "NBit (zero-dep web framework)",
    ),
    (
        "bun-utilities",
        "bun-utilities",
        "https://www.npmjs.com/package/bun-utilities",
        "bun-utilities (FS/shell helpers)",
    ),
    // Desktop / native apps
    (
        "electrobun",
        "electrobun",
        "https://github.com/blackboardsh/electrobun",
        "Electrobun (desktop apps Bun+Zig, alternative Electron)",
    ),
    (
        "electron",
        "electron-alt",
        "https://github.com/blackboardsh/electrobun",
        "Electron détecté — envisager Electrobun (Bun+Zig, bundle ~10× plus petit)",
    ),
    (
        "tauri",
        "tauri",
        "https://tauri.app",
        "Tauri (Rust + WebView) — compatible avec Bun côté frontend",
    ),
    // CLI frameworks
    (
        "ink",
        "ink",
        "https://github.com/vadimdemedes/ink",
        "Ink (React for CLIs) — tourne sous Bun",
    ),
    (
        "bunli",
        "bunli",
        "https://bunli.dev/docs",
        "Bunli (CLI framework Bun-native, type-safe, zero-dep)",
    ),
    (
        "@bunli/core",
        "bunli",
        "https://bunli.dev/docs",
        "Bunli core",
    ),
    (
        "commander",
        "commander",
        "https://github.com/tj/commander.js",
        "Commander — marche sous Bun (envisager Bunli pour Bun-native)",
    ),
    (
        "yargs",
        "yargs",
        "https://github.com/yargs/yargs",
        "yargs — marche sous Bun (envisager util.parseArgs natif ou Bunli)",
    ),
    // --- Rspack / Rstack ecosystem (Rust-based bundlers) ---
    (
        "@rspack/core",
        "rspack",
        "https://rspack.rs/",
        "Rspack (bundler Rust, webpack-compatible)",
    ),
    ("@rspack/cli", "rspack", "https://rspack.rs/", "Rspack CLI"),
    (
        "next-rspack",
        "next-rspack",
        "https://rspack.rs/guide/tech/next",
        "next-rspack (Next.js backed by Rspack)",
    ),
    (
        "@rsbuild/core",
        "rsbuild",
        "https://rsbuild.rs/",
        "Rsbuild (build tool Rust pour web apps)",
    ),
    ("rsbuild", "rsbuild", "https://rsbuild.rs/", "Rsbuild"),
    (
        "@rslib/core",
        "rslib",
        "https://lib.rsbuild.rs/",
        "Rslib (build libs Rust-powered)",
    ),
    ("rslib", "rslib", "https://lib.rsbuild.rs/", "Rslib"),
    (
        "rsdoctor",
        "rsdoctor",
        "https://rsdoctor.rs/",
        "Rsdoctor (build analyzer pour Rspack/webpack)",
    ),
    (
        "@rsdoctor/rspack-plugin",
        "rsdoctor",
        "https://rsdoctor.rs/",
        "Rsdoctor plugin",
    ),
    (
        "rspress",
        "rspress",
        "https://rspress.rs/",
        "Rspress (static site generator Rust-powered)",
    ),
    (
        "rsbuild-plugin-react",
        "rsbuild",
        "https://rsbuild.rs/",
        "Rsbuild React plugin",
    ),
    // --- Rstack écosystème étendu (awesome-rstack) ---
    (
        "@rstest/core",
        "rstest",
        "https://rstest.rs/",
        "Rstest (test runner Rust)",
    ),
    (
        "@rslint/core",
        "rslint",
        "https://rslint.rs/",
        "Rslint (linter Rust)",
    ),
    (
        "@rspress/core",
        "rspress",
        "https://rspress.rs/",
        "Rspress core",
    ),
    (
        "storybook-rsbuild",
        "storybook-rsbuild",
        "https://storybook.js.org/",
        "Storybook builder Rsbuild",
    ),
    ("@nx/rspack", "nx-rspack", "https://nx.dev/", "Nx + Rspack"),
    (
        "@nx/rsbuild",
        "nx-rsbuild",
        "https://nx.dev/",
        "Nx + Rsbuild",
    ),
    (
        "@nuxt/rspack-builder",
        "nuxt-rspack",
        "https://nuxt.com/",
        "Nuxt bundler Rspack",
    ),
    (
        "re.pack",
        "repack",
        "https://re-pack.dev/",
        "Re.Pack (React Native toolkit)",
    ),
    (
        "@callstack/repack",
        "repack",
        "https://re-pack.dev/",
        "Re.Pack",
    ),
    (
        "@modern-js/app-tools",
        "modernjs",
        "https://modernjs.dev/",
        "Modern.js (React framework)",
    ),
    (
        "@esmx/core",
        "esmx",
        "https://www.esmnext.com/",
        "Esmx (micro-frontend)",
    ),
    (
        "extension.js",
        "extension-js",
        "https://extension.js.org/",
        "Extension.js (browser ext)",
    ),
    (
        "@rsbuild/plugin-react",
        "rsbuild",
        "https://rsbuild.rs/",
        "Rsbuild React plugin",
    ),
    (
        "@rsbuild/plugin-vue",
        "rsbuild",
        "https://rsbuild.rs/",
        "Rsbuild Vue plugin",
    ),
    (
        "@rsbuild/plugin-svelte",
        "rsbuild",
        "https://rsbuild.rs/",
        "Rsbuild Svelte plugin",
    ),
    (
        "@rsbuild/plugin-solid",
        "rsbuild",
        "https://rsbuild.rs/",
        "Rsbuild Solid plugin",
    ),
    (
        "@rsbuild/plugin-tailwindcss",
        "rsbuild",
        "https://rsbuild.rs/",
        "Rsbuild Tailwind plugin",
    ),
    (
        "@rsbuild/plugin-mdx",
        "rsbuild",
        "https://rsbuild.rs/",
        "Rsbuild MDX plugin",
    ),
    // --- SWC (bundler/compiler Rust utilisé par Next.js et Parcel) ---
    (
        "@swc/core",
        "swc",
        "https://swc.rs/",
        "SWC (Rust JS/TS compiler, 20× plus rapide que Babel)",
    ),
    ("@swc/cli", "swc", "https://swc.rs/", "SWC CLI"),
    (
        "@swc/helpers",
        "swc",
        "https://swc.rs/",
        "SWC runtime helpers",
    ),
    (
        "@swc-node/register",
        "swc-node",
        "https://github.com/swc-project/swc-node",
        "SWC register (TS loader pour Node)",
    ),
    (
        "@swc/jest",
        "swc",
        "https://swc.rs/",
        "SWC transformer pour Jest",
    ),
    // --- Turborepo (monorepo manager Rust de Vercel) ---
    (
        "turbo",
        "turbo",
        "https://turborepo.com/",
        "Turborepo (monorepo tool Rust, tasks cache)",
    ),
    (
        "@turbo/gen",
        "turbo-gen",
        "https://turborepo.com/docs/guides/generating-code",
        "Turbo code generators",
    ),
    (
        "@turbo/types",
        "turbo",
        "https://turborepo.com/",
        "Turbo types",
    ),
    (
        "react-wasm",
        "react-wasm",
        "https://github.com/mbasso/react-wasm",
        "react-wasm (charge .wasm comme composants React)",
    ),
    // --- Tauri v2 ecosystem (desktop apps Rust + frontend web) ---
    (
        "@tauri-apps/api",
        "tauri-v2",
        "https://v2.tauri.app/",
        "Tauri v2 API (core frontend bindings)",
    ),
    (
        "@tauri-apps/cli",
        "tauri-v2",
        "https://v2.tauri.app/",
        "Tauri v2 CLI",
    ),
    (
        "@tauri-apps/plugin-shell",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/shell/",
        "Tauri plugin shell",
    ),
    (
        "@tauri-apps/plugin-fs",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/file-system/",
        "Tauri plugin fs",
    ),
    (
        "@tauri-apps/plugin-dialog",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/dialog/",
        "Tauri plugin dialog",
    ),
    (
        "@tauri-apps/plugin-store",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/store/",
        "Tauri plugin store",
    ),
    (
        "@tauri-apps/plugin-sql",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/sql/",
        "Tauri plugin sql",
    ),
    (
        "@tauri-apps/plugin-http",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/http-client/",
        "Tauri plugin http",
    ),
    (
        "@tauri-apps/plugin-notification",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/notification/",
        "Tauri plugin notification",
    ),
    (
        "@tauri-apps/plugin-updater",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/updater/",
        "Tauri plugin updater",
    ),
    (
        "@tauri-apps/plugin-window-state",
        "tauri-v2-plugin",
        "https://v2.tauri.app/plugin/window-state/",
        "Tauri plugin window-state",
    ),
    // --- UI : shadcn/ui (installe via CLI, pas dep npm classique) ---
    (
        "shadcn",
        "shadcn",
        "https://ui.shadcn.com/",
        "shadcn CLI (copy-paste components Radix+Tailwind)",
    ),
    (
        "shadcn-ui",
        "shadcn",
        "https://ui.shadcn.com/",
        "shadcn-ui (legacy name)",
    ),
    // Radix primitives (base shadcn/ui)
    (
        "@radix-ui/react-accordion",
        "radix-ui",
        "https://www.radix-ui.com/",
        "Radix UI primitives",
    ),
    (
        "@radix-ui/react-dialog",
        "radix-ui",
        "https://www.radix-ui.com/",
        "Radix UI primitives",
    ),
    (
        "@radix-ui/react-dropdown-menu",
        "radix-ui",
        "https://www.radix-ui.com/",
        "Radix UI primitives",
    ),
    (
        "@radix-ui/react-popover",
        "radix-ui",
        "https://www.radix-ui.com/",
        "Radix UI primitives",
    ),
    (
        "@radix-ui/react-select",
        "radix-ui",
        "https://www.radix-ui.com/",
        "Radix UI primitives",
    ),
    (
        "@radix-ui/react-slot",
        "radix-ui",
        "https://www.radix-ui.com/",
        "Radix UI primitives",
    ),
    (
        "@radix-ui/react-toast",
        "radix-ui",
        "https://www.radix-ui.com/",
        "Radix UI primitives",
    ),
    (
        "class-variance-authority",
        "cva",
        "https://cva.style/",
        "CVA (variants Tailwind) — compagnon shadcn",
    ),
    (
        "clsx",
        "clsx",
        "https://github.com/lukeed/clsx",
        "clsx (className concat)",
    ),
    (
        "tailwind-merge",
        "tailwind-merge",
        "https://github.com/dcastil/tailwind-merge",
        "tailwind-merge — dédupe classes",
    ),
    (
        "lucide-react",
        "lucide",
        "https://lucide.dev/",
        "Lucide icons (default shadcn)",
    ),
    // --- UI : Material Design 3 ---
    (
        "@material/web",
        "material-web",
        "https://material-web.dev/",
        "Material Web Components M3 (vanilla)",
    ),
    (
        "@material-tailwind/react",
        "material-tailwind",
        "https://www.material-tailwind.com/",
        "Material Tailwind (React)",
    ),
    (
        "@material-tailwind/html",
        "material-tailwind",
        "https://www.material-tailwind.com/",
        "Material Tailwind (HTML)",
    ),
    (
        "@emotion/react",
        "emotion",
        "https://emotion.sh/",
        "Emotion (CSS-in-JS, MUI default)",
    ),
    (
        "@emotion/styled",
        "emotion",
        "https://emotion.sh/",
        "Emotion styled",
    ),
    // --- Icons & Fonts ---
    (
        "material-symbols",
        "material-symbols",
        "https://fonts.google.com/icons",
        "Material Symbols (variable font icons)",
    ),
    (
        "@material-symbols/font-400",
        "material-symbols",
        "https://fonts.google.com/icons",
        "Material Symbols font 400",
    ),
    (
        "@fontsource/roboto",
        "fontsource",
        "https://fontsource.org/",
        "Fontsource Roboto (self-host)",
    ),
    (
        "@fontsource/google-sans",
        "fontsource",
        "https://fontsource.org/",
        "Fontsource Google Sans",
    ),
    (
        "@fontsource-variable/google-sans-flex",
        "fontsource",
        "https://fonts.google.com/specimen/Google+Sans+Flex",
        "Google Sans Flex variable",
    ),
    // --- UI utility libs complémentaires ---
    (
        "sonner",
        "sonner",
        "https://sonner.emilkowal.ski/",
        "Sonner (toasts, compat shadcn)",
    ),
    (
        "vaul",
        "vaul",
        "https://vaul.emilkowal.ski/",
        "Vaul (drawer, compat shadcn)",
    ),
    (
        "cmdk",
        "cmdk",
        "https://cmdk.paco.me/",
        "cmdk (command menu, compat shadcn)",
    ),
    (
        "react-hook-form",
        "react-hook-form",
        "https://react-hook-form.com/",
        "React Hook Form (forms)",
    ),
    ("zod", "zod", "https://zod.dev/", "Zod (schema validation)"),
    (
        "@hookform/resolvers",
        "hookform-resolvers",
        "https://github.com/react-hook-form/resolvers",
        "React Hook Form resolvers",
    ),
    // --- AI dev tooling (skills, MCP) ---
    (
        "skills",
        "skills",
        "https://github.com/fforres/skills",
        "skills CLI (ajoute des context packs AI : shadcn/ui, next, etc.)",
    ),
    // --- Biome (linter + formatter Rust, remplace ESLint + Prettier) ---
    (
        "@biomejs/biome",
        "biome",
        "https://biomejs.dev/",
        "Biome (linter + formatter Rust, ~100× ESLint+Prettier)",
    ),
    // --- OXC / Oxlint (linter Rust — alternative Biome) ---
    (
        "oxlint",
        "oxlint",
        "https://oxc.rs/docs/guide/usage/linter.html",
        "oxlint (linter Rust OXC, ~50× ESLint)",
    ),
    ("@oxlint/core", "oxlint", "https://oxc.rs/", "oxlint core"),
    // Note : Yew/Leptos/Dioxus/Sycamore et autres frameworks Rust→WASM sont
    // détectés par scanners/cargo_toml.rs (ils vivent dans Cargo.toml).
];

static JEST_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bjest(?:\s|$)").expect("invariant: JEST_RE regex literal is valid"));
static TSUP_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\btsup(?:\s|$)").expect("invariant: TSUP_RE regex literal is valid"));
static NEXT_DEV_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*next\s+dev\b").expect("invariant: NEXT_DEV_RE regex literal is valid")
});
static NEXT_START_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*next\s+start\b").expect("invariant: NEXT_START_RE regex literal is valid")
});
static NEXT_BUILD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*next\s+build\b").expect("invariant: NEXT_BUILD_RE regex literal is valid")
});

/// Parcourt src/ du package pour détecter un import de "bun" (statique ou dynamique).
fn package_uses_bun_import(package_json_path: &str) -> bool {
    let pkg_dir = std::path::Path::new(package_json_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let src_dir = pkg_dir.join("src");
    if !src_dir.is_dir() {
        return false;
    }
    fn walk(dir: &std::path::Path) -> bool {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return false;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if walk(&p) {
                    return true;
                }
            } else if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if matches!(ext, "ts" | "tsx" | "mts" | "cts" | "js" | "mjs" | "cjs") {
                    if let Ok(c) = std::fs::read_to_string(&p) {
                        if c.contains("from \"bun\"")
                            || c.contains("from 'bun'")
                            || c.contains("import(\"bun\")")
                            || c.contains("import('bun')")
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    walk(&src_dir)
}

pub fn scan_package_json(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();

    let mut parsed: Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            findings.push(make_finding(
                path,
                &[],
                0,
                "pkg/parse",
                format!("package.json invalide : {e}"),
                content.chars().take(40).collect::<String>(),
                None,
                MakeFindingOpts {
                    severity: Some(Severity::Error),
                    autofix: Some(false),
                    ..Default::default()
                },
            ));
            return (findings, content.to_string());
        }
    };

    let mut mutated = false;

    // 1. Rewrite scripts.
    if let Some(scripts) = parsed.get_mut("scripts").and_then(|v| v.as_object_mut()) {
        let keys: Vec<String> = scripts.keys().cloned().collect();
        for name in keys {
            let raw = match scripts.get(&name).and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let (script_findings, rewritten) =
                apply_cli_rules(&format!("{path} [scripts.{name}]"), &raw);
            findings.extend(script_findings);
            if rewritten != raw {
                scripts.insert(name.clone(), Value::String(rewritten.clone()));
                mutated = true;
            }

            // pkg/jest-script — détecte "jest ..." dans les scripts
            if JEST_RE.is_match(&rewritten) {
                findings.push(make_finding(
                    path,
                    &[],
                    0,
                    "pkg/jest-script",
                    format!(
                        "script {name:?}='{rewritten}' utilise jest — préférer 'bun test' (compatible describe/test/expect ; utiliser --preload reflect-metadata pour les décorateurs)"
                    ),
                    rewritten.clone(),
                    Some("bun test".into()),
                    MakeFindingOpts {
                        autofix: Some(false),
                        aggressive: Some(true),
                        severity: Some(Severity::Warn),
                    ..Default::default()
                },
                ));
            }

            // next/scripts-bun-runtime — forcer Bun runtime pour next dev/start
            if NEXT_DEV_RE.is_match(&rewritten) || NEXT_START_RE.is_match(&rewritten) {
                findings.push(make_finding(
                    path,
                    &[],
                    0,
                    "next/script-runtime",
                    format!(
                        "script {name:?}={rewritten:?} — préfixer avec `bunx --bun` pour forcer Bun comme runtime (sinon Node peut être utilisé selon PATH)"
                    ),
                    rewritten.clone(),
                    Some(format!("bunx --bun {}", rewritten.trim())),
                    MakeFindingOpts {
                        autofix: Some(false),
                        aggressive: Some(true),
                        severity: Some(Severity::Info),
                    ..Default::default()
                },
                ));
            }

            // next/build-turbo — suggérer `next build --turbopack` (Next 16 default)
            if NEXT_BUILD_RE.is_match(&rewritten) && !rewritten.contains("--turbopack") {
                findings.push(make_finding(
                    path,
                    &[],
                    0,
                    "next/build-turbopack",
                    format!(
                        "script {name:?}={rewritten:?} — Next 16 utilise Turbopack par défaut pour `dev` ; pour `build` ajouter `--turbopack` explicitement accélère le build (50-80%)"
                    ),
                    rewritten.clone(),
                    Some(format!("{} --turbopack", rewritten.trim_end())),
                    MakeFindingOpts {
                        autofix: Some(false),
                        aggressive: Some(true),
                        severity: Some(Severity::Info),
                    ..Default::default()
                },
                ));
            }

            // pkg/tsup-bun-external — détecte tsup sans --external bun
            // Ne flaggue que si le package contient effectivement un import de "bun".
            if TSUP_RE.is_match(&rewritten)
                && !rewritten.contains("--external bun")
                && package_uses_bun_import(path)
            {
                findings.push(make_finding(
                    path,
                    &[],
                    0,
                    "pkg/tsup-bun-external",
                    format!(
                        "script {name:?} utilise tsup et le code importe 'bun' — ajouter '--external bun' pour éviter l'échec d'esbuild au bundle-time"
                    ),
                    rewritten.clone(),
                    Some(format!("{} --external bun", rewritten.trim_end())),
                    MakeFindingOpts {
                        autofix: Some(false),
                        aggressive: Some(true),
                        severity: Some(Severity::Warn),
                    ..Default::default()
                },
                ));
            }
        }
    }

    // 2. packageManager
    if let Some(pm) = parsed.get("packageManager").and_then(|v| v.as_str()) {
        if !pm.starts_with("bun@") {
            findings.push(make_finding(
                path,
                &[],
                0,
                "pkg/package-manager",
                format!("packageManager='{pm}' — remplacer par 'bun@<version>' ou supprimer"),
                pm.to_string(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    ..Default::default()
                },
            ));
        }
    }

    // 3. engines.{npm,pnpm,yarn}
    if let Some(engines) = parsed.get("engines").and_then(|v| v.as_object()) {
        if engines.contains_key("npm")
            || engines.contains_key("pnpm")
            || engines.contains_key("yarn")
        {
            findings.push(make_finding(
                path,
                &[],
                0,
                "pkg/engines-pm",
                "engines.{npm,pnpm,yarn} est superflu avec Bun — utiliser 'engines.bun'",
                serde_json::to_string(engines).unwrap_or_default(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    ..Default::default()
                },
            ));
        }
    }

    // 4. Redundant deps + 4b. Ecosystem guides
    let dep_keys = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];
    let mut seen_ecosystem: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for key in dep_keys {
        if let Some(deps) = parsed.get(key).and_then(|v| v.as_object()) {
            for dep in deps.keys() {
                // Ecosystem guide mapping (info, pas de remplacement)
                for (name, suffix, url, label) in ECOSYSTEM_GUIDES {
                    if *name == dep.as_str() && seen_ecosystem.insert(*suffix) {
                        let rule_id = format!("ecosystem/{suffix}");
                        findings.push(make_finding(
                            path,
                            &[],
                            0,
                            &rule_id,
                            format!("{label} détecté — guide d'intégration Bun : {url}"),
                            (*name).to_string(),
                            Some((*url).to_string()),
                            MakeFindingOpts {
                                autofix: Some(false),
                                severity: Some(Severity::Info),
                                ..Default::default()
                            },
                        ));
                    }
                }
                if REDUNDANT_DEPS.contains(&dep.as_str()) {
                    findings.push(make_finding(
                        path,
                        &[],
                        0,
                        "pkg/redundant-dep",
                        format!("dépendance '{dep}' redondante avec Bun (voir Bun.file / Bun.env / fetch global / bun:sqlite / bun test)"),
                        dep.clone(),
                        None,
                        MakeFindingOpts {
                            autofix: Some(false),
                            aggressive: Some(true),
                            ..Default::default()
                        },
                    ));
                }
            }
        }
    }

    // 5. Root-only checks : pnpm-workspace.yaml à côté → workspace/root-missing
    //    + @types/bun manquant quand code utilise Bun.*
    let is_root_pkg = !path.contains('/');
    if is_root_pkg {
        let dir = std::path::Path::new(path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let pnpm_ws = dir.join("pnpm-workspace.yaml");
        if pnpm_ws.exists() && parsed.get("workspaces").is_none() {
            findings.push(make_finding(
                path,
                &[],
                0,
                "workspace/root-missing",
                "pnpm-workspace.yaml présent mais package.json racine n'a pas de champ \"workspaces\" — requis par Bun".to_string(),
                "workspaces".to_string(),
                Some(r#""workspaces": ["packages/*"]"#.to_string()),
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Warn),
                    ..Default::default()
                },
            ));
        }

        // onlyBuiltDependencies dans pnpm-workspace.yaml → trustedDependencies
        if pnpm_ws.exists() && parsed.get("trustedDependencies").is_none() {
            if let Ok(content) = std::fs::read_to_string(&pnpm_ws) {
                if let Some(info) = crate::pnpm_workspace::parse_pnpm_workspace(&content) {
                    if !info.only_built.is_empty() {
                        findings.push(make_finding(
                            path,
                            &[],
                            0,
                            "workspace/trusted-deps-missing",
                            format!(
                                "onlyBuiltDependencies de pnpm-workspace.yaml ({} pkg) non portées vers \"trustedDependencies\"",
                                info.only_built.len()
                            ),
                            "trustedDependencies".to_string(),
                            Some(format!(
                                r#""trustedDependencies": {}"#,
                                serde_json::to_string(&info.only_built).unwrap_or_default()
                            )),
                            MakeFindingOpts {
                                autofix: Some(false),
                                severity: Some(Severity::Warn),
                                ..Default::default()
                            },
                        ));
                    }
                }
            }
        }
    }

    // 6. "type":"module" + main .cjs
    if parsed.get("type").and_then(|v| v.as_str()) == Some("module") {
        if let Some(main) = parsed.get("main").and_then(|v| v.as_str()) {
            if main.ends_with(".cjs") {
                findings.push(make_finding(
                    path,
                    &[],
                    0,
                    "pkg/main-mismatch",
                    r#""type":"module" mais main pointe sur un fichier .cjs"#.to_string(),
                    main.to_string(),
                    None,
                    MakeFindingOpts {
                        autofix: Some(false),
                        ..Default::default()
                    },
                ));
            }
        }
    }

    let new_content = if mutated {
        let mut s = serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| content.to_string());
        if content.ends_with('\n') && !s.ends_with('\n') {
            s.push('\n');
        }
        s
    } else {
        content.to_string()
    };
    (findings, new_content)
}
