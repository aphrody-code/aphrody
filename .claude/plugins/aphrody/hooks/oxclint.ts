#!/usr/bin/env bun
/**
 * oxclint — PostToolUse hook for aphrody plugin.
 *
 * Reads the standard Claude Code hook JSON payload from stdin, extracts
 * the `tool_input.file_path`, and — if that path matches a JS/TS family
 * extension — runs `bunx oxlint` on it.
 *
 * Exit codes (per Claude Code hook protocol):
 *   0  → success, no message surfaced
 *   2  → blocking failure (oxlint found errors); message printed to stderr
 *         is surfaced back to the model
 *   1  → non-blocking warning (oxlint found warnings only); message goes
 *         to the transcript but does not block the Edit/Write
 *
 * Cross-platform: pure Bun, no shell quoting, no platform-specific paths.
 * Linux + Windows safe.
 */

const LINTED_EXTS = new Set([".ts", ".tsx", ".cts", ".mts", ".js", ".jsx", ".cjs", ".mjs"]);

interface HookPayload {
  session_id?: string;
  hook_event_name?: string;
  tool_name?: string;
  tool_input?: {
    file_path?: string;
    edits?: Array<{ file_path?: string }>;
  };
  tool_response?: {
    filePath?: string;
    success?: boolean;
  };
}

async function readStdinJson(): Promise<HookPayload> {
  const chunks: Uint8Array[] = [];
  // Bun.stdin is a ReadableStream<Uint8Array>
  // @ts-expect-error Bun globals
  const stream = Bun.stdin.stream() as ReadableStream<Uint8Array>;
  const reader = stream.getReader();
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    if (value) chunks.push(value);
  }
  const total = chunks.reduce((n, c) => n + c.length, 0);
  if (total === 0) return {};
  const buf = new Uint8Array(total);
  let off = 0;
  for (const c of chunks) {
    buf.set(c, off);
    off += c.length;
  }
  const text = new TextDecoder().decode(buf).trim();
  if (text.length === 0) return {};
  try {
    return JSON.parse(text) as HookPayload;
  } catch (err) {
    process.stderr.write(`oxclint: invalid hook payload: ${(err as Error).message}\n`);
    return {};
  }
}

function pickFilePaths(p: HookPayload): string[] {
  const out: string[] = [];
  const top = p.tool_input?.file_path ?? p.tool_response?.filePath;
  if (top) out.push(top);
  const edits = p.tool_input?.edits;
  if (Array.isArray(edits)) {
    for (const e of edits) {
      if (e?.file_path) out.push(e.file_path);
    }
  }
  return Array.from(new Set(out)).filter((fp) => {
    const dot = fp.lastIndexOf(".");
    if (dot < 0) return false;
    return LINTED_EXTS.has(fp.slice(dot).toLowerCase());
  });
}

async function runOxlint(files: string[]): Promise<number> {
  // bunx is portable; oxlint resolves the closest config (.oxlintrc.json
  // or package.json#oxlint) and walks up the tree, so we just pass file
  // paths.
  const args = ["x", "oxlint", "--quiet", "--max-warnings", "0", "--", ...files];
  // @ts-expect-error Bun globals
  const proc = Bun.spawn(["bun", ...args], {
    stdout: "pipe",
    stderr: "pipe",
    stdin: "ignore",
  });
  const [stdout, stderr] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  const code = await proc.exited;
  const combined = `${stdout}${stderr}`.trim();

  if (code === 0) {
    // clean
    return 0;
  }

  if (combined.length > 0) {
    process.stderr.write(`oxclint: ${files.length} file(s) linted\n${combined}\n`);
  } else {
    process.stderr.write(`oxclint: oxlint exited with code ${code}\n`);
  }

  // Distinguish blocking errors vs warnings:
  // oxlint exit code 1 → lint errors (block). Anything else nonzero
  // (e.g. config error, internal panic) → also block, conservative.
  return 2;
}

async function main(): Promise<void> {
  const payload = await readStdinJson();
  const files = pickFilePaths(payload);
  if (files.length === 0) {
    // Nothing to lint — silent success.
    process.exit(0);
  }
  const code = await runOxlint(files);
  process.exit(code);
}

await main();
