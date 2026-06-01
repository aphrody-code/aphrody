import type { LlmClient } from "./llm.js";
import { resolveClient } from "./llm.js";
import { withRetry, type RetryOptions } from "./retry.js";
import { extractStatus, isRetryableStatus, isTransientNetworkError } from "./retry.js";
import { LlmError, ParseError, toMessage } from "./errors.js";
import type { TargetLanguage } from "./validate.js";

/**
 * Translates a single text segment to `targetLanguage`. Implementations may be
 * network-backed; the offline pipeline defaults to a deterministic no-op so it
 * never touches the network.
 */
export type SegmentTranslator = (text: string, targetLanguage: TargetLanguage) => Promise<string>;

/**
 * Deterministic no-op segment translator: returns the input unchanged. This is
 * the default offline behaviour — structure-preserving, side-effect free, and
 * fully testable without network access.
 */
export const noopSegmentTranslator: SegmentTranslator = async (text) => text;

/**
 * Network-backed segment translator using Google Translate's free HTTP
 * endpoint. NOT used by default; callers must opt in explicitly. Exposed for
 * environments that want machine translation without an LLM key.
 */
export async function freeTranslate(text: string, targetLanguage: TargetLanguage): Promise<string> {
  if (!text.trim()) return text;
  const url = `https://translate.googleapis.com/translate_a/single?client=gtx&dt=t&sl=auto&tl=${targetLanguage}&q=${encodeURIComponent(text)}`;

  let response: Response;
  try {
    response = await fetch(url);
  } catch (err) {
    throw new LlmError(`Free translation request failed: ${toMessage(err)}`, {
      cause: err,
      retryable: isTransientNetworkError(err),
    });
  }
  if (!response.ok) {
    throw new LlmError(`Free translation request failed: ${response.statusText}`, {
      status: response.status,
      retryable: isRetryableStatus(response.status),
    });
  }
  let result: unknown;
  try {
    result = await response.json();
    return (result as [Array<[string]>])[0].map((x) => x[0]).join("");
  } catch (err) {
    throw new ParseError("Failed to parse free translation response", { cause: err });
  }
}

interface TranslationJob {
  lineIndex: number;
  text: string;
  apply: (translatedText: string) => void;
}

/**
 * Split markdown into translatable jobs while preserving structure: code
 * blocks, frontmatter (except title/description/summary values), HTML comments
 * and table divider rows are left untouched. Pure and deterministic.
 */
export function planMarkdownJobs(content: string): { lines: string[]; jobs: TranslationJob[] } {
  const lines = content.split("\n");
  const translatedLines = [...lines];
  const jobs: TranslationJob[] = [];
  let inCodeBlock = false;
  let inFrontmatter = false;
  let frontmatterCount = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Frontmatter check (usually at the very start of markdown)
    if (trimmed === "---") {
      frontmatterCount++;
      inFrontmatter = frontmatterCount === 1 && i === 0;
      continue;
    }

    if (inFrontmatter) {
      const match = line.match(/^([a-zA-Z0-9_-]+):\s*(.*)$/);
      if (match) {
        const [, key, val] = match;
        const valTrim = val.trim();
        if ((key === "title" || key === "description" || key === "summary") && valTrim) {
          const isQuoted =
            (valTrim.startsWith('"') && valTrim.endsWith('"')) ||
            (valTrim.startsWith("'") && valTrim.endsWith("'"));
          const cleanVal = isQuoted ? valTrim.slice(1, -1) : valTrim;

          jobs.push({
            lineIndex: i,
            text: cleanVal,
            apply: (transVal) => {
              translatedLines[i] = `${key}: ${isQuoted ? `"${transVal}"` : transVal}`;
            },
          });
        }
      }
      continue;
    }

    // Code block check
    if (trimmed.startsWith("```")) {
      inCodeBlock = !inCodeBlock;
      continue;
    }

    if (inCodeBlock) {
      continue;
    }

    // HTML comments check
    if (trimmed.startsWith("<!--") || trimmed.endsWith("-->")) {
      continue;
    }

    // Preserving markdown tables divider line: |---|---|
    if (trimmed.startsWith("|") && trimmed.includes("-") && !trimmed.match(/[a-zA-Z]/)) {
      continue;
    }

    if (trimmed) {
      // Headings
      const headingMatch = line.match(/^(#+)\s+(.*)$/);
      if (headingMatch) {
        const [, hashes, text] = headingMatch;
        jobs.push({
          lineIndex: i,
          text,
          apply: (transText) => {
            translatedLines[i] = `${hashes} ${transText}`;
          },
        });
        continue;
      }

      // List items
      const listMatch = line.match(/^(\s*[-*+]\s+)(.*)$/);
      if (listMatch) {
        const [, prefix, text] = listMatch;
        jobs.push({
          lineIndex: i,
          text,
          apply: (transText) => {
            translatedLines[i] = `${prefix}${transText}`;
          },
        });
        continue;
      }

      // Numeric list items
      const numListMatch = line.match(/^(\s*\d+\.\s+)(.*)$/);
      if (numListMatch) {
        const [, prefix, text] = numListMatch;
        jobs.push({
          lineIndex: i,
          text,
          apply: (transText) => {
            translatedLines[i] = `${prefix}${transText}`;
          },
        });
        continue;
      }

      // Tables (cell by cell)
      if (trimmed.startsWith("|") && trimmed.endsWith("|")) {
        const cells = line.split("|");
        for (let j = 1; j < cells.length - 1; j++) {
          const cell = cells[j];
          const cellTrim = cell.trim();
          if (cellTrim) {
            jobs.push({
              lineIndex: i,
              text: cellTrim,
              apply: (transCell) => {
                const leftPadding = cell.match(/^\s*/)?.[0] || "";
                const rightPadding = cell.match(/\s*$/)?.[0] || "";
                cells[j] = `${leftPadding}${transCell}${rightPadding}`;
                translatedLines[i] = cells.join("|");
              },
            });
          }
        }
        continue;
      }

      // General paragraph
      jobs.push({
        lineIndex: i,
        text: line,
        apply: (transLine) => {
          translatedLines[i] = transLine;
        },
      });
    }
  }

  return { lines: translatedLines, jobs };
}

export interface OfflineTranslateOptions {
  /** Segment translator; defaults to the deterministic no-op. */
  segmentTranslator?: SegmentTranslator;
  /** Parallel batch size for segment translation. Default 15. */
  batchSize?: number;
  /** Sink for non-fatal warnings (defaults to console.warn). */
  warn?: (message: string, detail?: unknown) => void;
}

/**
 * Structure-preserving offline translation pipeline. With the default no-op
 * segment translator this returns the input content unchanged (deterministic),
 * which is the correct behaviour when no translation backend is available.
 */
export async function translateMarkdownOffline(
  content: string,
  targetLanguage: TargetLanguage,
  options: OfflineTranslateOptions = {},
): Promise<string> {
  const translate = options.segmentTranslator ?? noopSegmentTranslator;
  const batchSize = Math.max(1, options.batchSize ?? 15);
  const warn = options.warn ?? ((m, d) => console.warn(m, d));
  const { lines, jobs } = planMarkdownJobs(content);

  for (let i = 0; i < jobs.length; i += batchSize) {
    const batch = jobs.slice(i, i + batchSize);
    await Promise.all(
      batch.map(async (job) => {
        try {
          const trans = await translate(job.text, targetLanguage);
          job.apply(trans);
        } catch (err) {
          warn(`Failed to translate line ${job.lineIndex}: "${job.text}"`, toMessage(err));
        }
      }),
    );
  }

  return lines.join("\n");
}

/** Build the LLM prompt for full-document translation. */
export function buildTranslatePrompt(content: string, targetLanguage: TargetLanguage): string {
  return `You are a professional documentation translator.
Translate the following Markdown documentation into ${targetLanguage === "fr" ? "French" : "English"}.
Ensure you strictly follow these rules:
1. Do NOT translate code blocks (\`\`\`ts ... \`\`\`), inline code (\`...\`), HTML tags, or HTML comments.
2. Maintain all Markdown syntax structure, links [link text](url), image tags, and YAML frontmatter keys exactly. Only translate the descriptive strings/text.
3. Keep the translation natural and matching technical developer conventions.
4. Output ONLY the translated markdown content. Do not include any introduction or wrapper.

Content to translate:
${content}`;
}

export interface TranslateOptions {
  /** LLM client; when omitted it is resolved from the environment. */
  client?: LlmClient | null;
  /** Retry policy overrides (mainly for tests). */
  retry?: RetryOptions;
  /** Options forwarded to the offline pipeline fallback. */
  offline?: OfflineTranslateOptions;
  /** Environment source for client resolution (defaults to process.env). */
  env?: Record<string, string | undefined>;
  /** Sink for non-fatal warnings (defaults to console.warn). */
  warn?: (message: string, detail?: unknown) => void;
}

/** LLM-backed whole-document translation through an injected client. */
async function translateMarkdownLLM(
  content: string,
  targetLanguage: TargetLanguage,
  client: LlmClient,
  retry: RetryOptions,
): Promise<string> {
  const prompt = buildTranslatePrompt(content, targetLanguage);
  try {
    return await withRetry(() => client.complete({ prompt }), retry);
  } catch (err) {
    // Normalise into an LlmError so the caller can decide on fallback.
    if (err instanceof LlmError) throw err;
    const status = extractStatus(err);
    throw new LlmError(`Translation request failed: ${toMessage(err)}`, {
      cause: err,
      status,
      retryable: isRetryableStatus(status) || isTransientNetworkError(err),
    });
  }
}

/**
 * Public translation API. Uses the LLM client when credentials are configured,
 * otherwise the deterministic offline pipeline. Signature is backward
 * compatible; an optional `options` argument enables dependency injection.
 */
export async function translateMarkdown(
  content: string,
  targetLanguage: TargetLanguage,
  options: TranslateOptions = {},
): Promise<string> {
  const warn = options.warn ?? ((m, d) => console.warn(m, d));
  const client =
    options.client !== undefined ? options.client : resolveClient(options.env ?? process.env);

  if (client) {
    try {
      const text = await translateMarkdownLLM(content, targetLanguage, client, options.retry ?? {});
      if (text && text.trim()) return text;
      warn("LLM returned empty translation, falling back to offline pipeline.");
    } catch (err) {
      warn(
        "LLM translation failed, falling back to offline markdown translation pipeline.",
        toMessage(err),
      );
    }
  }

  return translateMarkdownOffline(content, targetLanguage, options.offline);
}
