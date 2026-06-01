// @ts-nocheck
// SPDX-License-Identifier: Apache-2.0

declare const Bun: { version: string };
declare const process: { platform: string; arch: string };
declare const require: (m: string) => any;

/**
 * Bun Macro: Executed at bundle-time by Bun's bundler.
 * Inlines build-time git status, compiler information, and timestamps directly
 * into the compiled JS bundle.
 */
export function getBuildMetadata() {
  let commitHash = "dev";
  let commitDate = "n/a";
  try {
    const { execSync } = require("child_process");
    commitHash = execSync("git rev-parse --short HEAD").toString().trim();
    commitDate = execSync("git log -1 --format=%cd").toString().trim();
  } catch {
    /* fallback when git is missing */
  }

  return {
    bunVersion: typeof Bun !== "undefined" ? Bun.version : "unknown",
    commitHash,
    commitDate,
    buildTime: new Date().toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }),
    os: typeof process !== "undefined" ? `${process.platform}-${process.arch}` : "unknown",
  };
}
