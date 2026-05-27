// SPDX-License-Identifier: Apache-2.0
//! Smoke test for the in-process aphrody native bridge.
//
// Requires the `aphrody-ffi` cdylib built in the sibling Rust repo
// (`cargo build -p aphrody-ffi`). If the library is not present, `dlopen`
// throws at module load; the dynamic import below catches that and the suite
// is skipped, so this never fails an environment that has not built the lib.

import { describe, expect, test } from "bun:test";

type NativeModule = typeof import("./src/index.ts");

let native: NativeModule | null = null;
try {
    native = await import("./src/index.ts");
} catch {
    native = null;
}

const suite = native ? describe : describe.skip;

suite("@aphrody-code/native", () => {
    test("ABI matches the binding", () => {
        const mod = native as NativeModule;
        expect(() => mod.assertCompatible()).not.toThrow();
        expect(mod.abiVersion()).toBe(mod.EXPECTED_ABI_VERSION);
    });

    test("version() returns a non-empty string", () => {
        const mod = native as NativeModule;
        expect(mod.version().length).toBeGreaterThan(0);
    });

    test("runCaptured(version --json) parses, exit 0", () => {
        const mod = native as NativeModule;
        const result = mod.runCaptured(["version", "--json"]);
        expect(result.code).toBe(0);
        const parsed = JSON.parse(result.stdout) as { version: string };
        expect(parsed.version.length).toBeGreaterThan(0);
    });

    test("run(version) inherits stdio and exits 0", () => {
        const mod = native as NativeModule;
        expect(mod.run(["version"])).toBe(0);
    });
});
