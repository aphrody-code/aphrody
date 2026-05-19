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

use anyhow::{Context, Result, bail};
use colored::Colorize;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::StatusCode;
use serde_json::Value;
use std::path::Path;

use crate::github;

#[derive(Debug, Clone)]
pub struct Repo {
    pub owner: String,
    pub name: String,
    pub source: &'static str,
    /// Si le repo détecté est un fork, contient le slug du parent qu'on
    /// a utilisé à la place pour la recherche d'issues/PRs.
    pub upstream_of: Option<String>,
}

impl Repo {
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// Détecte le repo GitHub du package via, dans l'ordre :
///   1. package.json → `repository` (string ou objet `{url}`)
///   2. `.git/config` → remote origin URL
pub fn detect_repo(root: &Path) -> Result<Repo> {
    if let Some(r) = from_package_json(root)? {
        return Ok(r);
    }
    if let Some(r) = from_git_config(root)? {
        return Ok(r);
    }
    bail!(
        "repo GitHub introuvable : ni 'repository' dans package.json ni remote 'origin' dans .git/config à {}",
        root.display()
    );
}

fn from_package_json(root: &Path) -> Result<Option<Repo>> {
    let path = root.join("package.json");
    if !path.exists() {
        return Ok(None);
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("lecture {}", path.display()))?;
    let v: Value =
        serde_json::from_str(&content).with_context(|| format!("parse {}", path.display()))?;
    let url = match v.get("repository") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(o)) => match o.get("url").and_then(|x| x.as_str()) {
            Some(s) => s.to_string(),
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    Ok(parse_github_url(&url).map(|(o, n)| Repo {
        owner: o,
        name: n,
        source: "package.json:repository",
        upstream_of: None,
    }))
}

fn from_git_config(root: &Path) -> Result<Option<Repo>> {
    // Remonte les parents jusqu'à trouver un .git/config.
    let mut cur = Some(root.to_path_buf());
    while let Some(dir) = cur {
        let cfg = dir.join(".git").join("config");
        if cfg.exists() {
            let content = std::fs::read_to_string(&cfg).ok();
            if let Some(content) = content {
                let origin = remote_url(&content, "origin");
                let upstream = remote_url(&content, "upstream");
                // Préférence : si `upstream` est configuré (convention fork),
                // on l'utilise directement — les issues/PRs amont sont ce que
                // l'utilisateur veut voir pour sa migration.
                if let Some(u) = upstream {
                    if let Some((o, n)) = parse_github_url(&u) {
                        let fork_slug = origin
                            .as_deref()
                            .and_then(parse_github_url)
                            .map(|(fo, fn_)| format!("{fo}/{fn_}"));
                        return Ok(Some(Repo {
                            owner: o,
                            name: n,
                            source: "git:upstream",
                            upstream_of: fork_slug,
                        }));
                    }
                }
                if let Some(u) = origin {
                    if let Some((o, n)) = parse_github_url(&u) {
                        return Ok(Some(Repo {
                            owner: o,
                            name: n,
                            source: "git:origin",
                            upstream_of: None,
                        }));
                    }
                }
            }
            return Ok(None);
        }
        cur = dir.parent().map(|p| p.to_path_buf());
    }
    Ok(None)
}

fn remote_url(config: &str, remote_name: &str) -> Option<String> {
    let header = format!("[remote \"{remote_name}\"]");
    let mut in_section = false;
    for line in config.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_section = l == header;
            continue;
        }
        if in_section {
            if let Some(rest) = l.strip_prefix("url") {
                let u = rest.trim_start_matches([' ', '=', '\t']).trim();
                if !u.is_empty() {
                    return Some(u.to_string());
                }
            }
        }
    }
    None
}

static GH_URL_RE: Lazy<Regex> = Lazy::new(|| {
    // Capture owner/repo sur les formes :
    //   https://github.com/OWNER/REPO[.git]
    //   git@github.com:OWNER/REPO[.git]
    //   git+https://github.com/OWNER/REPO.git
    //   ssh://git@github.com/OWNER/REPO.git
    //   git@github-alias:OWNER/REPO.git  (SSH alias custom — fréquent pour multi-compte)
    Regex::new(
        r"(?:github\.com|github[\w-]*)[:/]([A-Za-z0-9_.\-]+)/([A-Za-z0-9_.\-]+?)(?:\.git)?/?$",
    )
    .expect("invariant: GH_URL_RE regex literal is valid")
});

fn parse_github_url(url: &str) -> Option<(String, String)> {
    let caps = GH_URL_RE.captures(url.trim())?;
    Some((caps[1].to_string(), caps[2].to_string()))
}

#[derive(Debug, Clone, Copy)]
pub enum ItemState {
    Open,
    Closed,
    All,
}

#[derive(Debug)]
pub struct AuditResult {
    pub repo: Repo,
    pub issues: Vec<Hit>,
    pub pulls: Vec<Hit>,
    pub query_terms: Vec<String>,
    /// Avertissements non-bloquants collectés pendant l'audit
    /// (ex. : rate-limit épuisé, requête rejetée par l'API).
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub url: String,
    pub is_pr: bool,
    pub comments: u32,
    pub updated_at: String,
    pub matched_terms: Vec<String>,
}

/// Si `repo` est un fork sur GitHub, retourne le parent. Sinon `repo`
/// inchangé. Échec silencieux (sans token, les forks privés ne sont pas
/// résolus — on garde le repo d'origine).
pub async fn resolve_upstream(gh: &octocrab::Octocrab, repo: Repo) -> Repo {
    match gh.repos(&repo.owner, &repo.name).get().await {
        Ok(info) => {
            if info.fork.unwrap_or(false) {
                if let Some(parent) = info.parent {
                    let p_owner = parent.owner.map(|o| o.login).unwrap_or_default();
                    let p_name = parent.name;
                    if !p_owner.is_empty() && !p_name.is_empty() {
                        let fork_slug = repo.slug();
                        return Repo {
                            owner: p_owner,
                            name: p_name,
                            source: repo.source,
                            upstream_of: Some(fork_slug),
                        };
                    }
                }
            }
            repo
        }
        Err(_) => repo,
    }
}

/// Vérifie le quota restant pour l'endpoint `search` (limite GitHub : 30/min
/// avec token, 10/min sans). Retourne `Ok(remaining)`, ou `Ok(usize::MAX)` si
/// l'appel échoue (on laisse la suite tenter sa chance plutôt que bloquer).
async fn check_search_rate(gh: &octocrab::Octocrab) -> usize {
    match gh.ratelimit().get().await {
        Ok(rl) => rl.resources.search.remaining,
        Err(_) => usize::MAX,
    }
}

/// Classifie une erreur octocrab en catégorie exploitable sans paniquer.
#[derive(Debug, PartialEq)]
enum SearchError {
    /// HTTP 403 — rate-limit épuisé ou accès interdit.
    RateLimited,
    /// HTTP 422 — requête mal formée ou syntaxe de recherche invalide.
    UnprocessableEntity,
    /// Toute autre erreur réseau ou API.
    Other(String),
}

#[allow(clippy::collapsible_match)] // preexisting: nested if-let over octocrab error enum variants
fn classify_octocrab_error(err: &anyhow::Error) -> SearchError {
    // octocrab expose ses propres erreurs via la chaîne `source`. On traverse
    // la chaîne avec `downcast_ref` sans consommer `err`.
    if let Some(octo_err) = err.downcast_ref::<octocrab::Error>() {
        if let octocrab::Error::GitHub { source, .. } = octo_err {
            match source.status_code {
                StatusCode::FORBIDDEN => return SearchError::RateLimited,
                StatusCode::UNPROCESSABLE_ENTITY => {
                    return SearchError::UnprocessableEntity;
                }
                _ => {}
            }
        }
    }
    SearchError::Other(err.to_string())
}

pub async fn run_audit(
    repo: Repo,
    terms: &[String],
    state: ItemState,
    limit: usize,
) -> Result<AuditResult> {
    let gh = github::client()?;
    let repo = resolve_upstream(&gh, repo).await;
    let mut notes: Vec<String> = Vec::new();

    // --- Vérification du quota search avant de lancer les requêtes ----------
    // L'endpoint /search/issues coûte 1 quota par appel. On vérifie qu'il en
    // reste au moins 2 (une pour issues, une pour PRs).
    let remaining = check_search_rate(&gh).await;
    if remaining < 2 {
        let msg = format!(
            "quota search GitHub épuisé ({remaining} restant) — audit ignoré pour {}",
            repo.slug()
        );
        eprintln!("{} {}", "warning:".yellow().bold(), msg);
        notes.push(msg);
        return Ok(AuditResult {
            repo,
            issues: Vec::new(),
            pulls: Vec::new(),
            query_terms: terms.to_vec(),
            notes,
        });
    }

    // `search/issues` accepte la syntaxe `repo:OWNER/NAME is:issue bun OR node`.
    let state_q = match state {
        ItemState::Open => "is:open",
        ItemState::Closed => "is:closed",
        ItemState::All => "",
    };

    let joined = terms
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(" OR ");
    let base_q = format!(
        "repo:{} in:title,body ({}) {}",
        repo.slug(),
        joined,
        state_q
    );

    let issues =
        search_tolerant(&gh, &format!("{base_q} is:issue"), limit, terms, &mut notes).await;

    let pulls = search_tolerant(&gh, &format!("{base_q} is:pr"), limit, terms, &mut notes).await;

    Ok(AuditResult {
        repo,
        issues,
        pulls,
        query_terms: terms.to_vec(),
        notes,
    })
}

/// Wrapper autour de [`search`] qui ne propage pas les erreurs : un 403 ou
/// 422 génère un avertissement dans `notes` et retourne `Vec::new()`.
async fn search_tolerant(
    gh: &octocrab::Octocrab,
    query: &str,
    limit: usize,
    terms: &[String],
    notes: &mut Vec<String>,
) -> Vec<Hit> {
    match search(gh, query, limit, terms).await {
        Ok(hits) => hits,
        Err(err) => {
            let note = match classify_octocrab_error(&err) {
                SearchError::RateLimited => {
                    format!("rate-limit GitHub (403) sur la requête : {query}")
                }
                SearchError::UnprocessableEntity => {
                    format!("requête rejetée par GitHub (422 — syntaxe invalide) : {query}")
                }
                SearchError::Other(msg) => {
                    format!("erreur GitHub sur la requête `{query}` : {msg}")
                }
            };
            eprintln!("{} {}", "warning:".yellow().bold(), note);
            notes.push(note);
            Vec::new()
        }
    }
}

async fn search(
    gh: &octocrab::Octocrab,
    query: &str,
    limit: usize,
    terms: &[String],
) -> Result<Vec<Hit>> {
    // Per-page plafonné à 100 (max GitHub). On ne pagine pas au-delà car :
    //   1. chaque page supplémentaire coûte 1 quota search (limité à 30/min),
    //   2. l'appelant contrôle déjà le volume via `limit`.
    // Si limit > 100 et l'utilisateur veut vraiment tout récupérer, il lui
    // appartient de paginer explicitement avec `all_pages` côté appelant.
    let per_page: u8 = limit.min(100) as u8;
    let page = gh
        .search()
        .issues_and_pull_requests(query)
        .sort("updated")
        .order("desc")
        .per_page(per_page)
        .send()
        .await
        .with_context(|| format!("recherche GitHub : {query}"))?;

    let lc_terms: Vec<String> = terms.iter().map(|t| t.to_lowercase()).collect();

    let hits = page
        .items
        .into_iter()
        .take(limit)
        .map(|it| {
            let haystack =
                format!("{} {}", it.title, it.body.clone().unwrap_or_default()).to_lowercase();
            let matched: Vec<String> = lc_terms
                .iter()
                .filter(|t| haystack.contains(t.as_str()))
                .cloned()
                .collect();
            Hit {
                number: it.number,
                title: it.title,
                state: format!("{:?}", it.state).to_lowercase(),
                url: it.html_url.to_string(),
                is_pr: it.pull_request.is_some(),
                comments: it.comments,
                updated_at: it.updated_at.to_rfc3339(),
                matched_terms: matched,
            }
        })
        .collect();
    Ok(hits)
}

pub fn render_text(res: &AuditResult) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {}\n",
        "node2bun audit".bold(),
        format!("({})", res.repo.source).dimmed()
    ));
    out.push_str(&format!(
        "  repo  : {}\n",
        format!("github.com/{}", res.repo.slug()).cyan()
    ));
    if let Some(fork) = &res.repo.upstream_of {
        out.push_str(&format!(
            "  fork  : {} → upstream utilisé à la place\n",
            fork.dimmed()
        ));
    }
    out.push_str(&format!(
        "  terms : {}\n",
        res.query_terms.join(", ").yellow()
    ));
    out.push_str(&format!(
        "  issues: {}, PRs: {}\n",
        res.issues.len(),
        res.pulls.len()
    ));
    if !res.notes.is_empty() {
        out.push_str(&format!("  notes : {}\n", res.notes.join("; ").dimmed()));
    }
    out.push('\n');
    render_section(&mut out, "Issues", &res.issues);
    render_section(&mut out, "Pull Requests", &res.pulls);
    out
}

fn render_section(out: &mut String, title: &str, hits: &[Hit]) {
    if hits.is_empty() {
        return;
    }
    out.push_str(&format!("{}\n", title.bold()));
    for h in hits {
        let state = match h.state.as_str() {
            "open" => h.state.green().to_string(),
            "closed" => h.state.red().to_string(),
            _ => h.state.dimmed().to_string(),
        };
        let tags = if h.matched_terms.is_empty() {
            String::new()
        } else {
            format!(" [{}]", h.matched_terms.join(",").yellow())
        };
        out.push_str(&format!(
            "  #{:<5} {} {}{}\n    {} {}\n",
            h.number.to_string().cyan(),
            state,
            h.title,
            tags,
            "→".dimmed(),
            h.url.dimmed(),
        ));
    }
    out.push('\n');
}

pub fn render_json(res: &AuditResult) -> String {
    let v = serde_json::json!({
        "repo": {
            "owner": res.repo.owner,
            "name": res.repo.name,
            "source": res.repo.source,
            "upstream_of": res.repo.upstream_of,
        },
        "terms": res.query_terms,
        "notes": res.notes,
        "issues": res.issues.iter().map(hit_json).collect::<Vec<_>>(),
        "pulls":  res.pulls .iter().map(hit_json).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

fn hit_json(h: &Hit) -> Value {
    serde_json::json!({
        "number": h.number,
        "title": h.title,
        "state": h.state,
        "url": h.url,
        "is_pr": h.is_pr,
        "comments": h.comments,
        "updated_at": h.updated_at,
        "matched_terms": h.matched_terms,
    })
}
