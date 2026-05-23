// SPDX-License-Identifier: Apache-2.0
//
// Smoke example for the aphrody native library via Bun `bun:ffi`.
//
// Build the cdylib first, then run this file with Bun:
//
//   cargo build --release -p aphrody-ffi
//   bun run crates/aphrody-ffi/bun/example.ts
//
// It loads the library, prints the ABI/version, then runs `aphrody version
// --json` with captured output and pretty-prints the parsed result.

import { abiVersion, assertCompatible, run, runCaptured, version } from "./index.ts";

assertCompatible();
console.log(`aphrody-ffi ABI ${abiVersion()} | aphrody ${version()}`);

const captured = runCaptured(["version", "--json"]);
console.log(`captured exit code: ${captured.code}`);

if (captured.stdout.trim().length > 0) {
  try {
    console.log("parsed version JSON:", JSON.parse(captured.stdout));
  } catch {
    console.log("stdout:", captured.stdout.trim());
  }
}
if (captured.stderr.trim().length > 0) {
  console.log("stderr:", captured.stderr.trim());
}

// Inherited-stdio variant: output goes straight to this terminal.
console.log("\n--- inherited stdio: aphrody version ---");
const code = run(["version"]);
console.log(`\ninherited exit code: ${code}`);
