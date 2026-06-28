// SPDX-License-Identifier: Apache-2.0
// `aphrody memory index` / `aphrody memory recall` — auto-connaissance.
//
// aphrody doit connaître son propre code source « par cœur » (cf. docs/JARVIS.md,
// organe Mémoire). Ce module construit un index sémantique du dépôt : il parcourt
// les sources + docs, les découpe en fragments, les vectorise via le endpoint
// local `/v1/embeddings` (Ollama / aphrody serve — aucune dépendance ONNX dans le
// binaire), et les range dans un `SqliteBackend` vectoriel persistant. `recall`
// fait une recherche sémantique top-k sur cet index.
//
// Verify : `aphrody memory index --root crates/aphrody-serve` puis
//          `aphrody memory recall "comment le serveur relaie /v1/embeddings ?"`.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use miette::{IntoDiagnostic, WrapErr};
use serde_json::{Value, json};
use walkdir::WalkDir;

use aphrody_memory::{MemoryBackend, MemoryRecord, SqliteBackend};

use crate::context::{GoogleContext, TerminalCommand};

/// Endpoint d'embeddings local par défaut (Ollama natif / surface `aphrody serve`).
pub(crate) const DEFAULT_EMBED_BASE_URL: &str = "http://127.0.0.1:11434/v1";
/// Modèle d'embeddings par défaut — léger et dédié au retrieval (`ollama pull nomic-embed-text`).
pub(crate) const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";

/// Extensions de fichiers ingérées (sources + docs first-party).
const SOURCE_EXTS: &[&str] = &["rs", "md", "py", "toml", "ts", "tsx", "js", "jsx"];
/// Répertoires élagués pendant le parcours (artefacts, dépendances, VCS).
const SKIP_DIRS: &[&str] = &[
    "target", "node_modules", ".git", "dist", ".venv", "venv", "vendor", "build",
    ".turbo", "__pycache__", ".next", "coreutils", "util-linux",
];
/// Taille de fenêtre de découpe (lignes) et recouvrement entre fragments.
const CHUNK_LINES: usize = 60;
const CHUNK_OVERLAP: usize = 10;
/// Taille de lot pour les requêtes d'embeddings.
const EMBED_BATCH: usize = 32;
/// Fichiers plus gros que ceci (octets) sont ignorés (générés / lockfiles).
const MAX_FILE_BYTES: u64 = 1_000_000;

/// Un fragment de fichier source avec sa provenance (chemin + plage de lignes).
struct Chunk {
    path: String,
    start_line: usize,
    end_line: usize,
    text: String,
}

/// `aphrody memory index` — (re)construit l'index sémantique d'auto-connaissance.
pub(crate) struct IndexCommand {
    /// Racine à indexer (défaut : le répertoire courant).
    pub root: PathBuf,
    /// Chemin du store vectoriel (défaut : `~/.aphrody/self-knowledge.sqlite`).
    pub store: Option<PathBuf>,
    /// URL de base du endpoint d'embeddings (avec `/v1`).
    pub base_url: String,
    /// Modèle d'embeddings.
    pub model: String,
    /// Plafond optionnel du nombre de fichiers (runs rapides).
    pub max_files: Option<usize>,
}

/// `aphrody memory recall` — recherche sémantique dans l'index d'auto-connaissance.
pub(crate) struct RecallCommand {
    /// Requête en langage naturel.
    pub query: String,
    /// Chemin du store vectoriel (défaut : `~/.aphrody/self-knowledge.sqlite`).
    pub store: Option<PathBuf>,
    /// URL de base du endpoint d'embeddings (avec `/v1`).
    pub base_url: String,
    /// Modèle d'embeddings (doit être le même qu'à l'indexation).
    pub model: String,
    /// Nombre de fragments retournés.
    pub top_k: usize,
    /// Sortie JSON (parsable par jq).
    pub json: bool,
}

/// Store par défaut du code (auto-connaissance) : `$HOME/.aphrody/self-knowledge.sqlite`.
fn default_store_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".aphrody").join("self-knowledge.sqlite"))
        .unwrap_or_else(|| PathBuf::from("self-knowledge.sqlite"))
}

/// Store par défaut des leçons (erreurs / feedbacks) : `$HOME/.aphrody/lessons.sqlite`.
fn default_lessons_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".aphrody").join("lessons.sqlite"))
        .unwrap_or_else(|| PathBuf::from("lessons.sqlite"))
}

/// Tronque un fragment pour l'injection dans un prompt (lignes + caractères).
fn trim_snippet(content: &str) -> String {
    let mut out: String = content.lines().take(12).collect::<Vec<_>>().join("\n");
    if out.len() > 700 {
        out.truncate(700);
        out.push('…');
    }
    out
}

/// Installe le provider crypto `ring` (rustls 0.23, cf. CLAUDE.md §7).
fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Vectorise un lot de textes via `POST {base_url}/embeddings` (format OpenAI).
async fn embed_batch(
    http: &reqwest::Client,
    base_url: &str,
    model: &str,
    inputs: &[String],
) -> miette::Result<Vec<Vec<f32>>> {
    let url = format!("{}/embeddings", base_url.trim_end_matches('/'));
    let resp = http
        .post(&url)
        .json(&json!({ "model": model, "input": inputs }))
        .send()
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("requête d'embeddings vers {url} échouée"))?;

    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .into_diagnostic()
        .wrap_err("réponse d'embeddings illisible (JSON attendu)")?;
    if !status.is_success() {
        let msg = body
            .get("error")
            .map_or_else(|| body.to_string(), std::string::ToString::to_string);
        return Err(miette::miette!(
            "embeddings: HTTP {} — {msg}. Le modèle « {model} » est-il disponible ? \
             (essayez `ollama pull {model}`)",
            status.as_u16(),
        ));
    }

    let data = body
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| miette::miette!("embeddings: champ `data` absent de la réponse"))?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let emb = item
            .get("embedding")
            .and_then(Value::as_array)
            .ok_or_else(|| miette::miette!("embeddings: champ `embedding` absent"))?;
        out.push(
            emb.iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect::<Vec<f32>>(),
        );
    }
    if out.len() != inputs.len() {
        return Err(miette::miette!(
            "embeddings: {} vecteurs reçus pour {} entrées",
            out.len(),
            inputs.len()
        ));
    }
    Ok(out)
}

/// Découpe le contenu d'un fichier en fragments à fenêtre glissante de lignes.
fn chunk_file(rel_path: &str, content: &str) -> Vec<Chunk> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let step = CHUNK_LINES.saturating_sub(CHUNK_OVERLAP).max(1);
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < lines.len() {
        let end = (start + CHUNK_LINES).min(lines.len());
        let text = lines[start..end].join("\n");
        if !text.trim().is_empty() {
            chunks.push(Chunk {
                path: rel_path.to_string(),
                start_line: start + 1,
                end_line: end,
                text,
            });
        }
        if end == lines.len() {
            break;
        }
        start += step;
    }
    chunks
}

/// Vrai si une entrée de répertoire doit être élaguée du parcours.
fn is_skipped(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// Collecte tous les fragments indexables sous `root`.
fn collect_chunks(root: &Path, max_files: Option<usize>) -> miette::Result<Vec<Chunk>> {
    let mut chunks = Vec::new();
    let mut files = 0usize;
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // Élague les répertoires bruyants ; garde tout le reste.
            !(e.file_type().is_dir()
                && e.file_name()
                    .to_str()
                    .is_some_and(is_skipped))
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext_ok = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| SOURCE_EXTS.contains(&e));
        if !ext_ok {
            continue;
        }
        if entry.metadata().map(|m| m.len()).unwrap_or(0) > MAX_FILE_BYTES {
            continue;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue, // non-UTF-8 / illisible : ignoré.
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        chunks.extend(chunk_file(&rel, &content));
        files += 1;
        if max_files.is_some_and(|cap| files >= cap) {
            break;
        }
    }
    if chunks.is_empty() {
        return Err(miette::miette!(
            "aucun fichier indexable sous {} (extensions : {})",
            root.display(),
            SOURCE_EXTS.join(", ")
        ));
    }
    Ok(chunks)
}

/// Résout le chemin du store, en garantissant l'existence du répertoire parent.
fn resolve_store(store: Option<&Path>) -> miette::Result<PathBuf> {
    let path = store
        .map(Path::to_path_buf)
        .unwrap_or_else(default_store_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .into_diagnostic()
                .wrap_err_with(|| format!("création du répertoire {} impossible", parent.display()))?;
        }
    }
    Ok(path)
}

#[async_trait]
impl TerminalCommand for IndexCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        install_crypto_provider();
        let store = resolve_store(self.store.as_deref())?;

        // Reconstruction propre : on repart d'un store vierge.
        if store.exists() {
            std::fs::remove_file(&store)
                .into_diagnostic()
                .wrap_err_with(|| format!("suppression de l'ancien store {} impossible", store.display()))?;
        }

        println!(
            "aphrody memory index : lecture de {} …",
            self.root.display()
        );
        let chunks = collect_chunks(&self.root, self.max_files)?;
        println!(
            "  {} fragments à vectoriser (modèle « {} » @ {})",
            chunks.len(),
            self.model,
            self.base_url
        );

        let http = reqwest::Client::new();
        let mut backend = SqliteBackend::open(&store)
            .await
            .map_err(|e| miette::miette!("ouverture du store {}: {e}", store.display()))?;

        let mut stored = 0usize;
        for batch in chunks.chunks(EMBED_BATCH) {
            let inputs: Vec<String> = batch.iter().map(|c| c.text.clone()).collect();
            let vectors = embed_batch(&http, &self.base_url, &self.model, &inputs).await?;
            for (chunk, emb) in batch.iter().zip(vectors) {
                let mut rec = MemoryRecord::new("self", chunk.text.clone()).with_embedding(emb);
                rec.metadata.insert("path".into(), json!(chunk.path));
                rec.metadata
                    .insert("start_line".into(), json!(chunk.start_line));
                rec.metadata.insert("end_line".into(), json!(chunk.end_line));
                let key = format!("self/{}", rec.id);
                backend
                    .put(&key, rec)
                    .await
                    .map_err(|e| miette::miette!("écriture dans le store: {e}"))?;
                stored += 1;
            }
            if stored % 256 == 0 || stored == chunks.len() {
                println!("  … {stored}/{} fragments indexés", chunks.len());
            }
        }

        println!(
            "aphrody se connaît un peu mieux : {stored} fragments indexés dans {}",
            store.display()
        );
        Ok(())
    }
}

#[async_trait]
impl TerminalCommand for RecallCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        install_crypto_provider();
        let store = resolve_store(self.store.as_deref())?;
        if !store.exists() {
            return Err(miette::miette!(
                "index introuvable : {}. Lancez d'abord `aphrody memory index`.",
                store.display()
            ));
        }

        let http = reqwest::Client::new();
        let query_vec = embed_batch(
            &http,
            &self.base_url,
            &self.model,
            std::slice::from_ref(&self.query),
        )
        .await?
            .into_iter()
            .next()
            .ok_or_else(|| miette::miette!("la requête n'a produit aucun vecteur"))?;

        let backend = SqliteBackend::open(&store)
            .await
            .map_err(|e| miette::miette!("ouverture du store {}: {e}", store.display()))?;
        let hits = backend
            .search(&query_vec, self.top_k)
            .await
            .map_err(|e| miette::miette!("recherche: {e}"))?;

        if self.json {
            let arr: Vec<Value> = hits
                .iter()
                .map(|(rec, score)| {
                    json!({
                        "path": rec.metadata.get("path").cloned().unwrap_or(Value::Null),
                        "start_line": rec.metadata.get("start_line").cloned().unwrap_or(Value::Null),
                        "end_line": rec.metadata.get("end_line").cloned().unwrap_or(Value::Null),
                        "score": score,
                        "snippet": rec.content.lines().take(4).collect::<Vec<_>>().join("\n"),
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&arr).into_diagnostic()?
            );
            return Ok(());
        }

        if hits.is_empty() {
            println!("aucun fragment pertinent (index vide ou requête hors sujet).");
            return Ok(());
        }
        println!("aphrody se souvient ({} fragments) :", hits.len());
        for (rec, score) in &hits {
            let path = rec
                .metadata
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let start = rec
                .metadata
                .get("start_line")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let end = rec
                .metadata
                .get("end_line")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            println!("\n  [{score:.3}] {path}:{start}-{end}");
            for line in rec.content.lines().take(4) {
                println!("    │ {line}");
            }
        }
        Ok(())
    }
}

/// `aphrody memory remember` — persiste une leçon (erreur / feedback / note) dans
/// le store des leçons, vectorisée pour le rappel sémantique automatique.
pub(crate) struct RememberCommand {
    /// Nature de la leçon : `mistake`, `feedback` ou `note`.
    pub kind: String,
    /// Contenu de la leçon.
    pub text: String,
    /// Store des leçons (défaut : `~/.aphrody/lessons.sqlite`).
    pub store: Option<PathBuf>,
    /// URL de base du endpoint d'embeddings (avec `/v1`).
    pub base_url: String,
    /// Modèle d'embeddings.
    pub model: String,
}

#[async_trait]
impl TerminalCommand for RememberCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        if self.text.trim().is_empty() {
            return Err(miette::miette!("memory remember : le texte est vide"));
        }
        install_crypto_provider();
        let store = self
            .store
            .clone()
            .map_or_else(default_lessons_path, |p| p);
        if let Some(parent) = store.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).into_diagnostic()?;
            }
        }

        let http = reqwest::Client::new();
        let emb = embed_batch(
            &http,
            &self.base_url,
            &self.model,
            std::slice::from_ref(&self.text),
        )
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| miette::miette!("la leçon n'a produit aucun vecteur"))?;

        let mut backend = SqliteBackend::open(&store)
            .await
            .map_err(|e| miette::miette!("ouverture du store {}: {e}", store.display()))?;
        let mut rec = MemoryRecord::new("lesson", self.text.clone()).with_embedding(emb);
        rec.metadata.insert("kind".into(), json!(self.kind));
        let key = format!("lesson/{}", rec.id);
        backend
            .put(&key, rec)
            .await
            .map_err(|e| miette::miette!("écriture de la leçon: {e}"))?;

        println!(
            "aphrody retient cette leçon [{}] : « {} »",
            self.kind,
            self.text.lines().next().unwrap_or(&self.text)
        );
        Ok(())
    }
}

/// Rappel automatique avant un tour d'agent (« recall-before-think »).
///
/// Vectorise `query`, fouille **les deux** stores par défaut (code +
/// leçons), fusionne par score décroissant, et rend un bloc texte prêt à
/// injecter dans le system prompt. Best-effort : si aucun store n'existe rend
/// `Ok(None)` ; les erreurs (embeddings indisponibles) remontent pour que
/// l'appelant les ignore proprement sans casser le tour.
pub(crate) async fn recall_for_agent(
    query: &str,
    top_k: usize,
) -> miette::Result<Option<String>> {
    install_crypto_provider();
    let stores: Vec<(PathBuf, &str)> = [
        (default_store_path(), "code"),
        (default_lessons_path(), "leçon"),
    ]
    .into_iter()
    .filter(|(p, _)| p.exists())
    .collect();
    if stores.is_empty() {
        return Ok(None);
    }

    let http = reqwest::Client::new();
    let qv = embed_batch(
        &http,
        DEFAULT_EMBED_BASE_URL,
        DEFAULT_EMBED_MODEL,
        std::slice::from_ref(&query.to_string()),
    )
    .await?
    .into_iter()
    .next()
    .ok_or_else(|| miette::miette!("la requête n'a produit aucun vecteur"))?;

    let mut hits: Vec<(f32, String)> = Vec::new();
    for (store, origin) in &stores {
        let backend = SqliteBackend::open(store)
            .await
            .map_err(|e| miette::miette!("ouverture du store {}: {e}", store.display()))?;
        for (rec, score) in backend
            .search(&qv, top_k)
            .await
            .map_err(|e| miette::miette!("recherche: {e}"))?
        {
            let label = match *origin {
                "code" => {
                    let path = rec.metadata.get("path").and_then(Value::as_str).unwrap_or("?");
                    let start = rec.metadata.get("start_line").and_then(Value::as_u64).unwrap_or(0);
                    let end = rec.metadata.get("end_line").and_then(Value::as_u64).unwrap_or(0);
                    format!("code {path}:{start}-{end}")
                }
                _ => {
                    let kind = rec.metadata.get("kind").and_then(Value::as_str).unwrap_or("note");
                    format!("leçon [{kind}]")
                }
            };
            hits.push((score, format!("({label})\n{}", trim_snippet(&rec.content))));
        }
    }

    hits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(top_k);
    if hits.is_empty() {
        return Ok(None);
    }

    let body = hits
        .iter()
        .map(|(_, t)| format!("---\n{t}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(Some(format!(
        "Contexte rappelé de ta mémoire (ton propre code source et tes leçons \
         passées). Appuie-toi dessus si c'est pertinent, ignore-le sinon :\n\n{body}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_file_windows_with_overlap_and_provenance() {
        let content = (1..=130)
            .map(|n| format!("ligne {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_file("a.rs", &content);
        // 130 lignes, fenêtre 60, pas 50 → fenêtres à 1, 51, 101.
        assert_eq!(chunks.len(), 3);
        assert_eq!((chunks[0].start_line, chunks[0].end_line), (1, 60));
        assert_eq!((chunks[1].start_line, chunks[1].end_line), (51, 110));
        assert_eq!((chunks[2].start_line, chunks[2].end_line), (101, 130));
        assert!(chunks[0].text.starts_with("ligne 1\n"));
        assert_eq!(chunks[0].path, "a.rs");
    }

    #[test]
    fn chunk_file_skips_empty() {
        assert!(chunk_file("e.rs", "   \n\n  ").is_empty());
        assert!(chunk_file("e.rs", "").is_empty());
    }

    #[test]
    fn skip_dirs_prunes_artifacts() {
        assert!(is_skipped("target"));
        assert!(is_skipped("node_modules"));
        assert!(!is_skipped("crates"));
        assert!(!is_skipped("src"));
    }

    #[test]
    fn default_store_under_home() {
        let p = default_store_path();
        assert_eq!(
            p.file_name().and_then(|s| s.to_str()),
            Some("self-knowledge.sqlite")
        );
    }
}
