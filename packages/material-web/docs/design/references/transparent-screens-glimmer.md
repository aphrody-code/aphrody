<!-- SPDX-License-Identifier: Apache-2.0 -->

# Référence — Concevoir pour écrans transparents (Jetpack Compose Glimmer)

Notes factuelles distillées (nos mots) de l'article Google Design
« Designing for Transparent Screens » (design system Android XR « Jetpack Compose Glimmer »).
Source : <https://design.google/library/transparent-screens>

## Faits techniques

- **Affichage additif** : l'écran ne peut qu'**ajouter de la lumière**, pas créer du noir → le **noir = transparent** (un vide, pas une couleur). UI projetée à ~**1 mètre** de profondeur perçue (longueur de bras) ; lire = choix actif de déplacer le focus.
- **Surfaces sombres + contenu clair** (l'inverse du Material classique). Les surfaces claires provoquent la **halation** (la lumière vive bave dans le contenu sombre → texte illisible). Le « noir » est redéfini comme un **conteneur** (« clean plate »). Système de profondeur via ombres sombres pour l'occlusion.
- **Typographie en angle visuel (degrés)**, pas en px : minimum lisible ≈ **0,6°** ; styles confortablement au-dessus. Usage de l'axe **optical size** de Google Sans Flex (contreformes plus larges, espacement optimisé) pour la lisibilité au coup d'œil.
- **Couleur** : ratio de contraste additif = **(luminosité environnement + luminosité écran) / luminosité écran**. Les couleurs saturées « disparaissent » sur le monde réel → palette **désaturée, proche du blanc**, interface **neutre par défaut**, couleur réservée pour attirer l'attention.
- **Motion** : transition de notification ≈ **2 s** (et non 500 ms) pour inviter le focus depuis la périphérie sans l'exiger ; **focus rings/highlights** pour un retour d'input basse latence.

## Pour aphrody

- Pertinent pour tout futur thème **dark/ambient** ou surface XR. Confirme le choix dark de Gemini (surfaces sombres, contenu clair) déjà capturé dans [`../gemini/theme.css`](../gemini/theme.css).
- Le ratio de contraste additif et le seuil 0,6° sont des contraintes d'accessibilité réutilisables (variante de la règle de contraste M3, cf. [`../m3-glossary.md`](../m3-glossary.md) « Contrast »).
- L'axe optical-size de Google Sans Flex renforce la note typographie ([`google-sans-family.md`](google-sans-family.md)).
