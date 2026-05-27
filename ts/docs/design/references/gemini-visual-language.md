<!-- SPDX-License-Identifier: Apache-2.0 -->
# Référence — Le langage visuel de Gemini

Notes factuelles distillées (nos mots) de l'article Google Design
« Illustrating the Gemini App ».
Source : <https://design.google/library/gemini-ai-visual-design>

## Principes (faits)

- **Objectifs de marque** : intuitif, immersif, accessible, aspirationnel, et avant tout digne de confiance.
- **Gradients** = élément central : « constructeurs de contexte » qui transmettent une énergie et une direction ; bord d'attaque net/opaque qui se diffuse vers la queue → pointeurs visuels dirigeant l'attention. Personnifient le « raisonnement » de l'IA.
- **Forme fondatrice = le cercle** : simplicité/harmonie/confort ; le logo Gemini naît de l'espace négatif de quatre cercles. Boutons/conteneurs à coins arrondis pour la continuité avec l'écosystème Google.
- **Références héritées** : les quatre points de couleur Google (rouge/jaune/vert/bleu) ; formes Material adoucies/floutées (qualité « éthérée »).
- **Motion intentionnelle** : chaque animation a un début/fin définis → flux directionnel qui reflète l'action utilisateur ; ondes radiales (gradient) pour la voix ; icônes animées pour signaler de nouvelles features.
- **Qualité** : « chaleureuse, spatiale, arrondie » ; douceur quand le système est difficile à appréhender (gradients pulsants guidés, langage clair, signaux transparents).
- L'**icône sparkle** est l'élément le plus ubiquitaire de la marque.

## Pour aphrody

- Les gradients de marque sont capturés verbatim dans [`../gemini/theme.css`](../gemini/theme.css) (`--gem-sys-color--brand-*`, transitions blue/orange/pink/purple/red/teal).
- Le cercle + coins arrondis ↔ tokens M3 `--md-sys-shape-*` (full/rounded) du framework.
- La motion directionnelle ↔ module Rust `m3-tokens/motion.rs` (easing/durations) côté peer ; côté JS/TS, animer les wrappers `apps/m3-react` avec ces courbes.
