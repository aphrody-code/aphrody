<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody-code/doc-ai

AI-first documentation and automatic translation CLI for the material-web M3
monorepo. It translates Markdown, generates Lit component API guides, and
bulk-syncs every `docs/` folder — backed by **Google Gemini** through the
Vercel AI SDK (`@ai-sdk/google` + `ai`), with a deterministic static fallback
when no credentials are set.

## Commands

```bash
doc-ai translate <file_or_dir> [lang]   # translate one .md file or a dir (recursive)
doc-ai generate <ts_file> [out_file]    # generate a Lit component API guide
doc-ai sync [lang]                      # translate every monorepo docs/ folder
doc-ai --help                           # full usage
```

From the monorepo root, the wrapper scripts are wired in:

```bash
bun run docs:translate <file_or_dir> [lang]   # packages/doc-ai/src/cli.ts translate
bun run docs:ai                               # packages/doc-ai/src/cli.ts sync
```

Translations are written next to each source as `<name>.<lang>.md` (default
language `fr`).

## Configuration

| Env var                              | Effect                                           |
| ------------------------------------ | ------------------------------------------------ |
| `GEMINI_API_KEY` \| `GOOGLE_API_KEY` | Enable Gemini-backed generation/translation.     |
| `DOC_AI_MODEL`                       | Override the model (default `gemini-2.5-flash`). |
| `DOC_AI_GEMINI_BASE_URL`             | Override the Generative Language API base URL.   |
| `DOC_AI_REQUEST_TIMEOUT_MS`          | Per-request network timeout (default `60000`).   |

Without credentials, `generate`/`translate` fall back to a deterministic static
template so the pipeline stays runnable offline and in CI.

## Develop

```bash
bun run build        # tsc -> dist
bun run typecheck    # tsc --noEmit (typecheck:fast = tsgo)
bun run test         # bun test
```
