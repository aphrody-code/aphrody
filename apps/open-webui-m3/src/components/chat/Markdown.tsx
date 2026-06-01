// Tiny zero-dependency Markdown renderer (the Bun + TanStack rule forbids pulling
// a markdown lib). Handles fenced code, headings, lists, blockquotes, paragraphs,
// and inline bold/italic/code/links — enough to render LLM replies in M3 style.

import { Fragment, type ReactNode } from "react";
import { MdIcon, MdIconButton } from "@aphrody-code/m3-react";

let keySeq = 0;
const nextKey = () => `md-${keySeq++}`;

function inline(text: string): ReactNode[] {
  const out: ReactNode[] = [];
  // Order matters: code first (so ** inside code is literal), then link, bold, italic.
  const re = /(`[^`]+`)|(\[[^\]]+\]\([^)]+\))|(\*\*[^*]+\*\*)|(\*[^*]+\*|_[^_]+_)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const tok = m[0];
    if (tok.startsWith("`")) {
      out.push(
        <code key={nextKey()} className="owui-code-inline">
          {tok.slice(1, -1)}
        </code>,
      );
    } else if (tok.startsWith("[")) {
      const label = tok.slice(1, tok.indexOf("]"));
      const href = tok.slice(tok.indexOf("(") + 1, -1);
      out.push(
        <a key={nextKey()} href={href} target="_blank" rel="noreferrer">
          {label}
        </a>,
      );
    } else if (tok.startsWith("**")) {
      out.push(<strong key={nextKey()}>{tok.slice(2, -2)}</strong>);
    } else {
      out.push(<em key={nextKey()}>{tok.slice(1, -1)}</em>);
    }
    last = m.index + tok.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

function CodeBlock({ code, lang }: { code: string; lang?: string }) {
  return (
    <div className="owui-codeblock">
      <div className="owui-codeblock__bar">
        <span className="owui-muted" style={{ fontSize: 12 }}>
          {lang || "code"}
        </span>
        <MdIconButton
          aria-label="Copy code"
          onClick={() => void navigator.clipboard?.writeText(code)}
          style={{ "--md-icon-button-icon-size": "18px" } as React.CSSProperties}
        >
          <MdIcon>content_copy</MdIcon>
        </MdIconButton>
      </div>
      <pre>
        <code>{code}</code>
      </pre>
    </div>
  );
}

export function Markdown({ source }: { source: string }) {
  const lines = source.replace(/\r\n/g, "\n").split("\n");
  const blocks: ReactNode[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Fenced code
    if (line.startsWith("```")) {
      const lang = line.slice(3).trim();
      const buf: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) buf.push(lines[i++]);
      i++; // closing fence
      blocks.push(<CodeBlock key={nextKey()} code={buf.join("\n")} lang={lang} />);
      continue;
    }

    // Headings
    const h = /^(#{1,4})\s+(.*)$/.exec(line);
    if (h) {
      const level = h[1].length;
      const Tag = (["h3", "h4", "h5", "h6"][level - 1] ?? "h6") as "h3" | "h4" | "h5" | "h6";
      blocks.push(
        <Tag key={nextKey()} style={{ margin: "8px 0 4px" }}>
          {inline(h[2])}
        </Tag>,
      );
      i++;
      continue;
    }

    // Blockquote
    if (line.startsWith(">")) {
      const buf: string[] = [];
      while (i < lines.length && lines[i].startsWith(">"))
        buf.push(lines[i++].replace(/^>\s?/, ""));
      blocks.push(
        <blockquote key={nextKey()} className="owui-quote">
          {inline(buf.join(" "))}
        </blockquote>,
      );
      continue;
    }

    // Lists (unordered / ordered)
    if (/^\s*([-*]|\d+\.)\s+/.test(line)) {
      const ordered = /^\s*\d+\.\s+/.test(line);
      const items: string[] = [];
      while (i < lines.length && /^\s*([-*]|\d+\.)\s+/.test(lines[i])) {
        items.push(lines[i++].replace(/^\s*([-*]|\d+\.)\s+/, ""));
      }
      const lis = items.map((it) => <li key={nextKey()}>{inline(it)}</li>);
      blocks.push(ordered ? <ol key={nextKey()}>{lis}</ol> : <ul key={nextKey()}>{lis}</ul>);
      continue;
    }

    // Blank line
    if (line.trim() === "") {
      i++;
      continue;
    }

    // Paragraph (gather consecutive non-blank, non-special lines)
    const buf: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].startsWith("```") &&
      !/^(#{1,4})\s/.test(lines[i]) &&
      !lines[i].startsWith(">") &&
      !/^\s*([-*]|\d+\.)\s+/.test(lines[i])
    ) {
      buf.push(lines[i++]);
    }
    blocks.push(
      <p key={nextKey()} style={{ margin: "4px 0" }}>
        {inline(buf.join(" "))}
      </p>,
    );
  }

  return <Fragment>{blocks}</Fragment>;
}
