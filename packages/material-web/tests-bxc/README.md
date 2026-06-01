# tests-bxc — smoke test haut niveau (navigateur réel) via bxc

Test E2E qui charge le bundle self-contained `dist-aphrody/aphrody-components.js` dans un vrai navigateur, instancie chaque composant ajouté (avatar, alert, charts, pickers…) et **s'auto-vérifie** : chaque tag est-il `customElements`-défini, a-t-il un `shadowRoot` peuplé, et zéro erreur console ? Le verdict est écrit dans `#bxc-result` (`DEFINED:n/N SHADOW:n/N ERRORS:k`), lu par l'outil d'automation.

## Lancer

```bash
# 1. builder le bundle
cd ../..               # racine monorepo
bun run build && (cd packages/material-web && bun run build:aphrody)

# 2. servir le harnais
cd packages/material-web
bun tests-bxc/serve.ts "$PWD"      # http://127.0.0.1:8799/

# 3. piloter un navigateur (depuis un autre shell)
BXC_CHROME_BIN=/usr/bin/google-chrome-stable \
  bxc scrape http://127.0.0.1:8799/ '#bxc-result' --profile fast
```

## Exigence moteur (constat vérifié)

- **Lightpanda** (moteur par défaut de `bxc`, `bxc install`) ne convient PAS : il expose `customElements` + `attachShadow` + `adoptedStyleSheets` + `ResizeObserver`, **mais pas `ElementInternals`/`attachInternals`** (vérifié par sonde de capacités). Or `@material/web`/Lit en dépendent (form-association, ARIA), donc l'import du bundle lève une exception. Lightpanda convient uniquement aux web components « plain » (sans ElementInternals).
- **Chromium / Chrome réel** requis pour exécuter le runtime Lit. Le pointer via `BXC_CHROME_BIN` (ex. `/usr/bin/google-chrome-stable`).
- `bxc scrape`/`recon` lisent le DOM sans attendre la fin du script async (import + `whenDefined`) → utiliser un pilote CDP « settle-aware » (attendre `#bxc-result[data-done="1"]`) ou Chrome `--dump-dom --virtual-time-budget`.

## Test d'intégration de référence (vert)

La preuve haut niveau qui passe aujourd'hui est le build de `examples/showcase` (`bun build ./src/index.html` + `bun run typecheck` + smoke `Bun.serve`) : il bundle, résout et type-check les composants dans la vraie chaîne React (cf. `packages/react/FRAMEWORKS.md` pour l'intégration SSR/Next côté consommateur). Le smoke bxc ci-dessus est complémentaire (rendu navigateur pur, hors framework) et s'exécute dès qu'un Chromium pilotable est disponible.
