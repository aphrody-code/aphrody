<!-- SPDX-License-Identifier: Apache-2.0 -->

# 11 — Material Symbols : intégration, couverture, optimisation

> Référence pour l'usage des icônes dans `material-web` (`md-icon` + tout
> composant qui slotte une icône) et la migration `@mui/icons-material` ->
> Material Symbols. Les valeurs numériques sont **vérifiées en direct**
> (codepoints officiels Google, API Google Fonts CSS2) le **2026-05-29**.

---

## 1. Le modèle : `md-icon` + Material Symbols variable

`<md-icon>` est un simple conteneur : il pose `font-family` et les 4 axes
variables, puis affiche son **texte enfant = nom de glyphe** (ligature). Il **ne
charge pas** la police — l'intégrateur le fait une fois (cf. §3).

```html
<md-icon>home</md-icon>
<!-- Outlined, défauts FILL 0 / wght 400 -->
```

**Material Symbols** (2022+) remplace Material Icons (legacy) : police **variable**
à 4 axes, 3 styles (Outlined / Rounded / Sharp ; plus de TwoTone), **4253 glyphes**
(set Outlined, compté sur les codepoints officiels).

---

## 2. Couverture : les 4 axes variables exposés en tokens

Depuis 2026-05, le `md-icon` **stable** expose les 4 axes Material Symbols comme
tokens (avant : seul `font-variation-settings: inherit` existait — aucun contrôle
par axe). Source : `packages/material-web/icon/internal/_icon.scss` +
`tokens/_md-comp-icon.scss`.

| Axe  | Token            | Plage    | Défaut | Effet                          |
| ---- | ---------------- | -------- | ------ | ------------------------------ |
| FILL | `--md-icon-fill` | 0..1     | 0      | 0 = contour, 1 = plein         |
| wght | `--md-icon-wght` | 100..700 | 400    | épaisseur du trait             |
| GRAD | `--md-icon-grad` | -50..200 | 0      | emphase sans changer la taille |
| opsz | `--md-icon-opsz` | 20..48   | 24     | taille optique (détails)       |

Ces tokens **héritent par cascade** (custom properties) : poser
`--md-icon-fill: 1` sur un ancêtre remplit toutes les icônes du sous-arbre, tout
en gardant un contrôle par icône.

```css
/* Remplir au survol — animable car les axes sont des custom properties */
.nav-item:hover md-icon {
  --md-icon-fill: 1;
}
/* Icône fine et grande dans un hero */
.hero md-icon {
  --md-icon-wght: 200;
  --md-icon-opsz: 48;
  --md-icon-size: 48px;
}
```

Les autres tokens existants restent : `--md-icon-size` (24px), `--md-icon-font`
(`Material Symbols Outlined` ; passer à `Rounded`/`Sharp` pour changer de style).

---

## 3. Intégration : charger la police (helper fourni)

`packages/material-web/icon/material-symbols.ts` fournit des helpers (calqués sur
ceux de Google Sans). **Trois voies** :

### a. CDN Google Fonts, plages variables (prototypage)

```ts
import { ensureMaterialSymbols } from "@aphrody/material-web/icon/material-symbols.js";
ensureMaterialSymbols(); // injecte le <link> Outlined, plages variables complètes
```

Charge l'URL avec les **plages** d'axes (et non une instance figée), donc les
tokens `--md-icon-fill/wght/grad/opsz` agissent réellement :

```
https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200&display=block
```

> **Piège** : l'ancienne URL figée `@24,400,0,0` (telle qu'utilisée par défaut
> partout) gèle FILL et wght — un `--md-icon-fill: 1` reste alors **sans effet**.
> L'ordre des axes est `opsz,wght,FILL,GRAD` (vérifié : sert un stylesheet valide).

### b. CDN Google Fonts, **subset** (recommandé en prod)

```ts
ensureMaterialSymbols({ iconNames: ["home", "search", "settings", "close"] });
```

Ajoute `&icon_names=` : Google ne sert **que ces glyphes**. Mesuré le 2026-05-29 :
la feuille subset pour 3 icônes = **2 857 octets** (vs police complète de plusieurs
Mo). C'est le **gain de payload dominant** quand le set d'icônes est connu — et le
codemod (§5) collecte précisément ce set.

### c. Self-hébergé woff2/ttf (offline, zéro CDN)

```ts
import { ensureMaterialSymbolsFontFace } from "@aphrody/material-web/icon/material-symbols.js";
ensureMaterialSymbolsFontFace("/fonts/MaterialSymbolsOutlined.woff2");
```

Fichiers officiels (repo `google/material-design-icons`, Apache-2.0) :
`variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].woff2` (+ `.ttf`,
`.codepoints`). Idem `Rounded` / `Sharp`.

---

## 4. Optimisation — recommandations actionnables

1. **`font-display: block`, jamais `swap`** pour des icônes en **ligature** : avec
   `swap`, le nom du glyphe (« home », « settings ») s'affiche en clair avant
   chargement (FOUT). Les helpers utilisent `display=block` / `font-display:
block`. (Avec des code points `&#xe88a;` au lieu de ligatures, `swap` est ok.)
2. **Subset en prod** via `&icon_names=` (voie 3b) ou `pyftsubset` en self-host :
   ne sert que les glyphes utilisés. Le set se récolte au build :
   ```bash
   rg -oN '<md-icon>([a-z_]+)</md-icon>' -r '$1' src | sort -u
   ```
3. **`preconnect`** vers `fonts.googleapis.com` + `fonts.gstatic.com` (crossorigin)
   si CDN ; **`preload`** uniquement un woff2 **déjà subseté** (jamais la police
   complète) pour les icônes above-the-fold.
4. **Une seule famille par style** : Outlined couvre le défaut M3 ; ne charger
   Rounded/Sharp que si réellement utilisés (chaque style = un fichier de plus).
5. **Cache CDN partitionné** : depuis Chrome 86 le cache Google Fonts n'est plus
   partagé entre sites — l'argument « cache mutualisé » du CDN est caduc. En prod
   exigeante, préférer self-host subseté.

---

## 5. Migration `@mui/icons-material` -> Material Symbols (96 % automatisable)

Contrairement à ce que laissait penser l'étude de cas initiale (« icônes = 100 %
manuel »), la migration des icônes est **déterministe et automatisée** par le
codemod dédié `migration/codemods/transforms/icons.ts`.

**Algorithme** (`lib/icon-names.ts`) : PascalCase MUI -> snake_case Material
Symbols, avec frontières d'acronymes ET de chiffres (`Brightness4` ->
`brightness_4`), suffixe de style retiré (`DeleteOutlined` -> `delete`, style
Outlined ; `TwoTone` -> Outlined), puis **validation contre les 4253 noms
officiels** (`data/material-symbols-names.json`). Trois issues :

| Cas                                  | Traitement                                                                             |
| ------------------------------------ | -------------------------------------------------------------------------------------- |
| glyphe valide                        | `<X/>` -> `<md-icon>glyph</md-icon>` + import d'effet de bord md-icon                  |
| logo de marque (GitHub, X, YouTube…) | **inchangé** + TODO (absent de Material Symbols par politique Google -> garder en SVG) |
| nom non validé                       | inchangé + TODO avec le snake_case deviné (à vérifier sur fonts.google.com/icons)      |

### Résultat mesuré (30 fichiers rpbey réels)

```
30/30 fichiers transformés · 0 erreur
108 <md-icon> générés automatiquement
  5 logos de marque -> TODO (gardés en SVG)
  0 nom non résolu
=> 96 % d'auto-conversion (108/113 usages d'icônes)
```

Sur l'inventaire complet rpbey (156 icônes distinctes) : 151 résolues (96 %), 5
marques. Les seules retouches manuelles : remettre les 5 logos de marque en SVG,
et reporter les props d'icône MUI non triviales (`fontSize` -> `--md-icon-size`,
`color` -> `currentColor`).

### Usage

```bash
cd migration/codemods
bunx jscodeshift -t transforms/icons.ts --parser=tsx --extensions=tsx 'src/**/*.tsx'
```

À lancer **après** l'orchestrateur (composants) ou seul. Les noms valides et les
exceptions vivent dans `data/` ; le mapping consolidé (composants + icônes) est
exporté en `migration/mui-m3-map.json`.

---

## Annexe — vérifier les données

```bash
# Liste autoritative des glyphes (4253) — set Outlined = surensemble des 3 styles
curl -s "https://raw.githubusercontent.com/google/material-design-icons/master/variablefont/MaterialSymbolsOutlined%5BFILL%2CGRAD%2Copsz%2Cwght%5D.codepoints" | wc -l

# Tester le subsetting (doit renvoyer une feuille minuscule)
curl -s "https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200&icon_names=home,search,settings&display=block" | wc -c
```
