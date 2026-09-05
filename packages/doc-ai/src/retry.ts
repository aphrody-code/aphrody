/**
 * Retry with exponential backoff and full jitter, for transient LLM/network
 * failures (HTTP 429, 5xx, and timeouts/aborts).
 */
import { LlmError, RetryExhaustedError, toMessage } from "./errors.js";

export interface RetryOptions {
  /** Maximum number of attempts (>= 1). Default 4. */
  readonly retries?: number;
  /** Base delay in milliseconds for the first backoff. Default 300. */
  readonly baseDelayMs?: number;
  /** Cap on any single backoff delay in milliseconds. Default 8000. */
  readonly maxDelayMs?: number;
  /** Returns true if an error should trigger another attempt. */
  readonly isRetryable?: (err: unknown) => boolean;
  /** Sleep implementation (injectable for deterministic tests). */
  readonly sleep?: (ms: number) => Promise<void>;
  /** Random source in [0, 1) for jitter (injectable for deterministic tests). */
  readonly random?: () => number;
  /** Invoked before each retry, after the delay is computed. */
  readonly onRetry?: (info: { attempt: number; delayMs: number; error: unknown }) => void;
}

const DEFAULT_SLEEP = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

/** HTTP status codes considered transient and therefore retryable. */
export function isRetryableStatus(status: number | undefined): boolean {
  if (status === undefined) return false;
  return status === 408 || status === 425 || status === 429 || (status >= 500 && status <= 599);
}

/**
 * Best-effort extraction of an HTTP-ish status code from an arbitrary error
 * shape (AI SDK errors, fetch Response errors, plain objects).
 */
export function extractStatus(err: unknown): number | undefined {
  if (typeof err !== "object" || err === null) return undefined;
  const anyErr = err as Record<string, unknown>;
  const candidates = [
    anyErr["status"],
    anyErr["statusCode"],
    (anyErr["response"] as Record<string, unknown> | undefined)?.["status"],
  ];
  for (const c of candidates) {
    if (typeof c === "number" && Number.isFinite(c)) return c;
  }
  return undefined;
}

/** Heuristic: is this a network timeout / abort / connection reset? */
export function isTransientNetworkError(err: unknown): boolean {
  if (typeof err !== "object" || err === null) return false;
  const anyErr = err as { name?: unknown; code?: unknown; message?: unknown };
  const name = typeof anyErr.name === "string" ? anyErr.name : "";
  const code = typeof anyErr.code === "string" ? anyErr.code : "";
  const message = typeof anyErr.message === "string" ? anyErr.message.toLowerCase() : "";
  if (name === "AbortError" || name === "TimeoutError") return true;
  if (["ETIMEDOUT", "ECONNRESET", "ECONNREFUSED", "EAI_AGAIN", "ENOTFOUND"].includes(code)) {
    return true;
  }
  return (
    message.includes("timeout") ||
    message.includes("timed out") ||
    message.includes("network") ||
    message.includes("fetch failed")
  );
}

/** Default policy used across LLM calls. */
export function defaultIsRetryable(err: unknown): boolean {
  if (err instanceof LlmError) {
    if (err.retryable) return true;
    return isRetryableStatus(err.status);
  }
  return isRetryableStatus(extractStatus(err)) || isTransientNetworkError(err);
}

/**
 * Run `fn` with exponential backoff + full jitter. Re-throws the last error
 * (wrapped in {@link RetryExhaustedError}) once attempts are exhausted, or
 * immediately if the error is judged non-retryable.
 */
export async function withRetry<T>(
  fn: (attempt: number) => Promise<T>,
  options: RetryOptions = {},
): Promise<T> {
  const retries = Math.max(1, options.retries ?? 4);
  const baseDelayMs = Math.max(0, options.baseDelayMs ?? 300);
  const maxDelayMs = Math.max(baseDelayMs, options.maxDelayMs ?? 8000);
  const isRetryable = options.isRetryable ?? defaultIsRetryable;
  const sleep = options.sleep ?? DEFAULT_SLEEP;
  const random = options.random ?? Math.random;

  let lastError: unknown;
  for (let attempt = 1; attempt <= retries; attempt++) {
    try {
      return await fn(attempt);
    } catch (err) {
      lastError = err;
      const hasAttemptsLeft = attempt < retries;
      if (!hasAttemptsLeft || !isRetryable(err)) {
        break;
      }
      // Exponential backoff with full jitter: rand(0, min(cap, base * 2^(attempt-1))).
      const exp = baseDelayMs * 2 ** (attempt - 1);
      const delayMs = Math.floor(random() * Math.min(maxDelayMs, exp));
      options.onRetry?.({ attempt, delayMs, error: err });
      await sleep(delayMs);
    }
  }

  throw new RetryExhaustedError(
    `Operation failed after ${retries} attempt(s): ${toMessage(lastError)}`,
    retries,
    { cause: lastError },
  );
}
