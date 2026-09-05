# Kit de migration MUI → material-web

Tout le nécessaire pour migrer une codebase **React + MUI** (`@mui/material@9`) vers le monorepo **material-web** (fork aphrody, web components Lit) via la couche React `@aphrody/m3-react`.

> Cible réalisée : `material-web/packages/react` (`@aphrody/m3-react`, **120 tags `md-*`** wrappés) + `material-web/packages/m3-tokens` (`@aphrody/m3-tokens`). Voir aussi `../docs/` pour l'état de Material Design 3 sur le web.

## Par où commencer

1. **[`00-CONVENTIONS.md`](./00-CONVENTIONS.md)** — le contrat : nommage des wrappers, mapping canonique, règles props/events/tokens, intégration Tailwind. _Source de vérité, à lire en premier._
2. **[`04-migration-playbook.md`](./04-migration-playbook.md)** — la stratégie : migration incrémentale (strangler), coexistence MUI+md, phases ordonnées, tests, checklist par composant.

## Référence

| Doc                                                            | Contenu                                                                                                |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| [`01-component-mapping.md`](./01-component-mapping.md)         | Mapping exhaustif MUI → `md-*` (props/slots/events vérifiés, par catégorie) + index des GAP            |
| [`02-theme-token-migration.md`](./02-theme-token-migration.md) | Thème MUI (M2) → tokens `--md-sys-*` ; génération via material-color-utilities                         |
| [`03-react-integration.md`](./03-react-integration.md)         | `@lit/react`, React 19 custom elements, events, controlled, refs, SSR/Next                             |
| [`05-gap-analysis.md`](./05-gap-analysis.md)                   | Composants MUI sans équivalent md + shims (gaps réels restants : Modal/Popper/transitions)             |
| [`06-tailwind-material-web.md`](./06-tailwind-material-web.md) | Intégration native Tailwind (mur Shadow DOM, `@theme` ← `--md-sys-*`, `::part()`)                      |
| [`07-coverage-mui.md`](./07-coverage-mui.md)                   | Couverture `@mui/material` v9 (119 wrappers Md\*) — composant par composant                            |
| [`08-coverage-mui-x.md`](./08-coverage-mui-x.md)               | Couverture **MUI X v9 Community** (Data Grid, charts, pickers, tree, scheduler) + verdict              |
| [`09-coverage-tailwind.md`](./09-coverage-tailwind.md)         | Quelles familles de tokens sont mappables en `@theme` Tailwind v4 (et lesquelles non)                  |
| [`11-material-symbols.md`](./11-material-symbols.md)           | Material Symbols : axes variables `md-icon`, chargement de police, optimisation, codemod icônes (96 %) |
| [`mui-m3-map.json`](./mui-m3-map.json)                         | **Mapping consolidé machine-readable** (composants + props + events + icônes) — généré                 |

## Outillage

| Dossier                                                          | Quoi                                                                                                                                                        | Statut                                                                           |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| [`codemods/`](./codemods/)                                       | Transforms jscodeshift (imports, JSX variant-aware, props/events) + règles ast-grep                                                                         | fixtures **4/4 PASS**, `tsc --strict` 0 erreur, validé sur app réelle (cf. `10`) |
| [`examples/`](./examples/)                                       | Écran « Paramètres de compte » migré avant/après + notes point par point                                                                                    | complet                                                                          |
| [`scripts/`](./scripts/)                                         | `md-elements.txt` (**120 tags** de référence, synchro `packages/react`), `theme-to-tokens.ts` (copie canonique dans le package `@aphrody/m3-tokens`)   | —                                                                                |
| [`../packages/eslint-plugin-m3/`](../packages/eslint-plugin-m3/) | Plugin lint `@aphrody/eslint-plugin-m3` (oxlint `jsPlugins` + ESLint) — 6 règles M3 pour les sites consommant la lib ; complément continu des codemods | testé sur oxlint réel (6/6)                                                      |

## Les pièges à retenir

- **Events** : MUI `onChange(e, value)` → events DOM natifs `input`/`change`, lire `e.target.value`. Composants fork = events namespacés (`table:sort`, `stepper:change`…).
- **Props renommées** : `Switch.checked`→`selected`, `Drawer.open`→`opened`, `Tooltip.title`→`text`, `Tabs.value`→`active-tab-index`, `LinearProgress` 0-100 → 0-1.
- **Slots** : les sous-composants MUI (`DialogTitle`, `CardHeader`…) deviennent du contenu slotté (`slot="headline"`…).
- **`sx`/`styled`** : aucun équivalent ; layout → utilitaires Tailwind (hors Shadow DOM), styling interne → tokens `--md-sys-*` uniquement.
- **Shadow DOM** : les classes Tailwind ne pénètrent pas les `md-*` ; voir `06`.

## Toolchain

**bun uniquement.** Tout livrable exécutable (codemods, `theme-to-tokens.ts`) a été vérifié en exécution réelle (bun + `tsc`/jscodeshift).
