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

// Helpers pour rendre la sortie AI-friendly :
//   - docs_url(rule_id) : lien stable vers la doc Bun pertinente
//   - context_lines(source, line) : 3 lignes avant + ligne + 3 après
//   - byte_offset(line_offsets, line, col) : offset octet de (line, col) 1-based

use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 2;

/// Catégorie fonctionnelle d'une règle — dérivée de son id.
/// Permet aux agents de filtrer par angle (ex : "ne toucher qu'aux imports").
pub fn category(rule_id: &str) -> &'static str {
    if rule_id.starts_with("api/") {
        "api"
    } else if rule_id.starts_with("imports/") {
        "imports"
    } else if rule_id.starts_with("cli/") {
        "cli"
    } else if rule_id.starts_with("pkg/") {
        "package"
    } else if rule_id.starts_with("ci/") {
        "ci"
    } else if rule_id.starts_with("shebang/") {
        "shebang"
    } else if rule_id.starts_with("lock/") {
        "lockfile"
    } else if rule_id.starts_with("docker/") {
        "docker"
    } else if rule_id.starts_with("tsconfig/") {
        "tsconfig"
    } else if rule_id.starts_with("env/") {
        "env"
    } else if rule_id.starts_with("workspace/") {
        "workspace"
    } else if rule_id.starts_with("husky/") {
        "husky"
    } else if rule_id.starts_with("bunfig/") {
        "bunfig"
    } else if rule_id.starts_with("ecosystem/") {
        "ecosystem"
    } else if rule_id.starts_with("next/") {
        "nextjs"
    } else if rule_id.starts_with("npmrc/") {
        "npmrc"
    } else if rule_id.starts_with("turbo/") {
        "turbo"
    } else if rule_id.starts_with("tauri/") {
        "tauri"
    } else if rule_id.starts_with("shadcn/") {
        "ui"
    } else {
        "other"
    }
}

/// Score de confiance 0.0-1.0. Les règles avec remplacement autofix et
/// signature discriminante ont un score élevé ; les conseils stylistiques
/// ou ambigus un score bas.
pub fn confidence(rule_id: &str, has_replacement: bool) -> f32 {
    match rule_id {
        // Extrêmement fiable : mapping littéral 1-1
        "shebang/node" | "lock/rival" => 1.0,
        _ if rule_id.starts_with("cli/") => 0.95,
        _ if rule_id.starts_with("pkg/") => 0.9,
        "imports/node-prefix" => 0.95,
        "imports/bun-native" => 0.8,
        _ if rule_id.starts_with("ci/") => 0.9,
        "docker/node-base" => 0.9,
        "tsconfig/bun-types" => 0.7,
        // api/ : dépend du type de match
        _ if rule_id.starts_with("api/fs-") => 0.85,
        _ if rule_id.starts_with("api/buffer-") => 0.75,
        _ if rule_id.starts_with("api/http-") => 0.6,
        "api/dirname-esm" | "api/filename-esm" => 0.85,
        "api/process-env"
        | "api/new-url-import-meta"
        | "api/performance-now"
        | "api/os-platform"
        | "api/os-homedir" => 0.3, // stylistique
        _ if rule_id.starts_with("workspace/") => 0.9,
        _ if rule_id.starts_with("husky/") => 0.95,
        _ if has_replacement => 0.75,
        _ => 0.5,
    }
}

pub const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/aphrody-code/n2b/main/schema/v2.json";

pub fn docs_url(rule_id: &str) -> &'static str {
    // Mapping préfixe → page Bun. On prend un lien stable par catégorie ;
    // les IDs précis (api/fs-readFileSync, etc.) sont trop nombreux pour
    // avoir chacun leur URL dédiée.
    match rule_id {
        // --- APIs ciblées : préférer les guides how-to (plus concrets que reference) ---
        "api/bcrypt-hash" | "api/bcrypt-compare" | "api/argon2-hash" => {
            "https://bun.sh/guides/util/hash-a-password"
        }
        "api/yaml-parse" | "api/yaml-stringify" => "https://bun.sh/guides/runtime/import-yaml",
        "api/json5-parse" | "api/json5-stringify" => "https://bun.sh/guides/runtime/import-json",
        "api/toml-parse" => "https://bun.sh/guides/runtime/import-toml",
        "api/marked-call" | "api/marked-parse" => "https://bun.sh/reference/bun/markdown",
        "api/escape-html" => "https://bun.sh/reference/bun/escapeHTML",
        "api/strip-ansi" | "api/string-width" | "api/slice-ansi" => "https://bun.sh/reference/bun",
        "api/which-call" => "https://bun.sh/reference/bun/which",
        "api/cron-schedule" | "api/cronjob-new" => "https://bun.sh/reference/bun/cron",
        "api/fast-deep-equal" => "https://bun.sh/guides/util/deep-equals",
        "api/pako-gzip" | "api/pako-gunzip" | "api/zlib-gzipSync" => {
            "https://bun.sh/guides/util/gzip"
        }
        "api/sleep-promise" => "https://bun.sh/guides/util/sleep",
        "api/uuid-v4" => "https://bun.sh/reference/bun/randomUUIDv7",
        "api/crypto-randomBytes" => {
            "https://developer.mozilla.org/docs/Web/API/Crypto/getRandomValues"
        }
        "api/http-createServer" => "https://bun.sh/guides/http/simple",
        "api/https-createServer" => "https://bun.sh/guides/http/tls",
        "api/express-app" => "https://bun.sh/guides/ecosystem/express",
        "api/fastify-app" => "https://bun.sh/guides",
        "api/koa-new" | "api/http-request" | "api/https-request" => {
            "https://bun.sh/guides/http/fetch"
        }
        "api/execSync" | "api/exec" | "api/child-process-spawn" => {
            "https://bun.sh/guides/process/spawn"
        }
        "api/fs-readFileSync" | "api/fs-readFile-utf8" | "api/fs-readFile-promise" => {
            "https://bun.sh/guides/read-file/string"
        }
        "api/fs-writeFileSync" => "https://bun.sh/guides/write-file/basic",
        "api/fs-existsSync" => "https://bun.sh/guides/read-file/exists",
        "api/json-parse-readFileSync" => "https://bun.sh/guides/read-file/json",
        "api/crypto-createHash" => "https://bun.sh/docs/api/hashing",
        "api/util-inspect" => "https://bun.sh/reference/bun/inspect",
        "api/eventsource-new" => "https://bun.sh/reference/bun/EventSource",
        "api/cookie-parse" | "api/cookie-serialize" => "https://bun.sh/reference/bun/Cookie",
        "api/aws-sdk-s3-client" => "https://bun.sh/reference/bun/S3Client",
        "api/file-based-routing" => "https://bun.sh/reference/bun/FileSystemRouter",
        "api/chalk-call" => "https://bun.sh/reference/bun/color",
        "api/process-hrtime-bigint" => "https://bun.sh/reference/bun/nanoseconds",
        "bunfig/registry-npmjs" | "bunfig/option-note" | "bunfig/unknown-option" => {
            "https://bun.sh/docs/runtime/bunfig"
        }
        "tsconfig/module-legacy"
        | "tsconfig/target-legacy"
        | "tsconfig/module-detection"
        | "tsconfig/verbatim-module-syntax"
        | "tsconfig/allow-ts-extensions"
        | "tsconfig/no-emit"
        | "tsconfig/duplicate-node-types" => "https://bun.sh/docs/typescript",
        _ if rule_id.starts_with("api/fs-")
            || rule_id.starts_with("api/json-parse-readFileSync") =>
        {
            "https://bun.sh/docs/api/file-io"
        }
        _ if rule_id.starts_with("api/http-") || rule_id.starts_with("api/https-") => {
            "https://bun.sh/docs/api/http"
        }
        _ if rule_id.starts_with("api/exec") || rule_id.starts_with("api/child-process") => {
            "https://bun.sh/docs/api/spawn"
        }
        "api/buffer-alloc"
        | "api/buffer-concat"
        | "api/buffer-from-string"
        | "api/buffer-from-base64"
        | "api/buffer-byteLength" => "https://bun.sh/docs/api/binary-data",
        "api/process-env" => "https://bun.sh/docs/runtime/env",
        "api/dirname-esm"
        | "api/filename-esm"
        | "api/fileURLToPath"
        | "api/new-url-import-meta"
        | "api/path-join-dirname" => "https://bun.sh/docs/api/import-meta",
        "api/performance-now" => "https://bun.sh/docs/api/utils#bun-nanoseconds",
        "api/util-promisify" | "api/set-immediate" => "https://bun.sh/docs/runtime/nodejs-apis",
        "api/semver" => "https://bun.sh/docs/api/semver",
        "api/process-stdout-write" | "api/process-stderr-write" => {
            "https://bun.sh/docs/api/console"
        }
        "api/require-resolve" => "https://bun.sh/docs/runtime/modules",
        "api/os-platform" | "api/os-homedir" => "https://bun.sh/docs/runtime/nodejs-apis",
        "api/express-server" => "https://bun.sh/docs/api/http",
        _ if rule_id.starts_with("imports/node-prefix") => {
            "https://bun.sh/docs/runtime/nodejs-apis"
        }
        _ if rule_id.starts_with("imports/bun-native") => "https://bun.sh/docs/runtime/modules",
        _ if rule_id.starts_with("cli/") => "https://bun.sh/docs/cli/run",
        _ if rule_id.starts_with("pkg/") => "https://bun.sh/docs/cli/install",
        _ if rule_id.starts_with("ci/") => "https://github.com/oven-sh/setup-bun",
        "shebang/node" => "https://bun.sh/docs/cli/run#shebangs",
        "lock/rival" => "https://bun.sh/docs/install/lockfile",
        _ if rule_id.starts_with("workspace/") => "https://bun.sh/docs/install/workspaces",
        _ if rule_id.starts_with("husky/") => "https://bun.sh/docs/cli/run",
        _ if rule_id.starts_with("ecosystem/") => "https://bun.sh/guides/ecosystem",
        _ if rule_id.starts_with("next/") => "https://bun.sh/guides/ecosystem/nextjs",
        _ if rule_id.starts_with("npmrc/") => "https://bun.sh/docs/runtime/bunfig",
        _ if rule_id.starts_with("turbo/") => "https://turborepo.com/docs/reference/configuration",
        _ if rule_id.starts_with("tauri/") => "https://v2.tauri.app/reference/config/",
        _ if rule_id.starts_with("shadcn/") => "https://ui.shadcn.com/docs/components-json",
        _ => "https://bun.sh/docs",
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Context {
    pub before: Vec<String>,
    pub line: String,
    pub after: Vec<String>,
}

/// 3 lignes avant + ligne cible + 3 après. `line` est 1-based.
pub fn context_lines(source: &str, line: u32) -> Context {
    let lines: Vec<&str> = source.split('\n').collect();
    let i = (line as usize)
        .saturating_sub(1)
        .min(lines.len().saturating_sub(1));
    let before_start = i.saturating_sub(3);
    let after_end = (i + 4).min(lines.len());
    let cur = lines.get(i).copied().unwrap_or("");
    Context {
        before: lines[before_start..i]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        line: cur.to_string(),
        after: if i + 1 < after_end {
            lines[i + 1..after_end]
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        },
    }
}

/// Convertit (line, col) 1-based UTF-8 bytes → offset absolu en octets.
pub fn byte_offset(line_offsets: &[u32], line: u32, col: u32) -> u32 {
    let col = col.saturating_sub(1);
    if line <= 1 {
        col
    } else {
        let idx = (line as usize).saturating_sub(2);
        // line_offsets[idx] = position du '\n' qui termine la ligne (line-1).
        // Donc début de `line` = cette position + 1.
        let start = line_offsets
            .get(idx)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        start + col
    }
}
