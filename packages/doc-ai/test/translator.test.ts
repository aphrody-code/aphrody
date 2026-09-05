import { describe, expect, test } from "bun:test";
import {
  planMarkdownJobs,
  translateMarkdownOffline,
  translateMarkdown,
  buildTranslatePrompt,
  noopSegmentTranslator,
} from "../src/translator.js";
import type { LlmClient } from "../src/llm.js";
import type { SegmentTranslator } from "../src/translator.js";

const DOC = `---
title: Hello
description: A widget
id: keep-me
---

# Heading One

Some paragraph text.

- list item one
- list item two

1. first
2. second

| Name | Type |
| --- | --- |
| foo | bar |

\`\`\`ts
const dontTranslate = 1;
\`\`\`

<!-- a comment -->
`;

describe("planMarkdownJobs (structure preservation)", () => {
  test("skips code blocks, comments, divider rows and non-text frontmatter keys", () => {
    const { jobs } = planMarkdownJobs(DOC);
    const texts = jobs.map((j) => j.text);
    expect(texts).toContain("Hello"); // frontmatter title value
    expect(texts).toContain("A widget"); // frontmatter description value
    expect(texts).not.toContain("keep-me"); // non-translatable frontmatter key
    expect(texts).toContain("Heading One");
    expect(texts).toContain("list item one");
    expect(texts).toContain("first");
    expect(texts).toContain("foo");
    expect(texts).toContain("bar");
    // code block contents never become jobs
    expect(texts.some((t) => t.includes("dontTranslate"))).toBe(false);
    // comment lines never become jobs
    expect(texts.some((t) => t.includes("a comment"))).toBe(false);
    // the |---|---| divider is preserved (not a job)
    expect(texts).not.toContain("---");
  });

  test("is deterministic", () => {
    const a = planMarkdownJobs(DOC).jobs.map((j) => j.text);
    const b = planMarkdownJobs(DOC).jobs.map((j) => j.text);
    expect(a).toEqual(b);
  });
});

describe("translateMarkdownOffline (deterministic no-op default)", () => {
  test("no-op translator returns content byte-for-byte", async () => {
    const out = await translateMarkdownOffline(DOC, "fr");
    expect(out).toBe(DOC);
  });

  test("noopSegmentTranslator is the identity", async () => {
    expect(await noopSegmentTranslator("abc", "fr")).toBe("abc");
  });

  test("applies an injected segment translator while preserving structure", async () => {
    const upper: SegmentTranslator = async (t) => t.toUpperCase();
    const out = await translateMarkdownOffline(DOC, "fr", { segmentTranslator: upper });
    // translatable text is transformed
    expect(out).toContain("# HEADING ONE");
    expect(out).toContain("- LIST ITEM ONE");
    // code block is untouched
    expect(out).toContain("const dontTranslate = 1;");
    // frontmatter key preserved, value transformed (source value is unquoted)
    expect(out).toContain("id: keep-me");
    expect(out).toContain("title: HELLO");
    expect(out).toContain("description: A WIDGET");
    // table divider untouched
    expect(out).toContain("| --- | --- |");
  });

  test("a failing segment translator degrades gracefully (keeps original line)", async () => {
    const warnings: string[] = [];
    const failing: SegmentTranslator = async () => {
      throw new Error("nope");
    };
    const out = await translateMarkdownOffline(DOC, "fr", {
      segmentTranslator: failing,
      warn: (m) => warnings.push(m),
      batchSize: 2,
    });
    expect(out).toContain("# Heading One");
    expect(warnings.length).toBeGreaterThan(0);
  });

  test("preserves quoting of quoted frontmatter values", async () => {
    const doc = `---\ntitle: "Quoted Title"\n---\n\nbody\n`;
    const upper: SegmentTranslator = async (t) => t.toUpperCase();
    const out = await translateMarkdownOffline(doc, "fr", { segmentTranslator: upper });
    expect(out).toContain('title: "QUOTED TITLE"');
  });

  test("empty content yields empty content", async () => {
    expect(await translateMarkdownOffline("", "en")).toBe("");
  });
});

describe("translateMarkdown (public API, injected client)", () => {
  test("offline fallback when client is null (no network)", async () => {
    const out = await translateMarkdown(DOC, "fr", { client: null });
    expect(out).toBe(DOC);
  });

  test("uses the injected client when provided", async () => {
    const client: LlmClient = {
      complete: async ({ prompt }) => {
        expect(prompt).toContain("French");
        return "Translated!";
      },
    };
    const out = await translateMarkdown(DOC, "fr", { client });
    expect(out).toBe("Translated!");
  });

  test("falls back to offline when the client fails after retries", async () => {
    const warnings: string[] = [];
    const client: LlmClient = {
      complete: async () => {
        throw new Error("offline now");
      },
    };
    const out = await translateMarkdown(DOC, "en", {
      client,
      retry: { retries: 1 },
      warn: (m) => warnings.push(m),
    });
    expect(out).toBe(DOC);
    expect(warnings.some((w) => w.includes("falling back"))).toBe(true);
  });

  test("falls back when the client returns empty text", async () => {
    const client: LlmClient = { complete: async () => "" };
    const out = await translateMarkdown(DOC, "fr", { client, warn: () => {} });
    expect(out).toBe(DOC);
  });

  test("no-credentials env resolves offline (no network)", async () => {
    const out = await translateMarkdown(DOC, "fr", { env: {} });
    expect(out).toBe(DOC);
  });
});

describe("buildTranslatePrompt", () => {
  test("selects target language label", () => {
    expect(buildTranslatePrompt("x", "fr")).toContain("French");
    expect(buildTranslatePrompt("x", "en")).toContain("English");
  });
});
