// Copyright 2026 aphrody-code
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

// plugin.ts — Bun.plugin() that delegates to the Rust binary `n2b` for
// scanning. One `n2b --report=json` run at onStart populates a per-file
// finding map, then onLoad surfaces the relevant findings per file
// during the build. No TS scanner duplication.

import type { BunPlugin } from "bun";
import { scan, binaryVersion, type ScanOptions } from "@n2b/core";
import type { Finding, N2BReport } from "@n2b/types";

const DIM = "\x1b[2m";
const RESET = "\x1b[0m";
const CYAN = "\x1b[36m";
const YELLOW = "\x1b[33m";
const GREEN = "\x1b[32m";
const RED = "\x1b[31m";
const BOLD = "\x1b[1m";

export interface N2BPluginOptions extends Pick<ScanOptions, "ignore" | "bin" | "cwd"> {
  /** Root to scan. Defaults to Bun.env.PWD (the build's cwd). */
  root?: string;
  /** "warn" (default) logs findings but lets the build complete. "error" throws at onEnd. */
  onFindings?: "warn" | "error";
  /** Suppress per-file logs. Summary at onEnd is still printed. */
  quiet?: boolean;
  /** Pre-computed report (e.g. reused across watched builds). Skips the subprocess call. */
  report?: N2BReport;
}

/**
 * Create the n2b Bun plugin. Usage:
 *
 *     import { n2bPlugin } from "@n2b/core";
 *     Bun.plugin(n2bPlugin({ onFindings: "warn" }));
 *
 * The plugin runs `n2b` on the project root exactly once per build. All
 * business logic (scanners, rules) lives in the Rust binary ; this plugin
 * only renders findings from the JSON output.
 */
export function n2bPlugin(opts: N2BPluginOptions = {}): BunPlugin {
  const onFindings = opts.onFindings ?? "warn";
  const quiet = opts.quiet ?? false;

  // Per-file findings keyed by absolute path.
  let byPath = new Map<string, Finding[]>();
  let total = 0;
  let disabled = false;

  return {
    name: "n2b",

    async setup(build) {
      const root = opts.root ?? Bun.env.PWD ?? process.cwd();

      if (!opts.report) {
        try {
          await binaryVersion(opts.bin ?? "n2b");
        } catch {
          if (!quiet) {
            console.warn(
              `${YELLOW}[n2b]${RESET} binary \`${opts.bin ?? "n2b"}\` not found on PATH — plugin disabled`,
            );
          }
          disabled = true;
          return;
        }
      }

      const report = opts.report ?? (await scan(root, {
        mode: "check",
        ignore: opts.ignore,
        quiet: true,
        bin: opts.bin,
        cwd: opts.cwd,
      }));

      byPath = new Map();
      total = 0;
      for (const file of report.files) {
        if (file.findings.length === 0) continue;
        // Keys stored both relative (as n2b emits) and absolute for onLoad matching.
        byPath.set(file.path, file.findings);
        byPath.set(`${root}/${file.path}`, file.findings);
        total += file.findings.length;
      }

      build.onLoad({ filter: /\.[jt]sx?$|\.m[jt]s$|\.c[jt]s$/ }, ({ path }) => {
        if (disabled || quiet) return undefined;
        const findings = byPath.get(path);
        if (!findings) return undefined;
        const rel = path.startsWith(root) ? path.slice(root.length + 1) : path;
        for (const f of findings) {
          const loc = `${DIM}${rel}:${f.line}:${f.col}${RESET}`;
          const tag = `${CYAN}${f.rule_id}${RESET}`;
          const repl = f.replacement ? ` ${DIM}→${RESET} ${GREEN}${trunc(f.replacement, 60)}${RESET}` : "";
          console.warn(`${YELLOW}[n2b]${RESET} ${loc} ${tag} ${f.message}${repl}`);
        }
        return undefined;
      });

      build.onEnd(() => {
        if (disabled) return;
        if (total === 0) {
          if (!quiet) console.log(`${GREEN}[n2b]${RESET} no Node→Bun issues detected`);
          return;
        }
        const filesCount = new Set([...byPath.keys()].filter((k) => !k.startsWith("/"))).size;
        const mark = onFindings === "error" ? `${RED}✗${RESET}` : `${YELLOW}!${RESET}`;
        console.warn(
          `\n${mark} ${BOLD}[n2b]${RESET} ${total} finding(s) across ${filesCount} file(s)`,
        );
        if (onFindings === "error") {
          throw new Error(`[n2b] ${total} finding(s) — fix them or set onFindings: "warn"`);
        }
      });
    },
  };
}

function trunc(s: string, n: number): string {
  return s.length > n ? `${s.slice(0, n - 1)}…` : s;
}
