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

// Commande `analyze` : scan + audit + crosslink ML par similarité cosinus.
//
// Pipeline par repo :
//   1. `run::run` → findings déterministes (règles regex)
//   2. `audit::run_audit` → issues/PRs GitHub mentionnant bun/node
//   3. fastembed (AllMiniLML6V2, 384 dims, ONNX) embed les messages de
//      findings uniques + les titres/bodies d'issues
//   4. cosine similarité → top-K issues pertinentes pour chaque finding

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use n2b_core::audit::{self, Hit, ItemState};
use n2b_core::run;
use n2b_core::types::{Mode, Report, RunOptions, Severity};

/// Alias pour l'embedder ML. Sous feature `ai`, c'est le vrai
/// `fastembed::TextEmbedding`. Sans, c'est un type non-instanciable →
/// toutes les fonctions qui en prennent un sont des no-ops.
#[cfg(feature = "ai")]
type Embedder = fastembed::TextEmbedding;

#[cfg(not(feature = "ai"))]
pub enum Embedder {}

#[derive(Debug, Serialize)]
pub struct AnalysisReport {
    pub generated_at: String,
    pub model: String,
    pub repos: Vec<RepoAnalysis>,
}

#[derive(Debug, Serialize)]
pub struct RepoAnalysis {
    pub path: String,
    pub repo: Option<RepoInfo>,
    pub summary: Summary,
    pub findings_by_rule: BTreeMap<String, usize>,
    pub issues: Vec<IssueWithMatches>,
    pub prs: Vec<IssueWithMatches>,
    pub top_findings: Vec<FindingWithIssues>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RepoInfo {
    pub owner: String,
    pub name: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_of: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct Summary {
    pub files_scanned: usize,
    pub files_with_findings: usize,
    pub findings_total: usize,
    pub errors: usize,
    pub warns: usize,
    pub infos: usize,
    pub issues: usize,
    pub prs: usize,
}

#[derive(Debug, Serialize)]
pub struct IssueWithMatches {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub url: String,
    pub matched_terms: Vec<String>,
    pub related_rules: Vec<RuleMatch>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RuleMatch {
    pub rule_id: String,
    pub similarity: f32,
}

#[derive(Debug, Serialize)]
pub struct FindingWithIssues {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub rule_id: String,
    pub message: String,
    pub replacement: Option<String>,
    pub related_issues: Vec<RelatedIssue>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RelatedIssue {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub is_pr: bool,
    pub similarity: f32,
}

pub struct AnalyzeOpts {
    pub paths: Vec<PathBuf>,
    pub issue_limit: usize,
    #[cfg_attr(not(feature = "ai"), allow(dead_code))]
    pub top_k: usize,
    #[cfg_attr(not(feature = "ai"), allow(dead_code))]
    pub threshold: f32,
    pub report: Report,
    pub apply: Option<Mode>,
    pub ignore: Vec<String>,
}

pub fn run_analyze(opts: AnalyzeOpts) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // Charge le modèle une seule fois, partagé entre les repos.
    let mut embedder = try_init_embedder();

    let mut repos: Vec<RepoAnalysis> = Vec::with_capacity(opts.paths.len());
    for path in &opts.paths {
        let path = path.canonicalize().unwrap_or(path.clone());
        eprintln!("{} {}", "analyzing".cyan().bold(), path.display());
        let analysis = analyze_one(&rt, &path, &opts, embedder.as_mut())?;
        repos.push(analysis);
    }

    let model = embedder
        .as_ref()
        .map(|_| "AllMiniLML6V2 (384d)".to_string())
        .unwrap_or_else(|| "(aucun — ML désactivé)".to_string());

    let report = AnalysisReport {
        generated_at: chrono_rfc3339_now(),
        model,
        repos,
    };

    match opts.report {
        Report::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        _ => print!("{}", render_text(&report)),
    }
    Ok(())
}

#[cfg(feature = "ai")]
fn try_init_embedder() -> Option<Embedder> {
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
    eprintln!(
        "{} modèle fastembed (AllMiniLML6V2, ~23 MB, cache ~/.cache)",
        "loading".dimmed()
    );
    match TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
    ) {
        Ok(m) => Some(m),
        Err(e) => {
            eprintln!(
                "{} fastembed indisponible : {} — analyse sans crosslink ML",
                "warn".yellow(),
                e
            );
            None
        }
    }
}

/// Fallback sans feature `ai` : crosslink ML désactivé, analyse dédiée
/// uniquement aux findings + audit.
#[cfg(not(feature = "ai"))]
fn try_init_embedder() -> Option<Embedder> {
    eprintln!(
        "{} build sans feature `ai` — crosslink ML désactivé (recompiler avec `cargo install --features ai` pour activer)",
        "info".dimmed()
    );
    None
}

fn analyze_one(
    rt: &tokio::runtime::Runtime,
    path: &Path,
    opts: &AnalyzeOpts,
    embedder: Option<&mut Embedder>,
) -> Result<RepoAnalysis> {
    // 1. Scan
    let scan_mode = opts.apply.unwrap_or(Mode::Check);
    let run_opts = RunOptions {
        root: path.to_path_buf(),
        mode: scan_mode,
        report: Report::Text,
        quiet: true,
        ignore: opts.ignore.clone(),
        agent: false,
        dry_run: false,
    };
    let fixes = run::run(&run_opts).with_context(|| format!("scan {}", path.display()))?;

    let mut summary = Summary {
        files_scanned: fixes.len(),
        files_with_findings: fixes.iter().filter(|f| !f.findings.is_empty()).count(),
        ..Summary::default()
    };
    let mut findings_by_rule: BTreeMap<String, usize> = BTreeMap::new();
    for fx in &fixes {
        for f in &fx.findings {
            summary.findings_total += 1;
            match f.severity {
                Severity::Error => summary.errors += 1,
                Severity::Warn => summary.warns += 1,
                Severity::Info => summary.infos += 1,
            }
            *findings_by_rule.entry(f.rule_id.clone()).or_default() += 1;
        }
    }

    // 2. Détection repo + audit
    let mut notes: Vec<String> = Vec::new();
    let (repo_info, issues, prs) = match audit::detect_repo(path) {
        Ok(repo) => {
            let terms = vec!["bun".to_string(), "node".to_string()];
            match rt.block_on(audit::run_audit(
                repo.clone(),
                &terms,
                ItemState::All,
                opts.issue_limit,
            )) {
                Ok(res) => {
                    // Utilise le repo retourné par run_audit (possiblement le parent si fork).
                    let repo_info = RepoInfo {
                        owner: res.repo.owner.clone(),
                        name: res.repo.name.clone(),
                        source: res.repo.source.to_string(),
                        upstream_of: res.repo.upstream_of.clone(),
                    };
                    summary.issues = res.issues.len();
                    summary.prs = res.pulls.len();
                    notes.extend(res.notes);
                    (Some(repo_info), res.issues, res.pulls)
                }
                Err(e) => {
                    notes.push(format!("audit GitHub a échoué : {e}"));
                    let repo_info = RepoInfo {
                        owner: repo.owner.clone(),
                        name: repo.name.clone(),
                        source: repo.source.to_string(),
                        upstream_of: repo.upstream_of.clone(),
                    };
                    (Some(repo_info), Vec::new(), Vec::new())
                }
            }
        }
        Err(e) => {
            notes.push(format!("repo GitHub non détecté : {e}"));
            (None, Vec::new(), Vec::new())
        }
    };
    // Si un fork a été remappé sur son upstream, émet une note dans le rapport.
    if let Some(ri) = &repo_info {
        if let Some(fork) = &ri.upstream_of {
            notes.push(format!(
                "fork {fork} → issues/PRs cherchées dans upstream {}/{}",
                ri.owner, ri.name
            ));
        }
    }

    // 3. Crosslink ML (feature `ai`) — fallback au no-ML si absent.
    #[cfg(feature = "ai")]
    let (issues_enriched, prs_enriched, top_findings) = if let Some(emb) = embedder {
        crosslink(emb, &fixes, &issues, &prs, opts.top_k, opts.threshold)?
    } else {
        (
            issues.iter().map(issue_basic).collect(),
            prs.iter().map(issue_basic).collect(),
            top_findings_without_ml(&fixes),
        )
    };
    #[cfg(not(feature = "ai"))]
    let (issues_enriched, prs_enriched, top_findings) = {
        let _ = embedder; // silence unused warning
        (
            issues.iter().map(issue_basic).collect(),
            prs.iter().map(issue_basic).collect(),
            top_findings_without_ml(&fixes),
        )
    };

    Ok(RepoAnalysis {
        path: path.display().to_string(),
        repo: repo_info,
        summary,
        findings_by_rule,
        issues: issues_enriched,
        prs: prs_enriched,
        top_findings,
        notes,
    })
}

fn issue_basic(h: &Hit) -> IssueWithMatches {
    IssueWithMatches {
        number: h.number,
        title: h.title.clone(),
        state: h.state.clone(),
        url: h.url.clone(),
        matched_terms: h.matched_terms.clone(),
        related_rules: Vec::new(),
    }
}

fn top_findings_without_ml(fixes: &[n2b_core::types::FileFix]) -> Vec<FindingWithIssues> {
    let mut out = Vec::new();
    for fx in fixes {
        for f in &fx.findings {
            out.push(FindingWithIssues {
                file: fx.file.clone(),
                line: f.line,
                col: f.col,
                rule_id: f.rule_id.clone(),
                message: f.message.clone(),
                replacement: f.replacement.clone(),
                related_issues: Vec::new(),
            });
        }
    }
    // Limite à 100 pour éviter un JSON énorme.
    out.truncate(100);
    out
}

#[cfg(feature = "ai")]
fn crosslink(
    emb: &mut Embedder,
    fixes: &[n2b_core::types::FileFix],
    issues: &[Hit],
    prs: &[Hit],
    top_k: usize,
    threshold: f32,
) -> Result<(
    Vec<IssueWithMatches>,
    Vec<IssueWithMatches>,
    Vec<FindingWithIssues>,
)> {
    // Unique rule-messages (dedup).
    let mut rule_order: Vec<String> = Vec::new();
    let mut rule_text: Vec<String> = Vec::new();
    {
        let mut seen = std::collections::HashSet::new();
        for fx in fixes {
            for f in &fx.findings {
                if seen.insert(f.rule_id.clone()) {
                    rule_order.push(f.rule_id.clone());
                    rule_text.push(format!("{}: {}", f.rule_id, f.message));
                }
            }
        }
    }

    // Textes à embed pour issues/PRs (title uniquement, suffit à discriminer).
    let issue_text: Vec<String> = issues.iter().map(|h| h.title.clone()).collect();
    let pr_text: Vec<String> = prs.iter().map(|h| h.title.clone()).collect();

    // Embed en un seul batch pour maximiser le cache ONNX.
    let mut all: Vec<String> = Vec::new();
    all.extend(rule_text.iter().cloned());
    all.extend(issue_text.iter().cloned());
    all.extend(pr_text.iter().cloned());

    if all.is_empty() {
        return Ok((
            issues.iter().map(issue_basic).collect(),
            prs.iter().map(issue_basic).collect(),
            top_findings_without_ml(fixes),
        ));
    }

    let vecs = emb.embed(all, None).context("fastembed.embed")?;
    let (rules_v, rest) = vecs.split_at(rule_order.len());
    let (issues_v, prs_v) = rest.split_at(issues.len());

    // Cosine entre chaque rule et chaque issue/PR.
    let rule_to_issues: Vec<Vec<(usize, f32)>> = rules_v
        .iter()
        .map(|rv| top_matches(rv, issues_v, top_k, threshold))
        .collect();
    let rule_to_prs: Vec<Vec<(usize, f32)>> = rules_v
        .iter()
        .map(|rv| top_matches(rv, prs_v, top_k, threshold))
        .collect();

    // Inverse : pour chaque issue, quelles rules lui ressemblent ?
    let issues_enriched = enrich_hits(issues, &rule_order, &rule_to_issues);
    let prs_enriched = enrich_hits(prs, &rule_order, &rule_to_prs);

    // Pour chaque finding, récupérer les top-K issues via sa rule.
    let mut rule_index: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (i, r) in rule_order.iter().enumerate() {
        rule_index.insert(r.as_str(), i);
    }

    let mut out: Vec<FindingWithIssues> = Vec::new();
    for fx in fixes {
        for f in &fx.findings {
            let i = match rule_index.get(f.rule_id.as_str()) {
                Some(&i) => i,
                None => continue,
            };
            let mut related: Vec<RelatedIssue> = Vec::new();
            for &(idx, score) in &rule_to_issues[i] {
                related.push(RelatedIssue {
                    number: issues[idx].number,
                    title: issues[idx].title.clone(),
                    url: issues[idx].url.clone(),
                    is_pr: false,
                    similarity: round4(score),
                });
            }
            for &(idx, score) in &rule_to_prs[i] {
                related.push(RelatedIssue {
                    number: prs[idx].number,
                    title: prs[idx].title.clone(),
                    url: prs[idx].url.clone(),
                    is_pr: true,
                    similarity: round4(score),
                });
            }
            out.push(FindingWithIssues {
                file: fx.file.clone(),
                line: f.line,
                col: f.col,
                rule_id: f.rule_id.clone(),
                message: f.message.clone(),
                replacement: f.replacement.clone(),
                related_issues: related,
            });
        }
    }
    // Cap à 200 pour le JSON.
    out.truncate(200);
    Ok((issues_enriched, prs_enriched, out))
}

#[cfg(feature = "ai")]
fn enrich_hits(
    hits: &[Hit],
    rule_order: &[String],
    rule_to_hits: &[Vec<(usize, f32)>],
) -> Vec<IssueWithMatches> {
    // Inverse index : hit_idx → list of (rule_id, score)
    let mut per_hit: Vec<Vec<RuleMatch>> = vec![Vec::new(); hits.len()];
    for (r_idx, matches) in rule_to_hits.iter().enumerate() {
        for &(h_idx, score) in matches {
            per_hit[h_idx].push(RuleMatch {
                rule_id: rule_order[r_idx].clone(),
                similarity: round4(score),
            });
        }
    }
    hits.iter()
        .zip(per_hit)
        .map(|(h, mut related)| {
            related.sort_by(|a, b| {
                b.similarity
                    .partial_cmp(&a.similarity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            IssueWithMatches {
                number: h.number,
                title: h.title.clone(),
                state: h.state.clone(),
                url: h.url.clone(),
                matched_terms: h.matched_terms.clone(),
                related_rules: related,
            }
        })
        .collect()
}

#[cfg(feature = "ai")]
fn top_matches(query: &[f32], pool: &[Vec<f32>], k: usize, threshold: f32) -> Vec<(usize, f32)> {
    let mut scored: Vec<(usize, f32)> = pool
        .iter()
        .enumerate()
        .map(|(i, v)| (i, cosine(query, v)))
        .filter(|(_, s)| *s >= threshold)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    scored
}

#[cfg(feature = "ai")]
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for i in 0..a.len().min(b.len()) {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[cfg(feature = "ai")]
fn round4(x: f32) -> f32 {
    (x * 10000.0).round() / 10000.0
}

fn chrono_rfc3339_now() -> String {
    // Pas de dep chrono — on utilise SystemTime + format manuel (UTC).
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    // Format grossier : le champ est informatif.
    format!("unix:{secs}")
}

pub fn render_text(r: &AnalysisReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {}\n",
        "node2bun analyze".bold(),
        format!("(model: {})", r.model).dimmed()
    ));
    out.push_str(&format!("  timestamp: {}\n", r.generated_at));
    out.push_str(&format!("  repos: {}\n\n", r.repos.len()));

    for a in &r.repos {
        let label = match &a.repo {
            Some(ri) => format!("{}/{}", ri.owner, ri.name).cyan().to_string(),
            None => "(no GitHub repo)".dimmed().to_string(),
        };
        out.push_str(&format!("{} {} {}\n", "▼".bold(), a.path.bold(), label));
        out.push_str(&format!(
            "  files: {} ({} w/ findings) · findings: {} ({}E/{}W/{}I) · issues: {} · PRs: {}\n",
            a.summary.files_scanned,
            a.summary.files_with_findings,
            a.summary.findings_total,
            a.summary.errors,
            a.summary.warns,
            a.summary.infos,
            a.summary.issues,
            a.summary.prs,
        ));

        if !a.findings_by_rule.is_empty() {
            let mut top: Vec<(&String, &usize)> = a.findings_by_rule.iter().collect();
            top.sort_by(|x, y| y.1.cmp(x.1));
            out.push_str("  top rules: ");
            for (i, (rule, count)) in top.iter().take(6).enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("{}×{}", rule.cyan(), count.to_string().yellow()));
            }
            out.push('\n');
        }

        // Lister quelques findings avec leur top issue ML.
        let cross: Vec<&FindingWithIssues> = a
            .top_findings
            .iter()
            .filter(|f| !f.related_issues.is_empty())
            .take(5)
            .collect();
        if !cross.is_empty() {
            out.push_str(&format!("  {}:\n", "ML crosslinks (top 5)".bold()));
            for f in cross {
                out.push_str(&format!(
                    "    {} {}:{} {} {}\n",
                    "·".dimmed(),
                    f.file,
                    f.line,
                    f.rule_id.cyan(),
                    f.message,
                ));
                for ri in f.related_issues.iter().take(2) {
                    let kind = if ri.is_pr { "PR" } else { "issue" };
                    out.push_str(&format!(
                        "        → {} #{} sim={:.3} {} {}\n",
                        kind.dimmed(),
                        ri.number.to_string().cyan(),
                        ri.similarity,
                        ri.title,
                        ri.url.dimmed(),
                    ));
                }
            }
        }

        for n in &a.notes {
            out.push_str(&format!("  {} {}\n", "!".yellow(), n.dimmed()));
        }
        out.push('\n');
    }
    out
}

pub fn resolve_default_paths(cwd: &Path) -> Vec<PathBuf> {
    // Candidats classiques si l'utilisateur ne spécifie rien.
    let candidates = [
        "discord.js",
        "discordjs",
        "discordx",
        "discordx-deps/discord.js",
        "nextjs",
        "next.js",
        "prisma",
    ];
    let mut out = Vec::new();
    for c in candidates {
        let p = cwd.join(c);
        if p.is_dir() {
            out.push(p);
        }
    }
    out
}

// Helper utilisé par render_json côté main si besoin.
#[allow(dead_code)]
pub fn to_json(report: &AnalysisReport) -> String {
    serde_json::to_string_pretty(&json!(report)).unwrap_or_default()
}
