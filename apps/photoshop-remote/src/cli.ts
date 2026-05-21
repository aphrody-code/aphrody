#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
//! CLI: run an ExtendScript against a remote Photoshop.
//!
//! Credentials come from the environment (preferred) or argv (local use only):
//!   PS_HOST=<host> PS_PASSWORD=<pw> bun src/cli.ts 'app.version'
//!   bun src/cli.ts <host> <password> [jsx]
//! The password is never written to disk by this tool.

import { PhotoshopRemote } from "./client";

const host = process.env.PS_HOST ?? Bun.argv[2];
const password = process.env.PS_PASSWORD ?? Bun.argv[3];
// JSX position depends on how creds were supplied: with both from the
// environment the first positional is the script; otherwise it follows
// <host> <password>.
const credsFromEnv = Boolean(process.env.PS_HOST && process.env.PS_PASSWORD);
const jsx = process.env.PS_JSX ?? (credsFromEnv ? Bun.argv[2] : Bun.argv[4]);

if (!host || !password) {
  console.error(
    "usage: PS_HOST=<host> PS_PASSWORD=<pw> bun src/cli.ts ['<jsx>']\n" +
      "   or: bun src/cli.ts <host> <password> ['<jsx>']",
  );
  process.exit(2);
}

const ps = new PhotoshopRemote({ host, password });
try {
  await ps.connect();
  const result = jsx ? await ps.exec(jsx) : await ps.info();
  console.log(JSON.stringify(result));
  ps.close();
  process.exit(result.isError ? 1 : 0);
} catch (e) {
  console.log(JSON.stringify({ isError: true, text: String((e as Error).message ?? e) }));
  ps.close();
  process.exit(1);
}
