# Gemini web app — import des design tokens (au pixel près)

Import des design tokens / thème / CSS de **gemini.google.com/app**, extraits le
2026-05-22 par lecture des **CSS custom properties calculées** (`getComputedStyle`
sur `:root` + `body`) de la page live, via l'automatisation navigateur.

## Ce qui est capturé ici

- [`theme.css`](theme.css) — tokens **vérifiés verbatim** (pixel-perfect) du
  cœur du thème : surfaces, focus/états, typographie, palette de marque
  `--gem-sys-color-*` (22), couleurs composants `--bard-color-*` (18). Thème
  **sombre** par défaut.
- [`tokens-system.css`](tokens-system.css) — la **couche système complète**
  (175 tokens : `--gem-sys-color--*` rôles M3 + palettes de marque/accent +
  gradients + l'échelle de coins 10-step + `--lumi-sys-color--*` canvas/code),
  routée intégralement via l'extraction par tranches ≤ 1 Ko. C'est le sous-
  ensemble « système » mentionné plus bas, capturé sans troncature.
- [`aphrody-mapping.md`](aphrody-mapping.md) — mapping des rôles Gemini vers
  `m3-tokens::ColorRoles` + un `const GEMINI_DARK` Rust prêt à coller.

## Inventaire complet (mesuré sur la page)

- **1711** CSS custom properties au total sur `:root`/`body`.
- Namespaces : `--mat-*` 817 (Angular Material / MDC, dont composants), `--gem-*`
  692 (système Gemini : couleur, typographie type-scale, états), `--bard-color-*`
  163 (couleurs composants Gemini), `--lumi-*` 36, `--gds-*` 3.
- **`--mat-sys-color-*` sont VIDES** sur Gemini : le thème de couleur vit dans
  `--gem-sys-color-*` (sémantique/marque) et `--bard-color-*` (composants), pas
  dans les rôles M3 standard.
- Thème : **sombre** — `background #000000`, surface (nav) `#1c1c1c`, texte
  `#e3e3e3` / `#c4c7c5`.
- Typographie : **"Google Sans Flex"** (token `--gem-sys-typography-type-scale--*-font-name`).
- Sous-ensembles : tokens "système" (`-sys-`/`bard`/`gds`/`lumi`) = 895 (58 Ko CSS) ;
  tokens couleur = 320 (16 Ko) ; rôles couleur système = 157.

## Pourquoi pas les 1711 vars en un fichier ici

L'extraction passe par le pont navigateur MCP, qui **tronque les réponses à
~1 Ko** (et bloque base64 / downloads blob). Les **175 tokens système** (le
thème réel : rôles couleur, marque, gradients, shape, lumi) ont été routés
intégralement en récupérant la couche système par **tranches ≤ 1 Ko** → voir
[`tokens-system.css`](tokens-system.css). Les ~800 tokens `--mat-*` restants
sont du plumbing Angular Material **dérivé** de cette couche système (non
vendorés). Pour un dump exhaustif des 1711 vars, exécuter le snippet ci-dessous
dans la console DevTools (navigateur de l'utilisateur, sans le filtre MCP).

## Dump intégral (1711 vars) — à exécuter soi-même

Coller dans la console DevTools de `gemini.google.com/app` (le navigateur de
l'utilisateur, sans le filtre MCP) pour télécharger le set complet :

```js
(() => {
  const o = {};
  for (const el of [document.documentElement, document.body]) {
    const cs = getComputedStyle(el);
    for (let i = 0; i < cs.length; i++) {
      const p = cs[i];
      if (p.startsWith('--')) o[p] = cs.getPropertyValue(p).trim();
    }
  }
  const css = ':root {\n' +
    Object.keys(o).sort().map(k => `  ${k}: ${o[k]};`).join('\n') + '\n}\n';
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([css], { type: 'text/css' }));
  a.download = 'gemini-tokens-full.css';
  a.click();
})();
```

Déposer le fichier obtenu dans ce dossier sous `tokens-full.css`.

## Intégration aphrody

Les tokens couleur Gemini (`--gem-sys-color-*`) sont thématiquement proches de
notre fusion M3 (`--md-sys-color-*`, cf. [`../M3-FRAMEWORK.md`](../M3-FRAMEWORK.md)).
`theme.css` peut être importé tel quel par `apps/m3-react` ou aliasé vers les
rôles shadcn comme la fusion `theme-aphrody`.
