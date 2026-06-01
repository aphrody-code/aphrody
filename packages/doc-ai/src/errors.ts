/**
 * Typed error hierarchy for the doc-ai CLI and library.
 *
 * Every operational failure is represented by a subclass of {@link DocAiError},
 * each carrying a stable `code` (used for diagnostics and tests) and an
 * `exitCode` consumed by the CLI to produce correct process exit statuses.
 */

export type DocAiErrorCode =
  | "INVALID_ARGS"
  | "INVALID_PATH"
  | "INVALID_LANGUAGE"
  | "NOT_A_MARKDOWN_FILE"
  | "IO_ERROR"
  | "LLM_ERROR"
  | "LLM_RETRY_EXHAUSTED"
  | "PARSE_ERROR";

/** Base class for all expected (non-bug) failures raised by doc-ai. */
export class DocAiError extends Error {
  readonly code: DocAiErrorCode;
  /** Process exit code the CLI should use for this error. */
  readonly exitCode: number;

  constructor(message: string, code: DocAiErrorCode, exitCode = 1, options?: { cause?: unknown }) {
    super(message, options);
    this.name = new.target.name;
    this.code = code;
    this.exitCode = exitCode;
    // Restore prototype chain (needed when targeting ES2015+ with downlevel emit).
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/** A CLI argument was missing or malformed. */
export class InvalidArgumentsError extends DocAiError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, "INVALID_ARGS", 2, options);
  }
}

/** A target file/directory path is missing, unreadable, or of the wrong kind. */
export class InvalidPathError extends DocAiError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, "INVALID_PATH", 1, options);
  }
}

/** An unsupported target language was requested. */
export class InvalidLanguageError extends DocAiError {
  constructor(value: string, supported: readonly string[]) {
    super(
      `Unsupported target language "${value}". Supported: ${supported.join(", ")}.`,
      "INVALID_LANGUAGE",
      2,
    );
  }
}

/** A filesystem read/write failed. */
export class IoError extends DocAiError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, "IO_ERROR", 1, options);
  }
}

/** A single LLM invocation failed (network, API, etc.). */
export class LlmError extends DocAiError {
  /** HTTP-ish status code if one could be inferred from the failure. */
  readonly status?: number;
  /** Whether this failure is considered transient (retryable). */
  readonly retryable: boolean;

  constructor(
    message: string,
    options?: { cause?: unknown; status?: number; retryable?: boolean },
  ) {
    super(message, "LLM_ERROR", 1, options);
    this.status = options?.status;
    this.retryable = options?.retryable ?? false;
  }
}

/** All retry attempts of a transient operation were exhausted. */
export class RetryExhaustedError extends DocAiError {
  readonly attempts: number;

  constructor(message: string, attempts: number, options?: { cause?: unknown }) {
    super(message, "LLM_RETRY_EXHAUSTED", 1, options);
    this.attempts = attempts;
  }
}

/** A remote response could not be parsed into the expected shape. */
export class ParseError extends DocAiError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, "PARSE_ERROR", 1, options);
  }
}

/** Normalise an unknown thrown value into a readable message. */
export function toMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}
