---
name: m3-doc-ai
description: >
  Translate and generate documentation for the monorepo with the @aphrody-code/doc-ai CLI
  (Google Gemini backend, offline fallback). Use when asked to "translate the docs",
  "generate an API guide for a Lit component", "sync docs to another language", or
  "run doc-ai".
---

# m3-doc-ai

`@aphrody-code/doc-ai` is the doc-as-code CLI of the monorepo: it translates Markdown,
generates Lit component API guides, and bulk-syncs every `docs/` folder — backed by
**Google Gemini** (via the Vercel AI SDK `@ai-sdk/google` + `ai`), with a deterministic
static fallback when no credentials are set (so it stays runnable offline and in CI).

## Commands

```bash
doc-ai translate <file_or_dir> [lang]   # translate one .md or a dir (recursive); default lang fr
doc-ai generate <ts_file> [out_file]    # generate a Lit component API guide from source
doc-ai sync [lang]                      # translate every monorepo docs/ folder
doc-ai --help
```

From the monorepo root the wrappers are wired in:

```bash
bun run docs:translate <file_or_dir> [lang]
bun run docs:ai                          # sync
```

Translations are written next to each source as `<name>.<lang>.md` (default `fr`) —
this is why the repo carries `*.fr.md` siblings.

## Configuration

| Env var                              | Effect                                           |
| ------------------------------------ | ------------------------------------------------ |
| `GEMINI_API_KEY` \| `GOOGLE_API_KEY` | Enable Gemini-backed generation/translation.     |
| `DOC_AI_MODEL`                       | Override the model (default `gemini-2.5-flash`). |
| `DOC_AI_GEMINI_BASE_URL`             | Override the Generative Language API base URL.   |
| `DOC_AI_REQUEST_TIMEOUT_MS`          | Per-request network timeout (default `60000`).   |

Without credentials, `generate`/`translate` fall back to a deterministic static
template — the pipeline never hard-fails for lack of a key.

## When to use

- A new `docs/*.md` was written in English -> `doc-ai translate docs/new.md fr`.
- A new `md-*` component shipped -> `doc-ai generate packages/material-web/<comp>/internal/<comp>.ts`
  to draft its API guide.
- Before a docs release -> `bun run docs:ai` to refresh every translation.

Never paste an API key into a tracked file; pass it via the environment only.

Develop in-repo: `cd packages/doc-ai && bun run build && bun run test`.
