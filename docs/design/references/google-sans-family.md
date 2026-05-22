<!-- SPDX-License-Identifier: Apache-2.0 -->
# Référence — La famille Google Sans (vers Google Sans Flex)

Notes factuelles distillées (nos mots) de l'article Google Design
« Google Sans: Evolving Google's Typeface ».
Source : <https://design.google/library/google-sans-flex-font>

## Faits

- **Google Sans** (2018) : typographie de marque géométrique, pensée pour le display/grand texte (créait un système bi-police avec Roboto pour le petit texte).
- **Google Sans Text** (2020) : variante pour petites tailles — caractères plus hauts, plus condensés, moins circulaires, espacement accru, proportions alignées sur Roboto. Déployée sur le Pixel 3. Co-conçue avec Colophon Foundry.
- **Google Sans Mono** (2020) : chasse fixe pour l'éditorial (medium/large). Peu lisible en petit (ambiguïté a/o).
- **Google Sans Code** (2025) : monospace open-source pour le code, par la fonderie Universal Thirst, après recherche sur les 20 langages les plus courants ; réduit l'ambiguïté des glyphes. **C'est la police d'affichage du code dans Gemini.**
- **Google Sans + Google Sans Flex** : passées **open-source en 2025** (disponibles sur Google Fonts). Flex = police **variable**.
- Support de **20+ systèmes d'écriture** (Arabe, CJK, Ge'ez, Thaï…) ⇒ l'une des plus grandes familles typographiques au monde.

## Pour aphrody

- C'est exactement la police observée dans les tokens Gemini (`--gem-sys-typography-*-font-name: "Google Sans Flex"`, cf. [`../gemini/README.md`](../gemini/README.md)).
- Le projet a déjà des modules tokens `google_sans_flex.rs` / `google_sans_code.rs` (côté Rust, géré par le peer).
- Côté JS/TS : charger Google Sans Flex (variable) + Google Sans Code via Google Fonts pour `apps/m3-react` et la fusion shadcn/tailwind. L'axe **optical size** (cf. transparent-screens) sert la lisibilité.
