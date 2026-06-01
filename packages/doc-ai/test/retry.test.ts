import { describe, expect, test } from "bun:test";
import {
  withRetry,
  defaultIsRetryable,
  isRetryableStatus,
  isTransientNetworkError,
  extractStatus,
} from "../src/retry.js";
import { LlmError, RetryExhaustedError } from "../src/errors.js";

// Deterministic helpers: never actually wait, never random.
const noSleep = async (): Promise<void> => {};
const zeroRandom = (): number => 0;

describe("status / network classification", () => {
  test("isRetryableStatus covers 408/425/429 and 5xx only", () => {
    expect(isRetryableStatus(429)).toBe(true);
    expect(isRetryableStatus(408)).toBe(true);
    expect(isRetryableStatus(425)).toBe(true);
    expect(isRetryableStatus(500)).toBe(true);
    expect(isRetryableStatus(503)).toBe(true);
    expect(isRetryableStatus(400)).toBe(false);
    expect(isRetryableStatus(404)).toBe(false);
    expect(isRetryableStatus(200)).toBe(false);
    expect(isRetryableStatus(undefined)).toBe(false);
  });

  test("extractStatus reads status / statusCode / response.status", () => {
    expect(extractStatus({ status: 503 })).toBe(503);
    expect(extractStatus({ statusCode: 429 })).toBe(429);
    expect(extractStatus({ response: { status: 500 } })).toBe(500);
    expect(extractStatus({})).toBeUndefined();
    expect(extractStatus(null)).toBeUndefined();
    expect(extractStatus("x")).toBeUndefined();
  });

  test("isTransientNetworkError detects aborts, codes and messages", () => {
    expect(isTransientNetworkError({ name: "AbortError" })).toBe(true);
    expect(isTransientNetworkError({ name: "TimeoutError" })).toBe(true);
    expect(isTransientNetworkError({ code: "ETIMEDOUT" })).toBe(true);
    expect(isTransientNetworkError({ code: "ECONNRESET" })).toBe(true);
    expect(isTransientNetworkError(new Error("fetch failed"))).toBe(true);
    expect(isTransientNetworkError(new Error("connection timed out"))).toBe(true);
    expect(isTransientNetworkError(new Error("bad request"))).toBe(false);
    expect(isTransientNetworkError(null)).toBe(false);
  });

  test("defaultIsRetryable honours LlmError flags and status", () => {
    expect(defaultIsRetryable(new LlmError("x", { retryable: true }))).toBe(true);
    expect(defaultIsRetryable(new LlmError("x", { status: 503 }))).toBe(true);
    expect(defaultIsRetryable(new LlmError("x", { status: 400 }))).toBe(false);
    expect(defaultIsRetryable(new LlmError("x"))).toBe(false);
    expect(defaultIsRetryable({ status: 429 })).toBe(true);
    expect(defaultIsRetryable(new Error("plain"))).toBe(false);
  });
});

describe("withRetry", () => {
  test("returns immediately on first success", async () => {
    let calls = 0;
    const result = await withRetry(
      async () => {
        calls++;
        return "ok";
      },
      { sleep: noSleep, random: zeroRandom },
    );
    expect(result).toBe("ok");
    expect(calls).toBe(1);
  });

  test("retries a transient failure then succeeds", async () => {
    let calls = 0;
    const attempts: number[] = [];
    const result = await withRetry(
      async (attempt) => {
        attempts.push(attempt);
        calls++;
        if (calls < 3) throw new LlmError("rate limited", { status: 429 });
        return "recovered";
      },
      { retries: 5, sleep: noSleep, random: zeroRandom },
    );
    expect(result).toBe("recovered");
    expect(calls).toBe(3);
    expect(attempts).toEqual([1, 2, 3]);
  });

  test("does NOT retry a non-retryable error and surfaces it", async () => {
    let calls = 0;
    await expect(
      withRetry(
        async () => {
          calls++;
          throw new LlmError("bad request", { status: 400 });
        },
        { retries: 5, sleep: noSleep, random: zeroRandom },
      ),
    ).rejects.toBeInstanceOf(RetryExhaustedError);
    expect(calls).toBe(1); // stopped after first attempt
  });

  test("throws RetryExhaustedError carrying attempt count and cause", async () => {
    let calls = 0;
    try {
      await withRetry(
        async () => {
          calls++;
          throw new LlmError("still 503", { status: 503 });
        },
        { retries: 3, sleep: noSleep, random: zeroRandom },
      );
      throw new Error("should have thrown");
    } catch (err) {
      expect(err).toBeInstanceOf(RetryExhaustedError);
      const re = err as RetryExhaustedError;
      expect(re.attempts).toBe(3);
      expect(re.code).toBe("LLM_RETRY_EXHAUSTED");
      expect((re.cause as LlmError).status).toBe(503);
    }
    expect(calls).toBe(3);
  });

  test("onRetry fires with computed delay and growing backoff", async () => {
    const delays: number[] = [];
    // random()=1 -> full jitter picks the cap, so delay == min(cap, base*2^(n-1)).
    await withRetry(
      async () => {
        throw new LlmError("503", { status: 503 });
      },
      {
        retries: 4,
        baseDelayMs: 100,
        maxDelayMs: 10_000,
        sleep: noSleep,
        random: () => 0.999999,
        onRetry: (info) => delays.push(info.delayMs),
      },
    ).catch(() => {});
    // 3 retries between 4 attempts; delays grow 100, 200, 400 (floored).
    expect(delays).toHaveLength(3);
    expect(delays[0]).toBe(99);
    expect(delays[1]).toBe(199);
    expect(delays[2]).toBe(399);
  });

  test("respects the delay cap", async () => {
    const delays: number[] = [];
    await withRetry(
      async () => {
        throw new LlmError("503", { status: 503 });
      },
      {
        retries: 6,
        baseDelayMs: 1000,
        maxDelayMs: 2000,
        sleep: noSleep,
        random: () => 0.999999,
        onRetry: (info) => delays.push(info.delayMs),
      },
    ).catch(() => {});
    // Beyond the 2nd retry the exponential exceeds the 2000ms cap.
    expect(Math.max(...delays)).toBeLessThanOrEqual(2000);
  });

  test("custom isRetryable overrides defaults", async () => {
    let calls = 0;
    await withRetry(
      async () => {
        calls++;
        if (calls < 2) throw new Error("plain error normally non-retryable");
        return "done";
      },
      { retries: 3, sleep: noSleep, random: zeroRandom, isRetryable: () => true },
    );
    expect(calls).toBe(2);
  });
});
