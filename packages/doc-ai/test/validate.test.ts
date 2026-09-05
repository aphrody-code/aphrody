import { describe, expect, test } from "bun:test";
import * as fs from "node:fs/promises";
import * as os from "node:os";
import * as path from "node:path";
import { assertLanguage, assertPath, SUPPORTED_LANGUAGES } from "../src/validate.js";
import { InvalidLanguageError, InvalidPathError } from "../src/errors.js";

describe("assertLanguage", () => {
  test("accepts supported languages", () => {
    expect(assertLanguage("fr")).toBe("fr");
    expect(assertLanguage("en")).toBe("en");
  });

  test("normalises case and whitespace", () => {
    expect(assertLanguage("  FR ")).toBe("fr");
    expect(assertLanguage("EN")).toBe("en");
  });

  test("rejects unsupported languages with a typed error", () => {
    expect(() => assertLanguage("de")).toThrow(InvalidLanguageError);
    try {
      assertLanguage("es");
    } catch (err) {
      expect(err).toBeInstanceOf(InvalidLanguageError);
      expect((err as InvalidLanguageError).exitCode).toBe(2);
      expect((err as InvalidLanguageError).message).toContain("es");
    }
  });

  test("SUPPORTED_LANGUAGES is exactly en + fr", () => {
    expect([...SUPPORTED_LANGUAGES]).toEqual(["en", "fr"]);
  });
});

describe("assertPath", () => {
  test("empty path is rejected", async () => {
    await expect(assertPath("")).rejects.toBeInstanceOf(InvalidPathError);
    await expect(assertPath("   ")).rejects.toBeInstanceOf(InvalidPathError);
  });

  test("non-existent path throws InvalidPathError", async () => {
    await expect(assertPath("/definitely/not/here/zzz")).rejects.toBeInstanceOf(InvalidPathError);
  });

  test("resolves an existing file and enforces kind", async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), "doc-ai-val-"));
    const file = path.join(dir, "f.md");
    await fs.writeFile(file, "x", "utf-8");
    try {
      expect(await assertPath(file, "file")).toBe(file);
      expect(await assertPath(dir, "directory")).toBe(dir);
      // kind mismatch
      await expect(assertPath(file, "directory")).rejects.toBeInstanceOf(InvalidPathError);
      await expect(assertPath(dir, "file")).rejects.toBeInstanceOf(InvalidPathError);
      // "any" accepts both
      expect(await assertPath(file, "any")).toBe(file);
      expect(await assertPath(dir, "any")).toBe(dir);
    } finally {
      await fs.rm(dir, { recursive: true, force: true });
    }
  });
});
