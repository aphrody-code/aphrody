<!-- SPDX-License-Identifier: Apache-2.0 -->
# Typographie et Iconographie (Material Symbols)

Le package `ui` intègre `material-symbols`, le standard absolu de l'iconographie Google pour Material Design 3.

## Material Symbols vs Material Icons

Les anciennes *Material Icons* étaient statiques. Les nouveaux **Material Symbols** utilisent la technologie des **Variable Fonts** (Polices Variables). Une seule police de caractères contient des millions de variations d'une icône modifiables via des propriétés CSS.

## Utilisation

Importez la police dans votre CSS ou HTML, puis utilisez la classe appropriée :

```html
<!-- HTML -->
<span class="material-symbols-outlined">search</span>
<span class="material-symbols-rounded">settings</span>
<span class="material-symbols-sharp">home</span>
```

### Axes de Police Variable

La puissance des Material Symbols réside dans les axes CSS `font-variation-settings`. Vous pouvez altérer l'icône à la volée de manière fluide.

1. **FILL (Remplissage)** `0` ou `1`
   Permet de passer d'une icône vide (outline) à une icône pleine. Souvent utilisé pour l'état actif/inactif d'un bouton.
2. **wght (Poids/Graisse)** `100` à `700`
   L'épaisseur des traits de l'icône.
3. **GRAD (Grade)** `-25` à `200`
   Ajuste visuellement l'épaisseur en fonction des fonds (sombre ou clair) sans modifier la taille physique de l'icône.
4. **opsz (Taille optique)** `20` à `48`
   Ajuste les détails de l'icône pour qu'elle reste lisible selon la taille d'affichage (20px, 24px, etc.).

### Exemple CSS

```css
.material-symbols-outlined {
  font-family: 'Material Symbols Outlined';
  font-variation-settings:
  'FILL' 0,
  'wght' 400,
  'GRAD' 0,
  'opsz' 24;
}

/* Au survol, l'icône se remplit et s'épaissit subtilement */
button:hover .material-symbols-outlined {
  font-variation-settings:
  'FILL' 1,
  'wght' 500,
  'GRAD' 20,
  'opsz' 24;
  transition: font-variation-settings 0.2s ease-in-out;
}
```

## Typographie (Google Sans & Roboto)

Dans le cadre du projet `aphrody` et de l'environnement WinClean, nous appliquons une charte typographique stricte basée sur l'écosystème **Google Sans** :

1. **Interface par défaut** : **Google Sans Flex**. Cette police variable permet un ajustement parfait de la graisse (wght), de la chasse (wdth) et de la taille optique (opsz) pour toutes les interfaces fluides MD3.
2. **Code et Éditeurs** : **Google Sans Text** / **Google Sans**. Utilisé pour l'affichage de code structuré dans les UI, garantissant une lisibilité maximale.
3. **Terminal** : **Google Sans Mono**. Exclusivement réservée aux CLI et aux affichages bruts.

### Intégration Terminal (Windows Terminal Canary)

Notre émulateur cible est **Windows Terminal Canary**, customisé spécifiquement avec les palettes de couleurs Material Design 3. L'intégration garantit que :
- La police par défaut est configurée sur `Google Sans Mono`.
- Les couleurs (ANSI) sont mappées sur les tokens `sys-color` de Material Design 3 (ex: le rouge ANSI pointe vers `--md-sys-color-error`).

Exemples de Tokens Typographiques MD3 :
*   `--md-sys-typescale-display-large-font-family` (Configuré sur `'Google Sans Flex', sans-serif`)
*   `--md-sys-typescale-body-medium-size`
*   `--md-sys-typescale-label-small-weight`
