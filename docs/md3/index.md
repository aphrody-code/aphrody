# Material Design 3 (MD3) - Overview

Bienvenue dans la documentation officielle de l'implémentation de Material Design 3 (MD3) pour le projet **Aphrody / Google OS**.
L'interface graphique est une expression native du **Pilier III (Bun/JSX)** de notre architecture God Mode. Elle génère le DOM utilisé par le **Pilier I (Rust Webview)**.

## L'Architecture FULL Bun JSX

Le package `packages/ui` a été entièrement refactorisé en pur Bun JSX :
1. **Zéro Dépendance React/Vue** : Un compilateur natif JSX vers HTML (`html.ts`) génère instantanément le balisage statique.
2. **Couverture Globale (Glossaire M3)** : 100% des concepts du glossaire officiel (Buttons, Navigation Rail, Dialogs, Cards) sont implémentés via les Custom Elements `@material/web`.
3. **God Mode Integration** : Le script de build Bun exporte directement le HTML dans le crate Rust `gui`, ce qui permet une compilation finale unifiée et une accélération matérielle (WebGPU/DX12) lors de l'exécution.

## Qu'est-ce que Material Design 3 ?

Material Design 3 (Material You) apporte :
1. **Personnalisation dynamique** : Espace colorimétrique HCT.
2. **Accessibilité renforcée** : Contraste défini algorithmiquement.
3. **Design Tokens** : `--md-sys-color-primary` et dérivés.

## Navigation

*   [**Global DESIGN.md**](../../DESIGN.md) - Règles d'architecture globales, God Mode, et Desktop Best Practices.
*   [Composants Natifs (Components)](components.md) - Utilisation des wrappers JSX pour `@material/web`.
*   [Le Système de Thème (Theming)](theming.md) - Design Tokens, HCT, et couleurs statiques.
*   [Typographie & Icônes (Typography & Icons)](typography-icons.md) - Google Sans et Material Symbols.
