<!-- SPDX-License-Identifier: Apache-2.0 -->
# Tokens de motion Material Design 3 (référence canonique aphrody)

Données extraites le 2026-05-21 de trois sources M3 officielles :

- Concept : <https://m3.material.io/styles/motion/overview/how-it-works>
- Conversion web (springs -> courbes) : <https://m3.material.io/styles/motion/overview/specs>
- Valeurs spring natives : <https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md>

(Pages M3 fetchées via `mcp__aphrody__universal_web_fetch` — le reader-proxy
extrait le SPA Angular là où le moteur headless d'obscura ne bootstrappe pas
l'app. Le doc MDC-Android via fetch GitHub direct.)

## 1. Modèle (M3 Expressive, mai 2025)

M3 a remplacé easing+duration par un **système de springs physiques**. Un
spring = 3 attributs : **stiffness** (dureté ; plus haut = résout plus vite),
**damping** (amortit le bounce ; **1.0 = aucun overshoot**), **initial
velocity**. Deux familles de tokens :

- **spatial** — position x/y, rotation, taille, coins. **Overshoot autorisé**
  (damping 0.9 -> léger rebond « vivant »).
- **effects** — couleur, **opacité**. **Aucun overshoot** (damping 1.0).

Trois vitesses : `fast` (petits composants), `default` (partiel écran), `slow`
(plein écran). Deux schémas : **expressive** (par défaut, héros) et
**standard** (utilitaire). Token : `md.sys.motion.spring.{speed}.{family}`.

## 2. Valeurs spring natives (MDC-Android — scheme expressive)

| Token | damping | stiffness | Usage M3 |
|---|---|---|---|
| `motionSpringFastSpatial` | 0.9 | 1400 | petits composants (switch, bouton) |
| `motionSpringFastEffects` | 1.0 | 3800 | effets petits composants (couleur/opacité) |
| `motionSpringDefaultSpatial` | 0.9 | 700 | partiel écran (bottom sheet, nav drawer) |
| `motionSpringDefaultEffects` | 1.0 | 1600 | effets partiel écran |
| `motionSpringSlowSpatial` | 0.9 | 300 | plein écran |
| `motionSpringSlowEffects` | 1.0 | 800 | effets plein écran (couleur/opacité) |

Invariant : spatial = damping **0.9** (overshoot), effects = damping **1.0**
(pas d'overshoot). Stiffness décroît fast -> slow.

## 3. Conversion web (springs -> cubic-bezier + durée)

Pour les plateformes sans springs natifs (courbes mimant le spring ; à utiliser
hors gestes/interruptions). `cubic-bezier(x1, y1, x2, y2)` + durée :

| Token | cubic-bezier | durée |
|---|---|---|
| Expressive fast spatial | 0.42, 1.67, 0.21, 0.90 | 350 ms |
| Expressive default spatial | 0.38, 1.21, 0.22, 1.00 | 500 ms |
| Expressive slow spatial | 0.39, 1.29, 0.35, 0.98 | 650 ms |
| Expressive fast effects | 0.31, 0.94, 0.34, 1.00 | 150 ms |
| Expressive default effects | 0.34, 0.80, 0.34, 1.00 | 200 ms |
| Expressive slow effects | 0.34, 0.88, 0.34, 1.00 | 300 ms |
| Standard fast spatial | 0.27, 1.06, 0.18, 1.00 | 350 ms |
| Standard default spatial | 0.27, 1.06, 0.18, 1.00 | 500 ms |
| Standard slow spatial | 0.27, 1.06, 0.18, 1.00 | 750 ms |
| Standard fast effects | 0.31, 0.94, 0.34, 1.00 | 150 ms |
| Standard default effects | 0.34, 0.80, 0.34, 1.00 | 200 ms |
| Standard slow effects | 0.34, 0.88, 0.34, 1.00 | 300 ms |

Note : les `y` > 1.0 des courbes spatial (ex. 1.67, 1.29) encodent l'overshoot.

## 4. Mapping vers aphrody (`animate` + crates motion)

`animate` (cf. [`animate-tui-motion.md`](animate-tui-motion.md)) expose des
springs `stiffness`/`damping` -> branchement **1:1** sur les valeurs §2.
Plan d'implémentation :

```rust
// Presets M3 au-dessus d'animate (scheme expressive par defaut).
pub struct M3Spring { pub damping: f32, pub stiffness: f32 }
pub const FAST_SPATIAL:    M3Spring = M3Spring { damping: 0.9, stiffness: 1400.0 };
pub const FAST_EFFECTS:    M3Spring = M3Spring { damping: 1.0, stiffness: 3800.0 };
pub const DEFAULT_SPATIAL: M3Spring = M3Spring { damping: 0.9, stiffness: 700.0 };
pub const DEFAULT_EFFECTS: M3Spring = M3Spring { damping: 1.0, stiffness: 1600.0 };
pub const SLOW_SPATIAL:    M3Spring = M3Spring { damping: 0.9, stiffness: 300.0 };
pub const SLOW_EFFECTS:    M3Spring = M3Spring { damping: 1.0, stiffness: 800.0 };
```

### Application au logo (`aphrody-logo`)
- **Respiration « Pause Point »** (opacité/luminance) = famille **effects**,
  `SLOW_EFFECTS` (damping 1.0 -> jamais d'overshoot, conforme « effects »),
  cycle `alternate`. Le 4 s/6 s de la design spec se réalise en jouant la
  montée (inspiration) sur `SLOW_EFFECTS` puis une descente plus lente.
- **Thought-state pulse** (scale/intensité de la pointe) = famille **spatial**,
  `FAST_SPATIAL` (damping 0.9 -> léger overshoot « vivant »).

### Application UI `aphrody-terminal`
- Transitions de panneaux / taille = `DEFAULT_SPATIAL` ; switches/toggles =
  `FAST_SPATIAL` ; changements de couleur/opacité = `*_EFFECTS`.
- Exposer `MotionScheme::{Expressive, Standard}` ; le standard remplace les
  spatial par damping ~1.0 (courbe `0.27,1.06,0.18,1.00`, bounce minimal).

## 5. Checklist actionnable

- [ ] Pin `animate = "0.4"` (MIT) workspace + `cargo deny check`.
- [ ] Crate/module `aphrody-motion` (ou feature `motion` d'`aphrody-logo`) :
      les 6 presets §4 au-dessus d'`animate`, + helper courbe-bezier §3 pour les
      cibles sans spring natif (wasm/CSS export).
- [ ] `aphrody logo --animate` : respiration `SLOW_EFFECTS` en demi-blocs.
- [ ] Verify : test que `*_EFFECTS` ne dépasse jamais 1.0 (pas d'overshoot) et
      que `*_SPATIAL` peut dépasser (overshoot présent).
