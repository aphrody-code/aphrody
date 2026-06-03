/**
 * Public API for @aphrody/doc-ai.
 *
 * Backward-compatible entry points {@link generateDocumentation} and
 * {@link translateMarkdown} keep their original signatures; the additional
 * exports expose the injectable LLM client, retry policy, validation and typed
 * errors for advanced/embedded use.
 */
export {
  generateDocumentation,
  generateDocsFromCode,
  generateDocsOffline,
  extractMetadata,
  buildDocPrompt,
} from "./generator.js";
export type { GenerateOptions } from "./generator.js";

export {
  translateMarkdown,
  translateMarkdownOffline,
  planMarkdownJobs,
  buildTranslatePrompt,
  noopSegmentTranslator,
  freeTranslate,
} from "./translator.js";
export type { TranslateOptions, OfflineTranslateOptions, SegmentTranslator } from "./translator.js";

export {
  createGeminiClient,
  resolveClient,
  resolveLlmConfig,
  hasCredentials,
  DEFAULT_MODEL,
  DEFAULT_REQUEST_TIMEOUT_MS,
} from "./llm.js";
export type { LlmClient, LlmRequest, LlmEnvConfig, EnvSource } from "./llm.js";

export {
  withRetry,
  defaultIsRetryable,
  isRetryableStatus,
  isTransientNetworkError,
  extractStatus,
} from "./retry.js";
export type { RetryOptions } from "./retry.js";

export { assertLanguage, assertPath, SUPPORTED_LANGUAGES } from "./validate.js";
export type { TargetLanguage, PathKind } from "./validate.js";

export {
  DocAiError,
  InvalidArgumentsError,
  InvalidPathError,
  InvalidLanguageError,
  IoError,
  LlmError,
  RetryExhaustedError,
  ParseError,
  toMessage,
} from "./errors.js";
export type { DocAiErrorCode } from "./errors.js";
