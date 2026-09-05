# 03 — Intégration des web components Lit (`md-*`) dans React

Ce document est le **socle technique** des wrappers `@aphrody/m3-react` : il explique _comment_ et _pourquoi_ on enrobe les éléments `<md-*>` (web components Lit, `@material/web@2.4.1`, fork aphrody) pour les consommer depuis React et Next.js. À lire **avant** de regarder les wrappers générés dans `migration/wrappers/`. Tout y est ancré dans le code réel du fork (`material-web/…`) et sourcé. Convention de nommage, mapping et garde-fous : voir `migration/00-CONVENTIONS.md` (§2, §4, §7) — ce doc ne les contredit jamais.

> Rappels du contrat (`00-CONVENTIONS.md`) : **bun uniquement** (`bun add @lit/react`, jamais npm/pnpm) ; `@lit/react` **n'est pas installé** ; ne jamais inventer un élément/prop/slot `md-*` — vérifier dans `material-web/`. Les utilitaires Tailwind **ne franchissent pas** le Shadow DOM (voir `06-tailwind-material-web.md`).

---

## 1. Le problème : pourquoi React ne « parle » pas nativement aux web components (≤18)

Un élément `<md-*>` est une classe Lit qui expose des **reactive properties** (propriétés JS) et émet des **events DOM natifs**. Exemples vérifiés dans le fork :

| Élément                                           | Propriétés (reactive)                                                                              | Events émis (`@fires`)                                       | Source                                                              |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------- |
| `md-checkbox`                                     | `checked: boolean`, `indeterminate: boolean`, `value: string`, `required`                          | `input` (`InputEvent`), `change` (`Event`)                   | `material-web/checkbox/internal/checkbox.ts:45-84`                  |
| `md-switch`                                       | `selected: boolean`, `value: string`, `required`                                                   | `input`, `change` (sur changement de `selected`)             | `material-web/switch/internal/switch.ts:45-87`                      |
| `md-radio`                                        | `checked: boolean`, `value: string`                                                                | `input`, `change`                                            | `material-web/radio/internal/radio.ts:45-84`                        |
| `md-filled-text-field` / `md-outlined-text-field` | `value: string`, `error`, `errorText`, …                                                           | `input` (`InputEvent`), `change`, `select`                   | `material-web/textfield/internal/text-field.ts:86-152`              |
| `md-filled-select` / `md-outlined-select`         | `value: string` (getter/setter), `selectedIndex: number`                                           | `input`, `change`, `opening`, `opened`, `closing`, `closed`  | `material-web/select/internal/select.ts:62-200`                     |
| `md-slider`                                       | `value?: number`, `valueStart?`, `valueEnd?` (range)                                               | `input` (`InputEvent`), `change`                             | `material-web/slider/internal/slider.ts:43-80`                      |
| `md-tabs`                                         | `activeTabIndex: number`, `activeTab`, `autoActivate`                                              | `change` (`Event`, `bubbles`)                                | `material-web/tabs/internal/tabs.ts:15-104`                         |
| `md-dialog`                                       | `open: boolean`, `returnValue: string`, `type?: 'alert'` + méthodes `show()`/`close(returnValue?)` | `open`, `opened`, `close`, `closed`, `cancel` (tous `Event`) | `material-web/dialog/internal/dialog.ts:30-34, 45-103`              |
| `md-menu`                                         | `open: boolean` (reflète), `anchor`, `positioning`, `quick`, … + méthodes `show()`/`close()`       | `opening`, `opened`, `closing`, `closed`                     | `material-web/menu/internal/menu.ts:83-248`                         |
| `md-menu-item`                                    | `selected`, `type`, …                                                                              | `close-menu` (`CustomEvent<{initiator, reason, itemPath}>`)  | `material-web/menu/internal/menuitem/menu-item.ts:33`               |
| `md-filter-chip` / `md-input-chip`                | `selected`, `removable`, …                                                                         | `remove` (`Event`, cancelable)                               | `material-web/chips/internal/filter-chip.ts:21`, `input-chip.ts:18` |

Deux frictions historiques côté React **≤ 18** (renderer DOM par attributs) :

1. **Propriétés vs attributs.** Le JSX de React 18 sérialise toute prop _connue comme attribut_ en `setAttribute(name, String(value))`. Conséquences : une valeur non-string (`number`, `object`, `array`, `boolean false`) est mal transmise, et une propriété qui n'a **pas** de reflected attribute (ex. `md-slider.value` est `@property({type:Number})` sans `reflect`, ou `md-select.value` qui est un **getter/setter** pur, `select.ts:171-185`) n'est tout simplement **jamais** assignée.
2. **Events custom.** React 18 n'a pas de syntaxe `onClose`/`onclose` qui s'abonne à `addEventListener('close', …)`. Les events natifs DOM (`change`, `input`) et surtout les events custom (`close-menu`, `closed`, `remove`) sont invisibles depuis le JSX — il faut un `ref` + `addEventListener` manuel + cleanup.

`@lit/react` `createComponent` résout les deux d'un coup, de manière typée. C'est pour ça que les wrappers `@aphrody/m3-react` reposent dessus (cf. `00-CONVENTIONS.md` §2).

---

## 2. `@lit/react` `createComponent` — l'API exacte

Installation (bun) :

```bash
bun add @lit/react
```

Signature des options de `createComponent` :

| Option         | Type                                     | Rôle                                                                                                                    |
| -------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `react`        | `typeof React`                           | L'objet React importé (ou `preact-compat`). Permet plusieurs runtimes.                                                  |
| `tagName`      | `string`                                 | Le tag de l'élément, ex. `'md-filled-button'`.                                                                          |
| `elementClass` | `typeof HTMLElement`                     | La **classe** importée (ex. `MdFilledButton`), utilisée pour le typage des props et pour distinguer propriété/attribut. |
| `events`       | `Record<string, string \| EventName<E>>` | Map **`{ nomPropReact: 'nom-event-natif' }`**. Crée un `addEventListener` géré (avec cleanup) sur l'élément.            |
| `displayName`  | `string?`                                | Nom React DevTools (optionnel).                                                                                         |

`createComponent` retourne un `ReactWebComponent` : un composant React qui (a) **assigne chaque prop comme propriété JS** sur l'instance de l'élément (pas comme attribut), et (b) **abonne** chaque clé de `events` au bon event DOM, avec gestion du cycle de vie (add/remove au mount/unmount, mise à jour si le handler change). `ref` est forwardé vers l'instance de l'élément custom.

### 2.1 Wrapper minimal — bouton (pas d'event)

```ts
// migration/wrappers/MdFilledButton.ts
import * as React from "react";
import { createComponent } from "@lit/react";
import { MdFilledButton as MdFilledButtonElement } from "@material/web/button/filled-button.js";
// ↑ l'import enregistre aussi l'élément custom (effet de bord). Voir §7.

export const MdFilledButton = createComponent({
  react: React,
  tagName: "md-filled-button",
  elementClass: MdFilledButtonElement,
});
```

### 2.2 Wrapper avec events — checkbox / text field

Convention `events` : **clé = nom de prop React, valeur = nom de l'event natif**. La recommandation lit.dev est de **préfixer la clé par `on`** (`onChange`, `onInput`) — ça aligne sur la future convention React custom-elements et sur l'ergonomie MUI.

```ts
// migration/wrappers/MdCheckbox.ts
import * as React from "react";
import { createComponent } from "@lit/react";
import { MdCheckbox as MdCheckboxElement } from "@material/web/checkbox/checkbox.js";

export const MdCheckbox = createComponent({
  react: React,
  tagName: "md-checkbox",
  elementClass: MdCheckboxElement,
  events: {
    onInput: "input", // → addEventListener('input', handler)
    onChange: "change", // → addEventListener('change', handler)
  },
});
```

Usage React :

```tsx
<MdCheckbox
  checked={agreed} // assigné comme PROPRIÉTÉ (boolean), pas attribut
  onChange={(e) => setAgreed((e.target as HTMLInputElement).checked)}
/>
```

> Note `e.target` : l'event `change`/`input` est **re-dispatché** depuis le `<input>` interne via `redispatchEvent` (`checkbox.ts:177`), donc `e.target` est l'élément `md-checkbox` lui-même. On lit la valeur sur l'élément md (`e.target.checked`), **pas** via un second argument comme MUI. Voir §4.

### 2.3 Typage TypeScript des events custom — `EventName`

Pour un event qui porte un payload (ex. `close-menu` de `md-menu-item` est un `CustomEvent<{initiator, reason, itemPath}>`, cf. `menu/internal/controllers/shared.ts:119-160`), on type la valeur de la map avec `EventName<E>` pour que le handler React soit typé :

```ts
import * as React from "react";
import { createComponent, type EventName } from "@lit/react";
import { MdMenu as MdMenuElement } from "@material/web/menu/menu.js";
import type { CloseMenuEvent } from "@material/web/menu/internal/controllers/shared.js";

export const MdMenu = createComponent({
  react: React,
  tagName: "md-menu",
  elementClass: MdMenuElement,
  events: {
    onOpening: "opening",
    onOpened: "opened",
    onClosing: "closing",
    onClosed: "closed",
    onCloseMenu: "close-menu" as EventName<CloseMenuEvent>,
  },
});
```

`EventName<E>` est purement compile-time (cast d'un `string`) : il informe TS que `onCloseMenu` reçoit un `CloseMenuEvent`. Sans lui, le handler recevrait un `Event` générique.

> **Pourquoi `createComponent` plutôt que des refs manuelles** (lit.dev, « Why wrappers matter ») : React (≤18) ne sait pas passer de données complexes ni s'abonner aux events custom ; le wrapper assigne les propriétés et gère les listeners à votre place, supprimant le boilerplate `ref` + `addEventListener` + cleanup. Bonus : un composant React **typé** (props dérivées de `elementClass`, events typés via `EventName`), nommé dans DevTools.

---

## 3. React 19 a changé la donne — ce qui reste nécessaire

React 19 (stable, déc. 2024) ajoute le **support natif des custom elements** : React passe désormais [tous les tests de Custom Elements Everywhere](https://custom-elements-everywhere.com/). Stratégie officielle (react.dev, _React 19_ / _New features_) :

- **Client (CSR)** : une prop qui **correspond à une propriété de l'instance** du custom element est **assignée comme propriété** ; sinon elle est posée en attribut. → `checked`/`value`/`open` (numbers, booleans, objects) passent enfin correctement.
- **Server (SSR)** : les props de type **primitif** (`string`, `number`, ou la valeur `true`) sont rendues comme **attributs** ; les types **non-primitifs** (`object`, `symbol`, `function`) et la valeur `false` sont **omis**.

Côté events, React 19 introduit la prise en charge des handlers `on…` pour les events custom (la doc `@lit/react` note explicitement que sa convention `on`+event « matches how React is planning to implement event support for custom elements »).

### Faut-il encore des wrappers en React 19 ?

**Oui, mais pour d'autres raisons que ≤18.** Le wrapper n'est plus _indispensable_ à la transmission des props/events, mais il reste fortement recommandé. Bilan factuel :

| Besoin                                                         | React ≤18 (sans wrapper)                           | React 19 (sans wrapper)                             | Wrapper `@lit/react`                                                    |
| -------------------------------------------------------------- | -------------------------------------------------- | --------------------------------------------------- | ----------------------------------------------------------------------- |
| Passer une prop **non-string** (`number`, `boolean`, `object`) | KO (sérialisée en attribut string)                 | OK si la propriété existe sur l'instance (client)   | OK (toujours assignée en propriété)                                     |
| S'abonner aux events DOM/custom                                | KO (ref + addEventListener manuel)                 | Partiel (`on…` natif, conventions encore mouvantes) | OK, **typé** via `EventName`                                            |
| **Typage TS** des props/events JSX                             | Aucun (`JSX.IntrinsicElements` à écrire à la main) | Aucun par défaut (idem)                             | **Props dérivées de `elementClass`, events typés**                      |
| **SSR** : propriété non-string                                 | Omise/incorrecte                                   | **Omise** (object/false non rendus)                 | Idem — voir §6 (le souci SSR n'est pas le wrapper, c'est le shadow DOM) |
| Ergonomie `onChange` camelCase, nommage stable                 | —                                                  | events en **lowercase** côté natif (`onclose`)      | `onClose` (camelCase, mappé)                                            |
| API stable d'équipe (DX, mocking, codemods)                    | —                                                  | —                                                   | **Point d'API unique** ciblé par les codemods MUI→md                    |

**Conclusion pour `@aphrody/m3-react`** : on garde `createComponent` parce qu'il apporte le **typage** (props + events), une **API camelCase stable** alignée MUI que les codemods peuvent cibler mécaniquement, et l'indépendance vis-à-vis de la version de React (le wrapper marche en 17/18/19). React 19 ne supprime _aucune_ de ces valeurs ; il rend simplement le wrapper non bloquant pour les apps qui voudraient s'en passer.

Sources : [react.dev — React 19](https://react.dev/blog/2024/12/05/react-19), [lit.dev — React](https://lit.dev/docs/frameworks/react/), [custom-elements-everywhere.com](https://custom-elements-everywhere.com/).

---

## 4. Events : MUI synthétique vs events natifs des éléments md

C'est **la** rupture sémantique de la migration (cf. `00-CONVENTIONS.md` §4). Les codemods doivent transformer la signature des handlers.

### 4.1 Différence de signature

|                | MUI (`onChange`)                                                                                      | md (`change`/`input` natifs)                                                     |
| -------------- | ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Système        | `SyntheticEvent` React                                                                                | `Event` / `InputEvent` DOM réels                                                 |
| Signature      | `(event, value)` — **2e arg** = valeur (ex. `Checkbox`, `Switch`, `Select`, `Slider`, `Autocomplete`) | `(event)` — valeur lue sur `event.target`                                        |
| Lecture valeur | `value` (2e arg)                                                                                      | `(e.target as MdTextField).value` / `.checked` / `.selected` / `.activeTabIndex` |

Exemple de transformation (TextField) :

```tsx
// AVANT (MUI)
<TextField value={name} onChange={(e, v) => setName(e.target.value)} />

// APRÈS (md, via wrapper)
<MdOutlinedTextField
  value={name}
  onInput={(e) => setName((e.target as HTMLInputElement & {value: string}).value)}
/>
```

> **`input` vs `change`** : comme sur un `<input>` natif, `md-text-field` émet `input` **à chaque frappe** et `change` **au blur/validation** (`text-field.ts:86-92`). Pour un champ _contrôlé_ en temps réel → écouter `onInput`. Pour ne réagir qu'à la fin de saisie → `onChange`. Même logique sur `md-slider` : `input` pendant le drag, `change` au relâchement (`slider.ts:43-46`).

### 4.2 Controlled vs uncontrolled

Les éléments md sont nativement **uncontrolled** : ils maintiennent leur propre état interne (`checked`, `value`, `activeTabIndex`) et émettent un event. Deux patterns :

**Uncontrolled (le plus simple, le plus proche du natif)** — on laisse l'élément gérer son état, on lit la valeur via `ref` ou à la soumission :

```tsx
const ref = React.useRef<HTMLElement & { value: string }>(null);
<MdOutlinedTextField ref={ref} defaultValue="initial" />;
// lecture: ref.current?.value
```

**Controlled (parité MUI)** — on re-pousse la prop à chaque event. Attention : pour les inputs textuels, re-rendre `value` à chaque frappe peut perturber le curseur si le re-render est asynchrone ; en pratique md re-synchronise proprement, mais préférez `onInput` + state local et évitez de transformer la valeur entre l'event et le `setState` :

```tsx
const [city, setCity] = React.useState("");
<MdOutlinedTextField
  value={city}
  onInput={(e) => setCity((e.currentTarget as HTMLInputElement).value)}
/>;
```

### 4.3 Debounce

Le debounce s'applique au **handler React**, pas à l'élément (qui émet à chaque frappe). Pour de la recherche live :

```tsx
const debounced = React.useMemo(() => debounce((v: string) => fetchResults(v), 300), []);
<MdOutlinedTextField onInput={(e) => debounced((e.target as HTMLInputElement).value)} />;
```

Affichez la valeur en **uncontrolled** (laissez md gérer le champ) pour que le debounce ne fasse jamais sauter le curseur.

### 4.4 Formulaires — form-associated custom elements

Les éléments interactifs du fork sont **form-associated** : ils implémentent `ElementInternals` via les behaviors `mixinFormAssociated` / `mixinConstraintValidation` (`material-web/labs/behaviors/form-associated.ts`, `constraint-validation.ts:85-250`, `form-submitter.ts:69`). Concrètement, à l'intérieur d'un `<form>` :

- ils participent à la soumission avec leur attribut **`name`** et leur **`value`** (ex. `md-checkbox.value` défaut `'on'`, `checkbox.ts:84` ; `md-switch.value` `'on'`, `switch.ts:87`) ;
- ils exposent l'API de **contrainte** : `checkValidity()`, `reportValidity()`, `setCustomValidity()`, et participent à `form.reportValidity()` ;
- ils répondent au **reset** du formulaire (`formResetCallback`).

```tsx
function ProfileForm() {
  const formRef = React.useRef<HTMLFormElement>(null);

  const onSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Déclenche la validation native md (affiche errorText, focus le 1er invalide)
    if (!formRef.current!.reportValidity()) return;
    const data = new FormData(formRef.current!); // collecte name/value des md-*
    save(Object.fromEntries(data));
  };

  return (
    <form ref={formRef} onSubmit={onSubmit} noValidate>
      <MdOutlinedTextField name="email" label="Email" type="email" required />
      <MdCheckbox name="optin" value="yes" />
      <MdFilledButton type="submit">Enregistrer</MdFilledButton>
    </form>
  );
}
```

> `reportValidity()` est aussi appelable **par élément** via un `ref` (ex. `ref.current.reportValidity()`), utile pour valider un champ au blur. Le texte d'erreur s'affiche via `errorText`/`error` (`text-field.ts`, `select.ts:100-125`).

---

## 5. Refs & API impérative

Plusieurs éléments md exposent des **méthodes** (pas seulement des props). On y accède via `useRef` typé sur la classe importée — `createComponent` forwarde le `ref` vers l'instance du custom element.

| Élément           | Méthode / prop impérative                                     | Source                                 |
| ----------------- | ------------------------------------------------------------- | -------------------------------------- |
| `md-dialog`       | `show()`, `close(returnValue?)`, prop `open`                  | `dialog/internal/dialog.ts:58-61, 103` |
| `md-menu`         | `show()`, `close()`, prop `open` (reflète)                    | `menu/internal/menu.ts:157, 614-688`   |
| `md-*-text-field` | `select()`, `setSelectionRange()`, `reportValidity()`         | `textfield/internal/text-field.ts`     |
| `md-*-select`     | `reportValidity()`, getter `options`, `value`/`selectedIndex` | `select/internal/select.ts`            |

```tsx
import { MdDialog as MdDialogElement } from "@material/web/dialog/dialog.js";

function ConfirmButton() {
  const dialogRef = React.useRef<MdDialogElement>(null);

  return (
    <>
      <MdFilledButton onClick={() => dialogRef.current?.show()}>Supprimer</MdFilledButton>
      <MdDialog
        ref={dialogRef}
        onClosed={(e) => {
          // returnValue renseigné par le bouton avec value="confirm" dans slot="actions"
          if ((e.target as MdDialogElement).returnValue === "confirm") doDelete();
        }}
      >
        <div slot="headline">Confirmer la suppression ?</div>
        <form slot="content" id="confirm-form" method="dialog">
          Action irréversible.
        </form>
        <div slot="actions">
          <MdTextButton value="cancel" form="confirm-form">
            Annuler
          </MdTextButton>
          <MdFilledButton value="confirm" form="confirm-form">
            Supprimer
          </MdFilledButton>
        </div>
      </MdDialog>
    </>
  );
}
```

Deux façons de piloter `md-dialog`/`md-menu` :

- **Déclaratif** : prop `open` (le setter de `dialog.open` appelle `show()`/`close()`, `dialog.ts:51-61`). Idéal contrôlé par du state React.
- **Impératif** : `ref.current.show()` / `.close()`. Idéal pour des ouvertures ponctuelles sans state.

> `md-menu` doit être ancré : prop `anchor` (id de l'anchor) ou `positioning`, et un élément ancre dans le même contexte (`menu/internal/menu.ts:97-138`). En React, l'anchor par id fonctionne ; pour un anchor par référence, utiliser un `ref` et l'API impérative.

---

## 6. SSR / Next.js

**Règle d'or : les web components ne se rendent pas sur le serveur en React.** React n'instancie pas les custom elements côté serveur — il émet juste le tag avec ses attributs primitifs (cf. §3, stratégie SSR). Le **shadow DOM** (et donc l'UI réelle des `md-*`) n'apparaît qu'après hydratation côté client, quand le navigateur **upgrade** l'élément (une fois sa définition `customElements.define` exécutée).

Conséquences pratiques en Next.js (App Router) :

1. **`'use client'` obligatoire** sur tout fichier qui rend un wrapper md ou en importe la définition. Les définitions custom elements touchent `window`/`document`/`customElements` → elles ne peuvent **pas** s'exécuter dans un Server Component.

   ```tsx
   "use client";
   import { MdFilledButton } from "@aphrody/m3-react";
   ```

2. **Import des définitions côté client uniquement.** Centraliser l'enregistrement dans un composant client monté haut dans l'arbre, ou laisser chaque wrapper importer sa propre définition (les wrappers sont déjà `'use client'`). Ne jamais importer `@material/web/...` depuis un module serveur.

3. **Hydration mismatch / `isServer`.** Les éléments md sont écrits pour ne pas exploser côté serveur : le setter `md-select.value` fait `if (isServer) return;` (`select.ts:177`). Pour le SSR de la valeur sélectionnée, la doc md recommande de poser `selected` + `displayText` sur l'option plutôt que `value` (`select/internal/select.ts:171-200`). Ne posez pas de props non-primitives qui divergeraient entre serveur (omises) et client.

4. **FOUC (Flash of Unstyled Content)** : entre le HTML serveur et l'upgrade des éléments, les `md-*` non encore définis sont du contenu brut non stylé. Parade standard — masquer les éléments tant qu'ils ne sont pas `:defined` :

   ```css
   /* global, hors shadow DOM */
   md-filled-button:not(:defined),
   md-checkbox:not(:defined),
   md-outlined-text-field:not(:defined) {
     visibility: hidden;
   }
   /* ou, générique */
   :not(:defined) {
     opacity: 0;
   }
   ```

   Alternative ciblée : réserver la place (hauteur/largeur via Tailwind sur le host) pour éviter le layout shift, et révéler à l'upgrade.

5. **`lazy` / `dynamic`** : pour exclure totalement un sous-arbre md du rendu serveur, l'importer via `next/dynamic` avec `ssr: false` :

   ```tsx
   const Editor = dynamic(() => import("./Editor"), { ssr: false });
   ```

> En résumé : le wrapper `@lit/react` n'apporte **pas** de SSR du shadow DOM (Lit SSR / declarative shadow DOM n'est pas branché ici). On vit avec un rendu client-only pour le contenu interne md, et on gère le FOUC avec `:not(:defined)`.

---

## 7. Chargement des définitions

L'enregistrement d'un élément se fait par **effet de bord d'import** : importer `@material/web/button/filled-button.js` exécute le `@customElement('md-filled-button')` qui appelle `customElements.define(...)` (cf. `dialog/dialog.ts`, `menu/menu.ts:78`).

Trois stratégies :

```ts
// (a) par composant — chaque wrapper importe SA définition (recommandé, code-splittable)
import { MdFilledButton as El } from "@material/web/button/filled-button.js";

// (b) en masse — tout le fork (gros bundle, à éviter en prod)
import "@material/web/all.js"; // re-exporte aphrody-components + aphrody-labs + upstream

// (c) groupé fork — composants du fork uniquement
import "@material/web/aphrody-components.js";
```

- **Pour `@aphrody/m3-react`** : stratégie (a). Chaque wrapper importe l'unique définition dont il a besoin → tree-shaking et code-splitting naturels (n'embarque que les éléments réellement utilisés). Les composants du **fork** s'importent depuis `aphrody-components.ts` / `aphrody-labs.ts` / `all.ts` (cf. `00-CONVENTIONS.md` §0, §2).
- **`customElements.whenDefined`** : pour attendre qu'un élément soit prêt avant une action impérative (utile si on pilote `.show()` juste après le mount), ou pour gérer le révélateur anti-FOUC manuellement :

  ```ts
  await customElements.whenDefined("md-dialog");
  dialogRef.current?.show();
  ```

---

## 8. TypeScript — typage des tags et des events

### 8.1 `HTMLElementTagNameMap`

Chaque élément md augmente déjà `HTMLElementTagNameMap` dans son fichier public (ex. `dialog/dialog.ts:13-16` déclare `'md-dialog': MdDialog`, `menu/menu.ts:24-25`). Donc `document.querySelector('md-dialog')` est **typé** dès qu'on a importé l'élément, et un `useRef<MdDialogElement>` se type sans effort en important la classe.

### 8.2 `JSX.IntrinsicElements` — seulement si on écrit le tag à la main

Si vous écrivez `<md-filled-button>` **directement** en JSX (sans wrapper), TS ne le connaît pas. Il faut augmenter `JSX.IntrinsicElements`. **Avec les wrappers `@aphrody/m3-react`, ce n'est pas nécessaire** : on écrit `<MdFilledButton>` (un composant React typé). À ne faire que pour l'usage tag brut :

```ts
// global.d.ts — UNIQUEMENT si vous utilisez les tags md-* nus dans le JSX
import type { MdFilledButton } from "@material/web/button/filled-button.js";

declare global {
  namespace JSX {
    interface IntrinsicElements {
      "md-filled-button": React.DetailedHTMLProps<
        React.HTMLAttributes<MdFilledButton> & Partial<MdFilledButton>,
        MdFilledButton
      >;
    }
  }
}
```

> React 19 a déplacé `JSX` dans le namespace `React.JSX` ; selon votre version de `@types/react`, augmentez `React.JSX.IntrinsicElements`. Les wrappers évitent entièrement ce piège.

### 8.3 Typage des events dans les handlers

Avec `createComponent` + `EventName<E>` (§2.3), le handler reçoit un event typé. Pour lire la valeur, caster `e.target` vers la classe md :

```ts
import type {MdOutlinedTextField as El} from '@material/web/textfield/outlined-text-field.js';
onInput={(e) => setValue((e.target as El).value)}
```

---

## 9. Pièges (checklist)

| Piège                                 | Détail                                                                                                                                                                    | Parade                                                                                                                                                                           |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `className` vs `class`                | **React 19** accepte `className` ET `class` sur les custom elements (`class` passe en attribut). **≤18** : seul `className`.                                              | Garder `className` partout (les wrappers sont des composants React). Rappel `00-CONVENTIONS.md` §6 : Tailwind ne stylise **que le host**, jamais l'intérieur.                    |
| `htmlFor` / `for`                     | React mappe `htmlFor`→`for`. Mais les `md-*` n'utilisent pas `<label for>` : le **`label`** est une **prop** de l'élément (`md-*-text-field.label`, `md-*-select.label`). | Utiliser la prop `label`, pas un `<label htmlFor>`.                                                                                                                              |
| Boolean attributes                    | En ≤18, `disabled={false}` pouvait poser un attribut vide. En 19, `false` n'est pas rendu et la propriété est assignée.                                                   | Avec wrapper : `disabled` est assigné en **propriété** boolean → toujours correct.                                                                                               |
| Valeur sur 2e argument                | Réflexe MUI `onChange(e, value)` : le 2e arg est **`undefined`** avec les events md.                                                                                      | Lire sur `e.target` (§4). Les codemods réécrivent la signature.                                                                                                                  |
| `value` getter/setter (select/slider) | `md-select.value` est un getter qui fait une **DOM query**, le setter ignore le serveur (`select.ts:171-185`).                                                            | Ne pas lire `value` en boucle serrée ; préférer `change` + state. SSR : `selected`+`displayText`.                                                                                |
| Focus management                      | Les `md-*` gèrent leur focus interne (`md-menu` restaure le focus, `default-focus`, `skip-restore-focus`, `menu.ts:231-248`).                                             | Ne pas forcer `autoFocus` React sur l'élément ; piloter via les props md (`anchor`, `default-focus`) ou `ref.current.focus()`.                                                   |
| a11y / ARIA                           | Les éléments md **gèrent l'ARIA en interne** (rôles, `aria-checked=mixed` pour l'indeterminate, `checkbox.ts:147`, etc.).                                                 | **Ne pas** ré-ajouter `role`/`aria-*` redondants côté React — risque de doublons/conflits. Fournir uniquement les labels (`aria-label` sur l'host quand pas de `label` visible). |
| Event lowercase natif sans wrapper    | En 19 sans wrapper, l'event custom s'écoute en **lowercase** (`onclose`, `onclosemenu`).                                                                                  | Wrapper = camelCase mappé (`onClosed`, `onCloseMenu`) — DX cohérente et codemod-able.                                                                                            |
| Import oublié de la définition        | Si la définition n'est jamais importée, l'élément reste un `HTMLUnknownElement` (inerte).                                                                                 | Chaque wrapper importe sa définition (§7) ; ne pas casser ce side-effect import au tree-shaking (`sideEffects` doit rester sain).                                                |
| Children string vs slots              | Les sous-composants MUI (`DialogTitle`, `CardHeader`) → **contenu slotté** (`slot="headline"`, etc.), pas des props.                                                      | Cf. `00-CONVENTIONS.md` §4 et le mapping §3.                                                                                                                                     |

---

## 10. Récapitulatif — flux de décision

```
Prop md non-string / objet ?      → wrapper assigne en PROPRIÉTÉ (toujours OK). React19 sans wrapper: OK client, omis SSR.
Event natif (input/change) ?      → events:{onInput:'input', onChange:'change'} ; lire e.target.value/.checked/.selected
Event custom (close-menu, remove) → events:{onCloseMenu:'close-menu' as EventName<CloseMenuEvent>}
Méthode impérative (.show())      → useRef<MdElement> + ref.current.show()
Formulaire                        → name + value + form.reportValidity() (form-associated md)
Next.js                           → 'use client' + import définition client-only + CSS :not(:defined)
Layout autour                     → Tailwind sur le host (ne traverse pas le shadow DOM) — voir 06-…
Theming interne                   → tokens --md-sys-* uniquement — voir 02-…
```

### Sources

- lit.dev — _React_ (`@lit/react` `createComponent`, `events`, `EventName`) : https://lit.dev/docs/frameworks/react/
- react.dev — _React 19_ (support custom elements, stratégie attribut/propriété, SSR) : https://react.dev/blog/2024/12/05/react-19
- Custom Elements Everywhere (React 19 = 100%) : https://custom-elements-everywhere.com/
- Code du fork (`material-web/`) — events & API réels :
  - `dialog/internal/dialog.ts:30-34, 45-103, 189-254` (events `open/opened/close/closed/cancel`, `show()/close()`, prop `open`)
  - `menu/internal/menu.ts:83-248, 614-688` ; `menu/internal/menuitem/menu-item.ts:33` ; `menu/internal/controllers/shared.ts:119-160` (`close-menu` / `CloseMenuEvent`)
  - `checkbox/internal/checkbox.ts:45-84, 147-177` ; `switch/internal/switch.ts:45-87` ; `radio/internal/radio.ts:45-84`
  - `textfield/internal/text-field.ts:86-152, 728-729` (`input/change/select`, `value`, `redispatchEvent`)
  - `select/internal/select.ts:62-200` (`value` getter/setter + `isServer`, `selectedIndex`, events menu)
  - `slider/internal/slider.ts:43-80` ; `tabs/internal/tabs.ts:15-206` ; `chips/internal/filter-chip.ts:21`, `input-chip.ts:18`
  - `labs/behaviors/form-associated.ts`, `constraint-validation.ts:85-250`, `form-submitter.ts:69` (form-associated, `reportValidity`)
- Contrat partagé : `migration/00-CONVENTIONS.md` (§2 nommage wrappers, §4 props/events, §6 Tailwind/shadow DOM, §7 robustesse)
