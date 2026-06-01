/**
 * Injectable LLM client abstraction.
 *
 * The rest of the codebase depends only on the {@link LlmClient} interface, so
 * tests can supply a deterministic in-memory client and never touch the
 * network. The default implementation wraps the Gemini provider of the Vercel
 * AI SDK, with model / base URL / timeout configurable through the environment.
 */
import { createGoogleGenerativeAI } from "@ai-sdk/google";
import { generateText } from "ai";
import { extractStatus, isTransientNetworkError, isRetryableStatus } from "./retry.js";
import { LlmError, toMessage } from "./errors.js";

/** A single-turn text completion. */
export interface LlmRequest {
  readonly prompt: string;
  /** Abort signal so callers can enforce timeouts. */
  readonly signal?: AbortSignal;
}

/**
 * Minimal contract every LLM client must satisfy. Implementations should throw
 * {@link LlmError} on failure (the default one does); a thrown non-LlmError is
 * still handled by the retry layer's heuristics.
 */
export interface LlmClient {
  complete(request: LlmRequest): Promise<string>;
}

export interface LlmEnvConfig {
  /** Resolved API key, or undefined when running offline. */
  readonly apiKey?: string;
  /** Model id, e.g. "gemini-2.5-flash". */
  readonly model: string;
  /** Optional custom Generative Language API base URL. */
  readonly baseURL?: string;
  /** Per-request network timeout in milliseconds. */
  readonly requestTimeoutMs: number;
}

export const DEFAULT_MODEL = "gemini-2.5-flash";
export const DEFAULT_REQUEST_TIMEOUT_MS = 60_000;

/** Source of environment variables (injectable for tests). */
export type EnvSource = Record<string, string | undefined>;

function parsePositiveInt(value: string | undefined, fallback: number): number {
  if (value === undefined) return fallback;
  const n = Number.parseInt(value, 10);
  return Number.isFinite(n) && n > 0 ? n : fallback;
}

/**
 * Resolve LLM configuration from environment variables.
 *
 * - `GEMINI_API_KEY` / `GOOGLE_API_KEY` — credentials (either accepted).
 * - `DOC_AI_MODEL` — override the model id.
 * - `DOC_AI_GEMINI_BASE_URL` — override the API base URL (proxies, regions).
 * - `DOC_AI_REQUEST_TIMEOUT_MS` — per-request network timeout.
 */
export function resolveLlmConfig(env: EnvSource = process.env): LlmEnvConfig {
  const apiKey = env["GEMINI_API_KEY"] || env["GOOGLE_API_KEY"] || undefined;
  return {
    apiKey,
    model: env["DOC_AI_MODEL"]?.trim() || DEFAULT_MODEL,
    baseURL: env["DOC_AI_GEMINI_BASE_URL"]?.trim() || undefined,
    requestTimeoutMs: parsePositiveInt(
      env["DOC_AI_REQUEST_TIMEOUT_MS"],
      DEFAULT_REQUEST_TIMEOUT_MS,
    ),
  };
}

/** True when credentials are present and the live client should be used. */
export function hasCredentials(env: EnvSource = process.env): boolean {
  return Boolean(env["GEMINI_API_KEY"] || env["GOOGLE_API_KEY"]);
}

/**
 * Combine a caller-provided signal with an internal timeout signal so a request
 * aborts on whichever fires first.
 */
function withTimeout(
  timeoutMs: number,
  external?: AbortSignal,
): { signal: AbortSignal; cancel: () => void } {
  const controller = new AbortController();
  const timer = setTimeout(() => {
    controller.abort(new DOMException("Request timed out", "TimeoutError"));
  }, timeoutMs);
  const onAbort = (): void => controller.abort(external?.reason);
  if (external) {
    if (external.aborted) controller.abort(external.reason);
    else external.addEventListener("abort", onAbort, { once: true });
  }
  const cancel = (): void => {
    clearTimeout(timer);
    external?.removeEventListener("abort", onAbort);
  };
  return { signal: controller.signal, cancel };
}

/**
 * Build the default Gemini-backed client. Throws if no API key is configured —
 * callers decide whether to fall back offline based on {@link hasCredentials}.
 */
export function createGeminiClient(config: LlmEnvConfig): LlmClient {
  if (!config.apiKey) {
    throw new LlmError("No Gemini/Google API key configured", { retryable: false });
  }
  const google = createGoogleGenerativeAI({
    apiKey: config.apiKey,
    ...(config.baseURL ? { baseURL: config.baseURL } : {}),
  });

  return {
    async complete({ prompt, signal }: LlmRequest): Promise<string> {
      const { signal: reqSignal, cancel } = withTimeout(config.requestTimeoutMs, signal);
      try {
        const { text } = await generateText({
          model: google(config.model),
          prompt,
          abortSignal: reqSignal,
        });
        return text;
      } catch (err) {
        const status = extractStatus(err);
        const retryable = isRetryableStatus(status) || isTransientNetworkError(err);
        throw new LlmError(`Gemini request failed: ${toMessage(err)}`, {
          cause: err,
          status,
          retryable,
        });
      } finally {
        cancel();
      }
    },
  };
}

/**
 * Resolve the LLM client to use, or `null` when no credentials are present
 * (signalling that callers should take the deterministic offline path).
 */
export function resolveClient(env: EnvSource = process.env): LlmClient | null {
  if (!hasCredentials(env)) return null;
  return createGeminiClient(resolveLlmConfig(env));
}
