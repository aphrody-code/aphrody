<!-- SPDX-License-Identifier: Apache-2.0 -->
# Extraction Google Design (google.com · Gemini · design.google) via bxc

Continuation du travail du module Google de bxc
(`packages/bxc/scripts/google-design-recon.ts`, module `src/google/`) :
extraction des surfaces CSS/JS/UI/tokens/fontes des canons design de Google,
puis complétion de `packages/material-web` (2026-05-23).

## Méthode

`bin/bxc.exe` (recon/detect) + `bun packages/bxc/scripts/google-design-recon.ts
--profile static --targets google.com,design,material,fonts,gemini,aistudio`
→ dossier `packages/bxc/var/google-design/{DOSSIER.md,dossier.json}` (gitignoré).
Le module Google de bxc porte un **Atlas** auto-généré (366 hôtes Google,
classés CDN `GFE` + framework `wiz`/`angular`/`lit`) et des profils stealth
`stealth-wiz`/`stealth-spa`/`stealth-lit`. Le profil `max` (Chrome connecté
réel) capture les corps CSS/JS ; non exécuté ici (invasif sur la session
navigateur de l'utilisateur) — l'extraction repose sur les signatures + le
dossier accumulé.

## Ce qui a été extrait (signaux agrégés, tout Google Design)

- **Frameworks** : `boq-wiz` (Wiz client + Boq serveur) sur google.com / fonts /
  gemini / aistudio ; `gemini-app`, `gemini-brand`, `gemini-model` sur Gemini.
- **Surfaces de tokens CSS (namespaces)** :
  - Material 3 : `--md-sys-color-*`, `--md-sys-typescale-*`, `--md-sys-elevation-*`,
    `--md-sys-shape-*`, `--md-ref-palette-*`.
  - Gemini : `--gem-sys-*`, `--gem-app-*`, `--bard-color-*`.
- **Familles de fontes** : Google Sans Flex, **Google Sans Code**,
  **Google Sans Mono**, Google Sans Text, Google Sans, Product Sans, Roboto.
- **Modèles Gemini vus** : gemini-2.5-flash, gemini-3-flash-preview,
  gemini-3-pro-image-preview.
- **Hôtes d'API** : `gemini.google.com`, `gemini.gstatic.com`,
  `fonts.gstatic.com`, `*.clients6.google.com` (feedback/ogads/signaler/waa),
  `region1.google-analytics.com`, `play.google.com`.

> Note classes/sélecteurs : les classes CSS de Google sont **atomiques /
> obfusquées** (non portables). La surface réutilisable est l'ensemble des
> **custom properties** (tokens ci-dessus) — déjà notre dénominateur commun
> (`--md-sys-*`) + le pont Gemini (`docs/design/gemini/theme.css`,
> `tokens-system.css`).

## Complétion material-web appliquée

Gap concret révélé : notre `packages/material-web/typography/` n'exposait que
**Google Sans Flex**, alors que Google utilise aussi **Google Sans Code** pour
les surfaces code (Gemini, AI Studio). Ajouté :

- `typography/internal/font-face.ts` : `CODE_FONT_FAMILY = 'Google Sans Code'`,
  `codeFontFaceCss(url)` (axes `MONO` + `wght` 300–800), `codeGoogleFontsHref()`.
- `typography/internal/md-type.ts` : attributs `code` (bascule sur Google Sans
  Code) + `mono` (axe `MONO` 0..1), `font-variation-settings: "MONO" m, "wght" w`.
  Token d'override `--md-sys-typescale-code-font`.

Aligné sur `crates/m3-tokens/src/google_sans_code.rs` (axes `mono`/`wght`,
défaut mono=1.0 wght=400). Validation : `bun run typecheck:aphrody` + `build:aphrody`
→ exit 0.

## Pointeurs
- Module Google bxc : `packages/bxc/src/google/` (atlas, detector, profiler, signatures).
- Dossier recon : `packages/bxc/var/google-design/DOSSIER.md`.
- Wiz/Boq : `docs/research/wiz-framework-and-material-web.md`.
- Tokens Gemini : `docs/design/gemini/`. Parité : `docs/design/angular-material-parity.md`.
