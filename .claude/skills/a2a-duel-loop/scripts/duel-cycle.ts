#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
//
// duel-cycle.ts — one tick of the A2A duel between the aphrody Claude and the
// winclean Claude. Reads the peer's last envelope from .coord/inbox-from-<peer>.jsonl,
// composes a reply, appends it to the correct inbox, mirrors via the HTTP listener
// when reachable, and bumps the matching heartbeat file.
//
// Usage:
//   bun .claude/skills/a2a-duel-loop/scripts/duel-cycle.ts --iteration 1
//   bun .claude/skills/a2a-duel-loop/scripts/duel-cycle.ts --iteration 7 --side winclean
//
// Flags:
//   --iteration N      tick number (required, integer ≥ 1)
//   --side <s>         "aphrody" (default) or "winclean" — who is speaking this tick
//   --coord-dir <path> override coord root (default C:/winclean/.coord)
//   --subject <s>      override subject line (default: "duel tick N")
//   --body <s>         override body text (default: see compose())
//   --type <t>         envelope type — ping | ask | fact | ack (default: ping)
//   --re <id>          envelope `re` field for ack (default: null)
//   --no-http          skip POST /msg to the listener
//   --dry-run          print the envelope, do not write
//
// Exit codes:
//   0 — appended + (optional) mirrored OK
//   1 — usage error
//   2 — coord dir missing / unreadable
//   3 — write failed

import { appendFile, readFile, writeFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { existsSync } from "node:fs";

type Side = "aphrody" | "winclean";
type EnvType = "ping" | "ask" | "fact" | "ack";

interface Envelope {
  id: string;
  ts: string;
  from: string;
  to: string;
  type: EnvType;
  re: string | null;
  subject: string;
  body: string;
  channel_hint?: string[];
}

interface Args {
  iteration: number;
  side: Side;
  coordDir: string;
  subject?: string;
  body?: string;
  envType: EnvType;
  re: string | null;
  noHttp: boolean;
  dryRun: boolean;
}

function parseArgs(argv: string[]): Args {
  const args: Partial<Args> = {
    side: "aphrody",
    coordDir: "C:/winclean/.coord",
    envType: "ping",
    re: null,
    noHttp: false,
    dryRun: false,
  };

  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i] ?? "";
    switch (a) {
      case "--iteration":
        args.iteration = parseInt(next(), 10);
        break;
      case "--side":
        args.side = next() as Side;
        break;
      case "--coord-dir":
        args.coordDir = next();
        break;
      case "--subject":
        args.subject = next();
        break;
      case "--body":
        args.body = next();
        break;
      case "--type":
        args.envType = next() as EnvType;
        break;
      case "--re":
        args.re = next();
        break;
      case "--no-http":
        args.noHttp = true;
        break;
      case "--dry-run":
        args.dryRun = true;
        break;
      case "-h":
      case "--help":
        console.log(`See ${import.meta.path} header for usage.`);
        process.exit(0);
    }
  }

  if (!args.iteration || args.iteration < 1) {
    console.error("error: --iteration N (integer ≥ 1) is required");
    process.exit(1);
  }
  if (args.side !== "aphrody" && args.side !== "winclean") {
    console.error(`error: --side must be aphrody or winclean (got ${args.side})`);
    process.exit(1);
  }
  if (!(["ping", "ask", "fact", "ack"] as const).includes(args.envType as EnvType)) {
    console.error(`error: --type must be ping|ask|fact|ack (got ${args.envType})`);
    process.exit(1);
  }

  return args as Args;
}

function nowUtcIso(): string {
  return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
}

function rand8Hex(): string {
  const buf = new Uint8Array(4);
  crypto.getRandomValues(buf);
  return Array.from(buf, (b) => b.toString(16).padStart(2, "0")).join("");
}

function envelopeId(side: Side, iteration: number): string {
  const prefix = side === "aphrody" ? "apx-duel" : "wc-duel";
  return `${prefix}-${iteration}-${rand8Hex()}`;
}

async function readLastEnvelope(path: string): Promise<Envelope | null> {
  if (!existsSync(path)) return null;
  const content = await readFile(path, "utf8");
  const lines = content.split("\n").filter((l) => l.trim().length > 0);
  if (lines.length === 0) return null;
  try {
    return JSON.parse(lines[lines.length - 1]) as Envelope;
  } catch {
    return null;
  }
}

function compose(args: Args, peerLast: Envelope | null): Envelope {
  const fromId =
    args.side === "aphrody"
      ? "aphrody@aphrody-code/aphrody"
      : "winclean@aphrody-code/winclean";
  const toId =
    args.side === "aphrody"
      ? "winclean@aphrody-code/winclean"
      : "aphrody@aphrody-code/aphrody";

  const defaultSubject = `duel tick ${args.iteration}`;
  let defaultBody = `Tick ${args.iteration} from ${args.side}.`;
  if (peerLast) {
    defaultBody += ` Peer's last envelope: id=${peerLast.id} type=${peerLast.type} subject="${peerLast.subject}".`;
  } else {
    defaultBody += ` No prior peer envelope visible.`;
  }
  defaultBody += ` heartbeat-${args.side}.txt bumped to ${nowUtcIso()}.`;

  return {
    id: envelopeId(args.side, args.iteration),
    ts: nowUtcIso(),
    from: fromId,
    to: toId,
    type: args.envType,
    re: args.re,
    subject: args.subject ?? defaultSubject,
    body: args.body ?? defaultBody,
    channel_hint: ["file_jsonl", "http_jsonrpc"],
  };
}

async function tryPostMsg(env: Envelope): Promise<boolean> {
  try {
    const res = await fetch("http://localhost:8788/msg", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(env),
      signal: AbortSignal.timeout(2000),
    });
    return res.ok;
  } catch {
    return false;
  }
}

async function main(): Promise<void> {
  const args = parseArgs(Bun.argv.slice(2));

  if (!existsSync(args.coordDir)) {
    console.error(`error: coord dir not found: ${args.coordDir}`);
    console.error(`hint: pass --coord-dir <path> or create the directory first.`);
    process.exit(2);
  }

  const peerSide: Side = args.side === "aphrody" ? "winclean" : "aphrody";
  const peerInbox = join(args.coordDir, `inbox-from-${peerSide}.jsonl`);
  const myInbox = join(args.coordDir, `inbox-from-${args.side}.jsonl`);
  const heartbeat = join(args.coordDir, `heartbeat-${args.side}.txt`);

  const peerLast = await readLastEnvelope(peerInbox);
  const env = compose(args, peerLast);

  console.log(JSON.stringify(env, null, 2));

  if (args.dryRun) {
    console.log("--dry-run: not writing");
    return;
  }

  try {
    await appendFile(myInbox, JSON.stringify(env) + "\n", "utf8");
    await writeFile(heartbeat, nowUtcIso() + "\n", "utf8");
  } catch (e) {
    console.error(`write failed: ${(e as Error).message}`);
    process.exit(3);
  }

  if (!args.noHttp) {
    const ok = await tryPostMsg(env);
    console.log(`POST /msg: ${ok ? "200 OK" : "skipped (listener unreachable)"}`);
  }

  console.log(`appended to ${myInbox}`);
  console.log(`bumped ${heartbeat}`);
}

await main();
