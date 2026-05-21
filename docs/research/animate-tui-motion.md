<!-- SPDX-License-Identifier: Apache-2.0 -->
# Évaluation : `animate` (vyfor) comme moteur de motion TUI

**Source** : <https://github.com/vyfor/animate> · crate <https://crates.io/crates/animate> `0.4.1` (évalué 2026-05-21).
**Statut** : recommandation de recherche. Aucune dépendance ajoutée par cette note.

## 1. Ce qu'est `animate`

Bibliothèque d'animation Rust légère, **MIT** (permissive, compatible
Apache-2.0 — pas de contamination, cf. politique licences §5).

- **Zéro dépendance par défaut** ; feature `ratatui` optionnelle pour les
  interpolateurs des types Ratatui.
- API **macro-driven** : `#[animate]` sur un struct, `#[tween(...)]`
  (animations temporelles) / `#[spring(...)]` (ressorts physiques) sur les
  champs, `animate::tick()` pour avancer d'une frame, trait `Tween` pour les
  types custom, `get()/set()` par champ.
- Modes : `once`, `cycle`, `alternate`.
- Easing intégrés : `linear`, `quad_in/out/in_out`, `cubic_in/out/in_out` +
  easing/interpolateurs custom.
- Version `0.4.1`, 100 % Rust, publié crates.io.

## 2. Pourquoi c'est pertinent pour aphrody

Deux besoins concrets, déjà spécifiés, n'ont pas de moteur de motion :

1. **Animation du logo terminal** (`crates/aphrody-logo`) :
   - **« Pause Point »** (design spec §4.3) : respiration guidée 4 s inspiration
     lumineuse / 6 s expiration diffuse — un `#[tween]` `alternate` sur
     l'opacité/luminance du logo, easing `cubic_in_out`.
   - **« Thought states »** (design spec §3.1) : pulse d'impulsion sur la
     pointe (leading edge) pendant le raisonnement Gemini — un `#[spring]` sur
     l'intensité.
   Le rendu statique actuel (`render_terminal`/`render_halfblocks`) devient une
   boucle : `animate::tick()` met à jour un facteur, on régénère la frame
   half-block (ou on module l'alpha Kitty) à ~30 fps.

2. **Motion `aphrody-terminal`** : transitions de panneaux, spinners,
   progress, curseurs — springs/tweens au lieu de pas discrets codés à la main.
   Si `aphrody-terminal` adopte des types Ratatui, la feature `ratatui` câble
   les interpolateurs directement.

Atouts d'alignement : zéro-dep par défaut (n'alourdit pas `cargo ci-offline`),
MIT, pur Rust, surface minuscule.

## 2.5. Modèle de motion M3 à adopter (spring physics)

Source : <https://m3.material.io/styles/motion/overview/how-it-works> (fetchée
2026-05-21 via `mcp__aphrody__universal_web_fetch` — reader-proxy, là où obscura
échoue sur ce SPA). Depuis M3 Expressive (mai 2025), M3 a **remplacé
easing+duration par un système de springs physiques**. `animate` (springs
stiffness/damping) en est l'analogue direct côté Rust/TUI :

- **Spring = 3 attributs** : `stiffness` (dureté → résout plus vite),
  `damping` (amortit le bounce ; **1.0 = aucun bounce**), `initial velocity`.
  → mappe sur le `#[spring(...)]` d'`animate`.
- **Deux schémas** : **expressive** (dépasse la valeur finale → bounce, pour les
  moments héros) et **standard** (ease-in, bounce minimal, utilitaire).
- **Deux familles de tokens** :
  - **spatial** (position x/y, rotation, taille, coins) — **overshoot autorisé**.
  - **effects** (couleur, **opacité**) — **jamais d'overshoot**.
  - 3 vitesses chacune : `default` / `fast` / `slow`. Forme du token :
    `md.sys.motion.spring.{fast|default|slow}.{spatial|effects}`.
- **Web** : « compatible with Compose springs » — donc transposable à `animate`.

**Conséquence directe pour le logo aphrody** :
- **Respiration Pause Point** (opacité/luminance) = token **effects** →
  spring **damping ≈ 1.0 (zéro overshoot)**, vitesse `slow` (cycle 4 s/6 s).
- **Thought-state pulse** (scale/intensité de la pointe) = token **spatial** →
  spring avec léger overshoot (damping < 1), vitesse `fast`.
- aphrody devrait exposer 2 presets `MotionScheme::{Expressive, Standard}` et
  des helpers `spring_spatial_{fast,default,slow}` / `spring_effects_*`
  reproduisant la sémantique des tokens M3 au-dessus d'`animate`.

## 3. Frictions / réserves

- **Orienté Ratatui** (desc crates.io : « Animation library for Ratatui »). Le
  cœur tween/spring/easing est générique et utilisable sans Ratatui, mais la
  valeur maximale suppose un front Ratatui — à confirmer côté
  `aphrody-terminal` (vérifier s'il est Ratatui-based avant d'activer la feature).
- **Jeune** (0.4.x, 92 stars) : surveiller la stabilité d'API avant un pin dur ;
  acceptable pour une feature non critique (cosmétique de logo / motion UI).
- **Boucle de rendu** : animer le logo exige une boucle de frames + contrôle
  terminal (alt-screen ou réécriture in-place) — coût d'intégration côté
  `aphrody-logo`/`aphrody-terminal`, pas côté `animate`.

## 4. Décision recommandée

**Adopter `animate 0.4` comme moteur de motion TUI**, en dépendance directe
(contrairement à Obscura qui reste une façade binaire) — c'est une lib pure
Rust légère, pas un runtime externe.

- Ajouter `animate = { version = "0.4", default-features = false }` au
  `[workspace.dependencies]`, activer la feature `ratatui` seulement dans les
  crates qui consomment des types Ratatui.
- Premier usage : `aphrody logo --animate` (ou `aphrody logo breathe`) jouant la
  respiration Pause Point en demi-blocs truecolor via un `#[tween] alternate`
  `cubic_in_out` sur la luminance.

## 5. Prochaines étapes actionnables

- [ ] Confirmer si `aphrody-terminal` est Ratatui-based (`grep ratatui
      crates/aphrody-terminal-*/Cargo.toml`) → décide de la feature `ratatui`.
- [ ] Pin `animate = "0.4"` workspace + `cargo deny check` (licence MIT,
      advisories) avant adoption.
- [ ] `aphrody-logo` : ajouter `breathe(cols, fps, cycle)` derrière feature
      `motion` (dep `animate`), boucle `tick()` → re-render half-block, easing
      `cubic_in_out`, 4 s/6 s par défaut. Verify : smoke visuel + test que la
      séquence d'opacité est monotone par phase.
- [ ] Wirer `aphrody logo --animate` dans le CLI (réutilise le dispatch `Logo`).

## 6. Verdict

**Recommandé** comme moteur de motion (logo breathing/pulse + UI
`aphrody-terminal`), en dépendance directe MIT légère. Complémentaire au logo
statique déjà livré (`aphrody-logo`) : `animate` n'apporte que la dimension
temporelle, le rendu reste celui d'`aphrody-logo`.
