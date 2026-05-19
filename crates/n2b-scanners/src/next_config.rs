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

use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::make_finding;
use once_cell::sync::Lazy;
use regex::Regex;

/// Détecte les options connues de next.config.{js,mjs,ts} qui interagissent
/// avec le portage Bun. On raisonne à la regex (pas d'AST) — le fichier n'est
/// pas critique à modifier, seulement à annoter.
///
/// Référence : https://bun.sh/guides/ecosystem/nextjs · https://nextjs.org/docs
pub fn scan_next_config(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();

    // 1. output: 'standalone' — build Node-oriented, pas directement exécutable par `bun run`.
    static OUTPUT_STANDALONE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"output\s*:\s*['"]standalone['"]"#)
            .expect("invariant: OUTPUT_STANDALONE regex literal is valid")
    });
    if OUTPUT_STANDALONE.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/output-standalone",
            "output: 'standalone' produit un build Node-only (server.js) — reste exécutable via `bun run .next/standalone/server.js` mais n'exploite pas Bun.serve",
            "output: 'standalone'",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 2. webpack() custom — Turbopack est le bundler par défaut Next 16.
    static WEBPACK_FN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\bwebpack\s*(?:\(|:\s*(?:function|\())")
            .expect("invariant: WEBPACK_FN regex literal is valid")
    });
    if WEBPACK_FN.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/webpack-custom",
            "Fonction webpack() custom détectée — Turbopack est le bundler par défaut Next 16 (option `turbopack: {}`). Vérifier si le hook webpack est toujours nécessaire",
            "webpack(",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 3. experimental.serverComponentsExternalPackages
    static SERVER_EXTERNAL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"serverComponentsExternalPackages\s*:")
            .expect("invariant: SERVER_EXTERNAL regex literal is valid")
    });
    if SERVER_EXTERNAL.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/server-external-packages",
            "experimental.serverComponentsExternalPackages : vérifier si ces packages ont un équivalent Bun-native (Bun.sql, Bun.redis, Bun.S3Client...) qui éliminerait le flag",
            "serverComponentsExternalPackages",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4. Absence de turbopack: (flag ou config) — Next 16 l'active par défaut,
    //    mais l'absence de mention explicite quand on porte est worth noting.
    //    On ne flaggue que si `webpack` custom est présent (sinon silencieux).
    static TURBOPACK_FN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\bturbopack\s*:").expect("invariant: TURBOPACK_FN regex literal is valid")
    });
    if WEBPACK_FN.is_match(content) && !TURBOPACK_FN.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/turbopack-missing",
            "webpack() custom sans `turbopack: {}` — Next 16 préfère Turbopack ; ajouter `turbopack: {}` fige l'intention (même vide)",
            "turbopack",
            Some("turbopack: {}".to_string()),
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4.5 next-rspack : withRspack() enveloppe la config
    static WITH_RSPACK: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\bwithRspack\s*\(").expect("invariant: WITH_RSPACK regex literal is valid")
    });
    if WITH_RSPACK.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/rspack-wrapper",
            "withRspack() détecté — Next.js backed by Rspack (next-rspack). Compatible Bun runtime ; voir https://rspack.rs/guide/tech/next",
            "withRspack(",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4.6 turbopack: { rules: {...} } — custom loader rules
    static TURBO_RULES: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"turbopack\s*:\s*\{[\s\S]*?\brules\s*:")
            .expect("invariant: TURBO_RULES regex literal is valid")
    });
    if TURBO_RULES.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/turbopack-rules",
            "turbopack.rules détecté — vérifier que les loaders sont compat Turbopack (voir https://nextjs.org/docs/app/api-reference/turbopack)",
            "turbopack.rules",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4.7 turbopack.resolveAlias
    static TURBO_ALIAS: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"turbopack\s*:\s*\{[\s\S]*?\bresolveAlias\s*:")
            .expect("invariant: TURBO_ALIAS regex literal is valid")
    });
    if TURBO_ALIAS.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/turbopack-alias",
            "turbopack.resolveAlias détecté — équivalent de webpack.resolve.alias, porté vers Turbopack",
            "turbopack.resolveAlias",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4.8 transpilePackages — utile pour ESM-only packages
    static TRANSPILE_PKGS: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\btranspilePackages\s*:")
            .expect("invariant: TRANSPILE_PKGS regex literal is valid")
    });
    if TRANSPILE_PKGS.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/transpile-packages",
            "transpilePackages détecté — utile pour les packages ESM-only ; vérifier si toujours nécessaire (la plupart marchent out-of-box sous Bun/Next 16)",
            "transpilePackages",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4.9 compiler.styledComponents
    static SWC_STYLED: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"compiler\s*:\s*\{[\s\S]*?\bstyledComponents\s*:")
            .expect("invariant: SWC_STYLED regex literal is valid")
    });
    if SWC_STYLED.is_match(content) {
        findings.push(make_finding(
            path, &[], 0,
            "next/compiler-styled",
            "compiler.styledComponents activé — SWC transform (ssr + displayName), remplace babel-plugin-styled-components",
            "compiler.styledComponents",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4.10 compiler.emotion
    static SWC_EMOTION: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"compiler\s*:\s*\{[\s\S]*?\bemotion\s*:")
            .expect("invariant: SWC_EMOTION regex literal is valid")
    });
    if SWC_EMOTION.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/compiler-emotion",
            "compiler.emotion activé — SWC transform remplace @emotion/babel-plugin",
            "compiler.emotion",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4.11 compiler.removeConsole
    static SWC_REMOVE_CONSOLE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"compiler\s*:\s*\{[\s\S]*?\bremoveConsole\s*:")
            .expect("invariant: SWC_REMOVE_CONSOLE regex literal is valid")
    });
    if SWC_REMOVE_CONSOLE.is_match(content) {
        findings.push(make_finding(
            path, &[], 0,
            "next/compiler-remove-console",
            "compiler.removeConsole détecté — strip console.* en prod (exclude: ['error'] pour garder console.error)",
            "compiler.removeConsole",
            None,
            MakeFindingOpts { autofix: Some(false), severity: Some(Severity::Info), ..Default::default() },
        ));
    }

    // 4.12 compiler.reactRemoveProperties
    static SWC_REACT_REMOVE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"compiler\s*:\s*\{[\s\S]*?\breactRemoveProperties\s*:")
            .expect("invariant: SWC_REACT_REMOVE regex literal is valid")
    });
    if SWC_REACT_REMOVE.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/compiler-react-remove-props",
            "compiler.reactRemoveProperties — strip data-test-* props en prod",
            "compiler.reactRemoveProperties",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // 4.13 compiler.relay
    static SWC_RELAY: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"compiler\s*:\s*\{[\s\S]*?\brelay\s*:")
            .expect("invariant: SWC_RELAY regex literal is valid")
    });
    if SWC_RELAY.is_match(content) {
        findings.push(make_finding(
            path, &[], 0,
            "next/compiler-relay",
            "compiler.relay détecté — vérifier que artifactDirectory est hors de 'pages' (sinon les artefacts deviennent des routes)",
            "compiler.relay",
            None,
            MakeFindingOpts { autofix: Some(false), severity: Some(Severity::Info), ..Default::default() },
        ));
    }

    // 4.14 compiler.define / defineServer (Next 15+)
    static SWC_DEFINE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"compiler\s*:\s*\{[\s\S]*?\bdefine(?:Server)?\s*:")
            .expect("invariant: SWC_DEFINE regex literal is valid")
    });
    if SWC_DEFINE.is_match(content) {
        findings.push(make_finding(
            path, &[], 0,
            "next/compiler-define",
            "compiler.define / defineServer — remplacement statique de variables au build (Next 15+)",
            "compiler.define",
            None,
            MakeFindingOpts { autofix: Some(false), severity: Some(Severity::Info), ..Default::default() },
        ));
    }

    // 4.15 experimental.swcPlugins
    static SWC_PLUGINS: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"experimental\s*:\s*\{[\s\S]*?\bswcPlugins\s*:")
            .expect("invariant: SWC_PLUGINS regex literal is valid")
    });
    if SWC_PLUGINS.is_match(content) {
        findings.push(make_finding(
            path, &[], 0,
            "next/swc-plugins",
            "experimental.swcPlugins — plugins WASM SWC custom (API expérimentale, breaking changes possibles)",
            "experimental.swcPlugins",
            None,
            MakeFindingOpts { autofix: Some(false), severity: Some(Severity::Warn), ..Default::default() },
        ));
    }

    // 4.16 experimental.swcTraceProfiling
    static SWC_TRACE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"experimental\s*:\s*\{[\s\S]*?\bswcTraceProfiling\s*:")
            .expect("invariant: SWC_TRACE regex literal is valid")
    });
    if SWC_TRACE.is_match(content) {
        findings.push(make_finding(
            path, &[], 0,
            "next/swc-trace",
            "experimental.swcTraceProfiling — génère .next/swc-trace-profile-*.json (chrome://tracing, speedscope)",
            "experimental.swcTraceProfiling",
            None,
            MakeFindingOpts { autofix: Some(false), severity: Some(Severity::Info), ..Default::default() },
        ));
    }

    // 5. images.loader custom
    static IMAGES_CUSTOM: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"loader\s*:\s*['"]custom['"]"#)
            .expect("invariant: IMAGES_CUSTOM regex literal is valid")
    });
    if IMAGES_CUSTOM.is_match(content) {
        findings.push(make_finding(
            path,
            &[],
            0,
            "next/images-custom-loader",
            "images.loader: 'custom' détecté — vérifier que loaderFile tourne sous Bun (fetch + sharp sont supportés nativement)",
            "loader: 'custom'",
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    (findings, content.to_string())
}

/// Détection d'un fichier next.config à la racine ou imbriqué.
pub fn is_next_config(name: &str) -> bool {
    matches!(
        name,
        "next.config.js" | "next.config.mjs" | "next.config.ts" | "next.config.cjs"
    )
}
