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

//! Scanner Cargo.toml : détecte les crates Rust de l'écosystème WASM/Bun
//! et émet des findings `ecosystem/*` avec liens vers leurs guides/docs.
//!
//! Pertinent quand Bun sert de runtime JS hôte pour un module Rust→WASM.

use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::make_finding;

/// (crate_name, rule_suffix, docs_url, label).
const RUST_ECOSYSTEM: &[(&str, &str, &str, &str)] = &[
    // Web frameworks Rust → WASM
    (
        "yew",
        "yew",
        "https://yew.rs/",
        "Yew (React-like, Rust → WASM)",
    ),
    (
        "leptos",
        "leptos",
        "https://leptos.dev/",
        "Leptos (fine-grained reactivity)",
    ),
    (
        "dioxus",
        "dioxus",
        "https://dioxuslabs.com/",
        "Dioxus (cross-platform GUI)",
    ),
    (
        "sycamore",
        "sycamore",
        "https://sycamore.dev/",
        "Sycamore (Solid-like)",
    ),
    (
        "seed",
        "seed",
        "https://seed-rs.org/",
        "Seed (Elm-like SPA)",
    ),
    // GPU / graphics
    (
        "wgpu",
        "wgpu",
        "https://wgpu.rs/",
        "wgpu (WebGPU, compute + render)",
    ),
    (
        "naga",
        "naga",
        "https://github.com/gfx-rs/wgpu/tree/trunk/naga",
        "naga (shader translator)",
    ),
    // WASM tooling
    (
        "wasm-bindgen",
        "wasm-bindgen",
        "https://rustwasm.github.io/wasm-bindgen/",
        "wasm-bindgen",
    ),
    (
        "js-sys",
        "js-sys",
        "https://rustwasm.github.io/wasm-bindgen/api/js_sys/",
        "js-sys",
    ),
    (
        "web-sys",
        "web-sys",
        "https://rustwasm.github.io/wasm-bindgen/api/web_sys/",
        "web-sys",
    ),
    (
        "wasm-bindgen-futures",
        "wasm-bindgen-futures",
        "https://rustwasm.github.io/wasm-bindgen/",
        "wasm-bindgen-futures",
    ),
    (
        "console_error_panic_hook",
        "panic-hook",
        "https://github.com/rustwasm/console_error_panic_hook",
        "panic hook → console",
    ),
    (
        "wee_alloc",
        "wee-alloc",
        "https://github.com/rustwasm/wee_alloc",
        "wee_alloc (small WASM allocator)",
    ),
    // Bun / napi bindings
    (
        "napi",
        "napi-rs",
        "https://napi.rs/",
        "napi-rs (Node/Bun addon)",
    ),
    (
        "napi-derive",
        "napi-rs",
        "https://napi.rs/",
        "napi-rs derive macros",
    ),
    (
        "bun-native-plugin",
        "bun-native-plugin",
        "https://docs.rs/bun-native-plugin",
        "bun-native-plugin-rs",
    ),
    // Serialization
    (
        "serde-wasm-bindgen",
        "serde-wasm",
        "https://github.com/RReverser/serde-wasm-bindgen",
        "serde → WASM",
    ),
    (
        "gloo",
        "gloo",
        "https://gloo-rs.web.app/",
        "gloo (toolkit Rust+WASM)",
    ),
    // App frameworks
    (
        "tauri",
        "tauri-rs",
        "https://tauri.app/",
        "Tauri (Rust + WebView)",
    ),
    (
        "bevy",
        "bevy",
        "https://bevyengine.org/",
        "Bevy (game engine, WASM-capable)",
    ),
    // MDX / build helpers
    (
        "mdxjs",
        "mdxjs-rs",
        "https://github.com/wooorm/mdxjs-rs",
        "mdxjs-rs",
    ),
    // Windows APIs
    (
        "windows",
        "windows-rs",
        "https://github.com/microsoft/windows-rs",
        "windows-rs (Win32 + WinRT)",
    ),
    (
        "windows-sys",
        "windows-rs",
        "https://github.com/microsoft/windows-rs",
        "windows-sys (raw bindings)",
    ),
    (
        "windows-targets",
        "windows-rs",
        "https://github.com/microsoft/windows-rs",
        "windows-targets",
    ),
    // libc / cross-platform
    (
        "libc",
        "libc",
        "https://github.com/rust-lang/libc",
        "libc (POSIX + Windows CRT)",
    ),
    (
        "nix",
        "nix-rs",
        "https://github.com/nix-rust/nix",
        "nix (POSIX APIs idiomatic)",
    ),
    // CSS
    (
        "lightningcss",
        "lightningcss",
        "https://lightningcss.dev/",
        "lightningcss (CSS bundler Rust)",
    ),
    // --- uutils : coreutils / findutils / diffutils / procps en Rust (cross-platform CLI) ---
    (
        "coreutils",
        "uutils",
        "https://github.com/uutils/coreutils",
        "uutils/coreutils (ls/cp/cat/… Rust, Windows-compat)",
    ),
    (
        "findutils",
        "uutils",
        "https://github.com/uutils/findutils",
        "uutils/findutils (find/xargs Rust)",
    ),
    (
        "diffutils",
        "uutils",
        "https://github.com/uutils/diffutils",
        "uutils/diffutils (diff/cmp Rust)",
    ),
    (
        "procps",
        "uutils",
        "https://github.com/uutils/procps",
        "uutils/procps (ps/top/watch Rust)",
    ),
    (
        "uu_ls",
        "uutils",
        "https://github.com/uutils/coreutils",
        "uu_ls (standalone ls crate)",
    ),
    (
        "uu_cat",
        "uutils",
        "https://github.com/uutils/coreutils",
        "uu_cat (standalone cat crate)",
    ),
    (
        "uu_cp",
        "uutils",
        "https://github.com/uutils/coreutils",
        "uu_cp (standalone cp crate)",
    ),
    (
        "util-linux",
        "util-linux-rs",
        "https://github.com/uutils/util-linux",
        "uutils/util-linux (mount/fdisk/lscpu/dmesg… Rust, Linux-only)",
    ),
    (
        "uu_mount",
        "util-linux-rs",
        "https://github.com/uutils/util-linux",
        "uu_mount",
    ),
    (
        "uu_lscpu",
        "util-linux-rs",
        "https://github.com/uutils/util-linux",
        "uu_lscpu",
    ),
    (
        "uu_dmesg",
        "util-linux-rs",
        "https://github.com/uutils/util-linux",
        "uu_dmesg",
    ),
    (
        "uu_fdisk",
        "util-linux-rs",
        "https://github.com/uutils/util-linux",
        "uu_fdisk",
    ),
    // --- GNU → Rust rewrites (alternatives aux binaires GNU historiques) ---
    (
        "ripgrep",
        "ripgrep",
        "https://github.com/BurntSushi/ripgrep",
        "ripgrep (grep successor)",
    ),
    (
        "fd-find",
        "fd-find",
        "https://github.com/sharkdp/fd",
        "fd (find successor)",
    ),
    (
        "bat",
        "bat",
        "https://github.com/sharkdp/bat",
        "bat (cat + syntax highlight)",
    ),
    (
        "tokei",
        "tokei",
        "https://github.com/XAMPPRocky/tokei",
        "tokei (cloc successor)",
    ),
    (
        "hyperfine",
        "hyperfine",
        "https://github.com/sharkdp/hyperfine",
        "hyperfine (benchmark, time)",
    ),
    (
        "du-dust",
        "du-dust",
        "https://github.com/bootandy/dust",
        "dust (du successor, interactive)",
    ),
    (
        "ouch",
        "ouch",
        "https://github.com/ouch-org/ouch",
        "ouch (zip/tar/… universal (de)compress)",
    ),
    (
        "zoxide",
        "zoxide",
        "https://github.com/ajeetdsouza/zoxide",
        "zoxide (cd successor, autojump)",
    ),
    (
        "eza",
        "eza",
        "https://github.com/eza-community/eza",
        "eza (ls successor, colors/tree)",
    ),
    (
        "sd",
        "sd",
        "https://github.com/chmln/sd",
        "sd (sed successor pour find/replace)",
    ),
    (
        "bottom",
        "bottom",
        "https://github.com/ClementTsang/bottom",
        "bottom (top/htop successor, btm)",
    ),
    (
        "delta",
        "delta",
        "https://github.com/dandavison/delta",
        "delta (diff viewer pour git)",
    ),
    (
        "just",
        "just",
        "https://github.com/casey/just",
        "just (make successor)",
    ),
    (
        "watchexec",
        "watchexec",
        "https://github.com/watchexec/watchexec",
        "watchexec (watch+exec files)",
    ),
    (
        "xh",
        "xh",
        "https://github.com/ducaale/xh",
        "xh (curl/httpie successor)",
    ),
    (
        "miniserve",
        "miniserve",
        "https://github.com/svenstaro/miniserve",
        "miniserve (HTTP serveur minimal)",
    ),
    (
        "duf",
        "duf",
        "https://github.com/muesli/duf",
        "duf (df successor) [Go, listé pour ref]",
    ),
    // --- OXC (Oxidation Compiler — JS/TS parser Rust, utilisé par Rolldown/Rspack/Vite) ---
    (
        "oxc",
        "oxc",
        "https://oxc.rs/",
        "oxc (JS/TS parser+linter+transformer Rust)",
    ),
    ("oxc_parser", "oxc", "https://oxc.rs/", "oxc_parser"),
    ("oxc_ast", "oxc", "https://oxc.rs/", "oxc_ast"),
    (
        "oxc_allocator",
        "oxc",
        "https://oxc.rs/",
        "oxc_allocator (arena)",
    ),
    ("oxc_semantic", "oxc", "https://oxc.rs/", "oxc_semantic"),
    ("oxc_span", "oxc", "https://oxc.rs/", "oxc_span"),
    (
        "oxc_resolver",
        "oxc",
        "https://oxc.rs/",
        "oxc_resolver (module resolution)",
    ),
    (
        "oxc_transformer",
        "oxc",
        "https://oxc.rs/",
        "oxc_transformer",
    ),
    ("oxc_codegen", "oxc", "https://oxc.rs/", "oxc_codegen"),
    ("oxc_minifier", "oxc", "https://oxc.rs/", "oxc_minifier"),
    // --- SWC stack (compiler JS/TS Rust) ---
    ("swc", "swc", "https://swc.rs/", "SWC (Rust compiler JS/TS)"),
    ("swc_core", "swc", "https://swc.rs/", "swc_core"),
    (
        "swc_common",
        "swc",
        "https://swc.rs/",
        "swc_common (spans, sourcemap)",
    ),
    (
        "swc_ecma_parser",
        "swc",
        "https://swc.rs/",
        "SWC ECMAScript parser",
    ),
    (
        "swc_ecma_ast",
        "swc",
        "https://swc.rs/",
        "SWC ECMAScript AST",
    ),
    (
        "swc_ecma_codegen",
        "swc",
        "https://swc.rs/",
        "SWC ECMAScript codegen",
    ),
    (
        "swc_ecma_visit",
        "swc",
        "https://swc.rs/",
        "SWC AST visitor",
    ),
    (
        "swc_ecma_utils",
        "swc",
        "https://swc.rs/",
        "SWC ECMAScript utilities",
    ),
    // --- TypeScript bindings generation ---
    (
        "ts-rs",
        "ts-rs",
        "https://github.com/Aleph-Alpha/ts-rs",
        "ts-rs (génère .ts types depuis Rust structs)",
    ),
    (
        "specta",
        "specta",
        "https://github.com/oscartbeaumont/specta",
        "Specta (TS type gen alternatif)",
    ),
    // --- Turbopack internals (repo vercel/next.js Rust) ---
    (
        "turbopack-core",
        "turbopack",
        "https://turbo.build/pack",
        "Turbopack core (Next.js bundler)",
    ),
    (
        "turbopack",
        "turbopack",
        "https://turbo.build/pack",
        "Turbopack",
    ),
    (
        "turbo-tasks",
        "turbo-tasks",
        "https://turbo.build/",
        "Turbo incremental computation engine",
    ),
    // --- Rust → React interop ---
    (
        "wasm-react",
        "wasm-react",
        "https://docs.rs/wasm-react",
        "wasm-react (composants React écrits en Rust→WASM)",
    ),
    // --- Other popular Rust libs useful with Bun ---
    (
        "serde",
        "serde",
        "https://serde.rs/",
        "Serde (ser/deserialize)",
    ),
    (
        "serde_json",
        "serde-json",
        "https://docs.rs/serde_json",
        "serde_json",
    ),
    (
        "tokio",
        "tokio",
        "https://tokio.rs/",
        "tokio (async runtime)",
    ),
    (
        "reqwest",
        "reqwest",
        "https://docs.rs/reqwest",
        "reqwest (HTTP client)",
    ),
    (
        "axum",
        "axum",
        "https://github.com/tokio-rs/axum",
        "axum (HTTP framework)",
    ),
    ("clap", "clap", "https://docs.rs/clap", "clap (CLI parser)"),
    (
        "anyhow",
        "anyhow",
        "https://docs.rs/anyhow",
        "anyhow (error handling)",
    ),
    (
        "thiserror",
        "thiserror",
        "https://docs.rs/thiserror",
        "thiserror (derive Error)",
    ),
    // --- Server web frameworks (voir flosse/rust-web-framework-comparison) ---
    (
        "actix-web",
        "actix-web",
        "https://actix.rs/",
        "Actix Web (server actor model)",
    ),
    (
        "rocket",
        "rocket",
        "https://rocket.rs/",
        "Rocket (server framework)",
    ),
    (
        "salvo",
        "salvo",
        "https://salvo.rs/",
        "Salvo (server, handler chain)",
    ),
    (
        "warp",
        "warp",
        "https://github.com/seanmonstar/warp",
        "Warp (filter-based server)",
    ),
    (
        "tide",
        "tide",
        "https://github.com/http-rs/tide",
        "Tide (server, async-std)",
    ),
    (
        "poem",
        "poem",
        "https://github.com/poem-web/poem",
        "Poem (server, fast)",
    ),
    (
        "gotham",
        "gotham",
        "https://gotham.rs/",
        "Gotham (server, type-safe router)",
    ),
    (
        "iron",
        "iron",
        "https://ironframework.io/",
        "Iron (server, legacy)",
    ),
    (
        "nickel",
        "nickel",
        "https://github.com/nickel-org/nickel.rs",
        "Nickel (server, Express-like)",
    ),
    (
        "cot",
        "cot",
        "https://cot.rs/",
        "cot (server, batteries-included)",
    ),
    (
        "pavex",
        "pavex",
        "https://pavex.dev/",
        "Pavex (server, compile-time DI)",
    ),
    // --- Frontend WASM GUI (au-delà de Yew/Leptos/Dioxus/Sycamore déjà listés) ---
    (
        "egui",
        "egui",
        "https://www.egui.rs/",
        "egui (immediate mode GUI → WASM)",
    ),
    (
        "iced",
        "iced",
        "https://iced.rs/",
        "Iced (Elm-inspired GUI → WASM)",
    ),
    (
        "silkenweb",
        "silkenweb",
        "https://github.com/silkenweb/silkenweb",
        "Silkenweb (fine-grained signals)",
    ),
    (
        "vizia",
        "vizia",
        "https://github.com/vizia/vizia",
        "Vizia (retained-mode GUI)",
    ),
    (
        "xilem",
        "xilem",
        "https://github.com/linebender/xilem",
        "Xilem (GUI experimental)",
    ),
    (
        "floem",
        "floem",
        "https://github.com/lapce/floem",
        "Floem (Leptos-like, native+WASM)",
    ),
    // --- Templating engines ---
    (
        "askama",
        "askama",
        "https://github.com/askama-rs/askama",
        "Askama (Jinja-like, compile-time)",
    ),
    (
        "handlebars",
        "handlebars",
        "https://docs.rs/handlebars",
        "Handlebars (runtime templating)",
    ),
    (
        "tera",
        "tera",
        "https://keats.github.io/tera/",
        "Tera (Jinja2/Django)",
    ),
    (
        "maud",
        "maud",
        "https://maud.lambda.xyz/",
        "Maud (HTML DSL macro)",
    ),
    (
        "sailfish",
        "sailfish",
        "https://sailfish.netlify.app/",
        "Sailfish (fast compile-time)",
    ),
    // --- WebSocket ---
    (
        "tokio-tungstenite",
        "tokio-tungstenite",
        "https://docs.rs/tokio-tungstenite",
        "tokio-tungstenite (WS async)",
    ),
    (
        "tungstenite",
        "tungstenite",
        "https://docs.rs/tungstenite",
        "tungstenite (WS blocking)",
    ),
    // --- HTTP clients ---
    (
        "hyper",
        "hyper",
        "https://hyper.rs/",
        "hyper (low-level HTTP)",
    ),
    (
        "ureq",
        "ureq",
        "https://github.com/algesten/ureq",
        "ureq (sync HTTP)",
    ),
    (
        "isahc",
        "isahc",
        "https://github.com/sagebind/isahc",
        "Isahc (libcurl wrapper)",
    ),
];

pub fn scan_cargo_toml(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Extrait le nom avant `=` ou `.`. Supporte `name = "..."`, `name.workspace = true`,
        // `name = { version = "..." }`, `[dependencies.name]`.
        let name = if let Some(rest) = trimmed.strip_prefix('[') {
            // [dependencies.yew] → yew
            let rest = rest.trim_end_matches(']');
            rest.split('.').nth(1).unwrap_or("").trim().to_string()
        } else if let Some(eq) = trimmed.find('=') {
            let k = trimmed[..eq].trim();
            // Strip optional `.workspace`, `.version`, `.features`, etc.
            k.split('.').next().unwrap_or("").trim().to_string()
        } else {
            continue;
        };

        if name.is_empty() {
            continue;
        }

        for (crate_name, suffix, url, label) in RUST_ECOSYSTEM {
            if *crate_name == name.as_str() && seen.insert(*suffix) {
                let rule_id = format!("ecosystem/{suffix}");
                findings.push(make_finding(
                    path,
                    &[],
                    0,
                    &rule_id,
                    format!("crate Rust `{crate_name}` détecté ({label}) — doc : {url}"),
                    (*crate_name).to_string(),
                    Some((*url).to_string()),
                    MakeFindingOpts {
                        autofix: Some(false),
                        severity: Some(Severity::Info),
                        ..Default::default()
                    },
                ));
            }
        }
    }

    (findings, content.to_string())
}

pub fn is_cargo_toml(name: &str) -> bool {
    name == "Cargo.toml"
}
