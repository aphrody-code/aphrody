# @aphrody-code/m3-showcase

The single, light, **Bun-native** example for this monorepo. It merges what the
former `nextjs` and `m3-world` examples demonstrated into one app, with **no
Next.js and no Vite** — only [Bun APIs](https://bun.com/docs/runtime/bun-apis)
(`Bun.serve`, `bun build`) on the server side and standard
[Web APIs](https://bun.com/docs/runtime/web-apis) (`fetch`, DOM) on the client.

It exercises the full shippable surface:

- **`@aphrody-code/m3-react`** — every `md-*` web component through its React
  wrapper (buttons, inputs, chips, cards, dialogs, pickers, table, …) in one
  interactive gallery (`src/showcase.tsx`).
- **`@aphrody-code/m3-tokens/dynamic-color`** — live Material You seed picker,
  light/dark, re-themed at runtime (`applyDynamicColor`).
- **`@react-three/fiber` + `@react-three/drei`** — a light WebGL backdrop
  (`src/components/three/`), no heavy image assets.

## Run

```bash
bun install                 # from the repo root
cd examples/showcase

bun run dev                 # Bun.serve fullstack dev server + HMR (port 3000)
bun run build               # bun build ./src/index.html -> dist/ (static, hostable anywhere)
bun run start               # serve without HMR
bun run typecheck           # browser tsconfig + bun tsconfig
bun run smoke               # boot the server, assert HTML + bundled client resolve
```

## Layout

```
src/
├── index.html              # entry: fonts, pre-paint theme bootstrap, #root, <script app.tsx>
├── server.ts               # Bun.serve({ routes: { "/*": index }, development: { hmr } })
├── app.tsx                 # client entry: inject base Material You theme, mount <Showcase/>
├── showcase.tsx            # the interactive Material 3 component gallery (client island)
├── theme.css / showcase.css
├── components/three/        # ThreeWorld + M3BackgroundShapes (fiber/drei backdrop)
└── smoke.ts                # Bun.serve + fetch smoke test
```

Two tsconfigs split the two runtimes: `tsconfig.json` (browser / Web APIs, no
`bun-types`) type-checks the client island and the `@aphrody-code/material-web`
source it traverses under the same DOM semantics the library was authored with;
`tsconfig.node.json` (`bun-types`) type-checks the `Bun.serve` server + smoke.
