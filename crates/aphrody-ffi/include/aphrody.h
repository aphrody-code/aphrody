/* SPDX-License-Identifier: Apache-2.0 */
/**
 * @file aphrody.h
 * @brief C ABI for the aphrody native library (crate `aphrody-ffi`).
 *
 * Exposes the ENTIRE aphrody command surface in-process. Link against the
 * cdylib (`libaphrody_ffi.so` / `aphrody_ffi.dll` / `libaphrody_ffi.dylib`) or
 * load it dynamically (`dlopen` / `LoadLibrary` / Bun `bun:ffi`).
 *
 * @section conv Conventions
 *  - Command arguments are passed WITHOUT the program name; a synthetic
 *    `argv[0]` (`"aphrody"`) is prepended internally, e.g. `{"doctor",
 *    "--json"}`.
 *  - All strings are UTF-8 and NUL-terminated. Invalid UTF-8 in inputs is
 *    rejected (#APHRODY_STATUS_USAGE); invalid UTF-8 in captured output is
 *    replaced lossily (U+FFFD).
 *  - A `char *` returned by ::aphrody_run_captured is owned by the CALLER and
 *    MUST be released with ::aphrody_string_free exactly once.
 *  - `const char *` results (::aphrody_version, ::aphrody_last_error) are owned
 *    by the LIBRARY; do NOT free them.
 *  - Every entry point catches Rust panics and the wrapped commands never call
 *    `exit(3)`, so a failing command cannot tear down the host process.
 *
 * @section abi ABI stability
 *  ::aphrody_abi_version returns the runtime ABI revision; #APHRODY_ABI_VERSION
 *  is the compile-time constant this header describes. A host should reject a
 *  library whose runtime version differs from the header it compiled against.
 *  The integer is bumped on ANY source-incompatible change to the symbol set.
 *
 * @copyright aphrody-code, Apache-2.0.
 */
#ifndef APHRODY_H
#define APHRODY_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint32_t, int32_t */

/**
 * @def APHRODY_ABI_VERSION
 * @brief Compile-time ABI revision described by this header.
 *
 * Kept in lock-step with the value returned by ::aphrody_abi_version at
 * runtime; a mismatch indicates the header and the loaded library disagree.
 */
#define APHRODY_ABI_VERSION 1u

/*
 * APHRODY_API — symbol visibility / linkage decoration.
 *
 * On Windows the consuming side may define APHRODY_STATIC (linking the static
 * archive) to suppress dllimport. The cdylib itself is built by Rust with
 * `#[unsafe(no_mangle)] pub extern "C"`, which already exports the symbols, so
 * dllexport here is only meaningful to non-Rust producers re-exporting them.
 */
#if defined(_WIN32) && !defined(APHRODY_STATIC)
#  if defined(APHRODY_BUILD)
#    define APHRODY_API __declspec(dllexport)
#  else
#    define APHRODY_API __declspec(dllimport)
#  endif
#elif defined(__GNUC__) || defined(__clang__)
#  define APHRODY_API __attribute__((visibility("default")))
#else
#  define APHRODY_API
#endif

/*
 * APHRODY_NODISCARD — warn when the return value (a status code or an owned
 * pointer) is dropped. Prefers the C23 / C++17 standard attribute, then the
 * GCC/Clang and MSVC vendor spellings.
 */
#if defined(__cplusplus) && __cplusplus >= 201703L
#  define APHRODY_NODISCARD [[nodiscard]]
#elif !defined(__cplusplus) && defined(__STDC_VERSION__) && __STDC_VERSION__ > 201710L
#  define APHRODY_NODISCARD [[nodiscard]] /* C23 */
#elif defined(__GNUC__) || defined(__clang__)
#  define APHRODY_NODISCARD __attribute__((warn_unused_result))
#elif defined(_MSC_VER)
#  define APHRODY_NODISCARD _Check_return_
#else
#  define APHRODY_NODISCARD
#endif

/*
 * APHRODY_NONNULL(...) — flag the listed (1-based) pointer parameters as
 * required-non-null for diagnostics. APHRODY_RETURNS_NONNULL marks a return
 * value that is never NULL. APHRODY_MALLOC marks a function returning fresh,
 * uniquely-owned, heap memory the caller must release. All degrade to nothing
 * on compilers without the attribute.
 */
#if defined(__GNUC__) || defined(__clang__)
#  define APHRODY_NONNULL(...) __attribute__((nonnull(__VA_ARGS__)))
#  define APHRODY_RETURNS_NONNULL __attribute__((returns_nonnull))
#  define APHRODY_MALLOC __attribute__((malloc))
#else
#  define APHRODY_NONNULL(...)
#  define APHRODY_RETURNS_NONNULL
#  define APHRODY_MALLOC
#endif

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief FFI-layer status codes (sysexits.h-compatible subset).
 *
 * ::aphrody_run, ::aphrody_run_json and the `"code"` field of
 * ::aphrody_run_captured return the wrapped command's process exit code, which
 * is an ARBITRARY 8-bit value — NOT restricted to this enum. These named
 * constants document the codes the FFI layer itself injects when a call fails
 * before (or instead of) reaching the command:
 *  - #APHRODY_STATUS_OK on success,
 *  - #APHRODY_STATUS_USAGE for a malformed request (bad argv / bad JSON / bad
 *    UTF-8),
 *  - #APHRODY_STATUS_SOFTWARE if a Rust panic was caught at the boundary.
 *
 * The underlying type is `int32_t` so the enum is layout-compatible with the
 * `int` return values of the run functions.
 */
typedef enum AphrodyStatus {
    /** Success. */
    APHRODY_STATUS_OK = 0,
    /** Malformed request: invalid argv, JSON, or UTF-8 (sysexits EX_USAGE). */
    APHRODY_STATUS_USAGE = 64,
    /** Internal error: a Rust panic was caught (sysexits EX_SOFTWARE). */
    APHRODY_STATUS_SOFTWARE = 70
} AphrodyStatus;

/**
 * @brief Runtime ABI revision of the symbol set exported by this library.
 *
 * Compare against #APHRODY_ABI_VERSION to detect a header/library mismatch.
 *
 * @return The ABI revision (currently #APHRODY_ABI_VERSION).
 * @since ABI 1
 */
APHRODY_NODISCARD APHRODY_API uint32_t aphrody_abi_version(void);

/**
 * @brief The aphrody version string (e.g. `"0.1.0"`).
 *
 * @return A static, NUL-terminated UTF-8 string valid for the entire program
 *         lifetime. Owned by the library; never NULL.
 * @note Do NOT free the returned pointer.
 * @since ABI 1
 */
APHRODY_NODISCARD APHRODY_RETURNS_NONNULL APHRODY_API const char *aphrody_version(void);

/**
 * @brief Run a command with stdout/stderr inherited by the host process.
 *
 * @param argc Number of entries in @p argv. Negative values are rejected
 *             (#APHRODY_STATUS_USAGE).
 * @param argv Array of @p argc NUL-terminated UTF-8 strings, excluding the
 *             program name. May be NULL if and only if @p argc is 0. The
 *             pointers are only read for the duration of the call.
 * @return The command's process exit code, or #APHRODY_STATUS_USAGE /
 *         #APHRODY_STATUS_SOFTWARE on a boundary failure
 *         (see ::aphrody_last_error).
 * @since ABI 1
 */
APHRODY_NODISCARD APHRODY_API int aphrody_run(int argc, const char *const *argv);

/**
 * @brief Run a command whose arguments are a JSON array of strings.
 *
 * Ergonomic alternative to ::aphrody_run for hosts that marshal a JSON string
 * more easily than a `char **` (e.g. Bun). Output is inherited.
 *
 * @param args_json A NUL-terminated UTF-8 JSON array of strings, e.g.
 *                  `"[\"doctor\",\"--json\"]"`. NULL or empty is treated as
 *                  `[]`.
 * @return The command's process exit code, or #APHRODY_STATUS_USAGE /
 *         #APHRODY_STATUS_SOFTWARE on a boundary failure
 *         (see ::aphrody_last_error).
 * @since ABI 1
 */
APHRODY_NODISCARD APHRODY_API int aphrody_run_json(const char *args_json);

/**
 * @brief Run a command with stdout AND stderr captured in-process.
 *
 * @param args_json A NUL-terminated UTF-8 JSON array of strings (NULL or empty
 *                  is treated as `[]`), as for ::aphrody_run_json.
 * @return A newly-allocated, NUL-terminated JSON document of the shape
 *         `{"code":<int>,"stdout":"<text>","stderr":"<text>"}`, or NULL only on
 *         allocation/serialisation failure (cause in ::aphrody_last_error).
 * @warning The CALLER owns a non-NULL result and MUST release it with
 *          ::aphrody_string_free exactly once.
 * @since ABI 1
 */
APHRODY_NODISCARD APHRODY_MALLOC APHRODY_API char *aphrody_run_captured(const char *args_json);

/**
 * @brief Release a string previously returned by this library.
 *
 * Currently the only such string is the result of ::aphrody_run_captured.
 *
 * @param ptr A pointer this library returned and that has not already been
 *            freed, or NULL (a no-op).
 * @since ABI 1
 */
APHRODY_API void aphrody_string_free(char *ptr);

/**
 * @brief The last error recorded on the CURRENT thread.
 *
 * Errors are thread-local: this returns the most recent failure observed by a
 * library call on the calling thread.
 *
 * @return A static, NUL-terminated UTF-8 string, or NULL if no error has been
 *         recorded on this thread. Owned by the library.
 * @note The pointer is valid only until the next library call on this thread —
 *       copy it if you need to keep it. Do NOT free it.
 * @since ABI 1
 */
APHRODY_NODISCARD APHRODY_API const char *aphrody_last_error(void);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* APHRODY_H */
