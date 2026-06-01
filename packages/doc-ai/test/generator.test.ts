import { describe, expect, test } from "bun:test";
import {
  extractMetadata,
  generateDocsOffline,
  generateDocsFromCode,
  buildDocPrompt,
} from "../src/generator.js";
import type { LlmClient } from "../src/llm.js";

const SAMPLE = `
import { customElement, property } from "lit/decorators.js";

@customElement("md-widget")
export class MdWidget extends LitElement {
  @property({ type: Boolean }) disabled = false;
  @property() label: string = "hi";

  private fire() {
    this.dispatchEvent(new CustomEvent("change", { bubbles: true }));
  }
}
`;

describe("extractMetadata", () => {
  test("parses tag, class, properties and events", () => {
    const meta = extractMetadata(SAMPLE);
    expect(meta.tagName).toBe("md-widget");
    expect(meta.className).toBe("MdWidget");
    const names = meta.properties.map((p) => p.name);
    expect(names).toContain("disabled");
    expect(names).toContain("label");
    expect(meta.events.map((e) => e.name)).toEqual(["change"]);
  });

  test("does not duplicate the same event", () => {
    const code = `
      class X {
        a() { this.dispatchEvent(new CustomEvent("input")); }
        b() { this.dispatchEvent(new CustomEvent("input")); }
      }`;
    expect(extractMetadata(code).events).toHaveLength(1);
  });

  test("empty source yields empty metadata", () => {
    const meta = extractMetadata("");
    expect(meta.tagName).toBe("");
    expect(meta.properties).toHaveLength(0);
    expect(meta.events).toHaveLength(0);
  });
});

describe("generateDocsOffline (deterministic)", () => {
  test("is pure: identical input produces identical output", () => {
    const meta = extractMetadata(SAMPLE);
    const a = generateDocsOffline(meta, "widget.ts");
    const b = generateDocsOffline(meta, "widget.ts");
    expect(a).toBe(b);
  });

  test("renders title, property and event tables", () => {
    const meta = extractMetadata(SAMPLE);
    const md = generateDocsOffline(meta, "widget.ts");
    expect(md).toContain("# MdWidget (`<md-widget>`)");
    expect(md).toContain("| `disabled` |");
    expect(md).toContain("| `change` |");
    expect(md).toContain("--md-sys-color-primary");
  });

  test("falls back to filename-derived names when metadata is empty", () => {
    const md = generateDocsOffline(extractMetadata(""), "fancy-thing.ts");
    expect(md).toContain("md-fancy-thing");
    expect(md).toContain("Mdfancy-thing");
    expect(md).toContain("No public reactive properties detected.");
    expect(md).toContain("No custom events detected.");
  });
});

describe("generateDocsFromCode (injected client)", () => {
  test("offline path used when client is explicitly null", async () => {
    const out = await generateDocsFromCode(SAMPLE, "widget.ts", { client: null });
    expect(out).toContain("# MdWidget (`<md-widget>`)");
  });

  test("uses the injected client output when provided", async () => {
    const client: LlmClient = {
      complete: async ({ prompt }) => {
        expect(prompt).toContain("md-widget");
        return "# LLM DOC\n\nRich content.";
      },
    };
    const out = await generateDocsFromCode(SAMPLE, "widget.ts", { client });
    expect(out).toBe("# LLM DOC\n\nRich content.");
  });

  test("falls back offline when the client throws (after retries)", async () => {
    const warnings: string[] = [];
    const client: LlmClient = {
      complete: async () => {
        throw new Error("boom");
      },
    };
    const out = await generateDocsFromCode(SAMPLE, "widget.ts", {
      client,
      retry: { retries: 1 },
      warn: (m) => warnings.push(m),
    });
    expect(out).toContain("# MdWidget");
    expect(warnings.some((w) => w.includes("falling back"))).toBe(true);
  });

  test("falls back offline when the client returns empty text", async () => {
    const client: LlmClient = { complete: async () => "   " };
    const out = await generateDocsFromCode(SAMPLE, "widget.ts", {
      client,
      warn: () => {},
    });
    expect(out).toContain("# MdWidget");
  });

  test("env with no credentials resolves to offline (no network)", async () => {
    const out = await generateDocsFromCode(SAMPLE, "widget.ts", { env: {} });
    expect(out).toContain("# MdWidget");
  });
});

describe("buildDocPrompt", () => {
  test("embeds the file name and source code", () => {
    const p = buildDocPrompt("const x = 1;", "thing.ts");
    expect(p).toContain("thing.ts");
    expect(p).toContain("const x = 1;");
    expect(p).toContain("```typescript");
  });
});
