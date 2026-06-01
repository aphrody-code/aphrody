import { describe, expect, test } from "bun:test";
import * as fs from "node:fs/promises";
import * as os from "node:os";
import * as path from "node:path";
import { parseArgs, main, USAGE } from "../src/cli.js";
import { InvalidArgumentsError, InvalidLanguageError } from "../src/errors.js";

describe("parseArgs", () => {
  test("no args -> help", () => {
    expect(parseArgs([])).toEqual({ kind: "help" });
  });

  test("-h / --help -> help", () => {
    expect(parseArgs(["-h"]).kind).toBe("help");
    expect(parseArgs(["--help"]).kind).toBe("help");
  });

  test("-v / --version -> version", () => {
    expect(parseArgs(["-v"]).kind).toBe("version");
    expect(parseArgs(["--version"]).kind).toBe("version");
  });

  test("translate with explicit lang", () => {
    expect(parseArgs(["translate", "docs", "en"])).toEqual({
      kind: "translate",
      targetPath: "docs",
      lang: "en",
    });
  });

  test("translate defaults lang to fr", () => {
    expect(parseArgs(["translate", "docs"])).toEqual({
      kind: "translate",
      targetPath: "docs",
      lang: "fr",
    });
  });

  test("translate without path throws InvalidArgumentsError", () => {
    expect(() => parseArgs(["translate"])).toThrow(InvalidArgumentsError);
  });

  test("translate with unsupported lang throws InvalidLanguageError", () => {
    expect(() => parseArgs(["translate", "docs", "de"])).toThrow(InvalidLanguageError);
  });

  test("generate with and without out file", () => {
    expect(parseArgs(["generate", "a.ts"])).toEqual({
      kind: "generate",
      tsFile: "a.ts",
      outFile: undefined,
    });
    expect(parseArgs(["generate", "a.ts", "out.md"])).toEqual({
      kind: "generate",
      tsFile: "a.ts",
      outFile: "out.md",
    });
  });

  test("generate without file throws", () => {
    expect(() => parseArgs(["generate"])).toThrow(InvalidArgumentsError);
  });

  test("sync defaults to fr and accepts a lang", () => {
    expect(parseArgs(["sync"])).toEqual({ kind: "sync", lang: "fr" });
    expect(parseArgs(["sync", "en"])).toEqual({ kind: "sync", lang: "en" });
  });

  test("unknown command throws", () => {
    expect(() => parseArgs(["frobnicate"])).toThrow(InvalidArgumentsError);
  });

  test("language is case-insensitive and trimmed", () => {
    expect(parseArgs(["sync", "FR"])).toEqual({ kind: "sync", lang: "fr" });
  });
});

describe("main (exit codes)", () => {
  test("empty invocation returns 1 (historical behaviour)", async () => {
    expect(await main([])).toBe(1);
  });

  test("--help returns 0", async () => {
    expect(await main(["--help"])).toBe(0);
  });

  test("--version returns 0", async () => {
    expect(await main(["--version"])).toBe(0);
  });

  test("invalid args return exit code 2", async () => {
    expect(await main(["translate"])).toBe(2);
    expect(await main(["bogus"])).toBe(2);
    expect(await main(["translate", "x", "de"])).toBe(2);
  });

  test("missing path for generate returns non-zero (InvalidPathError, code 1)", async () => {
    const code = await main(["generate", "/no/such/file/xyz.ts"]);
    expect(code).toBe(1);
  });

  test("end-to-end generate offline writes a markdown file (exit 0)", async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), "doc-ai-cli-"));
    const tsFile = path.join(dir, "thing.ts");
    await fs.writeFile(tsFile, '@customElement("md-thing")\nexport class MdThing {}\n', "utf-8");
    // Force offline: clear any inherited credentials for this process call.
    const prevG = process.env.GOOGLE_API_KEY;
    const prevGem = process.env.GEMINI_API_KEY;
    delete process.env.GOOGLE_API_KEY;
    delete process.env.GEMINI_API_KEY;
    try {
      const code = await main(["generate", tsFile]);
      expect(code).toBe(0);
      const outMd = await fs.readFile(path.join(dir, "thing.md"), "utf-8");
      expect(outMd).toContain("md-thing");
    } finally {
      if (prevG !== undefined) process.env.GOOGLE_API_KEY = prevG;
      if (prevGem !== undefined) process.env.GEMINI_API_KEY = prevGem;
      await fs.rm(dir, { recursive: true, force: true });
    }
  });
});

describe("USAGE", () => {
  test("documents commands, languages and env knobs", () => {
    expect(USAGE).toContain("translate");
    expect(USAGE).toContain("generate");
    expect(USAGE).toContain("sync");
    expect(USAGE).toContain("GEMINI_API_KEY");
    expect(USAGE).toContain("DOC_AI_MODEL");
    expect(USAGE).toContain("en, fr");
  });
});
