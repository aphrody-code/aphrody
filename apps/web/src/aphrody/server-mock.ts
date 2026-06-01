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
      return ok(
        "triage: ELF 64-bit LSB pie executable, x86-64\n  entry      0x1060\n  arch       x86_64\n  endianness little\n  stripped   no\n  imports    14 (libc)\n  protections NX, PIE, RELRO(full)\n  suspicious none",
      );
    }

    case "dns": {
      const host = flag(args, "--host") || args[args.length - 1] || "example.com";
      return ok(
        `dns recon · ${host}\n  A     93.184.216.34\n  AAAA  2606:2800:220:1:248:1893:25c8:1946\n  MX    10 mail.${host}\n  NS    a.iana-servers.net\n  TXT   "v=spf1 -all"\n  CAA   0 issue "letsencrypt.org"`,
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

    case "forensics":
    case "chromium":
      return ok("forensics: 3 chromium profiles, 1 240 history rows, 84 cookies (mock)");

    default:
      return ok(
        `[mock] aphrody ${args.join(" ")}\n\nLa commande s'exécuterait via le binaire aphrody dans l'app de bureau.`,
      );
  }
}
