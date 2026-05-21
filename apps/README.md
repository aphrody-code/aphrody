# apps/ — Bun applications (TypeScript/JS)

Home des applications **first-party Bun** du monorepo polyglotte aphrody.

- Géré par le **Bun workspace** racine (`package.json` → `workspaces.packages: ["apps/*"]`).
- Versions de dépendances partagées via le **Bun catalog** racine (`workspaces.catalog`) : référencer `"typescript": "catalog:"`, `"oxlint": "catalog:"`, etc. dans le `package.json` de chaque app.
- Lint : **oxlint** (config racine `.oxlintrc.json`), via `just lint-bun`.

Les forks UI sous `packages/*` (material-web, ui, tailwindcss) ne sont **pas** des membres de ce workspace : ils conservent leur gestionnaire natif (npm/pnpm) et leur propre catalog.

Arborescence par langage : `crates/` (Rust), `python/` (uv), `go/` (go.work), `apps/` + `packages/` (Bun).
