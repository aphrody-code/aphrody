<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody/eslint-plugin-m3

Règles de lint pour les sites qui **consomment** material-web
(`@aphrody/m3-react` + Material Symbols). Écrit avec l'API ESLint-compatible :
tourne **sous oxlint** (champ `jsPlugins`) **et sous ESLint** (champ `plugins`),
sans transpilation (ESM pur).

Cible les composants `Md*` (wrappers React) et `md-*` (custom elements). Pensé en
binôme avec le kit de migration MUI→M3 (`migration/`) : ce que les codemods
laissent en `MIGRATION-TODO`, ce plugin l'attrape ensuite en continu.

## Règles (8)

| Règle                          | Type  | Fix  | Attrape                                                                                                                                                                                                                                                                          |
| ------------------------------ | ----- | :--: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `m3/valid-icon-name`           | error | auto | Le texte d'un `<md-icon>`/`<MdIcon>` doit être un glyphe Material Symbols valide (snake_case). Détecte les restes PascalCase MUI (`Delete` → `delete`) et les noms introuvables — validé contre les **4253** glyphes officiels. Autofix quand le snake_case est un glyphe connu. |
| `m3/valid-color-role`          | error |  —   | Un `var(--md-sys-color-<rôle>)` doit référencer un des **~49 rôles** réellement émis par material-web. Attrape les typos (`primry` → suggère `primary`), rôles MUI/Tailwind, casse — sinon la couleur résout en `unset` silencieux.                                              |
| `m3/no-sx-prop`                | error |  —   | Le prop `sx` (MUI/Emotion) n'a **aucun effet** sur un `md-*` → `className`/Tailwind + tokens `--md-sys-*`.                                                                                                                                                                       |
| `m3/no-mui-prop-on-md`         | error | auto | Noms de props MUI laissés sur un wrapper (`checked`→`selected`, `title`→`text`, `open`→`opened`, `value`→`activeTabIndex`). Autofix : renomme l'attribut.                                                                                                                        |
| `m3/no-mui-import`             | warn  |  —   | Imports `@mui/material` / `@mui/icons-material` / `@mui/x-*` résiduels (mélange de systèmes, double bundle).                                                                                                                                                                     |
| `m3/prefer-icon-token`         | warn  |  —   | `fontVariationSettings` inline sur une icône → tokens `--md-icon-fill/wght/grad/opsz` (héritables, animables).                                                                                                                                                                   |
| `m3/require-icon-button-label` | warn  |  —   | Un bouton-icône (`md-*-icon-button`/`MdIconButton`) sans nom accessible (`aria-label`/`aria-labelledby`/`title`) — a11y, WCAG 4.1.2.                                                                                                                                             |
| `m3/no-hardcoded-color`        | warn  |  —   | Couleur en dur (`#hex`/`rgb()`) dans `style`/`sx` d'un `md-*` → rôle `var(--md-sys-color-*)` (suit le thème + dark + dynamic-color).                                                                                                                                             |

Deux règles sont **auto-corrigeables** (`oxlint --fix` ou `eslint --fix`) : `valid-icon-name` et `no-mui-prop-on-md` — idéal pour finir un port MUI→M3 en un passage.

## Installation

```bash
bun add -D @aphrody/eslint-plugin-m3
```

## Usage — oxlint (recommandé, Rust, rapide)

`.oxlintrc.json` :

```json
{
  "jsPlugins": ["./node_modules/@aphrody/eslint-plugin-m3/index.js"],
  "rules": {
    "m3/valid-icon-name": "error",
    "m3/no-sx-prop": "error",
    "m3/no-mui-prop-on-md": "error",
    "m3/no-mui-import": "warn",
    "m3/prefer-icon-token": "warn",
    "m3/no-hardcoded-color": "warn"
  }
}
```

> Le chemin de `jsPlugins` est résolu **relativement au fichier `.oxlintrc.json`**.
> Les JS plugins oxlint sont en alpha (oxlint ≥ 1.x) — vérifié sur **oxlint 1.67**.

## Usage — ESLint (flat config)

```js
// eslint.config.js
import m3 from "@aphrody/eslint-plugin-m3";

export default [
  m3.configs.recommended, // ou m3.configs.strict (tout en error)
];
```

Ou à la carte :

```js
import m3 from "@aphrody/eslint-plugin-m3";
export default [
  { plugins: { m3 }, rules: { "m3/no-sx-prop": "error", "m3/valid-icon-name": "error" } },
];
```

## Presets

- `m3.configs.recommended` — règles structurelles en `error`, suggestions en `warn`.
- `m3.configs.strict` — **tout** en `error` (utile pour clôturer un port MUI→M3).

## Développement

```bash
bun test/run.ts          # lance oxlint réel sur les fixtures (6/6 sur bad, 0 sur good)
bun scripts/sync-symbol-names.ts   # régénère data/material-symbols-names.js
```

La liste des glyphes (`data/material-symbols-names.js`) est synchronisée depuis
`migration/codemods/data/material-symbols-names.json` (source : codepoints
officiels Google `google/material-design-icons`, Apache-2.0).
