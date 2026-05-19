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

//! Extraction des specifiers d'import/require + construction d'un
//! `ImportGraph` (binding local → specifier source) via AST oxc.
//!
//! `extract_specifiers` reste pour la couche `node_imports` (rewriting du
//! string littéral). `build_import_graph` produit la structure consommée par
//! `engine.rs` / `bun_apis.rs` pour filtrer les `api/*` par `import_from`
//! (résout PS1).

use n2b_registry::{BindingKind, ImportBinding, ImportGraph};
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, Expression, ImportDeclarationSpecifier, Statement,
    VariableDeclarator,
};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Debug, Clone)]
pub struct Specifier {
    pub value: String,
    /// Offset byte du premier char du specifier (après la quote).
    pub inner_start: u32,
    pub inner_len: u32,
}

// ---------------------------------------------------------------------------
// Visitor CJS — uniquement pour require()
// ---------------------------------------------------------------------------

struct RequireCollect {
    out: Vec<Specifier>,
}

impl RequireCollect {
    fn push_string_literal(&mut self, lit: &oxc_ast::ast::StringLiteral) {
        let value = lit.value.as_str().to_string();
        let inner_start = lit.span.start + 1;
        let inner_len = value.len() as u32;
        self.out.push(Specifier {
            value,
            inner_start,
            inner_len,
        });
    }
}

impl<'a> Visit<'a> for RequireCollect {
    fn visit_call_expression(&mut self, it: &oxc_ast::ast::CallExpression<'a>) {
        let is_require =
            matches!(&it.callee, Expression::Identifier(ident) if ident.name.as_str() == "require");
        if is_require {
            if let Some(oxc_ast::ast::Argument::StringLiteral(lit)) = it.arguments.first() {
                self.push_string_literal(lit);
            }
        }
        oxc_ast_visit::walk::walk_call_expression(self, it);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn source_type_from_path(path: &str) -> SourceType {
    SourceType::from_path(path).unwrap_or_else(|_| SourceType::default())
}

#[inline]
fn is_string_quote(b: u8) -> bool {
    matches!(b, b'\'' | b'"' | b'`')
}

// ---------------------------------------------------------------------------
// API publique — extract_specifiers (rewriting node:* prefix)
// ---------------------------------------------------------------------------

pub fn extract_specifiers(path: &str, source: &str) -> Vec<Specifier> {
    let allocator = Allocator::default();
    let st = source_type_from_path(path);
    let ret = Parser::new(&allocator, source, st).parse();

    let module = &ret.module_record;
    let src_bytes = source.as_bytes();

    let cap = module.requested_modules.len() + module.dynamic_imports.len() + 4;
    let mut out: Vec<Specifier> = Vec::with_capacity(cap);

    for (name_str, occurrences) in &module.requested_modules {
        let value = name_str.as_str().to_string();
        for rm in occurrences.iter() {
            let inner_start = rm.span.start + 1;
            let raw_len = rm.span.end.saturating_sub(rm.span.start).saturating_sub(2);
            out.push(Specifier {
                value: value.clone(),
                inner_start,
                inner_len: raw_len,
            });
        }
    }

    for di in module.dynamic_imports.iter() {
        let span = di.module_request;
        if span.start < span.end {
            let start = span.start as usize;
            if src_bytes.get(start).copied().is_some_and(is_string_quote) {
                let value_bytes = &src_bytes[start + 1..span.end as usize - 1];
                let value = std::str::from_utf8(value_bytes).unwrap_or("").to_string();
                let inner_start = span.start + 1;
                let inner_len = span.end.saturating_sub(span.start).saturating_sub(2);
                out.push(Specifier {
                    value,
                    inner_start,
                    inner_len,
                });
            }
        }
    }

    let mut cjs = RequireCollect { out: Vec::new() };
    for stmt in &ret.program.body {
        oxc_ast_visit::walk::walk_statement(&mut cjs, stmt);
    }
    out.extend(cjs.out);

    out
}

// ---------------------------------------------------------------------------
// API publique — build_import_graph (binding → specifier)
// ---------------------------------------------------------------------------

/// Visitor qui résout les bindings locaux → (specifier, kind).
struct GraphCollect {
    graph: ImportGraph,
}

impl GraphCollect {
    fn add(&mut self, name: String, specifier: String, kind: BindingKind) {
        self.graph
            .bindings
            .insert(name, ImportBinding { specifier, kind });
    }

    /// `const x = require("foo")` ou `const { a, b: c } = require("foo")`.
    /// Ne traite que le cas où l'argument est un string littéral statique.
    fn handle_variable_declarator<'a>(&mut self, vd: &VariableDeclarator<'a>) {
        let Some(init) = vd.init.as_ref() else {
            return;
        };
        // Forme directe : require("foo")
        let specifier = match init {
            Expression::CallExpression(call) => extract_require_arg(call),
            // Forme via membre : require("foo").default ou require("foo").x
            // Non supportée pour le moment (ajouterait de la complexité ; les
            // bindings résolus ici servent à filtrer les api/*, et ces formes
            // sont rares dans le code Node moderne).
            _ => None,
        };
        let Some(spec) = specifier else { return };
        self.bind_pattern(&vd.id, &spec);
    }

    /// Lie tous les noms du pattern de destructuration au specifier.
    fn bind_pattern(&mut self, pattern: &BindingPattern<'_>, specifier: &str) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => {
                self.add(
                    id.name.as_str().to_string(),
                    specifier.to_string(),
                    BindingKind::Require,
                );
            }
            BindingPattern::ObjectPattern(op) => {
                for prop in &op.properties {
                    // `{ a }` → `a` (shorthand) ; `{ a: b }` → `b` (renamed)
                    self.bind_pattern(&prop.value, specifier);
                }
            }
            // Array/AssignmentPattern : non pertinents pour les bindings de require()
            _ => {}
        }
    }
}

fn extract_require_arg<'a>(call: &oxc_ast::ast::CallExpression<'a>) -> Option<String> {
    let is_require =
        matches!(&call.callee, Expression::Identifier(id) if id.name.as_str() == "require");
    if !is_require {
        return None;
    }
    let oxc_ast::ast::Argument::StringLiteral(lit) = call.arguments.first()? else {
        return None;
    };
    Some(lit.value.as_str().to_string())
}

impl<'a> Visit<'a> for GraphCollect {
    fn visit_import_declaration(&mut self, it: &oxc_ast::ast::ImportDeclaration<'a>) {
        let specifier = it.source.value.as_str().to_string();
        let Some(specs) = it.specifiers.as_ref() else {
            // import "foo" (side-effect) — pas de binding local.
            return;
        };
        for s in specs {
            match s {
                ImportDeclarationSpecifier::ImportDefaultSpecifier(d) => {
                    self.add(
                        d.local.name.as_str().to_string(),
                        specifier.clone(),
                        BindingKind::Default,
                    );
                }
                ImportDeclarationSpecifier::ImportSpecifier(n) => {
                    let imported = n.imported.name().as_str().to_string();
                    self.add(
                        n.local.name.as_str().to_string(),
                        specifier.clone(),
                        BindingKind::Named { imported },
                    );
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => {
                    self.add(
                        ns.local.name.as_str().to_string(),
                        specifier.clone(),
                        BindingKind::Namespace,
                    );
                }
            }
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        self.handle_variable_declarator(it);
        oxc_ast_visit::walk::walk_variable_declarator(self, it);
    }
}

/// Construit le graphe d'imports d'un fichier JS/TS.
///
/// Couvre :
/// - `import x from "foo"` (Default)
/// - `import { a, b as c } from "foo"` (Named)
/// - `import * as ns from "foo"` (Namespace)
/// - `const x = require("foo")` (Require, identifier)
/// - `const { a, b: c } = require("foo")` (Require, destructuring)
///
/// Hors scope (Phase 2) : require() via membre (`require("foo").default`),
/// require() dynamique (argument variable). Ces formes laissent le binding
/// non résolu — `resolves()` retournera `false` et le finding api/* sera
/// silencieux par défaut. Le pattern dynamique est un cas Phase 5
/// (rule `globals/require-dynamic`).
pub fn build_import_graph(path: &str, source: &str) -> ImportGraph {
    let allocator = Allocator::default();
    let st = source_type_from_path(path);
    let ret = Parser::new(&allocator, source, st).parse();

    let mut collect = GraphCollect {
        graph: ImportGraph::new(),
    };

    // Statements top-level — couvre le cas commun (imports en tête de fichier
    // ou require() au top-level/dans un bloc). Le visitor traverse récursivement.
    for stmt in &ret.program.body {
        match stmt {
            Statement::ImportDeclaration(d) => collect.visit_import_declaration(d),
            Statement::VariableDeclaration(d) => {
                for decl in &d.declarations {
                    collect.visit_variable_declarator(decl);
                }
            }
            // Pour les require() imbriqués (try/catch, fonction d'init), on
            // descend via le walker générique.
            other => oxc_ast_visit::walk::walk_statement(&mut collect, other),
        }
        // Visit the export-from declarations which can re-export bindings.
        if let Statement::ExportNamedDeclaration(end) = stmt {
            if let Some(Declaration::VariableDeclaration(vd)) = end.declaration.as_ref() {
                for decl in &vd.declarations {
                    collect.visit_variable_declarator(decl);
                }
            }
        }
    }

    collect.graph
}
