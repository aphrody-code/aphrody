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

// End-to-end tests that invoke the Rust binary `n2b` via the TS façade.
// Runs against `tests/rpb-dashboard-baseline/scan.json` baseline (bypassing
// subprocess cost) for the shape-level assertions, and against a real
// subprocess call on `test/fixture/` for the live integration check.

import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { scan, binaryVersion } from "../src/cli";
import type { N2BReport } from "@n2b/types";

const REPO_ROOT = join(import.meta.dir, "..", "..", "..");
const FIXTURE = join(REPO_ROOT, "test", "fixture");
const BASELINE = join(REPO_ROOT, "tests", "snapshots", "baseline", "fixture.json");

describe("@n2b/core CLI wrapper", () => {
  test("binaryVersion returns a semver-ish string", async () => {
    const v = await binaryVersion();
    expect(v).toMatch(/^\d+\.\d+\.\d+/);
  });

  test("scan on test/fixture returns a valid N2BReport", async () => {
    const report = await scan(FIXTURE, { quiet: true });
    expect(report.schema_version).toBe(2);
    expect(report.tool).toBe("node2bun");
    expect(report.mode).toBe("check");
    expect(report.files.length).toBeGreaterThan(0);
    for (const f of report.files) {
      expect(f).toHaveProperty("path");
      expect(f).toHaveProperty("changed");
      expect(f).toHaveProperty("findings");
    }
  });

  test("subprocess scan matches the captured baseline", async () => {
    const [live, baseline] = await Promise.all([
      scan(FIXTURE, { quiet: true }),
      Bun.file(BASELINE).json() as Promise<N2BReport>,
    ]);
    // Normalize volatile fields.
    const strip = (r: N2BReport) => ({ ...r, root: "<root>", version: "<version>" });
    expect(strip(live)).toEqual(strip(baseline));
  });
});
