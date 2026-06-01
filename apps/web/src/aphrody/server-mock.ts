// Server-side mock of the aphrody CLI for the web port. The Bun server routes
// POST /api/run here; it returns realistic ExecResults keyed on the sub-command
// so every ported screen is exercisable without the real binary / Tauri shell.

import type { Account, ExecResult, Meta } from "./types.ts";

export const META: Meta = {
  app_version: "web-dev 0.1.0",
  target_os: "linux",
  target_arch: "x86_64",
  family: "unix",
};

export const ACCOUNT: Account = {
  connected: true,
  email: "agent@aphrody.dev",
  name: "aphrody agent",
  initials: "AA",
};

const ok = (stdout: string): ExecResult => ({ code: 0, stdout, stderr: "" });

function flag(args: string[], name: string): string {
  const i = args.indexOf(name);
  return i >= 0 ? (args[i + 1] ?? "") : "";
}

/** Map an argv (without program name) to a canned ExecResult. */
export function runMock(args: string[]): ExecResult {
  const [cmd, sub] = args;

  switch (cmd) {
    case "chat": {
      const prompt = flag(args, "--prompt") || args.slice(1).join(" ");
      return ok(
        `aphrody · Gemini 3.5 Flash\n\n> ${prompt}\n\n` +
          `Voici une réponse simulée de l'agent aphrody. Dans l'app de bureau (Tauri) ` +
          `ce tour passerait par « aphrody chat --prompt » et appellerait réellement Gemini. ` +
          `Ici, le serveur Bun renvoie une sortie canned pour démontrer l'UI Material 3.`,
      );
    }

    case "doctor":
      return ok(
        [
          "aphrody doctor",
          "  [ok]   binary           aphrody 0.1.0 (web-dev)",
          "  [ok]   config           ~/.config/aphrody/config.toml",
          "  [ok]   auth (google)    agent@aphrody.dev",
          "  [ok]   gemini api       reachable (mock)",
          "  [warn] tauri shell      not detected (running in browser)",
          "  [ok]   a2a peer         127.0.0.1:7777",
          "  [ok]   mcp servers      3 connected",
          "",
          "5 ok · 1 warning · 0 errors",
        ].join("\n"),
      );

    case "version":
      return ok("aphrody 0.1.0\nbuild web-dev\ntarget linux/x86_64\nrustc 1.95.0");

    case "re": {
      if (sub === "strings")
        return ok(
          [
            "/lib64/ld-linux-x86-64.so.2",
            "libc.so.6",
            "GLIBC_2.34",
            "main",
            "_start",
            "puts",
            "%s: %d\\n",
            "usage: %s <file>",
          ].join("\n"),
        );
      if (sub === "sections")
        return ok(
          ".text    0x1040  0x2a1  R-X\n.rodata  0x2000  0x0f4  R--\n.data    0x3000  0x010  RW-\n.bss     0x3010  0x008  RW-",
        );
      // `re triage` ALWAYS emits the TriageReport JSON (pretty with `--pretty`).
      if (sub === "triage")
        return ok(
          JSON.stringify(
            {
              format: "elf64",
              size: 142_312,
              sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
              arch: "x86_64",
              entry_point: 0x1060,
              sections: [
                { name: ".text", vaddr: 0x1040, size: 0x2a10, entropy: 6.1 },
                { name: ".rodata", vaddr: 0x4000, size: 0x0f40, entropy: 4.7 },
                { name: ".data", vaddr: 0x6000, size: 0x0100, entropy: 3.2 },
                { name: ".packed", vaddr: 0x7000, size: 0x9000, entropy: 7.6 },
                { name: ".bss", vaddr: 0x10000, size: 0x0080, entropy: null },
              ],
              imports: ["puts", "printf", "malloc", "free", "__libc_start_main"],
              exports: ["main", "_start"],
              strings_sample: [
                "/lib64/ld-linux-x86-64.so.2",
                "libc.so.6",
                "GLIBC_2.34",
                "usage: %s <file>",
              ],
            },
            null,
            2,
          ),
        );
      return ok(
        "triage: ELF 64-bit LSB pie executable, x86-64\n  entry      0x1060\n  arch       x86_64\n  endianness little\n  stripped   no\n  imports    14 (libc)\n  protections NX, PIE, RELRO(full)\n  suspicious none",
      );
    }


    case "search": {
      const q = flag(args, "--query") || args.slice(1).join(" ");
      return ok(
        `web search · « ${q} »\n\n1. ${q} — overview\n   https://example.com/${encodeURIComponent(q)}\n2. Documentation\n   https://docs.example.com\n3. Discussion thread\n   https://forum.example.com/t/123`,
      );
    }

    case "scan":
      return ok("scan: 0 secrets, 0 high-severity findings across 128 files (mock)");

    case "mcp": {
      if (sub === "call") return ok('{\n  "ok": true,\n  "result": "mock tool result"\n}');
      return ok(
        "mcp servers\n  context7        connected   12 tools\n  microsoft-docs  connected    4 tools\n  aphrody         connected   31 tools",
      );
    }

    case "skill":
    case "skills":
      return ok(
        "skills\n  deep-research      research harness\n  color-expert       286K-word color reference\n  best-stack-2026    crate chooser\n  google-design      M3 authority",
      );

    case "antigravity":
      if (sub === "whoami") return ok(JSON.stringify(ACCOUNT));
      return ok("antigravity: ok");

    case "forensics": {
      if (sub === "map") {
        const out = flag(args, "--out") || "var/data/forensics";
        return ok(
          JSON.stringify({
            wrote: `${out}/map.json`,
            file_count: 1284,
            hashed_count: 1197,
            secret_meta_only_count: 87,
          }),
        );
      }
      if (sub === "sqlite") {
        const db = flag(args, "--db") || "History";
        return ok(
          JSON.stringify({
            db,
            object_count: 5,
            tables: [
              {
                type: "table",
                name: "urls",
                sql: "CREATE TABLE urls (id INTEGER PRIMARY KEY, url LONGVARCHAR, title LONGVARCHAR, visit_count INTEGER DEFAULT 0)",
              },
              {
                type: "table",
                name: "visits",
                sql: "CREATE TABLE visits (id INTEGER PRIMARY KEY, url INTEGER NOT NULL, visit_time INTEGER NOT NULL)",
              },
              {
                type: "index",
                name: "urls_url_index",
                sql: "CREATE INDEX urls_url_index ON urls (url)",
              },
              { type: "index", name: "sqlite_autoindex_urls_1", sql: null },
              {
                type: "view",
                name: "recent_urls",
                sql: "CREATE VIEW recent_urls AS SELECT url, title FROM urls ORDER BY visit_count DESC",
              },
            ],
          }),
        );
      }
      return ok("forensics: 3 chromium profiles, 1 240 history rows, 84 cookies (mock)");
    }
    case "chromium":
      return ok("forensics: 3 chromium profiles, 1 240 history rows, 84 cookies (mock)");

    default:
      return ok(
        `[mock] aphrody ${args.join(" ")}\n\nLa commande s'exécuterait via le binaire aphrody dans l'app de bureau.`,
      );
  }
}
