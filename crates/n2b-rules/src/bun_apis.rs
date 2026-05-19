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

//! Règles `api/*` et `next/*` — détection de patterns d'appel Node →
//! réécriture Bun. Les données vivent dans
//! `crates/n2b-registry/registry/apis.toml` ; cette couche fournit le
//! matching, l'application des édits, et le **filtrage AST** (Phase 2)
//! qui résout PS1 (faux positifs sur identifiants homonymes).
//!
//! Phase 2 : un `ApiEntry` peut porter un `import_from`. Quand présent,
//! l'entrée ne déclenche un finding que si le binding root du callee
//! provient effectivement d'un import du specifier attendu — l'AST
//! tranche, plus de regex contextuelle ad-hoc.

use n2b_registry::{APIS, ApiEntry, ImportGraph, MODULES};
use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::{Edit, apply_edits, line_offsets, make_finding};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::collections::HashSet;

enum ReplaceKind {
    None,
    Static(String),
    Template(String), // utilise $1..$n
}

/// Vue runtime d'une `ApiEntry` du registre — la regex est compilée une
/// fois au premier accès via `Lazy`.
struct ApiRule {
    id: String,
    re: Regex,
    message: String,
    replace: ReplaceKind,
    aggressive: bool,
    severity: Severity,
    /// Phase 2 : si défini, le finding ne sort que si le binding root du
    /// match provient d'un import vers ce specifier (résout PS1).
    /// Cas spéciaux : "node:fs" matche "fs" et inversement (canonisation
    /// des préfixes node:).
    import_from: Option<String>,
}

fn build_rule(e: &ApiEntry) -> ApiRule {
    let replace = match e.replace.as_str() {
        "none" => ReplaceKind::None,
        "static" => ReplaceKind::Static(
            e.replacement
                .clone()
                .expect("invariant: replace=static requires replacement"),
        ),
        "template" => ReplaceKind::Template(
            e.replacement
                .clone()
                .expect("invariant: replace=template requires replacement"),
        ),
        other => panic!("invariant: replace='{other}' inconnu pour {}", e.id),
    };
    ApiRule {
        id: e.id.clone(),
        re: Regex::new(&e.pattern).unwrap_or_else(|err| {
            panic!(
                "invariant: ApiRule pattern for rule '{}' is invalid: {err}",
                e.id
            )
        }),
        message: e.message.clone(),
        replace,
        aggressive: e.aggressive,
        severity: e.severity,
        import_from: e.import_from.clone().filter(|s| !s.is_empty()),
    }
}

static RULES: Lazy<Vec<ApiRule>> = Lazy::new(|| APIS.iter().map(build_rule).collect());

/// Set des modules Node builtins (sans préfixe `node:`). Un préfixe explicite
/// `<module>.` dans le code n'est considéré comme un usage namespace valide
/// que pour ces modules (`child_process.exec(`, `fs.readFileSync(`) — pas
/// pour un package npm comme `marked.parse(` qui peut être un objet local.
static NODE_BUILTINS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    MODULES
        .iter()
        .map(|m| strip_node_prefix(m.module.as_str()))
        .collect()
});

/// Wrapper rétrocompatible pour les appelants externes (tests). Construit
/// un graphe vide → tous les filtres `import_from` retournent `false` et
/// les règles concernées sont silencieuses. Préfère `apply_bun_api_rules_with_imports`.
pub fn apply_bun_api_rules(path: &str, source: &str, aggressive: bool) -> (Vec<Finding>, String) {
    let imports = crate::imports_ast::build_import_graph(path, source);
    apply_bun_api_rules_with_imports(path, source, aggressive, &imports)
}

pub fn apply_bun_api_rules_with_imports(
    path: &str,
    source: &str,
    aggressive: bool,
    imports: &ImportGraph,
) -> (Vec<Finding>, String) {
    let offsets = line_offsets(source);
    let mut findings: Vec<Finding> = Vec::new();
    let mut edits: Vec<Edit> = Vec::new();

    for r in RULES.iter() {
        for mat in r.re.captures_iter(source) {
            let whole = mat
                .get(0)
                .expect("invariant: capture group 0 is always present in a match");
            let index = whole.start();
            let original = whole.as_str();

            // Phase 2 : filtre AST. Le finding n'est émis que si :
            //  (a) le pattern matché commence par `<from>.` — usage
            //      namespace explicite (`child_process.exec(`), valide par
            //      construction ;
            //  (b) sinon, le binding root du match provient d'un import
            //      vers `from` (résolu par l'AST oxc).
            // Sans `import_from`, la règle reste textuelle (cas globals :
            // Buffer, process, etc.).
            if let Some(from) = &r.import_from {
                let from_norm = strip_node_prefix(from);
                let explicit_prefix = format!("{from_norm}.");
                // Préfixe explicite valide UNIQUEMENT si `from` est un module
                // Node builtin (le namespace est canonique). `marked.parse(`
                // n'est pas un namespace valide — `marked` peut être un objet
                // local. Pour les builtins (`child_process.exec(`), le préfixe
                // EST sa propre preuve.
                let explicit_ok =
                    original.starts_with(&explicit_prefix) && NODE_BUILTINS.contains(from_norm);
                let resolves = if explicit_ok {
                    true
                } else {
                    match first_meaningful_ident(original) {
                        Some(name) => binding_resolves(imports, name, from),
                        None => false,
                    }
                };
                if !resolves {
                    continue;
                }
            }

            let original_len = original.len();
            let replacement = match &r.replace {
                ReplaceKind::None => None,
                ReplaceKind::Static(s) => Some(s.clone()),
                ReplaceKind::Template(t) => Some(expand(&mat, t)),
            };

            // Heuristique préservée (Phase 2 : à supprimer quand l'AST
            // suit le flux du symbole jusqu'à `fs.mkdir`). Aujourd'hui
            // l'AST sait *qui* est `fs`, pas où *son* argument est passé.
            let skip_autofix = r.id == "api/fs-existsSync"
                && mat.get(1).is_some_and(|m| {
                    let arg = m.as_str().trim();
                    looks_like_dir_context(source, index, arg)
                });

            let has_repl = replacement.is_some() && !skip_autofix;
            let replacement_for_edit = if aggressive && r.aggressive && !skip_autofix {
                replacement.clone()
            } else {
                None
            };
            findings.push(make_finding(
                path,
                &offsets,
                index,
                &r.id,
                if skip_autofix {
                    "fs.existsSync(path) suivi d'un fs.mkdirSync(path) — chemin probablement un dossier, Bun.file().exists() inadapté (utiliser fs.mkdirSync(path, { recursive: true }))".to_string()
                } else {
                    r.message.clone()
                },
                original.to_string(),
                if skip_autofix { None } else { replacement },
                MakeFindingOpts {
                    autofix: Some(has_repl),
                    aggressive: if r.aggressive { Some(true) } else { None },
                    severity: Some(r.severity),
                ..Default::default()
            },
            ));
            if let Some(repl) = replacement_for_edit {
                edits.push(Edit {
                    index,
                    len: original_len,
                    replacement: repl,
                });
            }
        }
    }

    let out = apply_edits(source, edits);
    (findings, out)
}

fn expand(caps: &Captures, template: &str) -> String {
    let mut out = String::with_capacity(template.len());
    caps.expand(template, &mut out);
    out
}

/// Fenêtre de recherche (en octets) pour décider si un `fs.existsSync` vise un
/// dossier. Heuristique préservée Phase 2 : l'AST sait qui est `fs`, mais
/// suivre `fs.existsSync(p)` → trouver le `fs.mkdir(p, ...)` qui partage `p`
/// nécessite un *control-flow* léger qu'on n'a pas en place. Cible Phase 5/7.
const DIR_CONTEXT_WINDOW_BYTES: usize = 600;

fn looks_like_dir_context(source: &str, pos: usize, arg: &str) -> bool {
    let end = (pos + DIR_CONTEXT_WINDOW_BYTES).min(source.len());
    let window = &source[pos..end];
    let needle_sync = format!("fs.mkdirSync({arg}");
    let needle_async = format!("fs.mkdir({arg}");
    window.contains(&needle_sync) || window.contains(&needle_async)
}

/// Extrait le premier identifier "significatif" d'un match — celui qui sert
/// de binding root pour la résolution AST.
///
/// Saute les keywords JS qui peuvent précéder l'expression cible :
/// `const|let|var <name> =`, `new`, `await`. Pour `const x = chalk.red(...)`
/// retourne `chalk` (pas `const` ni `x`).
fn first_meaningful_ident(matched: &str) -> Option<&str> {
    let mut s = matched.trim_start();

    // (const|let|var) <name> = <expr>
    for kw in &["const ", "let ", "var "] {
        if let Some(rest) = s.strip_prefix(kw) {
            s = rest.trim_start();
            let eq = s.find('=')?;
            s = s[eq + 1..].trim_start();
            break;
        }
    }
    while let Some(rest) = s.strip_prefix("new ").or_else(|| s.strip_prefix("await ")) {
        s = rest.trim_start();
    }

    let mut end = 0;
    for (i, c) in s.char_indices() {
        if c.is_alphanumeric() || c == '_' || c == '$' {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 { None } else { Some(&s[..end]) }
}

/// Le `root` (binding local) résout-il vers le specifier `from` ?
///
/// Compare après normalisation `node:` pour préserver le zero-drift sur
/// `import { readFileSync } from "node:fs"` ↔ `import_from = "fs"`.
fn binding_resolves(imports: &ImportGraph, root: &str, from: &str) -> bool {
    let from_norm = strip_node_prefix(from);
    if let Some(b) = imports.bindings.get(root) {
        let bound_norm = strip_node_prefix(&b.specifier);
        return bound_norm == from_norm;
    }
    false
}

#[inline]
fn strip_node_prefix(s: &str) -> &str {
    s.strip_prefix("node:").unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use n2b_registry::{BindingKind, ImportBinding, ImportGraph};

    fn graph_with(name: &str, specifier: &str, kind: BindingKind) -> ImportGraph {
        let mut g = ImportGraph::new();
        g.bindings.insert(
            name.to_string(),
            ImportBinding {
                specifier: specifier.to_string(),
                kind,
            },
        );
        g
    }

    #[test]
    fn first_meaningful_ident_skips_keywords() {
        assert_eq!(first_meaningful_ident("chalk.red(x)"), Some("chalk"));
        assert_eq!(first_meaningful_ident("new CronJob(x)"), Some("CronJob"));
        assert_eq!(
            first_meaningful_ident("const app = express()"),
            Some("express")
        );
        assert_eq!(first_meaningful_ident("await fetch()"), Some("fetch"));
        assert_eq!(
            first_meaningful_ident("let x = new S3Client()"),
            Some("S3Client")
        );
        assert_eq!(
            first_meaningful_ident("child_process.exec("),
            Some("child_process")
        );
    }

    #[test]
    fn binding_resolves_direct_import() {
        let g = graph_with("marked", "marked", BindingKind::Default);
        assert!(binding_resolves(&g, "marked", "marked"));
    }

    #[test]
    fn binding_resolves_node_prefix_canonical() {
        let g = graph_with(
            "readFileSync",
            "node:fs",
            BindingKind::Named {
                imported: "readFileSync".to_string(),
            },
        );
        // import_from = "fs" devrait résoudre malgré le préfixe node: dans le binding
        assert!(binding_resolves(&g, "readFileSync", "fs"));
    }

    #[test]
    fn binding_does_not_resolve_unbound() {
        let g = ImportGraph::new();
        // Zéro fallback : un identifier qui ne provient pas d'un import ne
        // matche jamais — c'est le contrat anti-faux-positif de PS1.
        assert!(!binding_resolves(&g, "marked", "marked"));
        assert!(!binding_resolves(&g, "v4", "uuid"));
        assert!(!binding_resolves(&g, "child_process", "child_process"));
    }

    #[test]
    fn binding_resolves_renamed_named_import() {
        // import { v4 } from "uuid"
        let g = graph_with(
            "v4",
            "uuid",
            BindingKind::Named {
                imported: "v4".to_string(),
            },
        );
        assert!(binding_resolves(&g, "v4", "uuid"));
    }
}
