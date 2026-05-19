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

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use colored::Colorize;

// ─── Flavor ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum RustFlavor {
    Bin,
    Lib,
    Cdylib,
    ProcMacro,
    Workspace,
    Axum,
    Discord,
    Cli,
    Tauri,
    Leptos,
    Tui,
    Bevy,
    Grpc,
}

impl RustFlavor {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "bin" => Some(Self::Bin),
            "lib" => Some(Self::Lib),
            "cdylib" => Some(Self::Cdylib),
            "proc-macro" | "proc_macro" => Some(Self::ProcMacro),
            "workspace" => Some(Self::Workspace),
            "axum" => Some(Self::Axum),
            "discord" => Some(Self::Discord),
            "cli" => Some(Self::Cli),
            "tauri" => Some(Self::Tauri),
            "leptos" => Some(Self::Leptos),
            "tui" => Some(Self::Tui),
            "bevy" => Some(Self::Bevy),
            "grpc" => Some(Self::Grpc),
            _ => None,
        }
    }

    pub fn variants() -> &'static str {
        "bin, lib, cdylib, proc-macro, workspace, axum, discord, cli, tauri, leptos, tui, bevy, grpc"
    }
}

// ─── Command enum ────────────────────────────────────────────────────────────

pub enum RustCmd {
    New {
        name: String,
        flavor: RustFlavor,
        dir: Option<PathBuf>,
        force: bool,
    },
    Check {
        root: PathBuf,
    },
    Deps {
        root: PathBuf,
    },
    Doctor,
}

pub fn run(cmd: RustCmd, quiet: bool) -> Result<()> {
    match cmd {
        RustCmd::New {
            name,
            flavor,
            dir,
            force,
        } => run_new(name, flavor, dir, force, quiet),
        RustCmd::Check { root } => run_check(root, quiet),
        RustCmd::Deps { root } => run_deps(root, quiet),
        RustCmd::Doctor => run_doctor(quiet),
    }
}

// ─── new ─────────────────────────────────────────────────────────────────────

fn run_new(
    name: String,
    flavor: RustFlavor,
    dir: Option<PathBuf>,
    force: bool,
    quiet: bool,
) -> Result<()> {
    let base = dir.unwrap_or_else(|| PathBuf::from("."));
    let dest = base.join(&name);

    if dest.exists() {
        if force {
            fs::remove_dir_all(&dest).context("impossible de supprimer le dossier existant")?;
        } else {
            bail!(
                "le dossier '{}' existe déjà (--force pour écraser)",
                dest.display()
            );
        }
    }

    match flavor {
        RustFlavor::Bin => scaffold_bin(&dest, &name)?,
        RustFlavor::Lib => scaffold_lib(&dest, &name)?,
        RustFlavor::Cdylib => scaffold_cdylib(&dest, &name)?,
        RustFlavor::ProcMacro => scaffold_proc_macro(&dest, &name)?,
        RustFlavor::Workspace => scaffold_workspace(&dest, &name)?,
        RustFlavor::Axum => scaffold_axum(&dest, &name)?,
        RustFlavor::Discord => scaffold_discord(&dest, &name)?,
        RustFlavor::Cli => scaffold_cli(&dest, &name)?,
        RustFlavor::Tauri => scaffold_tauri(&dest, &name)?,
        RustFlavor::Leptos => scaffold_leptos(&dest, &name)?,
        RustFlavor::Tui => scaffold_tui(&dest, &name)?,
        RustFlavor::Bevy => scaffold_bevy(&dest, &name)?,
        RustFlavor::Grpc => scaffold_grpc(&dest, &name)?,
    }

    if !quiet {
        println!(
            "{} {} ({})",
            "créé".green().bold(),
            dest.display().to_string().cyan(),
            format!("{:?}", flavor).to_lowercase().dimmed()
        );
        println!("  {} cd {} && cargo build", "→".dimmed(), dest.display());
    }
    Ok(())
}

fn write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content).with_context(|| format!("écriture de {}", path.display()))
}

// ── bin ──────────────────────────────────────────────────────────────────────

fn scaffold_bin(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
anyhow = "1"
clap = {{ version = "4", features = ["derive"] }}
colored = "2"

[profile.release]
lto = "fat"
codegen-units = 1
strip = "symbols"
"#
        ),
    )?;
    write(
        &dest.join("src/main.rs"),
        &format!(
            r#"use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "{name}", version, about)]
struct Cli {{
    /// Entrée à traiter.
    input: Option<String>,
}}

fn main() -> Result<()> {{
    let cli = Cli::parse();
    println!("{{}}", cli.input.unwrap_or_else(|| "Hello from {name}!".into()));
    Ok(())
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    Ok(())
}

// ── lib ──────────────────────────────────────────────────────────────────────

fn scaffold_lib(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
name = "{}"
path = "src/lib.rs"

[dependencies]
anyhow = "1"
serde = {{ version = "1", features = ["derive"] }}
"#,
            name.replace('-', "_")
        ),
    )?;
    write(
        &dest.join("src/lib.rs"),
        &format!(
            r#"//! {name} — bibliothèque Rust.

/// Retourne la version du crate.
pub fn version() -> &'static str {{
    env!("CARGO_PKG_VERSION")
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn version_is_semver() {{
        let v = version();
        assert!(v.split('.').count() >= 2);
    }}
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    Ok(())
}

// ── cdylib ───────────────────────────────────────────────────────────────────

fn scaffold_cdylib(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
name = "{}"
crate-type = ["cdylib"]

[dependencies]

[profile.release]
lto = "fat"
codegen-units = 1
strip = "symbols"
opt-level = 3
"#,
            name.replace('-', "_")
        ),
    )?;
    write(
        &dest.join("src/lib.rs"),
        r#"// Exports FFI-safe pour bun:ffi, ctypes, etc.
// Toute valeur traversant la frontière C doit être un type C primitif ou un pointeur.

#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Retourne la version ABI de cette bibliothèque.
#[no_mangle]
pub extern "C" fn abi_version() -> u32 {
    1
}
"#,
    )?;
    write(
        &dest.join("bindings.ts"),
        &format!(
            r#"import {{ dlopen, FFIType, suffix }} from "bun:ffi";

const lib = dlopen(`./target/release/lib{}.${{suffix}}`, {{
  add: {{ args: [FFIType.i32, FFIType.i32], returns: FFIType.i32 }},
  abi_version: {{ args: [], returns: FFIType.u32 }},
}});

console.log("add(2, 3) =", lib.symbols.add(2, 3));
console.log("abi_version =", lib.symbols.abi_version());
"#,
            name.replace('-', "_")
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    Ok(())
}

// ── proc-macro ───────────────────────────────────────────────────────────────

fn scaffold_proc_macro(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
syn = {{ version = "2", features = ["full"] }}
quote = "1"
proc-macro2 = "1"
"#
        ),
    )?;
    let fn_name = name.replace('-', "_");
    let derive_name = name;
    write(
        &dest.join("src/lib.rs"),
        &format!(
            r#"use proc_macro::TokenStream;
use quote::quote;
use syn::{{parse_macro_input, DeriveInput}};

/// Derive macro {derive_name} : imprime le nom de la struct au runtime.
#[proc_macro_derive({derive_name})]
pub fn {fn_name}_derive(input: TokenStream) -> TokenStream {{
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let expanded = quote! {{
        impl #name {{
            pub fn type_name() -> &'static str {{
                stringify!(#name)
            }}
        }}
    }};
    TokenStream::from(expanded)
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    Ok(())
}

// ── workspace ────────────────────────────────────────────────────────────────

fn scaffold_workspace(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        r#"[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = ""

[workspace.dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
lto = "fat"
codegen-units = 1
strip = "symbols"
"#,
    )?;
    let core_name = format!("{name}-core");
    write(
        &dest.join(format!("crates/{core_name}/Cargo.toml")),
        &format!(
            r#"[package]
name = "{core_name}"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
anyhow = {{ workspace = true }}
serde = {{ workspace = true }}
"#
        ),
    )?;
    write(
        &dest.join(format!("crates/{core_name}/src/lib.rs")),
        &format!(
            r#"//! {core_name} — logique métier du workspace {name}.

pub fn version() -> &'static str {{
    env!("CARGO_PKG_VERSION")
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    write(
        &dest.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"stable\"\n",
    )?;
    Ok(())
}

// ── axum ─────────────────────────────────────────────────────────────────────

fn scaffold_axum(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
axum = {{ version = "0.8", features = ["macros"] }}
tokio = {{ version = "1", features = ["full"] }}
tower-http = {{ version = "0.6", features = ["trace", "cors", "compression-gzip"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}

[profile.release]
lto = "fat"
codegen-units = 1
strip = "symbols"
"#
        ),
    )?;
    write(
        &dest.join("src/main.rs"),
        &format!(
            r#"use axum::{{routing::get, Json, Router}};
use serde_json::{{json, Value}};
use tower_http::{{cors::CorsLayer, trace::TraceLayer}};
use tracing_subscriber::{{layer::SubscriberExt, util::SubscriberInitExt}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("écoute sur http://{{}}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}}

async fn root() -> &'static str {{
    "{name}"
}}

async fn health() -> Json<Value> {{
    Json(json!({{ "status": "ok", "service": "{name}" }}))
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n.env\n")?;
    write(
        &dest.join(".env.example"),
        "RUST_LOG=info\nADDR=0.0.0.0:3000\n",
    )?;
    Ok(())
}

// ── discord ───────────────────────────────────────────────────────────────────

fn scaffold_discord(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
serenity = {{ version = "0.12", default-features = false, features = [
    "client", "gateway", "rustls_backend", "model"
] }}
poise = "0.6"
tokio = {{ version = "1", features = ["full"] }}
anyhow = "1"
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
"#
        ),
    )?;
    write(
        &dest.join("src/main.rs"),
        r#"use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {}

/// Répond "Pong !" à la commande /ping.
#[poise::command(slash_command, prefix_command)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong !").await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        )
        .init();

    let token = std::env::var("DISCORD_TOKEN")
        .expect("DISCORD_TOKEN non défini dans l'environnement");

    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ping()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                tracing::info!("bot connecté");
                Ok(Data {})
            })
        })
        .build();

    serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?
        .start()
        .await?;

    Ok(())
}
"#,
    )?;
    write(&dest.join(".gitignore"), "/target\n.env\n")?;
    write(
        &dest.join(".env.example"),
        "DISCORD_TOKEN=your_token_here\nRUST_LOG=info\n",
    )?;
    Ok(())
}

// ── cli (full, avec subcommandes) ────────────────────────────────────────────

fn scaffold_cli(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
anyhow = "1"
clap = {{ version = "4", features = ["derive", "color"] }}
colored = "2"
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}

[profile.release]
lto = "fat"
codegen-units = 1
strip = "symbols"
panic = "abort"
"#
        ),
    )?;
    write(
        &dest.join("src/main.rs"),
        &format!(
            r#"use anyhow::Result;
use clap::{{Parser, Subcommand}};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "{name}", version, about = "{name} — description courte", long_about = None)]
struct Cli {{
    #[command(subcommand)]
    cmd: Cmd,

    /// Active les logs de debug.
    #[arg(long, global = true)]
    verbose: bool,
}}

#[derive(Subcommand)]
enum Cmd {{
    /// Affiche des informations sur l'environnement.
    Info,
    /// Lance un traitement sur un fichier.
    Run {{
        /// Chemin du fichier à traiter.
        path: std::path::PathBuf,
        /// Mode silencieux.
        #[arg(long, short)]
        quiet: bool,
    }},
}}

fn main() -> Result<()> {{
    let cli = Cli::parse();

    if cli.verbose {{
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .init();
    }}

    match cli.cmd {{
        Cmd::Info => {{
            println!("{{}} {{}}", "version:".bold(), env!("CARGO_PKG_VERSION"));
            println!("{{}} {{}}", "os:".bold(), std::env::consts::OS);
        }}
        Cmd::Run {{ path, quiet }} => {{
            if !path.exists() {{
                anyhow::bail!("fichier introuvable : {{}}", path.display());
            }}
            if !quiet {{
                println!("{{}} {{}}", "traitement:".green().bold(), path.display());
            }}
        }}
    }}

    Ok(())
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    Ok(())
}

// ── tauri ────────────────────────────────────────────────────────────────────

fn scaffold_tauri(dest: &Path, name: &str) -> Result<()> {
    // Frontend stub (HTML/TS via Bun)
    write(
        &dest.join("package.json"),
        &format!(
            r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "scripts": {{
    "dev": "tauri dev",
    "build": "tauri build"
  }},
  "dependencies": {{
    "@tauri-apps/api": "^2"
  }},
  "devDependencies": {{
    "@tauri-apps/cli": "^2"
  }}
}}
"#
        ),
    )?;
    write(
        &dest.join("index.html"),
        r#"<!doctype html>
<html lang="fr">
  <head><meta charset="UTF-8" /><title>App</title></head>
  <body>
    <h1>Tauri 2</h1>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
"#,
    )?;
    write(
        &dest.join("src/main.ts"),
        r#"import { invoke } from "@tauri-apps/api/core";

invoke<string>("greet", { name: "monde" }).then(console.log);
"#,
    )?;
    // Rust backend
    write(
        &dest.join("src-tauri/Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "{name}_core"
crate-type = ["staticlib", "cdylib"]

[dependencies]
tauri = {{ version = "2", features = ["protocol-asset"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
"#
        ),
    )?;
    write(
        &dest.join("src-tauri/tauri.conf.json"),
        &format!(
            r#"{{
  "productName": "{name}",
  "version": "0.1.0",
  "identifier": "com.example.{name}",
  "build": {{
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420"
  }},
  "app": {{
    "windows": [{{ "title": "{name}", "width": 1024, "height": 768 }}]
  }}
}}
"#
        ),
    )?;
    write(
        &dest.join("src-tauri/src/lib.rs"),
        r#"#[tauri::command]
fn greet(name: &str) -> String {
    format!("Bonjour, {name}!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("erreur lors du démarrage de l'application");
}
"#,
    )?;
    write(&dest.join(".gitignore"), "/target\n/dist\nnode_modules\n")?;
    Ok(())
}

// ── leptos ───────────────────────────────────────────────────────────────────

fn scaffold_leptos(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = {{ version = "0.7", features = ["ssr"] }}
leptos_axum = "0.7"
axum = "0.8"
tokio = {{ version = "1", features = ["rt-multi-thread", "macros"] }}
tower = "0.5"
tower-http = {{ version = "0.6", features = ["fs"] }}
serde = {{ version = "1", features = ["derive"] }}

[features]
default = ["ssr"]
ssr = []
hydrate = ["leptos/hydrate"]

[profile.release]
lto = "fat"
codegen-units = 1
"#
        ),
    )?;
    write(
        &dest.join(".cargo/config.toml"),
        r#"[build]
target = "wasm32-unknown-unknown"

[target.wasm32-unknown-unknown]
runner = "wasm-bindgen-test-runner"
"#,
    )?;
    write(
        &dest.join("src/main.rs"),
        &format!(
            r#"use leptos::prelude::*;
use leptos_axum::{{generate_route_list, LeptosRoutes}};
use axum::Router;

#[component]
fn App() -> impl IntoView {{
    let (count, set_count) = signal(0i32);
    view! {{
        <main>
            <h1>"{name}"</h1>
            <button on:click=move |_| set_count.update(|n| *n += 1)>
                "Clics : " {{count}}
            </button>
        </main>
    }}
}}

#[tokio::main]
async fn main() {{
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&conf.leptos_options, routes, App)
        .fallback(leptos_axum::file_and_error_handler(|| {{}}));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("écoute sur http://{{addr}}");
    axum::serve(listener, app).await.unwrap();
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n/pkg\n")?;
    Ok(())
}

// ── tui (ratatui) ────────────────────────────────────────────────────────────

fn scaffold_tui(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
anyhow = "1"
ratatui = "0.29"
crossterm = {{ version = "0.28", features = ["event-stream"] }}
tokio = {{ version = "1", features = ["rt-multi-thread", "macros", "time"] }}
tokio-stream = "0.1"

[profile.release]
lto = "fat"
codegen-units = 1
strip = "symbols"
"#
        ),
    )?;
    write(
        &dest.join("src/main.rs"),
        &format!(
            r#"use anyhow::Result;
use crossterm::{{
    event::{{self, Event, KeyCode, KeyEventKind}},
    terminal::{{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}},
    ExecutableCommand,
}};
use ratatui::{{
    prelude::*,
    widgets::{{Block, Borders, Paragraph}},
}};
use std::io::stdout;

struct App {{
    counter: i64,
    quit: bool,
}}

impl App {{
    fn new() -> Self {{
        Self {{ counter: 0, quit: false }}
    }}

    fn on_key(&mut self, key: KeyCode) {{
        match key {{
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.counter += 1,
            KeyCode::Down | KeyCode::Char('j') => self.counter -= 1,
            _ => {{}}
        }}
    }}
}}

fn ui(frame: &mut Frame, app: &App) {{
    let area = frame.area();
    let block = Block::default()
        .title(" {name} ")
        .borders(Borders::ALL);
    let text = format!("compteur : {{}}\n\n↑/k +1  ↓/j -1  q quitte", app.counter);
    frame.render_widget(Paragraph::new(text).block(block), area);
}}

fn main() -> Result<()> {{
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = App::new();

    while !app.quit {{
        terminal.draw(|f| ui(f, &app))?;
        if event::poll(std::time::Duration::from_millis(16))? {{
            if let Event::Key(key) = event::read()? {{
                if key.kind == KeyEventKind::Press {{
                    app.on_key(key.code);
                }}
            }}
        }}
    }}

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    Ok(())
}

// ── bevy ─────────────────────────────────────────────────────────────────────

fn scaffold_bevy(dest: &Path, name: &str) -> Result<()> {
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
bevy = {{ version = "0.16", features = ["default"] }}

# Accélère les builds en dev
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
"#
        ),
    )?;
    write(
        &dest.join("src/main.rs"),
        &format!(
            r#"use bevy::prelude::*;

fn main() {{
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {{
            primary_window: Some(Window {{
                title: "{name}".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }}),
            ..default()
        }}))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .run();
}}

#[derive(Component)]
struct Spinning;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {{
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.6, 0.9))),
        Spinning,
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}}

fn rotate(time: Res<Time>, mut query: Query<&mut Transform, With<Spinning>>) {{
    for mut transform in &mut query {{
        transform.rotate_y(time.delta_secs());
    }}
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    Ok(())
}

// ── grpc (tonic) ─────────────────────────────────────────────────────────────

fn scaffold_grpc(dest: &Path, name: &str) -> Result<()> {
    let snake = name.replace('-', "_");
    write(
        &dest.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}-server"
path = "src/server.rs"

[[bin]]
name = "{name}-client"
path = "src/client.rs"

[dependencies]
tonic = "0.13"
prost = "0.13"
tokio = {{ version = "1", features = ["rt-multi-thread", "macros"] }}
anyhow = "1"

[build-dependencies]
tonic-build = "0.13"
"#
        ),
    )?;
    write(
        &dest.join("build.rs"),
        r#"fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/service.proto")?;
    Ok(())
}
"#,
    )?;
    write(
        &dest.join("proto/service.proto"),
        &format!(
            r#"syntax = "proto3";

package {snake};

service {name}Service {{
  rpc Ping (PingRequest) returns (PingResponse);
}}

message PingRequest {{
  string message = 1;
}}

message PingResponse {{
  string reply = 1;
}}
"#
        ),
    )?;
    write(
        &dest.join("src/server.rs"),
        &format!(
            r#"use tonic::{{transport::Server, Request, Response, Status}};

pub mod proto {{
    tonic::include_proto!("{snake}");
}}

use proto::{{
    {snake}_service_server::{{self as svc, {name}ServiceServer}},
    PingRequest, PingResponse,
}};

#[derive(Default)]
struct Service;

#[tonic::async_trait]
impl svc::{name}Service for Service {{
    async fn ping(
        &self,
        req: Request<PingRequest>,
    ) -> Result<Response<PingResponse>, Status> {{
        let reply = PingResponse {{
            reply: format!("pong : {{}}", req.into_inner().message),
        }};
        Ok(Response::new(reply))
    }}
}}

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    let addr = "[::1]:50051".parse()?;
    println!("serveur gRPC sur {{addr}}");
    Server::builder()
        .add_service({name}ServiceServer::new(Service::default()))
        .serve(addr)
        .await?;
    Ok(())
}}
"#
        ),
    )?;
    write(
        &dest.join("src/client.rs"),
        &format!(
            r#"use proto::{snake}_service_client::{name}ServiceClient;
use proto::PingRequest;

pub mod proto {{
    tonic::include_proto!("{snake}");
}}

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    let mut client = {name}ServiceClient::connect("http://[::1]:50051").await?;
    let resp = client
        .ping(PingRequest {{ message: "bonjour".into() }})
        .await?;
    println!("réponse : {{}}", resp.into_inner().reply);
    Ok(())
}}
"#
        ),
    )?;
    write(&dest.join(".gitignore"), "/target\n")?;
    Ok(())
}

// ─── check ───────────────────────────────────────────────────────────────────

fn run_check(root: PathBuf, quiet: bool) -> Result<()> {
    let root = root.canonicalize().context("chemin introuvable")?;

    if !quiet {
        println!("{} cargo check…", "→".dimmed());
    }
    let check = Command::new("cargo")
        .args(["check", "--color", "always"])
        .current_dir(&root)
        .status()
        .context("impossible de lancer cargo check")?;

    if !check.success() {
        bail!("cargo check a échoué");
    }

    if !quiet {
        println!("{} cargo clippy…", "→".dimmed());
    }
    let clippy = Command::new("cargo")
        .args([
            "clippy",
            "--color",
            "always",
            "--",
            "-W",
            "clippy::all",
            "-W",
            "clippy::pedantic",
            "-A",
            "clippy::module_name_repetitions",
        ])
        .current_dir(&root)
        .status()
        .context("impossible de lancer cargo clippy")?;

    if !clippy.success() {
        bail!("cargo clippy a signalé des warnings (traités comme erreurs)");
    }

    if !quiet {
        println!("{} check OK", "✓".green().bold());
    }
    Ok(())
}

// ─── deps ────────────────────────────────────────────────────────────────────

fn run_deps(root: PathBuf, quiet: bool) -> Result<()> {
    let root = root.canonicalize().context("chemin introuvable")?;
    let mut any = false;

    // cargo outdated
    if tool_exists("cargo-outdated") {
        if !quiet {
            println!("{}", "── cargo outdated ──────────────────────".dimmed());
        }
        Command::new("cargo")
            .args(["outdated", "--color", "always"])
            .current_dir(&root)
            .status()
            .context("cargo outdated a échoué")?;
        any = true;
    } else if !quiet {
        println!(
            "{} cargo-outdated non trouvé — installe avec {}",
            "!".yellow(),
            "cargo install cargo-outdated".cyan()
        );
    }

    // cargo audit
    if tool_exists("cargo-audit") {
        if !quiet {
            println!("{}", "── cargo audit ─────────────────────────".dimmed());
        }
        Command::new("cargo")
            .args(["audit", "--color", "always"])
            .current_dir(&root)
            .status()
            .context("cargo audit a échoué")?;
        any = true;
    } else if !quiet {
        println!(
            "{} cargo-audit non trouvé — installe avec {}",
            "!".yellow(),
            "cargo install cargo-audit".cyan()
        );
    }

    if !any {
        bail!("aucun outil de deps disponible (cargo-outdated et cargo-audit manquants)");
    }
    Ok(())
}

// ─── doctor ──────────────────────────────────────────────────────────────────

struct Tool {
    bin: &'static str,
    cmd: &'static [&'static str],
    install: &'static str,
    required: bool,
}

static TOOLS: &[Tool] = &[
    Tool {
        bin: "rustc",
        cmd: &["rustc", "--version"],
        install: "rustup",
        required: true,
    },
    Tool {
        bin: "cargo",
        cmd: &["cargo", "--version"],
        install: "rustup",
        required: true,
    },
    Tool {
        bin: "rustup",
        cmd: &["rustup", "--version"],
        install: "https://rustup.rs",
        required: true,
    },
    Tool {
        bin: "rustfmt",
        cmd: &["rustfmt", "--version"],
        install: "rustup component add rustfmt",
        required: false,
    },
    Tool {
        bin: "clippy",
        cmd: &["cargo", "clippy", "--version"],
        install: "rustup component add clippy",
        required: false,
    },
    Tool {
        bin: "cargo-audit",
        cmd: &["cargo", "audit", "--version"],
        install: "cargo install cargo-audit",
        required: false,
    },
    Tool {
        bin: "cargo-outdated",
        cmd: &["cargo", "outdated", "--version"],
        install: "cargo install cargo-outdated",
        required: false,
    },
    Tool {
        bin: "cargo-watch",
        cmd: &["cargo", "watch", "--version"],
        install: "cargo install cargo-watch",
        required: false,
    },
    Tool {
        bin: "cargo-expand",
        cmd: &["cargo", "expand", "--version"],
        install: "cargo install cargo-expand",
        required: false,
    },
    Tool {
        bin: "cargo-typify",
        cmd: &["cargo", "typify", "--version"],
        install: "cargo install cargo-typify",
        required: false,
    },
    Tool {
        bin: "wasm-pack",
        cmd: &["wasm-pack", "--version"],
        install: "cargo install wasm-pack",
        required: false,
    },
    Tool {
        bin: "just",
        cmd: &["just", "--version"],
        install: "cargo install just",
        required: false,
    },
    Tool {
        bin: "trunk",
        cmd: &["trunk", "--version"],
        install: "cargo install trunk",
        required: false,
    },
    Tool {
        bin: "cargo-leptos",
        cmd: &["cargo", "leptos", "--version"],
        install: "cargo install cargo-leptos",
        required: false,
    },
    Tool {
        bin: "tauri-cli",
        cmd: &["cargo", "tauri", "--version"],
        install: "cargo install tauri-cli",
        required: false,
    },
];

fn run_doctor(quiet: bool) -> Result<()> {
    let mut missing_required = false;

    for tool in TOOLS {
        let ok = Command::new(tool.cmd[0])
            .args(&tool.cmd[1..])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !quiet {
            if ok {
                // get version string
                let ver = Command::new(tool.cmd[0])
                    .args(&tool.cmd[1..])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_default();
                let ver = ver.lines().next().unwrap_or("").trim().to_string();
                println!("  {} {:<18} {}", "✓".green().bold(), tool.bin, ver.dimmed());
            } else if tool.required {
                println!(
                    "  {} {:<18} {}",
                    "✗".red().bold(),
                    tool.bin.red(),
                    format!("REQUIS — installe : {}", tool.install).yellow()
                );
                missing_required = true;
            } else {
                println!(
                    "  {} {:<18} {}",
                    "·".dimmed(),
                    tool.bin.dimmed(),
                    format!("optionnel — cargo install {}", tool.bin).dimmed()
                );
            }
        }
    }

    if missing_required {
        bail!("outils requis manquants");
    }
    Ok(())
}

fn tool_exists(name: &str) -> bool {
    Command::new("cargo")
        .args([name.strip_prefix("cargo-").unwrap_or(name), "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
