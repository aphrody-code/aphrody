#!/usr/bin/env bun
import * as fs from "node:fs/promises";
import * as path from "node:path";
import { translateMarkdown } from "./translator.js";
import { generateDocumentation } from "./generator.js";
import {
  assertLanguage,
  assertPath,
  SUPPORTED_LANGUAGES,
  type TargetLanguage,
} from "./validate.js";
import { DocAiError, InvalidArgumentsError, IoError, toMessage } from "./errors.js";

export const USAGE = `
AI-First Documentation & Translation System CLI

Usage:
  doc-ai translate <file_or_dir> [lang]   Translate markdown (default lang: fr)
  doc-ai generate <ts_file> [out_file]    Generate a Lit component API guide
  doc-ai sync [lang]                       Translate all monorepo docs folders
  doc-ai --help                            Show this help

Commands:
  translate  Translate a single markdown file or every markdown file in a
             directory (recursive). Output is written next to each source as
             <name>.<lang>.md.
  generate   Analyze a Lit component TS file and write its Markdown API guide.
  sync       Scan the monorepo 'docs' folders and translate them.

Options:
  -h, --help     Show this help and exit.
  -v, --version  Print the package version and exit.

Languages: ${SUPPORTED_LANGUAGES.join(", ")}

Environment:
  GEMINI_API_KEY | GOOGLE_API_KEY   Enable Gemini-backed generation/translation.
  DOC_AI_MODEL                      Override the model (default: gemini-2.5-flash).
  DOC_AI_GEMINI_BASE_URL            Override the Generative Language API base URL.
  DOC_AI_REQUEST_TIMEOUT_MS         Per-request network timeout (default: 60000).

When no credentials are set, generation uses a deterministic static template
and translation uses a structure-preserving offline pipeline.
`;

export type ParsedCommand =
  | { kind: "help" }
  | { kind: "version" }
  | { kind: "translate"; targetPath: string; lang: TargetLanguage }
  | { kind: "generate"; tsFile: string; outFile?: string }
  | { kind: "sync"; lang: TargetLanguage };

/**
 * Parse CLI arguments (everything after `node script`) into a typed command.
 * Pure and side-effect free so it can be unit-tested. Throws
 * {@link InvalidArgumentsError} (exit code 2) for malformed input.
 */
export function parseArgs(argv: readonly string[]): ParsedCommand {
  const args = [...argv];
  if (args.length === 0) {
    return { kind: "help" };
  }

  const first = args[0];
  if (first === "-h" || first === "--help") return { kind: "help" };
  if (first === "-v" || first === "--version") return { kind: "version" };

  const command = first;
  const rest = args.slice(1).filter((a) => {
    if (a === "-h" || a === "--help") throw new HelpRequested();
    return true;
  });

  switch (command) {
    case "translate": {
      const targetPath = rest[0];
      if (!targetPath) {
        throw new InvalidArgumentsError(
          "translate: please provide a file or directory path to translate.",
        );
      }
      const lang = assertLanguage(rest[1] ?? "fr");
      return { kind: "translate", targetPath, lang };
    }
    case "generate": {
      const tsFile = rest[0];
      if (!tsFile) {
        throw new InvalidArgumentsError(
          "generate: please provide a TypeScript component file to analyze.",
        );
      }
      return { kind: "generate", tsFile, outFile: rest[1] };
    }
    case "sync": {
      const lang = assertLanguage(rest[0] ?? "fr");
      return { kind: "sync", lang };
    }
    default:
      throw new InvalidArgumentsError(`Unknown command: ${command}`);
  }
}

/** Internal sentinel: `--help` was found in a command's argument list. */
class HelpRequested extends Error {}

async function readPackageVersion(): Promise<string> {
  try {
    const pkgPath = path.resolve(import.meta.dir, "..", "package.json");
    const raw = await fs.readFile(pkgPath, "utf-8");
    const parsed = JSON.parse(raw) as { version?: string };
    return parsed.version ?? "0.0.0";
  } catch {
    return "0.0.0";
  }
}

async function handleTranslate(targetPath: string, lang: TargetLanguage): Promise<void> {
  const absolutePath = await assertPath(targetPath, "any");
  const stat = await fs.stat(absolutePath);
  if (stat.isFile()) {
    await translateFile(absolutePath, lang);
  } else {
    await translateDirectory(absolutePath, lang);
  }
}

async function translateFile(filePath: string, lang: TargetLanguage): Promise<void> {
  const ext = path.extname(filePath);
  if (ext !== ".md") {
    console.log(`Skipping non-markdown file: ${filePath}`);
    return;
  }
  // Skip files that are already translated.
  if (filePath.endsWith(`.${lang}.md`)) {
    return;
  }

  console.log(`Translating: ${path.basename(filePath)} -> Target Language: ${lang}...`);
  let content: string;
  try {
    content = await fs.readFile(filePath, "utf-8");
  } catch (err) {
    throw new IoError(`Failed to read ${filePath}: ${toMessage(err)}`, {
      cause: err,
    });
  }
  const translated = await translateMarkdown(content, lang);

  let baseName = path.basename(filePath, ".md");
  if (baseName.endsWith(".en")) baseName = baseName.slice(0, -3);
  if (baseName.endsWith(".fr")) baseName = baseName.slice(0, -3);

  const outPath = path.join(path.dirname(filePath), `${baseName}.${lang}.md`);
  try {
    await fs.writeFile(outPath, translated, "utf-8");
  } catch (err) {
    throw new IoError(`Failed to write ${outPath}: ${toMessage(err)}`, {
      cause: err,
    });
  }
  console.log(`Saved translation: ${outPath}`);
}

async function translateDirectory(dirPath: string, lang: TargetLanguage): Promise<void> {
  const entries = await fs.readdir(dirPath, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      await translateDirectory(fullPath, lang);
    } else if (entry.isFile()) {
      await translateFile(fullPath, lang);
    }
  }
}

async function handleGenerate(tsFile: string, outFile?: string): Promise<void> {
  const absoluteTsPath = await assertPath(tsFile, "file");
  console.log(`Analyzing Lit component: ${path.basename(tsFile)}...`);
  const generatedDocs = await generateDocumentation(absoluteTsPath);

  const baseName = path.basename(tsFile, ".ts");
  const outPath = outFile
    ? path.resolve(outFile)
    : path.join(path.dirname(absoluteTsPath), `${baseName}.md`);

  try {
    await fs.writeFile(outPath, generatedDocs, "utf-8");
  } catch (err) {
    throw new IoError(`Failed to write ${outPath}: ${toMessage(err)}`, {
      cause: err,
    });
  }
  console.log(`Documentation generated and saved to: ${outPath}`);
}

async function handleSync(lang: TargetLanguage): Promise<void> {
  console.log(`Starting full monorepo documentation synchronization. Target language: ${lang}`);
  const pathsToSync = [path.resolve("docs")];

  for (const folder of pathsToSync) {
    try {
      const stat = await fs.stat(folder);
      if (stat.isDirectory()) {
        console.log(`Syncing folder: ${folder}`);
        await translateDirectory(folder, lang);
      }
    } catch {
      console.log(`Path not found or accessible: ${folder}, skipping.`);
    }
  }
  console.log("Documentation sync complete!");
}

/** Dispatch a parsed command to its handler. */
export async function run(parsed: ParsedCommand): Promise<void> {
  switch (parsed.kind) {
    case "help":
      console.log(USAGE);
      return;
    case "version":
      console.log(await readPackageVersion());
      return;
    case "translate":
      await handleTranslate(parsed.targetPath, parsed.lang);
      return;
    case "generate":
      await handleGenerate(parsed.tsFile, parsed.outFile);
      return;
    case "sync":
      await handleSync(parsed.lang);
      return;
  }
}

/**
 * CLI entry point. Returns the process exit code instead of calling
 * `process.exit` directly, so it stays testable. Never leaves an unhandled
 * rejection: all failures are caught and mapped to an exit code.
 */
export async function main(argv: readonly string[]): Promise<number> {
  let parsed: ParsedCommand;
  try {
    parsed = parseArgs(argv);
  } catch (err) {
    if (err instanceof HelpRequested) {
      console.log(USAGE);
      return 0;
    }
    if (err instanceof DocAiError) {
      console.error(`Error: ${err.message}`);
      return err.exitCode;
    }
    console.error(`Error: ${toMessage(err)}`);
    return 2;
  }

  // `help` with no args is informational but historically exited non-zero.
  const emptyInvocation = argv.length === 0 && parsed.kind === "help";

  try {
    await run(parsed);
    return emptyInvocation ? 1 : 0;
  } catch (err) {
    if (err instanceof DocAiError) {
      console.error(`Error: ${err.message}`);
      return err.exitCode;
    }
    console.error(`Execution failed: ${toMessage(err)}`);
    return 1;
  }
}

// Only auto-run when executed as a script, not when imported by tests.
if (import.meta.main) {
  main(process.argv.slice(2))
    .then((code) => {
      process.exitCode = code;
    })
    .catch((err: unknown) => {
      console.error(`Fatal: ${toMessage(err)}`);
      process.exitCode = 1;
    });
}
