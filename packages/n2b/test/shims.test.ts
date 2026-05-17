// Copyright 2026 aphrody-code
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { describe, expect, test } from "bun:test";
import { env, fs, path, shell } from "@n2b/shims";
import { EnvError } from "@n2b/shims/env";

describe("shims/env", () => {
  test("str respects default when absent", () => {
    expect(env.str("DEFINITELY_NOT_SET_XYZ", { default: "fallback" })).toBe("fallback");
  });

  test("str throws EnvError when required + missing", () => {
    expect(() => env.str("DEFINITELY_NOT_SET_XYZ", { required: true })).toThrow(EnvError);
  });

  test("int parses integers", () => {
    Bun.env.TEST_INT_SHIM = "42";
    expect(env.int("TEST_INT_SHIM")).toBe(42);
    delete Bun.env.TEST_INT_SHIM;
  });

  test("bool accepts truthy strings", () => {
    Bun.env.TEST_BOOL_SHIM = "yes";
    expect(env.bool("TEST_BOOL_SHIM")).toBe(true);
    Bun.env.TEST_BOOL_SHIM = "off";
    expect(env.bool("TEST_BOOL_SHIM")).toBe(false);
    delete Bun.env.TEST_BOOL_SHIM;
  });

  test("json parses valid JSON", () => {
    Bun.env.TEST_JSON_SHIM = '{"a":1}';
    expect(env.json("TEST_JSON_SHIM")).toEqual({ a: 1 });
    delete Bun.env.TEST_JSON_SHIM;
  });
});

describe("shims/path", () => {
  test("dirOf returns the importing module's directory", () => {
    const d = path.dirOf(import.meta);
    expect(d.endsWith("/test")).toBe(true);
  });

  test("fileOf returns the importing module's path", () => {
    const f = path.fileOf(import.meta);
    expect(f.endsWith("/shims.test.ts")).toBe(true);
  });

  test("relativeTo resolves relative segments", () => {
    const r = path.relativeTo(import.meta, "..", "src", "schema.ts");
    expect(r.endsWith("/packages/n2b/src/schema.ts")).toBe(true);
  });
});

describe("shims/fs", () => {
  test("readText + exists + size", async () => {
    const tmp = `/tmp/n2b-shim-${Bun.env.PID ?? process.pid}.txt`;
    await fs.writeText(tmp, "hello");
    expect(await fs.exists(tmp)).toBe(true);
    expect(await fs.size(tmp)).toBe(5);
    expect(await fs.readText(tmp)).toBe("hello");
    await Bun.$`rm -f ${tmp}`.quiet();
  });

  test("readJson round-trips", async () => {
    const tmp = `/tmp/n2b-shim-${Bun.env.PID ?? process.pid}.json`;
    await fs.writeJson(tmp, { x: 1, y: "two" });
    expect(await fs.readJson(tmp)).toEqual({ x: 1, y: "two" });
    await Bun.$`rm -f ${tmp}`.quiet();
  });
});

describe("shims/shell", () => {
  test("run captures stdout + stderr + exit code", async () => {
    const r = await shell.run("echo hi && echo err 1>&2 && exit 3");
    expect(r.stdout.trim()).toBe("hi");
    expect(r.stderr.trim()).toBe("err");
    expect(r.code).toBe(3);
  });
});
