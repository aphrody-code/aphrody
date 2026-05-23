// SPDX-License-Identifier: Apache-2.0
//
// Bun `bun:ffi` binding for the aphrody native library (crate `aphrody-ffi`).
//
// Loads the cdylib and exposes the full aphrody command surface in-process:
//
//   import { run, runCaptured, version } from "./index.ts";
//   const r = runCaptured(["doctor", "--json"]);   // { code, stdout, stderr }
//   console.log(JSON.parse(r.stdout));
//
// Set APHRODY_FFI_LIB to override the library path; otherwise it is resolved
// from target/{release,debug}/ relative to this file.

import { CString, dlopen, FFIType, suffix } from "bun:ffi";
import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** ABI version this binding is written against. */
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
  if (override && existsSync(override)) return override;

  // On Windows the cdylib is `aphrody_ffi.dll`; elsewhere `libaphrody_ffi.{so,dylib}`.
  const prefix = process.platform === "win32" ? "" : "lib";
  const fileName = `${prefix}aphrody_ffi.${suffix}`;

  const here = dirname(fileURLToPath(import.meta.url));
  // crates/aphrody-ffi/bun -> repo root is three levels up.
  const targetDir = join(here, "..", "..", "..", "target");

  const candidates: string[] = [];
  for (const profile of ["release", "debug"]) {
    candidates.push(join(targetDir, profile, fileName));
  }
  // Also probe target/<triple>/{release,debug}/ — present when .cargo/config
  // pins a default --target (aphrody defaults to x86_64-pc-windows-msvc).
  try {
    for (const entry of readdirSync(targetDir, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      for (const profile of ["release", "debug"]) {
        candidates.push(join(targetDir, entry.name, profile, fileName));
      }
    }
  } catch {
    // target/ may not exist yet; fall through to the bare name.
  }

  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate;
  }
  // Last resort: let the dynamic loader search the default paths.
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

/** Native ABI version reported by the loaded library. */
export function abiVersion(): number {
  return lib.symbols.aphrody_abi_version();
}

/** aphrody version string. */
export function version(): string {
  return lib.symbols.aphrody_version().toString();
}

/** The last error recorded on this thread by the native library, if any. */
export function lastError(): string | null {
  const ptr = lib.symbols.aphrody_last_error();
  if (!ptr) return null;
  const text = new CString(ptr).toString();
  return text.length > 0 ? text : null;
}

/**
 * Run an aphrody command with stdout/stderr inherited by the host process.
 * Returns the exit code. Arguments exclude the program name.
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
      `aphrody-ffi ABI mismatch: binding expects ${EXPECTED_ABI_VERSION}, library reports ${actual}`,
    );
  }
}

/** Close the native library handle (optional; the process exit also frees it). */
export function close(): void {
  lib.close();
}
