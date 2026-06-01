# Exemple de migration de bout en bout — Paramètres de compte

Cet exemple montre **la même UI** écrite deux fois : en MUI (`before/`) puis
migrée vers les wrappers `@aphrody-code/m3-react` + layout Tailwind + tokens M3
(`after/`). Il sert de référence concrète au reste du kit de migration et
respecte le contrat partagé [`../00-CONVENTIONS.md`](../00-CONVENTIONS.md).

> Le code n'est pas destiné à être buildé ici (pas de toolchain montée dans
> `examples/`). Il est **réaliste, cohérent et fidèle** aux éléments `md-*`
> réels : tous les noms d'éléments, props, slots et events ont été vérifiés
> dans `material-web/` (renvois `material-web/...:ligne` dans
> [`MIGRATION-NOTES.md`](./MIGRATION-NOTES.md)).

## L'écran migré

Une page **« Paramètres de compte »** non-triviale, avec un panel de composants
volontairement varié :

- **App bar + Toolbar** avec icône de menu et interrupteur de thème clair/sombre ;
- **Tabs** (Profil / Sécurité / Sessions) ;
- un **formulaire controlled** : `TextField` (texte + e-mail avec validation),
  `Select`, `RadioGroup`, `Checkbox`, `Switch` ;
- une **Card** récapitulative (`outlined`) avec en-tête, contenu, actions ;
- une **List** de sessions actives (icône + texte principal/secondaire) ;
- un **Dialog** de confirmation de suppression ;
- une **Snackbar** de feedback ;
- une **Alert** d'avertissement (composant MUI **sans équivalent md** → shim) ;
- du **layout** `Box`/`Stack`/`Grid` + `sx` ;
- un **thème** `createTheme` (palette M2, forme, typo) en mode clair/sombre.

## Arborescence

```
examples/
├── README.md                 ← ce fichier
├── MIGRATION-NOTES.md        ← table AVANT → APRÈS → pourquoi, par composant
├── before/
│   ├── AccountSettings.tsx   ← écran 100 % MUI (Material 2)
│   └── theme.ts              ← createTheme (palette M2)
└── after/
    ├── AccountSettings.tsx   ← même UI, wrappers md + Tailwind + tokens
    ├── theme.css             ← tokens --md-sys-* (light + dark)
    └── shims/
        └── M3Alert.tsx       ← shim du gap MUI Alert (cf. 05-gap-analysis.md)
```

## Le diff conceptuel (ce que chaque changement illustre)

| Dimension              | AVANT (MUI)                                                    | APRÈS (M3)                                                                             | Illustre                                                                                        |
| ---------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **Composants**         | `<Button variant>`, `<TextField>`, `<Select>`, `<Dialog>`…     | wrappers `Md*` d'éléments `<md-*>` réels                                               | mapping canonique — contrat §3 / [`01-component-mapping.md`](../01-component-mapping.md)        |
| **Variantes**          | une prop `variant`                                             | **un élément par variante** (`MdFilledButton` vs `MdOutlinedButton` vs `MdTextButton`) | éclatement variant→élément — §3                                                                 |
| **Theming**            | `createTheme` + `ThemeProvider` (Emotion)                      | tokens `--md-sys-*` dans `theme.css`                                                   | source de vérité couleur — §5 / [`02-theme-token-migration.md`](../02-theme-token-migration.md) |
| **Layout**             | `Box`/`Stack`/`Grid` + `sx`                                    | `<div>` + utilitaires **Tailwind**                                                     | layout hors shadow DOM — §6 / [`06-tailwind-material-web.md`](../06-tailwind-material-web.md)   |
| **Events controlled**  | `onChange={(e, val) => …}` / `e.target.value`                  | events natifs `onInput`/`onChange` → `e.target.value` / `.checked` / `.selected`       | signature React modifiée — §4 / [`03-react-integration.md`](../03-react-integration.md)         |
| **Ouverture overlays** | `open={state}` (Dialog)                                        | **ref impérative** `ref.show()` / `ref.close()`                                        | pattern Lit + top-layer — §3 / `03-…`                                                           |
| **Slots**              | sous-composants (`DialogTitle`, `CardHeader`, `ListItemText`…) | **slots** `slot="headline"`, `slot="start"`…                                           | children/props → slots — §4                                                                     |
| **Gaps**               | `Alert`                                                        | **shim** `M3Alert` (pas de `md-alert`)                                                 | composant sans équivalent — [`05-gap-analysis.md`](../05-gap-analysis.md)                       |

## Comment lire l'exemple

1. Ouvrir `before/AccountSettings.tsx` puis `after/AccountSettings.tsx` côte à
   côte : la structure logique (état React, branches d'onglets) est **conservée**,
   seuls le rendu et les handlers changent.
2. Suivre [`MIGRATION-NOTES.md`](./MIGRATION-NOTES.md) pour le détail composant par
   composant et la liste des pièges (controlled inputs, slots, `sx` supprimé,
   `selected` vs `checked`).
3. Pour les wrappers eux-mêmes, voir `../wrappers/` (créés selon le contrat §2).

## Pièges résumés (détaillés dans MIGRATION-NOTES)

- **`Switch` : `checked` → `selected`.** `md-switch` expose `selected`, pas
  `checked` (vérifié `material-web/switch/internal/switch.ts:61`). Erreur
  silencieuse classique.
- **Controlled inputs.** Les events md sont natifs (`input`/`change`) ; la valeur
  se lit sur `e.target` (`.value` / `.checked` / `.selected` / `.activeTabIndex`),
  plus jamais via un 2ᵉ argument `(e, value)`.
- **`sx` n'existe pas.** Tout `sx` devient soit des classes Tailwind (layout sur
  le host), soit du `style` inline tirant des tokens `--md-sys-*`. Les utilitaires
  Tailwind **ne franchissent pas** le shadow DOM (§6).
- **Card sans sous-slots.** `md-*-card` n'a qu'un slot par défaut
  (`labs/card/internal/card.ts:19`) ; `CardHeader`/`CardContent`/`CardActions`
  deviennent du markup + layout Tailwind interne.
- **Dialog impératif.** Préférer `ref.show()`/`ref.close(returnValue)` à la prop
  `open` pour piloter correctement le top-layer.

```

```
