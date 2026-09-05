import { describe, expect, test } from "bun:test";
import {
  resolveLlmConfig,
  hasCredentials,
  resolveClient,
  createGeminiClient,
  DEFAULT_MODEL,
  DEFAULT_REQUEST_TIMEOUT_MS,
} from "../src/llm.js";
import { LlmError } from "../src/errors.js";

describe("resolveLlmConfig", () => {
  test("uses defaults when only a key is present", () => {
    const cfg = resolveLlmConfig({ GEMINI_API_KEY: "k" });
    expect(cfg.apiKey).toBe("k");
    expect(cfg.model).toBe(DEFAULT_MODEL);
    expect(cfg.baseURL).toBeUndefined();
    expect(cfg.requestTimeoutMs).toBe(DEFAULT_REQUEST_TIMEOUT_MS);
  });

  test("GOOGLE_API_KEY is accepted as an alternative", () => {
    expect(resolveLlmConfig({ GOOGLE_API_KEY: "g" }).apiKey).toBe("g");
  });

  test("honours model, base URL and timeout overrides", () => {
    const cfg = resolveLlmConfig({
      GEMINI_API_KEY: "k",
      DOC_AI_MODEL: "gemini-1.5-pro",
      DOC_AI_GEMINI_BASE_URL: "https://proxy.example/v1beta",
      DOC_AI_REQUEST_TIMEOUT_MS: "1234",
    });
    expect(cfg.model).toBe("gemini-1.5-pro");
    expect(cfg.baseURL).toBe("https://proxy.example/v1beta");
    expect(cfg.requestTimeoutMs).toBe(1234);
  });

  test("invalid timeout falls back to the default", () => {
    expect(resolveLlmConfig({ DOC_AI_REQUEST_TIMEOUT_MS: "nope" }).requestTimeoutMs).toBe(
      DEFAULT_REQUEST_TIMEOUT_MS,
    );
    expect(resolveLlmConfig({ DOC_AI_REQUEST_TIMEOUT_MS: "-5" }).requestTimeoutMs).toBe(
      DEFAULT_REQUEST_TIMEOUT_MS,
    );
  });
});

describe("hasCredentials / resolveClient", () => {
  test("hasCredentials reflects key presence", () => {
    expect(hasCredentials({})).toBe(false);
    expect(hasCredentials({ GEMINI_API_KEY: "k" })).toBe(true);
    expect(hasCredentials({ GOOGLE_API_KEY: "k" })).toBe(true);
  });

  test("resolveClient returns null without credentials (offline)", () => {
    expect(resolveClient({})).toBeNull();
  });

  test("resolveClient builds a client when credentials exist", () => {
    const client = resolveClient({ GEMINI_API_KEY: "k" });
    expect(client).not.toBeNull();
    expect(typeof client?.complete).toBe("function");
  });

  test("createGeminiClient throws without an API key", () => {
    expect(() => createGeminiClient({ model: DEFAULT_MODEL, requestTimeoutMs: 1000 })).toThrow(
      LlmError,
    );
  });
});
