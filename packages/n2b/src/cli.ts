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

// cli.ts — thin wrapper around the Rust binary `n2b`. All business logic
// lives on the Rust side ; this façade just spawns the subprocess, parses
// the JSON v2 output (schema/v2.json) and returns typed results.
//
// This is the canonical way to invoke n2b from TypeScript code — both
// for automation scripts and for the Bun plugin (packages/n2b/src/plugin.ts).
// No schema duplication: schema.ts is codegen'd from schema/v2.json.

import type { N2BReport, Mode } from "@n2b/types";

export interface ScanOptions {
  /** Scan mode. "check" is dry-run, "fix" applies safe autofixes, "aggressive" also applies API migrations. */
  mode?: Mode;
  /** Extra glob patterns to ignore. */
  ignore?: string[];
  /** Suppress stdout/stderr noise on the subprocess side. */
  quiet?: boolean;
  /** Override the `n2b` binary path. Defaults to `n2b` on $PATH. */
  bin?: string;
  /** Override the working directory of the subprocess. Defaults to `Bun.env.PWD`. */
  cwd?: string;
}

export interface N2BError extends Error {
  stderr: string;
  code: number;
}

function makeError(msg: string, stderr: string, code: number): N2BError {
  const err = new Error(`${msg}\n${stderr.trim()}`) as N2BError;
  err.stderr = stderr;
  err.code = code;
  return err;
}

async function runBin(
  bin: string,
  args: string[],
  cwd?: string,
): Promise<{ stdout: string; stderr: string; code: number }> {
  const proc = Bun.spawn([bin, ...args], {
    stdout: "pipe",
    stderr: "pipe",
    cwd,
  });
  const [stdout, stderr, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  return { stdout, stderr, code };
}

/**
 * Scan a directory with n2b. Equivalent to `n2b <root> --report=json [--fix|--aggressive] [--ignore=…]`.
 * Exit code 0 (no findings) and 1 (findings present in check mode) are both treated as success.
 * Exit code 2 throws an N2BError carrying stderr.
 */
export async function scan(root: string, opts: ScanOptions = {}): Promise<N2BReport> {
  const bin = opts.bin ?? "n2b";
  const args: string[] = [root, "--report=json"];
  if (opts.mode === "fix") args.push("--fix");
  else if (opts.mode === "aggressive") args.push("--aggressive");
  if (opts.quiet) args.push("--quiet");
  for (const g of opts.ignore ?? []) args.push(`--ignore=${g}`);

  const { stdout, stderr, code } = await runBin(bin, args, opts.cwd);
  if (code === 2) throw makeError(`n2b scan failed (exit ${code})`, stderr, code);
  return JSON.parse(stdout) as N2BReport;
}

/**
 * Rules list. Equivalent to `n2b rules --report=json`. Returns the raw JSON
 * payload (array of rule metadata). The exact shape is implementation-defined
 * and lives outside schema/v2.json for now.
 */
export async function rules(opts: Pick<ScanOptions, "bin" | "cwd"> = {}): Promise<unknown> {
  const bin = opts.bin ?? "n2b";
  const { stdout, stderr, code } = await runBin(bin, ["rules", "--report=json"], opts.cwd);
  if (code !== 0) throw makeError(`n2b rules failed (exit ${code})`, stderr, code);
  return JSON.parse(stdout);
}

/**
 * LLM prompt (markdown) for a given root. Equivalent to `n2b prompt <root>`.
 */
export async function promptMarkdown(
  root: string,
  opts: Pick<ScanOptions, "bin" | "cwd"> = {},
): Promise<string> {
  const bin = opts.bin ?? "n2b";
  const { stdout, stderr, code } = await runBin(bin, ["prompt", root], opts.cwd);
  if (code !== 0) throw makeError(`n2b prompt failed (exit ${code})`, stderr, code);
  return stdout;
}

/**
 * Quick check for the presence and version of the `n2b` binary.
 * Throws N2BError if absent. Returns semver string on success.
 */
export async function binaryVersion(bin = "n2b"): Promise<string> {
  const { stdout, stderr, code } = await runBin(bin, ["--version"]);
  if (code !== 0) throw makeError(`n2b --version failed (exit ${code})`, stderr, code);
  const m = stdout.trim().match(/(\d+\.\d+\.\d+[^\s]*)/);
  return m?.[1] ?? stdout.trim();
}
