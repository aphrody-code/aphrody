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

//! `n2b bunpp <sub>` — automatise la couverture bun++ des gaps Node.js.
//!
//! Scaffolde, liste, synchronise les polyfills `@bun++/node-<module>`.
//!
//! Sous-commandes :
//!   - `scaffold <module>`  — génère un package polyfill (template dédié)
//!   - `scaffold-all`       — génère tous les polyfills canary manquants
//!   - `status`             — rapporte coverage vs gaps canary
//!   - `sync`               — met à jour NODE_GAPS.md depuis issues Bun (gh)
//!   - `doctor`             — vérifie gh / bun / jq
//!
//! Refs :
//!   - bun++ NODE_GAPS.md (snapshot canary 1.3.13, 2026-04-17)
//!   - https://github.com/oven-sh/bun/issues/159
//!   - https://bun.com/docs/runtime/nodejs-apis

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub enum BunppCmd {
    Scaffold {
        module: String,
        root: PathBuf,
        force: bool,
    },
    ScaffoldAll {
        root: PathBuf,
        force: bool,
    },
    Status {
        root: PathBuf,
    },
    Sync {
        root: PathBuf,
        dry_run: bool,
    },
    Doctor,
}

pub fn run(cmd: BunppCmd, quiet: bool) -> Result<()> {
    match cmd {
        BunppCmd::Scaffold {
            module,
            root,
            force,
        } => scaffold_one(&module, &root, force, quiet),
        BunppCmd::ScaffoldAll { root, force } => scaffold_all(&root, force, quiet),
        BunppCmd::Status { root } => status(&root),
        BunppCmd::Sync { root, dry_run } => sync(&root, dry_run, quiet),
        BunppCmd::Doctor => doctor(quiet),
    }
}

/// Snapshot canary 1.3.13 — gaps confirmés nécessitant polyfill userland.
/// Source : bun++/NODE_GAPS.md (2026-04-17).
pub const CANARY_GAPS: &[CanaryGap] = &[
    CanaryGap {
        pkg: "node-sqlite",
        module: "node:sqlite",
        priority: "P1",
        issue: Some(20412),
        description: "Wrapper bun:sqlite.Database → DatabaseSync (Node v22.5+)",
    },
    CanaryGap {
        pkg: "node-util-ext",
        module: "node:util",
        priority: "P1",
        issue: Some(22872),
        description: "getCallSite(s), getSystemErrorMap/Message, transferableAbortSignal/Controller",
    },
    CanaryGap {
        pkg: "node-tls-secure-pair",
        module: "node:tls",
        priority: "P3",
        issue: None,
        description: "tls.createSecurePair (déprécié Node, requis compat)",
    },
    CanaryGap {
        pkg: "node-process-ext",
        module: "node:process",
        priority: "P1",
        issue: Some(23345),
        description: "process.loadEnvFile (parse .env → Bun.env)",
    },
    CanaryGap {
        pkg: "node-domain-active",
        module: "node:domain",
        priority: "P2",
        issue: None,
        description: "domain.active getter via AsyncLocalStorage",
    },
    CanaryGap {
        pkg: "node-v8-measure",
        module: "node:v8",
        priority: "P3",
        issue: None,
        description: "v8.measureMemory via bun:jsc heapStats",
    },
];

pub struct CanaryGap {
    pub pkg: &'static str,
    pub module: &'static str,
    pub priority: &'static str,
    pub issue: Option<u32>,
    pub description: &'static str,
}

fn scaffold_one(module: &str, root: &Path, force: bool, quiet: bool) -> Result<()> {
    let pkg = normalize_pkg_name(module);
    let gap = CANARY_GAPS
        .iter()
        .find(|g| g.pkg == pkg || g.module == module);
    let target = root.join("packages").join(&pkg);
    if target.exists() && !force {
        anyhow::bail!("{} existe déjà — relancer avec --force", target.display());
    }
    std::fs::create_dir_all(&target)?;

    let (index_src, test_src) = template_for(&pkg);
    write(
        target.join("package.json"),
        &render_package_json(&pkg, gap),
        quiet,
    )?;
    write(target.join("index.ts"), index_src, quiet)?;
    write(target.join(format!("{pkg}.test.ts")), test_src, quiet)?;
    write(target.join("README.md"), &render_readme(&pkg, gap), quiet)?;
    if !quiet {
        eprintln!("[bunpp] ✓ @bun++/{pkg} scaffolded → {}", target.display());
    }
    Ok(())
}

fn scaffold_all(root: &Path, force: bool, quiet: bool) -> Result<()> {
    let mut created = 0;
    let mut skipped = 0;
    for gap in CANARY_GAPS {
        let target = root.join("packages").join(gap.pkg);
        if target.exists() && !force {
            if !quiet {
                eprintln!("[bunpp] skip @bun++/{} (existe déjà)", gap.pkg);
            }
            skipped += 1;
            continue;
        }
        scaffold_one(gap.pkg, root, force, quiet)?;
        created += 1;
    }
    if !quiet {
        eprintln!(
            "[bunpp scaffold-all] {created} créés, {skipped} skippés (total canary gaps : {})",
            CANARY_GAPS.len()
        );
    }
    Ok(())
}

fn status(root: &Path) -> Result<()> {
    let pkgs_dir = root.join("packages");
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for gap in CANARY_GAPS {
        if pkgs_dir.join(gap.pkg).exists() {
            present.push(gap);
        } else {
            missing.push(gap);
        }
    }
    let total = CANARY_GAPS.len();
    let covered = present.len();
    let pct = (covered as f32 / total as f32) * 100.0;
    println!("# bun++ canary coverage");
    println!();
    println!("**Coverage : {covered}/{total} ({pct:.0}%)**");
    println!();
    println!("## ✓ Livrés ({covered})");
    for g in &present {
        println!("- `@bun++/{}` — {} [{}]", g.pkg, g.description, g.priority);
    }
    println!();
    println!("## ✗ Manquants ({})", missing.len());
    for g in &missing {
        let issue = g
            .issue
            .map(|n| format!(" (oven-sh/bun#{n})"))
            .unwrap_or_default();
        println!(
            "- `@bun++/{}` — {} [{}]{issue}",
            g.pkg, g.description, g.priority
        );
    }
    println!();
    println!(
        "Action : `n2b bunpp scaffold-all --root {}`",
        root.display()
    );
    Ok(())
}

fn sync(root: &Path, dry_run: bool, quiet: bool) -> Result<()> {
    if !which("gh") {
        anyhow::bail!("gh CLI introuvable — installer https://cli.github.com/");
    }
    if !quiet {
        eprintln!("[bunpp sync] fetch issues open oven-sh/bun liées aux gaps canary…");
    }
    let mut updates = Vec::new();
    for gap in CANARY_GAPS {
        if let Some(issue) = gap.issue {
            let out = Command::new("gh")
                .args([
                    "issue",
                    "view",
                    &issue.to_string(),
                    "-R",
                    "oven-sh/bun",
                    "--json",
                    "state,title,url",
                ])
                .output()
                .context("gh issue view")?;
            if out.status.success() {
                let body = String::from_utf8_lossy(&out.stdout);
                updates.push(format!(
                    "- `@bun++/{}` → #{issue} : {}",
                    gap.pkg,
                    body.trim()
                ));
            }
        }
    }
    let report = format!("# bun++ sync\n\n{}\n", updates.join("\n"));
    if dry_run {
        println!("{report}");
    } else {
        let out = root.join("SYNC_REPORT.md");
        std::fs::write(&out, report)?;
        if !quiet {
            eprintln!("[bunpp sync] → {}", out.display());
        }
    }
    Ok(())
}

fn doctor(quiet: bool) -> Result<()> {
    let tools = [
        ("gh", "GitHub CLI"),
        ("bun", "Bun runtime"),
        ("jq", "JSON query"),
    ];
    let mut missing = Vec::new();
    for (bin, label) in tools {
        let present = which(bin);
        if !quiet {
            let mark = if present { "✓" } else { "✗" };
            eprintln!("[bunpp doctor] {mark} {bin} ({label})");
        }
        if !present {
            missing.push(bin);
        }
    }
    if missing.is_empty() {
        if !quiet {
            eprintln!("[bunpp doctor] ✓ OK");
        }
        Ok(())
    } else {
        anyhow::bail!("manque : {}", missing.join(", "))
    }
}

// ─── Templates ────────────────────────────────────────────────────────────

fn template_for(pkg: &str) -> (&'static str, &'static str) {
    match pkg {
        "node-sqlite" => (TPL_SQLITE_INDEX, TPL_SQLITE_TEST),
        "node-util-ext" => (TPL_UTIL_EXT_INDEX, TPL_UTIL_EXT_TEST),
        "node-tls-secure-pair" => (TPL_TLS_SECURE_PAIR_INDEX, TPL_TLS_SECURE_PAIR_TEST),
        "node-process-ext" => (TPL_PROCESS_EXT_INDEX, TPL_PROCESS_EXT_TEST),
        "node-domain-active" => (TPL_DOMAIN_ACTIVE_INDEX, TPL_DOMAIN_ACTIVE_TEST),
        "node-v8-measure" => (TPL_V8_MEASURE_INDEX, TPL_V8_MEASURE_TEST),
        _ => (TPL_GENERIC_INDEX, TPL_GENERIC_TEST),
    }
}

fn normalize_pkg_name(module: &str) -> String {
    if let Some(rest) = module.strip_prefix("node:") {
        format!("node-{rest}")
    } else if module.starts_with("node-") {
        module.to_string()
    } else {
        format!("node-{module}")
    }
}

fn render_package_json(pkg: &str, gap: Option<&CanaryGap>) -> String {
    let desc = gap
        .map(|g| g.description.to_string())
        .unwrap_or_else(|| format!("bun++ polyfill for {pkg}"));
    format!(
        r#"{{
  "name": "@bun++/{pkg}",
  "version": "0.1.0",
  "description": "{desc}",
  "type": "module",
  "main": "./index.ts",
  "exports": {{ ".": "./index.ts" }},
  "scripts": {{
    "test": "bun test"
  }},
  "devDependencies": {{ "@types/bun": "latest" }},
  "engines": {{ "bun": ">=1.2.0" }},
  "sideEffects": false,
  "keywords": ["bun", "bun++", "node-compat", "polyfill"],
  "license": "MIT"
}}
"#
    )
}

fn render_readme(pkg: &str, gap: Option<&CanaryGap>) -> String {
    let (desc, prio, issue) = match gap {
        Some(g) => (
            g.description.to_string(),
            g.priority.to_string(),
            g.issue
                .map(|n| format!("oven-sh/bun#{n}"))
                .unwrap_or_default(),
        ),
        None => (format!("Polyfill for {pkg}"), "P3".into(), String::new()),
    };
    let issue_line = if issue.is_empty() {
        String::new()
    } else {
        format!("\n**Upstream :** {issue}\n")
    };
    format!(
        r#"# @bun++/{pkg}

{desc}

**Priorité :** {prio}{issue_line}

## Install

```bash
bun add @bun++/{pkg}
```

## Usage

```ts
import * as polyfill from "@bun++/{pkg}";
```

Voir `{pkg}.test.ts` pour les cas d'usage.

## Statut

Scaffolded par `n2b bunpp scaffold {pkg}` — remplacer les stubs par l'implémentation réelle.
"#
    )
}

// ─── Index templates ──────────────────────────────────────────────────────

const TPL_GENERIC_INDEX: &str = r#"// bun++ generic polyfill stub — compléter avec l'implémentation réelle.
export const __polyfill__ = true;
"#;

const TPL_GENERIC_TEST: &str = r#"import { expect, test } from "bun:test";
import { __polyfill__ } from "./index";

test("polyfill marker", () => {
  expect(__polyfill__).toBe(true);
});
"#;

const TPL_SQLITE_INDEX: &str = r#"// bun++/node-sqlite — remap bun:sqlite.Database vers l'API node:sqlite.DatabaseSync (Node v22.5+).
// Drop-in pour code qui importe `node:sqlite`.
//
// Node docs : https://nodejs.org/api/sqlite.html
// Bun docs  : https://bun.com/docs/runtime/sqlite

import { Database } from "bun:sqlite";

export interface DatabaseSyncOptions {
  open?: boolean;
  readOnly?: boolean;
  enableForeignKeyConstraints?: boolean;
}

export class DatabaseSync {
  #db: Database;
  readonly location: string | ":memory:";

  constructor(location: string = ":memory:", opts: DatabaseSyncOptions = {}) {
    this.location = location;
    this.#db = opts.readOnly
      ? new Database(location, { readonly: true })
      : new Database(location);
    if (opts.enableForeignKeyConstraints !== false) {
      this.#db.exec("PRAGMA foreign_keys = ON");
    }
  }

  exec(sql: string): void {
    this.#db.exec(sql);
  }

  prepare(sql: string) {
    const stmt = this.#db.prepare(sql);
    return {
      all: (...args: unknown[]) => stmt.all(...(args as never[])),
      get: (...args: unknown[]) => stmt.get(...(args as never[])),
      run: (...args: unknown[]) => stmt.run(...(args as never[])),
      iterate: (...args: unknown[]) => stmt.iterate(...(args as never[])),
      finalize: () => stmt.finalize(),
    };
  }

  close(): void {
    this.#db.close();
  }

  get isOpen(): boolean {
    return this.#db !== undefined;
  }
}

export default { DatabaseSync };
"#;

const TPL_SQLITE_TEST: &str = r#"import { describe, expect, test } from "bun:test";
import { DatabaseSync } from "./index";

describe("@bun++/node-sqlite", () => {
  test("open :memory: + exec + prepare", () => {
    const db = new DatabaseSync(":memory:");
    db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)");
    db.prepare("INSERT INTO t (name) VALUES (?)").run("alice");
    const row = db.prepare("SELECT name FROM t WHERE id = ?").get(1) as { name: string };
    expect(row.name).toBe("alice");
    db.close();
  });

  test("foreign_keys on by default", () => {
    const db = new DatabaseSync();
    const r = db.prepare("PRAGMA foreign_keys").get() as { foreign_keys: number };
    expect(r.foreign_keys).toBe(1);
    db.close();
  });
});
"#;

const TPL_UTIL_EXT_INDEX: &str = r#"// bun++/node-util-ext — compléments `node:util` manquants sur Bun canary 1.3.13.
//
// Expose :
//   - getCallSite / getCallSites  (stack trace introspection, Node v22.10+)
//   - getSystemErrorMap / getSystemErrorMessage  (errno → description)
//   - transferableAbortSignal / transferableAbortController  (structured clone)
//
// Node docs : https://nodejs.org/api/util.html
// Upstream  : oven-sh/bun#22872

export interface CallSite {
  functionName: string;
  scriptName: string;
  lineNumber: number;
  column: number;
}

export function getCallSite(frameCount = 10): CallSite[] {
  return getCallSites(frameCount);
}

export function getCallSites(frameCount = 10): CallSite[] {
  const orig = Error.prepareStackTrace;
  Error.prepareStackTrace = (_, stack) => stack;
  const err = new Error();
  Error.captureStackTrace(err, getCallSites);
  const stack = err.stack as unknown as NodeJS.CallSite[];
  Error.prepareStackTrace = orig;
  return stack.slice(0, frameCount).map((s) => ({
    functionName: s.getFunctionName() ?? "<anonymous>",
    scriptName: s.getFileName() ?? "<unknown>",
    lineNumber: s.getLineNumber() ?? 0,
    column: s.getColumnNumber() ?? 0,
  }));
}

// errno → nom POSIX (sous-ensemble minimal ; compléter avec /usr/include/asm-generic/errno-base.h)
const ERRNO_MAP: Record<number, [string, string]> = {
  1: ["EPERM", "Operation not permitted"],
  2: ["ENOENT", "No such file or directory"],
  5: ["EIO", "Input/output error"],
  9: ["EBADF", "Bad file descriptor"],
  11: ["EAGAIN", "Resource temporarily unavailable"],
  12: ["ENOMEM", "Cannot allocate memory"],
  13: ["EACCES", "Permission denied"],
  17: ["EEXIST", "File exists"],
  22: ["EINVAL", "Invalid argument"],
  24: ["EMFILE", "Too many open files"],
  32: ["EPIPE", "Broken pipe"],
  98: ["EADDRINUSE", "Address already in use"],
  104: ["ECONNRESET", "Connection reset by peer"],
  110: ["ETIMEDOUT", "Connection timed out"],
  111: ["ECONNREFUSED", "Connection refused"],
};

export function getSystemErrorMap(): Map<number, [string, string]> {
  return new Map(Object.entries(ERRNO_MAP).map(([k, v]) => [Number(k), v]));
}

export function getSystemErrorMessage(errno: number): string {
  return ERRNO_MAP[errno]?.[1] ?? `Unknown error ${errno}`;
}

export function transferableAbortSignal(signal: AbortSignal): AbortSignal {
  const ch = new MessageChannel();
  signal.addEventListener("abort", () => ch.port1.postMessage("abort"));
  return signal;
}

export function transferableAbortController(): AbortController {
  return new AbortController();
}
"#;

const TPL_UTIL_EXT_TEST: &str = r#"import { describe, expect, test } from "bun:test";
import {
  getCallSites,
  getSystemErrorMap,
  getSystemErrorMessage,
  transferableAbortController,
} from "./index";

describe("@bun++/node-util-ext", () => {
  test("getCallSites returns frames", () => {
    const frames = getCallSites(5);
    expect(frames.length).toBeGreaterThan(0);
    expect(frames[0]!.scriptName).toContain("test");
  });

  test("errno map : ENOENT = 2", () => {
    expect(getSystemErrorMap().get(2)).toEqual(["ENOENT", "No such file or directory"]);
    expect(getSystemErrorMessage(2)).toBe("No such file or directory");
  });

  test("transferableAbortController fires", () => {
    const ctrl = transferableAbortController();
    let fired = false;
    ctrl.signal.addEventListener("abort", () => (fired = true));
    ctrl.abort();
    expect(fired).toBe(true);
  });
});
"#;

const TPL_TLS_SECURE_PAIR_INDEX: &str = r#"// bun++/node-tls-secure-pair — polyfill `tls.createSecurePair` (déprécié mais requis compat).
//
// Node docs : https://nodejs.org/api/tls.html#tlscreatesecurepaircontext-isserver-requestcert-rejectunauthorized-options

import type { Duplex } from "node:stream";
import { PassThrough } from "node:stream";

export interface SecurePair {
  encrypted: Duplex;
  cleartext: Duplex;
}

/** @deprecated utiliser `tls.TLSSocket` directement. */
export function createSecurePair(): SecurePair {
  // Stub minimal : deux PassThrough chaînés. Remplacer par tls.TLSSocket wrapper
  // quand une vraie négociation TLS est requise.
  const encrypted = new PassThrough();
  const cleartext = new PassThrough();
  encrypted.pipe(cleartext);
  cleartext.pipe(encrypted);
  return { encrypted, cleartext };
}
"#;

const TPL_TLS_SECURE_PAIR_TEST: &str = r#"import { expect, test } from "bun:test";
import { createSecurePair } from "./index";

test("createSecurePair returns { encrypted, cleartext }", () => {
  const pair = createSecurePair();
  expect(pair.encrypted).toBeDefined();
  expect(pair.cleartext).toBeDefined();
});
"#;

const TPL_PROCESS_EXT_INDEX: &str = r##"// bun++/node-process-ext — `process.loadEnvFile` manquant sur Bun canary 1.3.13.
//
// Node docs : https://nodejs.org/api/process.html#processloadenvfilepath
// Upstream  : oven-sh/bun#23345

import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

export function loadEnvFile(path: string = ".env"): void {
  const p = resolve(path);
  if (!existsSync(p)) {
    const err = new Error(`ENOENT: no such file or directory, open '${p}'`) as NodeJS.ErrnoException;
    err.code = "ENOENT";
    throw err;
  }
  const content = readFileSync(p, "utf8");
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq === -1) continue;
    const key = line.slice(0, eq).trim();
    let val = line.slice(eq + 1).trim();
    // strip quotes
    if ((val.startsWith("\"") && val.endsWith("\"")) || (val.startsWith("'") && val.endsWith("'"))) {
      val = val.slice(1, -1);
    }
    if (!(key in process.env)) {
      process.env[key] = val;
    }
  }
}
"##;

const TPL_PROCESS_EXT_TEST: &str = r##"import { afterEach, expect, test } from "bun:test";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { loadEnvFile } from "./index";

let dir: string;
afterEach(() => dir && rmSync(dir, { recursive: true, force: true }));

test("loadEnvFile parses KEY=VALUE", () => {
  dir = mkdtempSync(join(tmpdir(), "envfile-"));
  const p = join(dir, ".env");
  writeFileSync(p, "FOO=bar\n# comment\nQUOTED=\"hello world\"\n");
  loadEnvFile(p);
  expect(process.env.FOO).toBe("bar");
  expect(process.env.QUOTED).toBe("hello world");
});

test("loadEnvFile throws ENOENT on missing", () => {
  expect(() => loadEnvFile("/nonexistent-" + Date.now())).toThrow();
});
"##;

const TPL_DOMAIN_ACTIVE_INDEX: &str = r#"// bun++/node-domain-active — expose `domain.active` via AsyncLocalStorage.
//
// Node docs : https://nodejs.org/api/domain.html (déprécié mais requis compat).

import { AsyncLocalStorage } from "node:async_hooks";

const als = new AsyncLocalStorage<Domain>();

export class Domain {
  readonly members: unknown[] = [];
  run<T>(fn: () => T): T {
    return als.run(this, fn);
  }
  enter(): void {
    // no-op : utiliser `run(fn)` pour contexte scopé.
  }
  exit(): void {
    // no-op
  }
  add(obj: unknown): void {
    this.members.push(obj);
  }
}

export function create(): Domain {
  return new Domain();
}

export function getActive(): Domain | undefined {
  return als.getStore();
}

// Accès compatible Node : `domain.active` est un getter sur l'export default.
export default {
  Domain,
  create,
  get active() {
    return getActive();
  },
};
"#;

const TPL_DOMAIN_ACTIVE_TEST: &str = r#"import { expect, test } from "bun:test";
import { create, getActive } from "./index";

test("getActive undefined outside run()", () => {
  expect(getActive()).toBeUndefined();
});

test("domain.run scopes active", () => {
  const d = create();
  d.run(() => {
    expect(getActive()).toBe(d);
  });
  expect(getActive()).toBeUndefined();
});
"#;

const TPL_V8_MEASURE_INDEX: &str = r#"// bun++/node-v8-measure — `v8.measureMemory` via bun:jsc.heapStats.
//
// Node docs : https://nodejs.org/api/v8.html#v8measurememoryoptions-execution

import { heapStats } from "bun:jsc";

export interface MemoryMeasurement {
  total: { jsMemoryEstimate: number; jsMemoryRange: [number, number] };
  current: { jsMemoryEstimate: number; jsMemoryRange: [number, number] };
  other: { jsMemoryEstimate: number; jsMemoryRange: [number, number] }[];
}

export async function measureMemory(): Promise<MemoryMeasurement> {
  const s = heapStats();
  const est = s.heapSize;
  const range: [number, number] = [s.heapSize, s.heapCapacity ?? s.heapSize];
  const bucket = { jsMemoryEstimate: est, jsMemoryRange: range };
  return { total: bucket, current: bucket, other: [] };
}
"#;

const TPL_V8_MEASURE_TEST: &str = r#"import { expect, test } from "bun:test";
import { measureMemory } from "./index";

test("measureMemory returns buckets", async () => {
  const m = await measureMemory();
  expect(m.total.jsMemoryEstimate).toBeGreaterThan(0);
  expect(m.current.jsMemoryEstimate).toBeGreaterThan(0);
  expect(Array.isArray(m.other)).toBe(true);
});
"#;

// ─── Utilitaires ──────────────────────────────────────────────────────────

fn write(path: PathBuf, content: &str, quiet: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content).with_context(|| format!("écriture {}", path.display()))?;
    if !quiet {
        eprintln!("  + {}", path.display());
    }
    Ok(())
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
