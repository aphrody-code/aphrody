# Notes de migration — Paramètres de compte (AVANT → APRÈS)

Détail composant par composant de la migration de
[`before/AccountSettings.tsx`](./before/AccountSettings.tsx) vers
[`after/AccountSettings.tsx`](./after/AccountSettings.tsx), conforme au contrat
[`../00-CONVENTIONS.md`](../00-CONVENTIONS.md). Chaque élément `md-*`, prop et
slot cité a été **vérifié dans `material-web/`** (renvois `chemin:ligne`) — rien
n'est inventé (contrat §7.2).

## Table de correspondance

| #   | AVANT (MUI)                                                       | APRÈS (md / wrapper)                                                          | Pourquoi / règle                                                                                                                                                                        | Source vérifiée                                                                                                              |
| --- | ----------------------------------------------------------------- | ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| 1   | `ThemeProvider` + `createTheme`                                   | import `theme.css` (tokens `--md-sys-*`)                                      | Les éléments md sont self-contained et lisent les tokens directement ; plus de provider Emotion. Contrat §5.                                                                            | `material-web/tokens/`, `02-theme-token-migration.md`                                                                        |
| 2   | `CssBaseline`                                                     | reset global + tokens (hors exemple)                                          | `CssBaseline` est un gap (contrat §3) → reset CSS + `:root` tokens.                                                                                                                     | `05-gap-analysis.md`                                                                                                         |
| 3   | `AppBar` + `Toolbar`                                              | `MdTopAppBar variant="small"`                                                 | `Toolbar` fusionne dans la barre ; slots `leading` / défaut(titre) / `trailing`.                                                                                                        | `appbar/internal/top-app-bar.ts:24-30`                                                                                       |
| 4   | `IconButton` + `<MenuIcon/>`                                      | `MdIconButton slot="leading"` + `MdIcon`                                      | `@mui/icons-material` → glyphe Material Symbols dans `md-icon` (texte).                                                                                                                 | `md-elements.txt` (`md-icon-button`, `md-icon`)                                                                              |
| 5   | `Typography variant="h6"` (titre)                                 | `<span>` dans le slot par défaut                                              | Le titre est un slot du top-app-bar ; typescale via tokens.                                                                                                                             | `appbar/internal/top-app-bar.ts:25`                                                                                          |
| 6   | `Switch checked onChange e.target.checked` (thème)                | `MdSwitch selected onChange e.target.selected`                                | **`md-switch` utilise `selected`, PAS `checked`.** Event `change`. Contrat §4.                                                                                                          | `switch/internal/switch.ts:45-61`                                                                                            |
| 7   | `Box sx={{maxWidth,mx,p}}`                                        | `<div className="mx-auto max-w-[880px] p-6">`                                 | `Box`/`sx` n'ont pas d'équivalent → `<div>` + Tailwind (layout host). Contrat §3/§6.                                                                                                    | `00-CONVENTIONS.md §6`                                                                                                       |
| 8   | `Tabs value onChange={(e,v)}` + `Tab label`                       | `MdTabs activeTabIndex onChange` + `MdPrimaryTab`                             | Sélection lue sur `e.target.activeTabIndex` (plus de 2ᵉ arg). Label = children du tab.                                                                                                  | `tabs/internal/tabs.ts:15-16`, `tabs/internal/tab.ts:48`, `tabs/primary-tab.ts:27`                                           |
| 9   | `Card variant="outlined"`                                         | `MdOutlinedCard`                                                              | Variante `outlined` → élément dédié (fork labs).                                                                                                                                        | `labs/card/outlined-card.ts`, `aphrody-labs.ts:23`                                                                           |
| 10  | `CardHeader title/subheader`                                      | `<header>` + `<h2>`/`<p>` tokenisés                                           | `md-card` n'a **qu'un slot par défaut** — pas de sous-slots header. Contrat §4.                                                                                                         | `labs/card/internal/card.ts:19`                                                                                              |
| 11  | `CardContent`                                                     | markup direct dans la carte                                                   | Idem : pas de slot `content` sur md-card.                                                                                                                                               | `labs/card/internal/card.ts:19`                                                                                              |
| 12  | `Grid container spacing` + `Grid item xs/sm`                      | `<div className="grid grid-cols-1 gap-4 sm:grid-cols-2">`                     | `Grid` → grille Tailwind. Contrat §6.                                                                                                                                                   | `00-CONVENTIONS.md §6`                                                                                                       |
| 13  | `TextField label value onChange helperText`                       | `MdOutlinedTextField label value onInput errorText`                           | TextField MUI défaut `variant` → ici outlined. `helperText`→`supportingText`/`errorText`. Controlled : `onInput` → `e.target.value`. Contrat §4.                                        | `textfield/internal/text-field.ts:109-180`                                                                                   |
| 14  | `TextField type="email" error helperText`                         | `MdOutlinedTextField type="email" error errorText`                            | `error` (bool) + `error-text` existent réellement sur l'élément.                                                                                                                        | `textfield/internal/text-field.ts:109,119`                                                                                   |
| 15  | `FormControl`+`InputLabel`+`Select`+`MenuItem`                    | `MdFilledSelect label` + `MdSelectOption value`                               | `InputLabel`/`FormControl` disparaissent : `label` est une prop du select. `MenuItem`→`md-select-option` (valeur = prop, libellé = slot `headline`). Event `change` → `e.target.value`. | `select/internal/select.ts:62-105,168-180`, `select/select-option.ts`, `select/internal/selectoption/select-option.ts:31-41` |
| 16  | `RadioGroup`+`FormControlLabel`+`Radio`                           | `<fieldset>`/`<label>` + `MdRadio name value checked`                         | Pas de `RadioGroup`/`FormControlLabel` md : groupage par `name` partagé, label composé en JSX. `checked` + event `change` → `e.target.value`.                                           | `radio/internal/radio.ts:45-69`                                                                                              |
| 17  | `FormControlLabel`+`Checkbox checked onChange`                    | `<label>` + `MdCheckbox checked onChange`                                     | `md-checkbox` utilise bien `checked`. Label composé soi-même. `e.target.checked`.                                                                                                       | `checkbox/internal/checkbox.ts:45-62`                                                                                        |
| 18  | `Button variant="text"`                                           | `MdTextButton`                                                                | `variant="text"` → élément dédié. Contrat §3.                                                                                                                                           | `00-CONVENTIONS.md §3`, `md-elements.txt` (`md-text-button`)                                                                 |
| 19  | `Button variant="contained"`                                      | `MdFilledButton`                                                              | `contained` (défaut MUI) → `md-filled-button`. Contrat §3.                                                                                                                              | `00-CONVENTIONS.md §3`, `button/internal/button.ts`                                                                          |
| 20  | `CardActions sx justify`                                          | `<div className="mt-4 flex justify-end gap-2">`                               | Pas de slot actions sur md-card → layout Tailwind.                                                                                                                                      | `labs/card/internal/card.ts:19`                                                                                              |
| 21  | `Stack spacing`                                                   | `<div className="flex flex-col gap-4">`                                       | `Stack` → flex Tailwind. Contrat §6.                                                                                                                                                    | `00-CONVENTIONS.md §6`                                                                                                       |
| 22  | `Alert severity="warning"`                                        | `M3Alert severity="warning"` (shim)                                           | **GAP** : aucun `md-alert`. Shim surface tokenisée + `md-icon`. Contrat §3 (gaps) / §7.2.                                                                                               | `05-gap-analysis.md`, `examples/after/shims/M3Alert.tsx`                                                                     |
| 23  | `Switch` (2FA)                                                    | `MdSwitch selected`                                                           | idem ligne 6 (`selected`).                                                                                                                                                              | `switch/internal/switch.ts:61`                                                                                               |
| 24  | `Divider`                                                         | `MdDivider`                                                                   | Correspondance 1:1.                                                                                                                                                                     | `md-elements.txt` (`md-divider`)                                                                                             |
| 25  | `Button variant="outlined" color="error"`                         | `MdOutlinedButton` + `style` override token                                   | Pas de prop `color` sur les boutons md : on surcharge `--md-sys-color-primary`/`-outline` vers `error` via `style` (ex-`sx`). Contrat §4.                                               | `00-CONVENTIONS.md §4`, `button/internal/button.ts`                                                                          |
| 26  | `List`/`ListItem`/`ListItemIcon`/`ListItemText`                   | `MdList`/`MdListItem` + slots `start`/`headline`/`supporting-text`            | `ListItemIcon`→`slot="start"`, `primary`→`slot="headline"`, `secondary`→`slot="supporting-text"`.                                                                                       | `list/internal/listitem/list-item.ts:90-91,184-186`                                                                          |
| 27  | `Dialog open onClose` + `DialogTitle/Content/ContentText/Actions` | `MdDialog ref` + slots `headline`/`content`/`actions`, ouverture `ref.show()` | Sous-composants → slots. **Ouverture impérative** `show()`/`close(returnValue)` (top-layer). Contrat §3.                                                                                | `dialog/internal/dialog.ts:30-34,176-235,307-325`                                                                            |
| 28  | `Snackbar open autoHideDuration message onClose`                  | `MdSnackbar ref labelText timeout-ms` + `ref.show()`                          | `message`→`label-text`, `autoHideDuration`→`timeout-ms`, ouverture `show()`. Events `closing`/`closed{reason}`.                                                                         | `snackbar/internal/snackbar.ts:34-65,79-111`                                                                                 |

## Pièges rencontrés (à retenir)

### 1. `selected` ≠ `checked` (le piège silencieux n°1)

`md-switch` expose **`selected`** (`switch/internal/switch.ts:61`), alors que
`md-checkbox` et `md-radio` exposent **`checked`**
(`checkbox/internal/checkbox.ts:62`, `radio/internal/radio.ts:58`). Migrer un
`Switch` MUI (`checked`) sans renommer en `selected` compile mais ne lie rien.

### 2. Controlled inputs : la signature des handlers change (contrat §4)

MUI : `onChange={(e, value) => …}` ou `e.target.value`. Les éléments md émettent
des events **natifs** :

- `MdOutlinedTextField` → `onInput`, lire `e.target.value`
  (`text-field.ts:86-92`) ;
- `MdFilledSelect` → `onChange`, lire `e.target.value`
  (`select.ts:62-66`) ;
- `MdCheckbox` → `onChange`, lire `e.target.checked` (`checkbox.ts:45`) ;
- `MdSwitch` → `onChange`, lire `e.target.selected` (`switch.ts:45-47`) ;
- `MdRadio` → `onChange`, lire `e.target.value` (`radio.ts:45-47`) ;
- `MdTabs` → `onChange`, lire `e.target.activeTabIndex` (`tabs.ts:15-16`).

Le **2ᵉ argument** `value` de MUI n'existe plus. Les codemods doivent réécrire
`(e, v) => set(v)` en `(e) => set(e.target.<prop>)`.

### 3. Sous-composants MUI → slots md (contrat §4)

`DialogTitle/Content/Actions`, `CardHeader`, `ListItemText/Icon`,
`FormControlLabel`, `InputLabel` ne migrent pas vers des composants : ils
deviennent du **contenu slotté** (`slot="headline"`, `slot="content"`,
`slot="start"`…) ou des props de l'élément (ex : `label` du select/textfield).
Cas particulier : **`md-card` n'a qu'un slot par défaut**
(`labs/card/internal/card.ts:19`) — tout l'en-tête/actions devient du markup +
layout Tailwind interne.

### 4. `sx` supprimé (contrat §4 / §6)

Aucun équivalent à `sx`. Deux destinations :

- **layout / espacement / host** → utilitaires **Tailwind** sur des `<div>`
  (les utilitaires ne franchissent pas le shadow DOM, contrat §6) ;
- **couleur d'un composant md** (ex : bouton « error ») → `style` inline qui
  surcharge un token `--md-sys-*` sur le host (ici `--md-sys-color-primary`),
  car les boutons md n'ont pas de prop `color`.

### 5. Overlays : ouverture impérative (contrat §3)

`Dialog`/`Snackbar` MUI sont pilotés par une prop `open`. Côté md, on privilégie
les **refs impératives** `ref.show()` / `ref.close(reason?)` pour gérer le
top-layer (popover) correctement (`dialog.ts:176`, `snackbar.ts:79`). La prop
`open` existe mais le pattern recommandé reste impératif. `returnValue` du dialog
permet de savoir quel bouton a fermé (`dialog.ts:76,225`).

### 6. Gaps : ne pas inventer (contrat §7.2)

`Alert` n'a **aucun** élément md. On crée un **shim** (`M3Alert`) plutôt
qu'inventer `<md-alert>`. Tous les gaps de cet écran (`Alert`, `CssBaseline`)
sont recensés dans [`../05-gap-analysis.md`](../05-gap-analysis.md).

## Renvois

- Mapping exhaustif : [`../01-component-mapping.md`](../01-component-mapping.md)
- Thème / tokens : [`../02-theme-token-migration.md`](../02-theme-token-migration.md)
- Intégration React (@lit/react, events, refs, controlled) : [`../03-react-integration.md`](../03-react-integration.md)
- Gaps & shims : [`../05-gap-analysis.md`](../05-gap-analysis.md)
- Tailwind ⇄ material-web : [`../06-tailwind-material-web.md`](../06-tailwind-material-web.md)
