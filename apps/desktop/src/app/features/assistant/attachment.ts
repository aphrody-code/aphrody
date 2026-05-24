// SPDX-License-Identifier: Apache-2.0

/** The kind of attachment, driving the chip icon + how context is built. */
export type AttachmentKind = "image" | "video" | "audio" | "github" | "url" | "drive" | "file";

/**
 * A composer attachment. Textual kinds (file/url/github) carry inlined `text`
 * that is honestly folded into the prompt context; binary media (image/video/
 * audio) carry only a `name`/`detail` reference, because `aphrody chat` is
 * text-only today — we never fake multimodal vision.
 */
export interface Attachment {
  /** Stable id for tracking/removal. */
  id: string;
  kind: AttachmentKind;
  /** Display label (file name, URL, handle…). */
  name: string;
  /** Secondary detail (size, host, path) shown under the name. */
  detail?: string;
  /** Inlined textual content (text files / fetched URL or GitHub readme). */
  text?: string;
  /** True while async fetch/read is in flight. */
  loading?: boolean;
  /** Non-fatal note when content could not be inlined (binary, fetch failed). */
  note?: string;
}

/** Material Symbol per attachment kind. */
export function kindIcon(kind: AttachmentKind): string {
  switch (kind) {
    case "image":
      return "image";
    case "video":
      return "movie";
    case "audio":
      return "graphic_eq";
    case "github":
      return "code";
    case "url":
      return "link";
    case "drive":
      return "cloud";
    default:
      return "description";
  }
}

/** Human label per attachment kind (French). */
export function kindLabel(kind: AttachmentKind): string {
  switch (kind) {
    case "image":
      return "Image";
    case "video":
      return "Vidéo";
    case "audio":
      return "Audio";
    case "github":
      return "GitHub";
    case "url":
      return "Lien";
    case "drive":
      return "Drive";
    default:
      return "Fichier";
  }
}

const TEXT_EXT = new Set([
  "txt", "md", "markdown", "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "json", "jsonc",
  "toml", "yaml", "yml", "html", "htm", "css", "scss", "sass", "less", "py", "go", "java",
  "kt", "kts", "c", "h", "cpp", "hpp", "cc", "cs", "rb", "php", "sh", "bash", "zsh", "ps1",
  "sql", "xml", "svg", "csv", "tsv", "ini", "cfg", "conf", "env", "lock", "gradle", "dockerfile",
  "vue", "svelte", "astro", "graphql", "proto", "lua", "r", "swift", "dart", "ex", "exs", "log",
]);

/** Whether a filename looks like inlineable UTF-8 text/code. */
export function isTextFile(name: string, mime: string): boolean {
  if (mime.startsWith("text/")) return true;
  if (mime === "application/json" || mime === "application/xml") return true;
  const dot = name.lastIndexOf(".");
  const ext = dot >= 0 ? name.slice(dot + 1).toLowerCase() : name.toLowerCase();
  return TEXT_EXT.has(ext);
}

/** Format a byte count compactly (e.g. "12,3 ko"). */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  const ko = bytes / 1024;
  if (ko < 1024) return `${ko.toFixed(1).replace(".", ",")} ko`;
  return `${(ko / 1024).toFixed(1).replace(".", ",")} Mo`;
}

/** Cap inlined text so a huge file cannot blow the prompt budget. */
export const MAX_INLINE_CHARS = 24_000;

/**
 * Convert a public GitHub blob/repo URL into a raw.githubusercontent.com URL
 * (blob) or the repo README candidate list. Returns the list of URLs to try in
 * order. Non-GitHub inputs return an empty list.
 */
export function githubRawCandidates(input: string): string[] {
  let url: URL;
  try {
    url = new URL(input.trim());
  } catch {
    return [];
  }
  if (url.hostname !== "github.com") return [];
  const parts = url.pathname.split("/").filter(Boolean);
  // /owner/repo/blob/<ref>/<path...> -> raw file
  if (parts.length >= 5 && parts[2] === "blob") {
    const [owner, repo, , ref, ...rest] = parts;
    return [`https://raw.githubusercontent.com/${owner}/${repo}/${ref}/${rest.join("/")}`];
  }
  // /owner/repo (or with trailing) -> try README on common branches.
  if (parts.length >= 2) {
    const [owner, repo] = parts;
    const branches = ["main", "master"];
    const names = ["README.md", "readme.md", "README.MD"];
    const out: string[] = [];
    for (const b of branches) {
      for (const n of names) {
        out.push(`https://raw.githubusercontent.com/${owner}/${repo}/${b}/${n}`);
      }
    }
    return out;
  }
  return [];
}
