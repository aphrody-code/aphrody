# Intégration frameworks (SSR / RSC) — `@aphrody/m3-react`

Guide d'intégration des wrappers Material 3 dans un framework React à rendu
serveur — **Next.js App Router**, Remix, ou tout setup RSC / SSR / hydration.
Les composants sont des **web components Lit**, donc **client-only** : ils ne
rendent rien côté serveur (le shadow DOM n'existe qu'au navigateur) et
s'« upgradent » après l'hydration. Les exemples ci-dessous utilisent Next.js App
Router (le cas le plus contraint), mais les principes valent pour tout SSR.

L'exemple du monorepo ([`examples/showcase/`](../../examples/showcase)) est
bun-natif (pas Next) ; ce guide couvre les spécificités **Next.js App Router**
côté consommateur. Le package se consomme tel quel dans une app Next.

---

## 1. Ce que le package livre

Le package est **pré-buildé** dans `dist/` (`bun run build`, via `tsup` + `tsc`) :

- **ESM** (`dist/index.js`, `dist/transitions/index.js`) — pas de `.ts` brut, donc
  consommable depuis `node_modules` **sans `transpilePackages`**.
- Chaque chunk `.js` porte la directive **`'use client';`** en tête → Next RSC
  traite tout le package comme des **client components**, sans erreur
  « You're importing a component that needs … ».
- **Types** (`dist/**/*.d.ts`).
- `react`, `react-dom` et `@material/web` sont **externes** (peer deps / résolus
  chez le consommateur). Les imports profonds `@material/web/<dir>/<el>.js` sont
  conservés tels quels : c'est l'import du wrapper qui déclenche le
  `customElements.define()` (effet de bord) côté client.

`package.json` pointe `main`/`module`/`types`/`exports` sur `dist/`, déclare
`sideEffects` (les imports d'éléments enregistrent les custom elements — ne pas
tree-shaker) et `files: ["dist"]`.

```jsonc
"exports": {
  ".":            { "types": "./dist/index.d.ts",             "default": "./dist/index.js" },
  "./transitions":{ "types": "./dist/transitions/index.d.ts", "default": "./dist/transitions/index.js" }
}
```

---

## 2. Boundaries client / serveur

Règle : **tout ce qui touche un `Md…` vit dans un fichier `'use client'`.**

- `layout.tsx` / `page.tsx` restent des **Server Components** (pas de directive) ;
  ils importent le CSS de thème et rendent un **îlot client**.
- L'îlot client (ex. `demo.tsx`) porte `'use client'`, importe les wrappers,
  gère state/refs/handlers.

```tsx
// app/page.tsx — Server Component
import { Demo } from "./demo";
export default function Home() {
  return (
    <main>
      <Demo />
    </main>
  );
}
```

```tsx
// app/demo.tsx — Client island
"use client";
import {
  MdFilledButton,
  MdOutlinedTextField,
  MdDialog,
  MdLineChart,
  MdTabs,
  MdPrimaryTab,
} from "@aphrody/m3-react";
import { Fade } from "@aphrody/m3-react/transitions";
import { useRef, useState } from "react";

export function Demo() {
  const dialogRef = useRef<React.ComponentRef<typeof MdDialog>>(null);
  const [name, setName] = useState("");
  return (
    <>
      <MdFilledButton onClick={() => dialogRef.current?.show()}>Ouvrir</MdFilledButton>
      <MdOutlinedTextField
        label="Nom"
        value={name}
        onInput={(e) => setName((e.target as HTMLInputElement).value)}
      />
      <MdTabs>
        <MdPrimaryTab>A</MdPrimaryTab>
        <MdPrimaryTab>B</MdPrimaryTab>
      </MdTabs>
      <MdLineChart
        smooth
        showMarkers
        categories={["Jan", "Fév", "Mar"]}
        series={[{ label: "CA", data: [12, 19, 8] }]}
      />
      <MdDialog ref={dialogRef}>
        <div slot="headline">Titre</div>
        <div slot="content">Ouvert via ref impérative.</div>
        <div slot="actions">
          <MdFilledButton onClick={() => dialogRef.current?.close()}>Fermer</MdFilledButton>
        </div>
      </MdDialog>
    </>
  );
}
```

### Refs impératives (dialog, etc.)

Le `ref` d'un wrapper pointe sur **l'instance du custom element** (avec ses
méthodes `show()` / `close()` / `returnValue`…). Pour typer le ref sans importer
les sources de `@material/web`, dérivez-le du composant :

```tsx
const dialogRef = useRef<React.ComponentRef<typeof MdDialog>>(null);
dialogRef.current?.show();
```

### Propriétés objet (charts, etc.)

`@lit/react` passe les props objet/tableau comme **propriétés** de l'élément
(pas des attributs). `<MdLineChart series={[…]} categories={[…]} />` règle donc
`.series` / `.categories` directement — pas de JSON à sérialiser.

---

## 3. SSR / hydration + parade FOUC

Côté serveur, un `<md-filled-button>` se rend en élément non-`:defined` (pas de
shadow DOM). **Aucun crash SSR** : la page se pré-rend en statique (vérifié,
`next build` → route `/` en `○ Static`). Pas de hydration mismatch tant que le
markup serveur = markup client (c'est le cas, le wrapper rend juste la balise).

Pour éviter le **flash of unstyled content** (l'élément brut visible avant
l'upgrade), masquez les éléments tant qu'ils ne sont pas définis :

```css
/* app/theme.css — FOUC parade */
:not(:defined) {
  visibility: hidden;
}
```

Optionnellement, révélez après upgrade complet via `customElements.whenDefined`
dans un `useEffect` (cf. `examples/showcase`).

### Tokens de thème

Définissez les `--md-sys-*` au niveau `:root` (ou `body`) dans un CSS importé par
le layout serveur :

```css
:root {
  --md-sys-color-primary: #6750a4;
  --md-sys-color-on-primary: #fff;
  /* … générés depuis @aphrody/m3-tokens en prod … */
}
```

---

## 4. Fallback : consommer les **sources** (`transpilePackages`)

Le dist pré-buildé fonctionne sans config. Si vous préférez consommer les
**sources TS** du package (ou épingler `@material/web` sur ses sources `.ts`,
p. ex. en monorepo lié par symlink), activez `transpilePackages` pour que Next
les compile :

```ts
// next.config.ts
import type { NextConfig } from "next";
const nextConfig: NextConfig = {
  transpilePackages: ["@aphrody/m3-react", "@material/web"],
};
export default nextConfig;
```

> En monorepo, le lien workspace expose aussi les `.ts` de `@material/web` à côté
> des `.js`/`.d.ts`. Le `moduleResolution: "bundler"` de TS résout alors `…/x.js`
> vers la **source `.ts`** (décorateurs → erreur de typecheck). Deux parades :
> soit `transpilePackages` ci-dessus, soit ne typer que les artefacts publiés
> (l'app d'exemple redirige `@material/web/*` vers une copie `.js`+`.d.ts`-only
> via `compilerOptions.paths`, exactement ce qu'un `npm install` produirait).

---

## 5. Checklist build

```bash
# 1. builder la lib + le package react (turbo)
bun run build              # à la racine du monorepo → @material/web .js + dist react

# 2. dans VOTRE app Next.js : importer les wrappers (déjà marqués 'use client')
#    import { MdFilledButton } from "@aphrody/m3-react";
#    next build → vert, sans transpilePackages
```

> Le monorepo n'embarque plus d'exemple Next.js dédié : l'exemple de référence
> est `examples/showcase` (bun-natif). Ce guide reste valable pour intégrer les
> wrappers dans **votre** app Next.js App Router.
