//! aphrody-translate — CLI : extrait, nettoie, traduit et réécrit les
//! commentaires source d'un projet en passant des règles déterministes.
//!
//! Exemples :
//!
//!     aphrody-translate --root . --dry-run
//!     aphrody-translate --root . --in-place --languages rust,ts
//!     aphrody-translate --root . --no-translate     # juste scrub + aphrodify

use anyhow::{Context, Result};
use aphrody_translate::{
    ai_patterns::{self, Action},
    aphrodify,
    extract::extract,
    translate::{default_cache_path, Translator},
    Lang,
};
use clap::Parser;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(name = "aphrody-translate", version, about, long_about = None)]
struct Args {
    /// Racine du projet à scanner.
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Langages à traiter, séparés par des virgules (rust,ts,js,py,go,c,cpp,sh,md,toml,all).
    #[arg(long, default_value = "all")]
    languages: String,

    /// Écrit les modifications en place (sinon dry-run par défaut).
    #[arg(short = 'i', long)]
    in_place: bool,

    /// Désactive l'appel réseau de traduction (utile pour scrub seul ou offline).
    #[arg(long)]
    no_translate: bool,

    /// E-mail noreply joint aux requêtes MyMemory (quota porté à 50 000 mots/jour).
    #[arg(long)]
    contact_email: Option<String>,

    /// Chemin du cache (par défaut : <root>/.aphrody-translate-cache.json).
    #[arg(long)]
    cache: Option<PathBuf>,

    /// Force le re-traitement même si le commentaire est déjà en français.
    #[arg(long)]
    force: bool,

    /// Niveau de log : trace, debug, info, warn, error.
    #[arg(long, default_value = "info")]
    log: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing(&args.log)?;

    let root = std::fs::canonicalize(&args.root).context("canonicalize root")?;
    let cache_path = args.cache.unwrap_or_else(|| default_cache_path(&root));
    let translator = Arc::new(Mutex::new(Translator::new(
        cache_path.clone(),
        args.contact_email.clone(),
    )?));

    let langs = parse_lang_filter(&args.languages);
    info!(?root, ?langs, in_place = args.in_place, "aphrody-translate start");

    let files = walk_project(&root, &langs);
    info!(count = files.len(), "files queued");

    let mut total_comments = 0usize;
    let mut total_changed = 0usize;

    for file in files {
        let original = match std::fs::read_to_string(&file) {
            Ok(s) => s,
            Err(e) => {
                warn!(?file, error = %e, "read failure, skipping");
                continue;
            }
        };
        let lang = Lang::from_path(&file);
        let comments = extract(&original, lang);
        if comments.is_empty() {
            continue;
        }
        total_comments += comments.len();

        let mut new_source = original.clone();
        for c in comments.iter().rev() {
            let action = ai_patterns::classify(&c.body);
            let body_clean = match action {
                Action::Drop => {
                    new_source.replace_range(c.start..c.end, "");
                    continue;
                }
                Action::Scrub => ai_patterns::scrub(&c.body),
                Action::Keep => c.body.clone(),
            };

            let translated = if args.no_translate || (!args.force && is_french(&body_clean)) {
                body_clean.clone()
            } else {
                let t = translator.lock().await.translate(&body_clean).await?;
                if t.trim().is_empty() {
                    body_clean.clone()
                } else {
                    t
                }
            };

            let final_body = aphrodify::aphrodify(&translated);
            if final_body.is_empty() {
                new_source.replace_range(c.start..c.end, "");
                continue;
            }
            let final_text = c.style.rewrap(&final_body);
            new_source.replace_range(c.start..c.end, &final_text);
        }

        if new_source == original {
            continue;
        }
        total_changed += 1;

        if args.in_place {
            std::fs::write(&file, &new_source).with_context(|| format!("write {:?}", file))?;
            info!(?file, "rewritten in place");
        } else {
            println!("--- DRY: {file:?} would be rewritten");
        }
    }

    translator.lock().await.persist().ok();

    info!(total_comments, total_changed, "aphrody-translate done");
    Ok(())
}

fn parse_lang_filter(s: &str) -> Vec<Lang> {
    let s = s.trim().to_lowercase();
    if s == "all" {
        return vec![
            Lang::Rust,
            Lang::TypeScript,
            Lang::JavaScript,
            Lang::Python,
            Lang::Go,
            Lang::C,
            Lang::Cpp,
            Lang::Shell,
            Lang::Toml,
            Lang::Markdown,
        ];
    }
    s.split(',')
        .map(|p| p.trim())
        .filter_map(|p| match p {
            "rust" | "rs" => Some(Lang::Rust),
            "ts" | "tsx" => Some(Lang::TypeScript),
            "js" | "jsx" => Some(Lang::JavaScript),
            "py" | "python" => Some(Lang::Python),
            "go" => Some(Lang::Go),
            "c" => Some(Lang::C),
            "cpp" | "c++" | "cxx" => Some(Lang::Cpp),
            "sh" | "shell" | "bash" => Some(Lang::Shell),
            "md" | "markdown" => Some(Lang::Markdown),
            "toml" => Some(Lang::Toml),
            _ => None,
        })
        .collect()
}

fn walk_project(root: &Path, accept_langs: &[Lang]) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .standard_filters(true)
        .git_ignore(true)
        .git_exclude(true)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.into_path())
        .filter(|p| {
            let l = Lang::from_path(p);
            l.supports_comments() && accept_langs.contains(&l)
        })
        .collect()
}

fn is_french(text: &str) -> bool {
    // Heuristique simple : la présence d'au moins un caractère accentué français
    // ou d'un mot court fréquent ("le ", "la ", "les ", "de ", "et ") suggère
    // que la phrase est déjà en français. Évite des allers-retours inutiles.
    let lower = text.to_lowercase();
    let accents = lower.chars().any(|c| matches!(c, 'é'|'è'|'ê'|'à'|'â'|'ô'|'î'|'û'|'ç'|'ù'));
    if accents {
        return true;
    }
    let hits = [" le ", " la ", " les ", " de ", " du ", " des ", " et ", " est "]
        .iter()
        .filter(|w| lower.contains(*w))
        .count();
    hits >= 2
}

fn init_tracing(level: &str) -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("aphrody_translate={level}")));
    fmt().with_env_filter(filter).with_target(false).init();
    Ok(())
}
