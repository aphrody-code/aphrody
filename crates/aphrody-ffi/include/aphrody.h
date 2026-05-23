/* SPDX-License-Identifier: Apache-2.0 */
/*
 * aphrody.h - C ABI for the aphrody native library (crate `aphrody-ffi`).
 *
 * Exposes the ENTIRE aphrody command surface in-process. Link against the
 * cdylib (libaphrody_ffi.so / aphrody_ffi.dll / libaphrody_ffi.dylib) or load
 * it dynamically (dlopen / LoadLibrary / Bun `bun:ffi`).
 *
 * Conventions
 * -----------
 *  - Command arguments are passed WITHOUT the program name; a synthetic argv[0]
 *    ("aphrody") is prepended internally. e.g. {"doctor", "--json"}.
 *  - Strings are UTF-8, NUL-terminated.
 *  - A `char *` returned by aphrody_run_captured is owned by the caller and
 *    MUST be released with aphrody_string_free exactly once.
 *  - `const char *` results (aphrody_version, aphrody_last_error) are owned by
 *    the library; do NOT free them.
 *  - Every entry point catches Rust panics and the wrapped commands never call
 *    process::exit, so a failing command cannot tear down the host process.
 */
#ifndef APHRODY_H
#define APHRODY_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ABI version of the symbol set below. Bump on any incompatible change. */
uint32_t aphrody_abi_version(void);

/* aphrody version string. Owned by the library; do NOT free. */
const char *aphrody_version(void);

/*
 * Run a command with inherited stdout/stderr; returns the process exit code.
 * `argv` points to `argc` NUL-terminated UTF-8 strings (or NULL iff argc == 0).
 */
int aphrody_run(int argc, const char *const *argv);

/*
 * Same as aphrody_run, but arguments are a JSON array of strings, e.g.
 * "[\"doctor\",\"--json\"]". `args_json` may be NULL (treated as []).
 */
int aphrody_run_json(const char *args_json);

/*
 * Run a command with stdout AND stderr captured. Returns a newly-allocated,
 * NUL-terminated JSON document: {"code":<int>,"stdout":"...","stderr":"..."}.
 * Returns NULL only on allocation failure (see aphrody_last_error). The caller
 * MUST free the result with aphrody_string_free.
 */
char *aphrody_run_captured(const char *args_json);

/* Free a string returned by this library (e.g. aphrody_run_captured). NULL ok. */
void aphrody_string_free(char *ptr);

/*
 * Last error recorded on the CURRENT thread, or NULL if none. Valid until the
 * next library call on this thread. Owned by the library; do NOT free.
 */
const char *aphrody_last_error(void);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* APHRODY_H */
