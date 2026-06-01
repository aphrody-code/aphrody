# web (aphrody)

A **full Material Design 3 React rebuild of [Open WebUI](https://github.com/open-webui/open-webui)** — the SvelteKit LLM chat app — rebuilt on a strict **Bun + TanStack** stack and rendered entirely with [`@aphrody-code/m3-react`](https://github.com/aphrody-code/material-web) (`<md-*>` web components wrapped for React), consumed from GitHub Packages.

> Stack rule for this example: **only Bun and TanStack**. No Next, no Vite, no Svelte, no markdown lib, no state lib. Bun is the runtime + bundler + dev server; TanStack Router + TanStack Query own routing and server state; everything visible is a Material 3 component.

## What it covers

A faithful slice of Open WebUI's surface, re-expressed in M3:

| Open WebUI area | Here | Key M3 components |
| --- | --- | --- |
| App shell + sidebar | `components/AppShell`, `components/Sidebar` | `md-top-app-bar`, `md-list`, `md-search-bar`, `md-menu` |
| **Chat (streaming)** | `components/chat/*` | `md-surface` bubbles, `md-avatar`-style heads, `md-outlined-text-field`, `md-icon-button`, `md-assist-chip` |
| Model picker | `components/chat/ModelSelector` | `md-menu` + `md-text-button` |
| Auth (sign in/up + OAuth) | `components/auth/AuthScreen` | `md-elevated-card`, `md-filled-button`, `md-outlined-button` |
| Settings modal | `components/settings/SettingsDialog` | `md-dialog`, `md-tabs`, `md-switch`, `md-slider`, `md-outlined-select` |
| Admin (users / settings / evals) | `routes/AdminRoute` | **`md-table`** (sort + filter + pagination), `md-dialog`, `md-switch` |
| Workspace (models/knowledge/prompts/tools) | `routes/WorkspaceRoute` | `md-tabs`, `md-outlined-card` grid, `md-search-bar` |
| Notes | `routes/NotesRoute` | `md-outlined-card`, `md-outlined-text-field` |

**Material You**: the whole app re-themes live from a seed colour (Settings → Accent) via `@aphrody-code/m3-tokens/dynamic-color`, with light / dark / system modes.

**Streaming chat**: a `Bun.serve` mock backend exposes an OpenAI-compatible **SSE** `/api/chat/completions` endpoint; the client parses deltas, renders them through a tiny zero-dependency Markdown renderer (code blocks, lists, inline styles), auto-scrolls, and persists the message tree (`{messages, currentId}`, the open-webui history model) through TanStack Query mutations. Edit-in-place and regenerate are wired.

## Architecture

```
src/
  server.ts              Bun.serve — bundles the app (HTML import) + mock API + SSE
  api/                   types, typed fetch client, TanStack Query hooks, SSE parser, mock data
  store.ts               tiny useSyncExternalStore UI store (theme/seed/sidebar/session)
  theme/ThemeProvider    applies Material You roles onto <html> from the seed + mode
  router.tsx             TanStack Router tree (pathless _app layout + auth guard)
  components/ , routes/   the M3 screens
  main.tsx               QueryClient + ThemeProvider + RouterProvider, Material Symbols font
```

## Run

```bash
bun install                 # from the repo root (links the workspace)
cd apps/web

bun run dev                 # Bun.serve with HMR  → http://localhost:3210
bun run build               # bun build ./src/index.html → dist/ (production bundle)
bun run typecheck           # tsc: browser tsconfig + bun tsconfig
bun run smoke               # headless: boots the server, asserts HTML + bundle + API + SSE
```

Sign-in accepts any credentials (the backend is a mock). The seeded user is `ada@example.com`.

## Notes / limitations

- The backend is an **in-memory mock** — no real Ollama/OpenAI calls, no DB. Replies are canned and streamed so the M3 chat UX (bubbles, typing indicator, auto-scroll, token counts) is fully exercised.
- Markdown is a deliberately small renderer (the Bun + TanStack rule forbids pulling a markdown dependency); it covers fenced code, headings, lists, blockquotes, and inline bold/italic/code/links.
- Premium MUI-X-only surfaces from upstream (pivot, recurrence) are out of scope, matching the monorepo's coverage policy.
