# Codemods — MUI React → `@aphrody-code/m3-react`

Transforms automatisés pour migrer du code **MUI React** (`@mui/material`) vers les
wrappers **`@aphrody-code/m3-react`** (web components `<md-*>` du fork material-web).

> Contrat de référence : [`../00-CONVENTIONS.md`](../00-CONVENTIONS.md).
> Tout ce kit respecte §2 (nommage des wrappers), §3 (mapping canonique) et §4
> (règles props/events). **bun uniquement** (§7.1). Aucun élément `md-*` inventé (§7.2).

Deux outils complémentaires :

| Outil                           | Rôle                                                                   | Force                                        |
| ------------------------------- | ---------------------------------------------------------------------- | -------------------------------------------- |
| **jscodeshift** (`transforms/`) | réécriture profonde : imports + JSX + props + slots, **variant-aware** | transformation complète, idempotente, testée |
| **ast-grep** (`rules/`)         | détection/rapport des occurrences + quelques fix triviaux              | inventaire rapide, zéro setup Node           |

---

## Installation

Déjà fait dans ce dossier (`package.json` + `bun.lock`) :

```bash
bun install                       # jscodeshift + @types/jscodeshift
# ast-grep : pas d'install nécessaire, lancé via bunx @ast-grep/cli (binaire runtime)
```

---

## jscodeshift — transforms

Tous les transforms utilisent le **parser `tsx`** (exporté par chaque module, donc
`--parser=tsx` est optionnel mais on le met par sûreté) et partagent le moteur
`lib/engine.ts` (table de mapping `lib/mapping.ts`, helpers AST `lib/jsx-helpers.ts`).

| Transform                    | Périmètre                                                               | Usage                                                                                         |
| ---------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `transforms/orchestrator.ts` | **tout** le mapping (recommandé pour migration de masse)                | `bunx jscodeshift -t transforms/orchestrator.ts --parser=tsx --extensions=tsx 'src/**/*.tsx'` |
| `transforms/mui-imports.ts`  | identique à l'orchestrateur (alias sémantique « imports génériques »)   | idem                                                                                          |
| `transforms/button.ts`       | **dédié** `Button` / `IconButton` / `Fab` (variant-aware)               | `bunx jscodeshift -t transforms/button.ts --parser=tsx --extensions=tsx <fichiers>`           |
| `transforms/fields.ts`       | **dédié** `TextField` / `Select` / `NativeSelect` (+ `MenuItem`)        | `bunx jscodeshift -t transforms/fields.ts --parser=tsx --extensions=tsx <fichiers>`           |
| `transforms/icons.ts`        | **dédié** `@mui/icons-material` → `<md-icon>` (Material Symbols, ~96 %) | `bunx jscodeshift -t transforms/icons.ts --parser=tsx --extensions=tsx <fichiers>`            |

Le transform `icons.ts` est **séparé** (l'orchestrateur ne touche pas `@mui/icons-material`) : convertit `<CloseIcon/>` → `<md-icon>close</md-icon>` via `lib/icon-names.ts` (PascalCase → snake_case, validé contre les **4253 glyphes** officiels `data/material-symbols-names.json`). Logos de marque (GitHub, X…) gardés + TODO ; voir `../11-material-symbols.md`. Le mapping consolidé (composants + icônes) est exporté en `../mui-m3-map.json` (`bun run scripts/export-map-json.ts`).

Les transforms ciblés (`button`, `fields`) ne touchent **que** leurs composants ; les
autres imports/JSX MUI sont laissés intacts (migration par lots).

### Ce qui est AUTOMATISÉ

- **Imports** (§2/§3) — réécriture `@mui/material` → `@aphrody-code/m3-react` :
  - imports nommés `import { Button } from '@mui/material'`
  - imports default sous-chemin `import Button from '@mui/material/Button'`
  - **alias** `import { Button as Btn } from '@mui/material'`
  - regroupe les wrappers en un seul `import { ... } from '@aphrody-code/m3-react'` (trié)
  - ajoute l'import d'effet de bord `@material/web/icon/icon.js` si un `<md-icon>` est généré
- **Choix du wrapper variant-aware** (§3) :
  - `Button variant="contained"|absent` → `MdFilledButton`,
    `"outlined"` → `MdOutlinedButton`, `"text"` → `MdTextButton`,
    `"elevated"` → `MdElevatedButton`, `"tonal"` → `MdFilledTonalButton`
  - `TextField variant="filled"|absent` → `MdFilledTextField`, `"outlined"` → `MdOutlinedTextField`
  - `Select` filled/outlined → `MdFilledSelect` / `MdOutlinedSelect`
  - la prop `variant` est **consommée** (retirée) pour ces composants
- **Renommage JSX** des composants 1:1 (§3) : `Checkbox`→`MdCheckbox`, `Switch`→`MdSwitch`,
  `Dialog`→`MdDialog`, `MenuItem`→`MdMenuItem`, `Divider`→`MdDivider`, `Typography`→`MdType`, etc.
- **Icônes** (§4) : `startIcon={<X/>}` / `endIcon={<X/>}` → `<md-icon slot="icon">{<X/>}</md-icon>`
  injecté en premier enfant (+ TODO pour `endIcon`, le slot trailing étant à repositionner).
- **Sous-composants slottés** (§4) : `DialogTitle`/`DialogContent`/`DialogActions`,
  `CardContent`/`CardHeader`/`CardMedia` → `<div slot="headline|content|actions|media">` (+ TODO).
- **Layout** (§3/§6) : `Box`/`Container`/`Stack`/`Grid`/`Paper` → `<div>` (+ TODO Tailwind).
- **Marqueurs `MIGRATION-TODO`** pour tout le non-automatisable, **toujours en JSX valide**
  (`{/* ... */}` entre enfants, commentaire de bloc ailleurs).

### Ce qui reste MANUEL (signalé par `MIGRATION-TODO`)

- **`sx`** — retirée : à convertir en classes Tailwind (host/layout) + tokens `--md-sys-*`.
  Le shadow DOM des `md-*` n'est **pas** atteignable par Tailwind (§6).
- **`onChange(e, value)`** — material-web émet des events natifs `input`/`change` :
  lire `e.target.value` (le 2ᵉ paramètre disparaît, §4). Le corps du handler **n'est pas**
  réécrit automatiquement (risqué) — TODO posé quand une signature à 2 args est détectée.
- **Props sans équivalent** retirées + TODO : `color`, `size`, `fullWidth`,
  `disableElevation`, `disableRipple` (à gérer via tokens/density/thème).
- **Composants « gap »** (§3 / `05-gap-analysis.md`) : `Avatar`, `Alert`, `Breadcrumbs`,
  `Rating`, `Skeleton`, `Backdrop`, `Modal`/`Popover`/`Popper`, `Link`, transitions
  (`Collapse`/`Fade`/…), `CssBaseline`… **NON transformés** : le JSX et **son import MUI
  sont conservés** (pas de référence orpheline), un TODU signale le shim à écrire.
- **Imbrication des slots** Dialog/Card : vérifier que les `<div slot>` sont enfants
  **directs** du parent `md-*`.

### Limites connues

- Le moteur ne réécrit **pas** le corps des handlers ni la logique d'état (controlled).
- Les composants en notation membre (`<Foo.Bar/>`) ne sont pas traités.
- Pas de résolution de re-exports indirects (`import { Button } from './ui'`).
- Le renommage `Tab`→`MdPrimaryTab` ne distingue pas primary/secondary (choix par défaut).
- jscodeshift via `recast` reformate parfois l'indentation autour des nœuds modifiés
  (cosmétique — passer un formatter type prettier/biome après coup).

---

## ast-grep — règles (`rules/` + `sgconfig.yml`)

Complément léger pour **inventorier** ce qui reste à migrer et appliquer des fix triviaux.

```bash
bunx @ast-grep/cli scan                      # rapport sur tout le projet (depuis ce dossier)
bunx @ast-grep/cli scan src/                  # rapport ciblé
bunx @ast-grep/cli scan --filter '^fix-divider$' --update-all   # applique le fix Divider
```

| Règle                   | Type              | Effet                                                                            |
| ----------------------- | ----------------- | -------------------------------------------------------------------------------- |
| `detect-mui-imports`    | rapport (warning) | signale tout `import … from '@mui/material'` (racine **et** sous-chemins)        |
| `detect-gap-components` | rapport (error)   | signale les composants « gap » sans équivalent md (migration manuelle)           |
| `detect-sx-prop`        | rapport (warning) | signale les props `sx` à convertir (Tailwind + tokens)                           |
| `fix-divider`           | **fixable**       | `<Divider …/>` → `<md-divider …>` (mapping 1:1 trivial ; l'import reste à gérer) |

> ast-grep ne fait **pas** le choix variant-aware ni la gestion d'imports propre :
> pour une vraie migration, utiliser jscodeshift. Les règles servent surtout au **rapport**.

---

## Tests / vérification

Fixtures avant/après dans `__testfixtures__/` (Button, TextField, Checkbox, Dialog) :

```bash
bun run scripts/run-fixtures.ts            # applique l'orchestrateur et compare aux *.output.tsx
bun run scripts/run-fixtures.ts --update   # régénère les attendus (après modif volontaire d'un transform)
```

Sortie attendue : `PASS` sur les 4 cas + « Tous les fixtures passent. ».

Chaque `*.output.tsx` est la **sortie réelle vérifiée** du transform (re-parsée sans
erreur par jscodeshift). C'est le contrat de non-régression du kit.

### État vérifié (dernière exécution)

- `bun run scripts/run-fixtures.ts` → 4/4 PASS.
- `tsc --noEmit --strict` sur `lib/*.ts` + `transforms/*.ts` → 0 erreur.
- `ast-grep scan` sur fichier d'épreuve → 4 règles déclenchées (2 imports, 1 gap, 1 sx, 2 divider).

---

## Arborescence

```
codemods/
├── README.md                      ← ce fichier
├── package.json / bun.lock
├── sgconfig.yml                   ← config ast-grep
├── lib/
│   ├── mapping.ts                 ← table MUI→wrappers (source : 00-CONVENTIONS.md §3/§4)
│   ├── jsx-helpers.ts             ← utilitaires AST (slots, TODO, props)
│   └── engine.ts                  ← moteur partagé (imports + JSX + props)
├── transforms/
│   ├── orchestrator.ts            ← tout-en-un (recommandé)
│   ├── mui-imports.ts             ← imports génériques (= orchestrateur)
│   ├── button.ts                  ← dédié Button/IconButton/Fab (variant-aware)
│   └── fields.ts                  ← dédié TextField/Select/NativeSelect
├── rules/
│   ├── detect-mui-imports.yml
│   ├── detect-gap-components.yml
│   ├── detect-sx-prop.yml
│   └── fix-divider.yml
├── scripts/
│   └── run-fixtures.ts            ← test rapide avant/après
└── __testfixtures__/
    ├── button.input.tsx   / button.output.tsx
    ├── textfield.input.tsx/ textfield.output.tsx
    ├── checkbox.input.tsx / checkbox.output.tsx
    └── dialog.input.tsx   / dialog.output.tsx
```
