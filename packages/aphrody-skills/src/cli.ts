#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
//
// @aphrody/skills — CLI entrypoint.
//
// Subcommands:
//   list            flat table of every discovered skill
//   info <name>     print the full SKILL.md
//   sync            mirror SKILL.md files into $APHRODY_SKILLS_HOME
//   install <name>  copy one skill into $APHRODY_SKILLS_HOME
//   where <name>    print source absolute path
//   sources         print the source registry
//   --help          this message
//   --version       print version
//
// Default subcommand: `list`.

import { runInfo } from "./commands/info.ts";
import { runInstall } from "./commands/install.ts";
import { runList } from "./commands/list.ts";
import { runSources } from "./commands/sources.ts";
import { runSync } from "./commands/sync.ts";
import { runWhere } from "./commands/where.ts";

const VERSION = "0.1.0";

function printHelp(): void {
  process.stdout.write(
    `@aphrody/skills v${VERSION} — unified skill-aggregator CLI

USAGE
  skills <command> [options]
  bunx @aphrody/skills <command> [options]

COMMANDS
  list                     Flat table of every discovered skill (default).
  list --source=<slug>     Filter by source slug.
  list --json              Machine-readable JSON output.
  info <name>              Print the full SKILL.md body.
  sync                     Copy every discovered SKILL.md into APHRODY_SKILLS_HOME.
  sync --source=<slug>     Sync only one source.
  sync --dry-run           Show what would happen without writing.
  install <name>           Copy ONE SKILL.md into APHRODY_SKILLS_HOME.
  where <name>             Print the source absolute path.
  sources                  Print the source registry (slug, schema, count, path).
  sources --json           Machine-readable JSON output.

GLOBAL
  --help, -h               Print this message.
  --version, -v            Print version.

SOURCE SLUGS
  open-design  openclaw  claude-code  gemini-cli  vercel-labs  google-labs

ENVIRONMENT
  APHRODY_SKILLS_HOME      Destination dir for sync/install
                           (default: ~/.aphrody/skills/).
  APHRODY_SKILLS_SOURCES   Comma-separated allowlist of source slugs.
  APHRODY_OD_SKILLS        Override path to open-design skills dir.
  APHRODY_OC_SKILLS        Override path to openclaw skills dir.
  APHRODY_CC_SKILLS        Override path to claude-code skills dir.
  APHRODY_GEMINI_SKILLS    Override path to gemini-cli skills dir.
  APHRODY_VERCEL_SKILLS    Override path to vercel-labs skills dir.
  APHRODY_GOOGLE_SKILLS    Override path to google-labs skills dir.
  APHRODY_WORKTREE         Override the worktree root (default: C:/worktree on
                           Windows, ~/worktree elsewhere).
`,
  );
}

function dispatch(argv: readonly string[]): number {
  if (argv.length === 0) {
    return runList([]);
  }
  const head = argv[0];
  const tail = argv.slice(1);
  switch (head) {
    case "list":
      return runList(tail);
    case "info":
      return runInfo(tail);
    case "sync":
      return runSync(tail);
    case "install":
      return runInstall(tail);
    case "where":
      return runWhere(tail);
    case "sources":
      return runSources(tail);
    case "-h":
    case "--help":
    case "help":
      printHelp();
      return 0;
    case "-v":
    case "--version":
    case "version":
      process.stdout.write(`${VERSION}\n`);
      return 0;
    default:
      // Bare flag at root? Treat unknown as 'list' if it looks like a list flag.
      if (head !== undefined && (head.startsWith("--source=") || head === "--json")) {
        return runList(argv);
      }
      process.stderr.write(`skills: unknown command '${head ?? ""}'. Try 'skills --help'.\n`);
      return 2;
  }
}

const code = dispatch(process.argv.slice(2));
process.exit(code);
