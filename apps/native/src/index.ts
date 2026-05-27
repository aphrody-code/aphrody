// SPDX-License-Identifier: Apache-2.0
//! In-process bridge to the aphrody native library via Bun `bun:ffi`.
//
// Loads the `aphrody-ffi` cdylib built in the sibling Rust repo
// (`C:\src\aphrody`, crate `crates/aphrody-ffi`) and drives the ENTIRE aphrody
// command surface in-process — no subprocess spawn. The C ABI is versioned;
// `assertCompatible()` rejects a library whose ABI differs from this binding,
// so the two repos cannot silently drift.
//
// Build the cdylib first, in the Rust repo:
//   cargo build --release -p aphrody-ffi
// Set APHRODY_FFI_LIB to the cdylib path, or APHRODY_REPO to the Rust repo
// root, to override discovery.

import { CString, dlopen, FFIType, suffix } from "bun:ffi";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";

/**
 * ABI revision this binding is written against — mirrors `APHRODY_ABI_VERSION`
 * in the Rust repo's `crates/aphrody-ffi/include/aphrody.h`.
 */
export const EXPECTED_ABI_VERSION = 1;

const ENCODER = new TextEncoder();

/**
 * Encode a JS string as a NUL-terminated UTF-8 buffer for a `cstring` argument.
 * Bun does not auto-marshal a JS string into a pointer for FFI args, so we hand
 * it a `Uint8Array` (Bun passes its pointer, pinned for the synchronous call).
 */
function cstr(text: string): Uint8Array {
  return ENCODER.encode(`${text}\0`);
}

function resolveLibPath(): string {
  const override = process.env.APHRODY_FFI_LIB;
  if (override && existsSync(override)) {
    return override;
  }

  // On Windows the cdylib is `aphrody_ffi.dll`; elsewhere `libaphrody_ffi.{so,dylib}`.
  const prefix = process.platform === "win32" ? "" : "lib";
  const fileName = `${prefix}aphrody_ffi.${suffix}`;

  // The cdylib is produced by the sibling Rust repo. apps/native/src -> the
  // aphrody-ts root is three up, and the Rust repo is its sibling.
  const rustRepo =
    process.env.APHRODY_REPO ?? join(import.meta.dir, "..", "..", "..", "..", "aphrody");
  const targetDir = join(rustRepo, "target");

  const candidates: string[] = [];
  for (const profile of ["release", "debug"]) {
    candidates.push(join(targetDir, profile, fileName));
  }
  // aphrody pins a default --target (x86_64-pc-windows-msvc), so artifacts land
  // under target/<triple>/{release,debug}/ rather than target/{release,debug}/.
  try {
    for (const entry of readdirSync(targetDir, { withFileTypes: true })) {
      if (!entry.isDirectory()) {
        continue;
      }
      for (const profile of ["release", "debug"]) {
        candidates.push(join(targetDir, entry.name, profile, fileName));
      }
    }
  } catch {
    // target/ may not exist yet (the cdylib has not been built).
  }

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  // Last resort: let the dynamic loader search the default search paths.
  return fileName;
}

const lib = dlopen(resolveLibPath(), {
  aphrody_abi_version: { args: [], returns: FFIType.u32 },
  aphrody_version: { args: [], returns: FFIType.cstring },
  aphrody_run_json: { args: [FFIType.cstring], returns: FFIType.i32 },
  aphrody_run_captured: { args: [FFIType.cstring], returns: FFIType.ptr },
  aphrody_string_free: { args: [FFIType.ptr], returns: FFIType.void },
  aphrody_last_error: { args: [], returns: FFIType.ptr },
});

/** Native ABI revision reported by the loaded library. */
export function abiVersion(): number {
  return lib.symbols.aphrody_abi_version();
}

/** The aphrody version string (e.g. `"1.0.0-canary"`). */
export function version(): string {
  return lib.symbols.aphrody_version().toString();
}

/** The last error recorded on this thread by the native library, if any. */
export function lastError(): string | null {
  const ptr = lib.symbols.aphrody_last_error();
  if (!ptr) {
    return null;
  }
  const text = new CString(ptr).toString();
  return text.length > 0 ? text : null;
}

/**
 * Run an aphrody command with stdout/stderr inherited by the host process,
 * returning the exit code. Arguments exclude the program name, e.g.
 * `run(["doctor"])`.
 */
export function run(args: string[]): number {
  return lib.symbols.aphrody_run_json(cstr(JSON.stringify(args)));
}

/** Structured result of a captured run. */
export interface CapturedResult {
  code: number;
  stdout: string;
  stderr: string;
}

/**
 * Run an aphrody command capturing stdout AND stderr in-process. Arguments
 * exclude the program name, e.g. `runCaptured(["version", "--json"])`.
 */
export function runCaptured(args: string[]): CapturedResult {
  const resultPtr = lib.symbols.aphrody_run_captured(cstr(JSON.stringify(args)));
  if (!resultPtr) {
    return {
      code: 70,
      stdout: "",
      stderr: lastError() ?? "aphrody_run_captured returned NULL",
    };
  }
  try {
    return JSON.parse(new CString(resultPtr).toString()) as CapturedResult;
  } finally {
    lib.symbols.aphrody_string_free(resultPtr);
  }
}

/** Throws if the loaded library's ABI does not match this binding. */
export function assertCompatible(): void {
  const actual = abiVersion();
  if (actual !== EXPECTED_ABI_VERSION) {
    throw new Error(
      `@aphrody-code/native ABI mismatch: binding expects ${EXPECTED_ABI_VERSION}, library reports ${actual}`,
    );
  }
}

/** Close the native library handle (optional; process exit also frees it). */
export function close(): void {
  lib.close();
}
