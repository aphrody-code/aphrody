import { describe, expect, test } from "bun:test";
import {
  DocAiError,
  InvalidArgumentsError,
  InvalidPathError,
  InvalidLanguageError,
  IoError,
  LlmError,
  RetryExhaustedError,
  ParseError,
  toMessage,
} from "../src/errors.js";

describe("typed errors", () => {
  test("each error carries a stable code and exit code", () => {
    expect(new InvalidArgumentsError("x").code).toBe("INVALID_ARGS");
    expect(new InvalidArgumentsError("x").exitCode).toBe(2);
    expect(new InvalidPathError("x").code).toBe("INVALID_PATH");
    expect(new InvalidPathError("x").exitCode).toBe(1);
    expect(new InvalidLanguageError("de", ["en", "fr"]).code).toBe("INVALID_LANGUAGE");
    expect(new InvalidLanguageError("de", ["en", "fr"]).exitCode).toBe(2);
    expect(new IoError("x").code).toBe("IO_ERROR");
    expect(new ParseError("x").code).toBe("PARSE_ERROR");
  });

  test("all subclasses are instances of DocAiError and Error", () => {
    for (const e of [
      new InvalidArgumentsError("a"),
      new InvalidPathError("b"),
      new IoError("c"),
      new LlmError("d"),
      new RetryExhaustedError("e", 3),
      new ParseError("f"),
    ]) {
      expect(e).toBeInstanceOf(DocAiError);
      expect(e).toBeInstanceOf(Error);
    }
  });

  test("LlmError tracks status and retryable", () => {
    const e = new LlmError("rate", { status: 429, retryable: true });
    expect(e.status).toBe(429);
    expect(e.retryable).toBe(true);
    const d = new LlmError("plain");
    expect(d.retryable).toBe(false);
    expect(d.status).toBeUndefined();
  });

  test("RetryExhaustedError preserves attempt count and cause", () => {
    const cause = new Error("root");
    const e = new RetryExhaustedError("gave up", 5, { cause });
    expect(e.attempts).toBe(5);
    expect(e.cause).toBe(cause);
  });

  test("InvalidLanguageError lists supported languages in its message", () => {
    const e = new InvalidLanguageError("xx", ["en", "fr"]);
    expect(e.message).toContain("xx");
    expect(e.message).toContain("en, fr");
  });

  test("error name matches the subclass", () => {
    expect(new IoError("x").name).toBe("IoError");
    expect(new LlmError("x").name).toBe("LlmError");
  });
});

describe("toMessage", () => {
  test("handles Error, string, object and primitive", () => {
    expect(toMessage(new Error("boom"))).toBe("boom");
    expect(toMessage("plain")).toBe("plain");
    expect(toMessage({ a: 1 })).toBe('{"a":1}');
    expect(toMessage(42)).toBe("42");
  });

  test("handles circular objects without throwing", () => {
    const circ: Record<string, unknown> = {};
    circ.self = circ;
    expect(typeof toMessage(circ)).toBe("string");
  });
});
