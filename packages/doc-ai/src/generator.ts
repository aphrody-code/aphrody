import * as fs from "node:fs/promises";
import * as path from "node:path";
import type { LlmClient } from "./llm.js";
import { resolveClient, resolveLlmConfig } from "./llm.js";
import { withRetry, type RetryOptions } from "./retry.js";
import { IoError, toMessage } from "./errors.js";

interface ComponentMetadata {
  tagName: string;
  className: string;
  properties: Array<{ name: string; type: string; defaultVal?: string; description?: string }>;
  events: Array<{ name: string; description?: string }>;
  slots: Array<{ name: string; description?: string }>;
}

/**
 * Extracts component metadata from typescript source files using regex.
 */
export function extractMetadata(code: string): ComponentMetadata {
  const meta: ComponentMetadata = {
    tagName: "",
    className: "",
    properties: [],
    events: [],
    slots: [],
  };

  // Find customElement decorator
  const customElementMatch = code.match(/@customElement\((['"])(.*?)\1\)/);
  if (customElementMatch) {
    meta.tagName = customElementMatch[2];
  }

  // Find class name
  const classMatch = code.match(/export\s+class\s+(\w+)/);
  if (classMatch) {
    meta.className = classMatch[1];
  }

  // Find properties: @property(...) name = val;
  const propRegex =
    /@property\(([\s\S]*?)\)\s*(?:@\w+\s*)*(\w+)(?:\??\s*:\s*([^=;]+))?(?:\s*=\s*([^;]+))?/g;
  let match;
  while ((match = propRegex.exec(code)) !== null) {
    const [, options, name, type, defaultVal] = match;
    meta.properties.push({
      name: name.trim(),
      type: (type || "string").trim(),
      defaultVal: defaultVal ? defaultVal.trim() : undefined,
      description: options.includes("type:")
        ? `Reactive property with type configuration: ${options.trim()}`
        : "Reactive property",
    });
  }

  // Find custom events: new CustomEvent('event-name', ...)
  const eventRegex = /this\.dispatchEvent\(\s*new\s*(?:CustomEvent|Event)\(\s*(['"])(.*?)\1/g;
  while ((match = eventRegex.exec(code)) !== null) {
    const name = match[2];
    if (!meta.events.some((e) => e.name === name)) {
      meta.events.push({ name, description: "Dispatched component event" });
    }
  }

  return meta;
}

/**
 * Offline fallback generator using static component analysis. Pure and
 * deterministic: identical input always yields identical output.
 */
export function generateDocsOffline(meta: ComponentMetadata, fileName: string): string {
  const tagName = meta.tagName || `md-${path.basename(fileName, ".ts")}`;
  const className = meta.className || `Md${path.basename(fileName, ".ts")}`;

  let markdown = `# ${className} (\`<${tagName}>\`)\n\n`;
  markdown += `Component documentation generated statically from \`${path.basename(fileName)}\`.\n\n`;

  markdown += `## API Reference\n\n`;
  markdown += `### Properties\n\n`;

  if (meta.properties.length > 0) {
    markdown += `| Property | Type | Default | Description |\n`;
    markdown += `| --- | --- | --- | --- |\n`;
    for (const prop of meta.properties) {
      markdown += `| \`${prop.name}\` | \`${prop.type}\` | \`${prop.defaultVal || "undefined"}\` | ${prop.description} |\n`;
    }
  } else {
    markdown += `No public reactive properties detected.\n`;
  }

  markdown += `\n### Events\n\n`;
  if (meta.events.length > 0) {
    markdown += `| Event | Description |\n`;
    markdown += `| --- | --- |\n`;
    for (const event of meta.events) {
      markdown += `| \`${event.name}\` | ${event.description} |\n`;
    }
  } else {
    markdown += `No custom events detected.\n`;
  }

  markdown += `\n### CSS Shadow Parts & Variables\n\n`;
  markdown += `| Token / Part | Fallback | Description |\n`;
  markdown += `| --- | --- | --- |\n`;
  markdown += `| \`--md-sys-color-primary\` | \`#6750a4\` | Theme primary color |\n`;

  return markdown;
}

/** Build the LLM prompt for documentation generation. */
export function buildDocPrompt(code: string, fileName: string): string {
  return `You are a technical writer specialized in Material Design 3 and Lit Web Components.
Analyze the following source code for a web component (file name: ${fileName}) and generate a complete, premium Markdown documentation file for it.
Include:
1. Title: Component name and tag name.
2. Introduction: What the component does, its visual representation, and alignment with M3 guidelines.
3. HTML Example: A clear, clean specimen example showing standard usage.
4. React Wrapper Example: Examples of import and consumption using @aphrody/m3-react wrapper.
5. API Reference: Tables for:
   - Reactive properties/attributes (type, default, description).
   - Custom events dispatched.
   - Slotted elements.
   - CSS custom properties (design tokens like --md-sys-color-*) and shadow parts.
6. Design Details: Description of states (hover, focus, pressed, active, disabled) and sizing/elevation rules.

Output ONLY the markdown documentation file. Do not include any introduction or wrapper.

Source Code:
\`\`\`typescript
${code}
\`\`\``;
}

export interface GenerateOptions {
  /** LLM client; when omitted it is resolved from the environment. */
  client?: LlmClient | null;
  /** Retry policy overrides (mainly for tests). */
  retry?: RetryOptions;
  /** Environment source for client resolution (defaults to process.env). */
  env?: Record<string, string | undefined>;
  /** Sink for non-fatal warnings (defaults to console.warn). */
  warn?: (message: string, detail?: unknown) => void;
}

/**
 * Generate documentation from already-loaded source code. Pure with respect to
 * the filesystem; network access happens only through the injected `client`.
 * Falls back to the deterministic offline generator when no client is supplied
 * or when the LLM call fails after retries.
 */
export async function generateDocsFromCode(
  code: string,
  fileName: string,
  options: GenerateOptions = {},
): Promise<string> {
  const warn = options.warn ?? ((m, d) => console.warn(m, d));
  const client =
    options.client !== undefined ? options.client : resolveClient(options.env ?? process.env);

  if (client) {
    const prompt = buildDocPrompt(code, fileName);
    try {
      const text = await withRetry(() => client.complete({ prompt }), options.retry ?? {});
      if (text && text.trim()) return text;
      warn("LLM returned empty documentation, falling back to static offline template.");
    } catch (err) {
      warn("LLM doc generation failed, falling back to static offline template.", toMessage(err));
    }
  }

  const meta = extractMetadata(code);
  return generateDocsOffline(meta, fileName);
}

/**
 * Public document generator API. Reads `filePath`, then delegates to
 * {@link generateDocsFromCode}. Signature is unchanged for backward
 * compatibility; an optional `options` argument enables dependency injection.
 */
export async function generateDocumentation(
  filePath: string,
  options: GenerateOptions = {},
): Promise<string> {
  let code: string;
  try {
    code = await fs.readFile(filePath, "utf-8");
  } catch (err) {
    throw new IoError(`Failed to read source file ${filePath}: ${toMessage(err)}`, {
      cause: err,
    });
  }
  const fileName = path.basename(filePath);
  return generateDocsFromCode(code, fileName, options);
}

/** Re-exported so callers can preflight model/endpoint configuration. */
export { resolveLlmConfig };
