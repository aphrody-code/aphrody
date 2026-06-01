# Gap analysis — composants MUI sans équivalent `md-*`

Ce document recense **exhaustivement** les composants/modules de `@mui/material@9.0.1` (`material-ui/packages/mui-material/src/*/`) qui **n'ont aucun équivalent** parmi les **93 éléments `md-*` réels** (`migration/scripts/md-elements.txt`) et qui ne sont **pas couverts par le mapping canonique** (§3 du contrat `00-CONVENTIONS.md`). Pour chaque gap : fonction MUI, raison de l'absence côté M3/material-web, **solution recommandée** (shim custom Lit ou React + tokens `--md-sys-*` + `md-ripple`/`md-focus-ring`/`md-elevation` ; primitive plateforme ; garder MUI ; abandonner) et niveau d'effort. Les gaps sont confirmés par vérification directe de l'absence du tag dans `md-elements.txt` (pas seulement par lecture du tableau §3).

Toutes les briques réutilisées existent réellement dans le fork :

- `md-elevation` — `material-web/elevation/elevation.ts`, `material-web/elevation/internal/elevation.ts`
- `md-ripple` — `material-web/ripple/internal/ripple.ts` (attachable via `htmlFor`/`for`)
- `md-focus-ring` — `material-web/focus/internal/focus-ring.ts` (attachable via `htmlFor`/`for`, props `visible`/`inward`)
- Tokens motion réels — `material-web/tokens/versions/v0_192/_md-sys-motion.scss` : durées `--md-sys-motion-duration-{short1..4,medium1..4,long1..4,extra-long1..4}`, easings `--md-sys-motion-easing-{standard,standard-accelerate,standard-decelerate,emphasized,emphasized-accelerate,emphasized-decelerate,legacy,linear}`.

Le **layout** (`Box`/`Container`/`Stack`/`Grid`/`Paper`) est traité dans `migration/06-tailwind-material-web.md` ; il est listé ici pour complétude avec renvoi.

---

## Tableau récapitulatif

| Composant MUI              | Gap ?        | Solution recommandée                                                                   | Effort |
| -------------------------- | ------------ | -------------------------------------------------------------------------------------- | ------ |
| `Avatar`                   | Oui          | (a) shim Lit `md-avatar` (img/initiales/icône, forme via `--md-sys-shape-corner-full`) | faible |
| `AvatarGroup`              | Oui          | (a) wrapper React + CSS overlap (`margin-left` négatif)                                | faible |
| `Alert`                    | Oui          | (a) shim Lit `md-alert` (container tonal + `md-icon` + slot action)                    | moyen  |
| `AlertTitle`               | Oui          | slot/élément texte dans `md-alert` (typescale title)                                   | faible |
| `Breadcrumbs`              | Oui          | (a) shim React/`<nav>` + `Link` tokenisé + séparateur                                  | faible |
| `Rating`                   | Oui          | (a) shim Lit `md-rating` (radiogroup d'étoiles, `md-icon` star/star_border)            | moyen  |
| `Skeleton`                 | Oui          | (a) shim CSS pur (`md-skeleton` ou classe) + keyframes motion tokens                   | faible |
| `Backdrop`                 | Oui          | (b) `<div>` scrim + token `--md-sys-color-scrim` (souvent géré par `md-dialog`)        | faible |
| `Modal`                    | Oui          | (b) `<dialog>` natif **ou** `md-dialog` selon le cas                                   | moyen  |
| `Popover`                  | Oui          | (b) **Popover API** (`popover` attr + `popovertarget`) + anchor positioning            | moyen  |
| `Popper`                   | Oui          | (b) CSS Anchor Positioning (`anchor()`) / primitive `@floating-ui`                     | moyen  |
| `Link`                     | Oui          | (a) `<a>` tokenisé (`md-link` ou classe) + `md-focus-ring`                             | faible |
| `Collapse`                 | Oui          | (b) `grid-template-rows: 0fr→1fr` + motion tokens, ou `md-expansion-panel`             | faible |
| `Fade`                     | Oui          | (b) CSS transition `opacity` + motion tokens                                           | faible |
| `Grow`                     | Oui          | (b) CSS transition `transform: scale()` + opacity + motion tokens                      | faible |
| `Slide`                    | Oui          | (b) CSS transition `translate` + motion tokens                                         | faible |
| `Zoom`                     | Oui          | (b) CSS transition `transform: scale()` + motion tokens                                | faible |
| `MobileStepper`            | Oui          | (a) shim React (dots/progress + 2 `md-text-button`)                                    | faible |
| `CssBaseline`              | Oui          | (a) feuille de reset + injection tokens `--md-sys-*` (global)                          | faible |
| `ScopedCssBaseline`        | Oui          | (a) classe `.md-baseline` scopée (reset + tokens locaux)                               | faible |
| `GlobalStyles`             | Oui          | (b) `<style>` injecté / fichier CSS global (pas un composant M3)                       | faible |
| `InitColorSchemeScript`    | Oui          | (a) script inline `color-scheme` + classe `dark` (équiv. `data-md-theme`)              | faible |
| `Box`                      | Oui (layout) | renvoi `06-tailwind-material-web.md` — `<div>` + utilitaires Tailwind                  | faible |
| `Container`                | Oui (layout) | renvoi `06` — `<div>` + `max-width`/`mx-auto` Tailwind                                 | faible |
| `Stack`                    | Oui (layout) | renvoi `06` — `<div class="flex gap-*">`                                               | faible |
| `Grid` / `PigmentGrid`     | Oui (layout) | renvoi `06` — `<div class="grid …">`                                                   | faible |
| `Paper`                    | Oui (layout) | renvoi `06` — `<div>` surface + `md-elevation`                                         | faible |
| `ClickAwayListener`        | Oui          | (a) hook React `useClickAway` (déjà fourni nativement par dialog/menu md)              | faible |
| `Portal`                   | Oui          | (b) `createPortal` React (primitive), ou natif pour les overlays md                    | faible |
| `NoSsr`                    | Oui          | (a) garde React `useIsClient()` (pattern, pas un composant M3)                         | faible |
| `TextareaAutosize`         | Oui          | (a) hook/comportement auto-resize + `md-filled-text-field type="textarea"`             | faible |
| `ImageListItemBar`         | Oui          | (a) overlay slotté dans `md-grid-tile` (label/icon)                                    | faible |
| `SpeedDialIcon`            | Oui          | couvert indirectement (`md-fab-menu` gère l'icône morph) — sinon icône swap            | faible |
| `ButtonGroup`              | Oui          | (a) wrapper layout autour de `md-*-button` (ou `md-button-group` existe)               | faible |
| `Unstable_TrapFocus`       | Oui          | (b) focus trap (`inert` + sentinelles) — fourni par `md-dialog`/`<dialog>`             | moyen  |
| `useMediaQuery`            | Oui          | (b) `window.matchMedia` + hook React                                                   | faible |
| `useScrollTrigger`         | Oui          | (a) hook React `IntersectionObserver`/scroll listener                                  | faible |
| `usePagination`            | Oui          | (a) hook React de calcul d'items (ou logique de `md-paginator`)                        | faible |
| `useLazyRipple`            | Non\*        | remplacé par `md-ripple` (attachable) — pas de shim                                    | nul    |
| `PaginationItem`           | Partiel      | sous-partie de `Pagination`→`md-paginator` ; sinon bouton tokenisé                     | faible |
| `darkScrollbar` (util)     | Oui          | (a) snippet CSS `scrollbar-color` tokenisé                                             | faible |
| `TabScrollButton`          | Partiel      | interne à `md-tabs` (scroll géré nativement) — pas de shim                             | nul    |
| `StepConnector`/`StepIcon` | Partiel      | internes à `md-stepper`/`md-step` (fork) — slots                                       | faible |

\* Non-gaps : modules MUI couverts par le mapping §3 ou purement internes (`InputBase`, `OutlinedInput`, `FilledInput`, `Input`, `InputAdornment`, `InputLabel`, `FormControl`, `FormGroup`, `FormLabel`, `FormHelperText`, `FormControlLabel`, `ListItemAvatar`, `ListItemButton`, `ListItemIcon`, `ListItemText`, `ListItemSecondaryAction`, `ListSubheader`, `CardActionArea`, `Card*`, `Dialog*`, `Accordion*`, `Table*`, `Step*` partiels, `Toolbar`, `SnackbarContent`, `BottomNavigationAction`, `SpeedDialAction`, `DefaultPropsProvider`, `OverridableComponent`, `styles`/`colors`/`transitions`/`utils`/`internal`/`locale`/`version`/`zero-styled`/`themeCssVarsAugmentation`/`generateUtilityClass(es)`/`className`/`types` = infra de build/theming, hors composants UI).

---

## Sections détaillées

### Avatar / AvatarGroup

**Fonction MUI** (`material-ui/packages/mui-material/src/Avatar/Avatar.js`) : conteneur rond/carré affichant une image, des initiales (fallback `children`) ou une icône. `AvatarGroup` empile plusieurs avatars avec chevauchement et un `+N` de débordement.
**Pourquoi pas d'équivalent** : l'avatar **n'est pas une primitive M3** livrée par material-web (absent de `md-elements.txt`). C'est un pattern de composition (forme + image/texte + couleur de surface), pas un composant spec.
**Solution recommandée — (a) shim Lit minimal `md-avatar`** : surface circulaire tokenisée, slot image / initiales / `md-icon`.

```ts
// migration/wrappers/shims/md-avatar.ts
import { LitElement, css, html } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("md-avatar")
export class MdAvatar extends LitElement {
  @property() src = "";
  @property() alt = "";
  @property({ type: Number }) size = 40;
  static styles = css`
    :host {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      inline-size: var(--_size, 40px);
      block-size: var(--_size, 40px);
      border-radius: var(--md-sys-shape-corner-full, 9999px);
      overflow: hidden;
      background: var(--md-sys-color-primary-container, #e8def8);
      color: var(--md-sys-color-on-primary-container, #1d192b);
      font: var(--md-sys-typescale-title-medium, 500 16px/24px system-ui);
      user-select: none;
    }
    img {
      inline-size: 100%;
      block-size: 100%;
      object-fit: cover;
    }
  `;
  render() {
    this.style.setProperty("--_size", `${this.size}px`);
    return this.src ? html`<img src=${this.src} alt=${this.alt} />` : html`<slot></slot>`;
  }
}
```

`AvatarGroup` = wrapper React pur (pas de web component) qui pose un `display:flex` et applique `margin-inline-start:-8px` + `border` aux enfants, gère `max` et le badge `+N`. **Effort : faible.**

---

### Alert / AlertTitle

**Fonction MUI** (`material-ui/packages/mui-material/src/Alert/Alert.js`) : bannière de feedback (`severity` = success/info/warning/error), variantes `standard`/`filled`/`outlined`, icône de tête, action(s), bouton de fermeture. `AlertTitle` = titre en gras dans l'alerte.
**Pourquoi pas d'équivalent** : **Material 3 ne définit pas de composant « Alert »** ; le feedback transitoire passe par `md-snackbar` (fork) et le feedback persistant par des surfaces banner non standardisées. Absent de `md-elements.txt`.
**Solution recommandée — (a) shim Lit `md-alert`** : surface tonale dérivée de la sévérité via tokens couleur, `md-icon` de tête, slots `title`/contenu/`action`.

```ts
// migration/wrappers/shims/md-alert.ts
import { LitElement, css, html, nothing } from "lit";
import { customElement, property } from "lit/decorators.js";
import "@material/web/icon/icon.js";

type Severity = "success" | "info" | "warning" | "error";
const ICONS: Record<Severity, string> = {
  success: "check_circle",
  info: "info",
  warning: "warning",
  error: "error",
};

@customElement("md-alert")
export class MdAlert extends LitElement {
  @property() severity: Severity = "info";
  @property({ type: Boolean }) closable = false;
  static styles = css`
    :host {
      display: flex;
      gap: 12px;
      align-items: flex-start;
      padding: 12px 16px;
      border-radius: var(--md-sys-shape-corner-medium, 12px);
      /* couleurs dérivées de --_bg/--_fg pilotés par la sévérité */
      background: var(--_bg);
      color: var(--_fg);
      font: var(--md-sys-typescale-body-medium, 400 14px/20px system-ui);
    }
    :host([severity="error"]) {
      --_bg: var(--md-sys-color-error-container);
      --_fg: var(--md-sys-color-on-error-container);
    }
    :host([severity="success"]) {
      --_bg: var(--md-sys-color-tertiary-container);
      --_fg: var(--md-sys-color-on-tertiary-container);
    }
    :host([severity="warning"]) {
      --_bg: var(--md-sys-color-secondary-container);
      --_fg: var(--md-sys-color-on-secondary-container);
    }
    :host([severity="info"]) {
      --_bg: var(--md-sys-color-surface-container-high);
      --_fg: var(--md-sys-color-on-surface);
    }
    .title {
      font: var(--md-sys-typescale-title-small, 500 14px/20px system-ui);
      margin-block-end: 2px;
    }
    .body {
      flex: 1;
    }
  `;
  render() {
    return html`
      <md-icon aria-hidden="true">${ICONS[this.severity]}</md-icon>
      <div class="body">
        <slot name="title" class="title"></slot>
        <slot></slot>
      </div>
      <slot name="action"></slot>
      ${this.closable
        ? html`<button @click=${() => this.dispatchEvent(new Event("close"))}>
            <md-icon>close</md-icon>
          </button>`
        : nothing}
    `;
  }
}
```

`AlertTitle` → `<span slot="title">`. Mapping `severity` direct ; `variant="outlined"` ajoutable via `border` + `background:transparent`. **Effort : moyen** (gestion des 4 sévérités × 3 variantes + a11y `role="alert"`).

---

### Skeleton

**Fonction MUI** (`material-ui/packages/mui-material/src/Skeleton/Skeleton.js`) : placeholder de chargement animé (`variant` text/circular/rectangular/rounded, `animation` pulse/wave).
**Pourquoi pas d'équivalent** : **état de chargement non spécifié par M3** comme composant ; material-web fournit `md-circular-progress`/`md-linear-progress`/`md-loading-indicator` (indicateurs actifs) mais pas de skeleton. Absent de `md-elements.txt`.
**Solution recommandée — (a) shim CSS pur** (web component trivial ou simple classe), animation via durées/easings motion réels.

```ts
// migration/wrappers/shims/md-skeleton.ts
import { LitElement, css, html } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("md-skeleton")
export class MdSkeleton extends LitElement {
  @property() variant: "text" | "circular" | "rectangular" | "rounded" = "text";
  static styles = css`
    :host {
      display: block;
      background: var(--md-sys-color-surface-container-highest, #e6e0e9);
      border-radius: var(--md-sys-shape-corner-extra-small, 4px);
      animation: pulse var(--md-sys-motion-duration-extra-long2, 800ms)
        var(--md-sys-motion-easing-standard, cubic-bezier(0.2, 0, 0, 1)) infinite alternate;
    }
    :host([variant="text"]) {
      block-size: 1em;
      transform: scale(1, 0.7);
      border-radius: var(--md-sys-shape-corner-small, 8px);
    }
    :host([variant="circular"]) {
      border-radius: var(--md-sys-shape-corner-full, 9999px);
    }
    :host([variant="rounded"]) {
      border-radius: var(--md-sys-shape-corner-medium, 12px);
    }
    @keyframes pulse {
      from {
        opacity: 1;
      }
      to {
        opacity: 0.4;
      }
    }
    @media (prefers-reduced-motion: reduce) {
      :host {
        animation: none;
      }
    }
  `;
  render() {
    return html`<slot></slot>`;
  }
}
```

Largeur/hauteur pilotées par l'hôte (Tailwind `w-*`/`h-*`, conforme §6 : agit sur le host). **Effort : faible.**

---

### Breadcrumbs

**Fonction MUI** (`material-ui/packages/mui-material/src/Breadcrumbs/Breadcrumbs.js`) : fil d'Ariane — liste de `Link` séparés par un séparateur, repliable (`maxItems`/`expandText`).
**Pourquoi pas d'équivalent** : **pattern de navigation non standardisé en M3** (absent de `md-elements.txt`). Se compose à partir de liens et d'un séparateur.
**Solution recommandée — (a) shim React** sémantique `<nav aria-label="breadcrumb"><ol>` + `Link` tokenisé.

```tsx
// migration/wrappers/shims/Breadcrumbs.tsx
import * as React from "react";

export function Breadcrumbs({
  separator = "/",
  children,
}: {
  separator?: React.ReactNode;
  children: React.ReactNode;
}) {
  const items = React.Children.toArray(children);
  return (
    <nav aria-label="breadcrumb">
      <ol
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          listStyle: "none",
          margin: 0,
          padding: 0,
        }}
      >
        {items.map((child, i) => (
          <li
            key={i}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              color: "var(--md-sys-color-on-surface-variant)",
              font: "var(--md-sys-typescale-body-medium)",
            }}
          >
            {child}
            {i < items.length - 1 && <span aria-hidden="true">{separator}</span>}
          </li>
        ))}
      </ol>
    </nav>
  );
}
```

Le repliement `maxItems` (bouton « … » qui expand) est une couche optionnelle. **Effort : faible.**

---

### Link

**Fonction MUI** (`material-ui/packages/mui-material/src/Link/Link.js`) : `<a>` stylé (couleur, `underline` none/hover/always, intégrable router).
**Pourquoi pas d'équivalent** : un lien est une **primitive HTML** ; M3 ne livre pas de `md-link`. Absent de `md-elements.txt`.
**Solution recommandée — (a) `<a>` tokenisé** (classe ou shim léger) + `md-focus-ring` attachable pour l'état focus.

```tsx
// migration/wrappers/shims/Link.tsx  (option React + classe)
import * as React from "react";
import "@material/web/focus/md-focus-ring.js";

export const Link = React.forwardRef<
  HTMLAnchorElement,
  React.AnchorHTMLAttributes<HTMLAnchorElement>
>(function Link(props, ref) {
  const id = React.useId();
  return (
    <span style={{ position: "relative", display: "inline" }}>
      <a
        id={id}
        ref={ref}
        {...props}
        style={{
          color: "var(--md-sys-color-primary)",
          textDecorationColor: "currentColor",
          font: "var(--md-sys-typescale-body-medium)",
          ...props.style,
        }}
      />
      {/* @ts-expect-error custom element */}
      <md-focus-ring for={id} />
    </span>
  );
});
```

Variante CSS-only : classe `.md-link { color: var(--md-sys-color-primary); }`. **Effort : faible.**

---

### Rating

**Fonction MUI** (`material-ui/packages/mui-material/src/Rating/Rating.js`) : sélection d'une note par étoiles (demi-valeurs, hover preview, read-only).
**Pourquoi pas d'équivalent** : **non spécifié en M3** comme composant material-web. Absent de `md-elements.txt`.
**Solution recommandée — (a) shim Lit `md-rating`** : groupe de boutons-icônes (`md-icon` `star`/`star_border`) avec `role="radiogroup"`, émettant `change`.

```ts
// migration/wrappers/shims/md-rating.ts (esquisse)
import { LitElement, css, html } from "lit";
import { customElement, property } from "lit/decorators.js";
import "@material/web/icon/icon.js";
import "@material/web/ripple/ripple.js";

@customElement("md-rating")
export class MdRating extends LitElement {
  @property({ type: Number }) value = 0;
  @property({ type: Number }) max = 5;
  @property({ type: Boolean }) readonly = false;
  static styles = css`
    :host {
      display: inline-flex;
      color: var(--md-sys-color-primary);
    }
    button {
      position: relative;
      border: none;
      background: none;
      padding: 4px;
      color: inherit;
      cursor: pointer;
      border-radius: var(--md-sys-shape-corner-full, 9999px);
    }
    :host([readonly]) button {
      cursor: default;
    }
  `;
  private pick(v: number) {
    if (this.readonly) return;
    this.value = v;
    this.dispatchEvent(new Event("change", { bubbles: true }));
  }
  render() {
    return html`<div role="radiogroup">
      ${Array.from({ length: this.max }, (_, i) => i + 1).map(
        (i) =>
          html`<button role="radio" aria-checked=${i === this.value} @click=${() => this.pick(i)}>
            <md-ripple></md-ripple>
            <md-icon>${i <= this.value ? "star" : "star_border"}</md-icon>
          </button>`,
      )}
    </div>`;
  }
}
```

Demi-étoiles = couche optionnelle (icône `star_half`). **Effort : moyen.**

---

### Paper (layout — voir `06-tailwind-material-web.md`)

**Fonction MUI** (`material-ui/packages/mui-material/src/Paper/Paper.js`) : surface élevée (`elevation` 0–24, `variant="outlined"`, `square`).
**Pourquoi pas d'équivalent** : ce n'est pas un composant M3 dédié — M3 modélise l'élévation comme un **attribut de surface** (`md-elevation` overlay). Absent de `md-elements.txt`. Détail complet dans `06-…` ; esquisse :

```tsx
// <div> surface + md-elevation
import "@material/web/elevation/elevation.js";

export function Paper({
  elevation = 1,
  children,
  ...rest
}: { elevation?: number } & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      {...rest}
      style={{
        position: "relative",
        background: "var(--md-sys-color-surface-container-low)",
        color: "var(--md-sys-color-on-surface)",
        borderRadius: "var(--md-sys-shape-corner-medium, 12px)",
        // niveau d'élévation M3 (0..5) piloté par la prop --md-elevation-level
        ["--md-elevation-level" as any]: Math.min(5, Math.round(elevation / 4)),
        ...rest.style,
      }}
    >
      {/* @ts-expect-error custom element */}
      <md-elevation />
      {children}
    </div>
  );
}
```

`Box`/`Container`/`Stack`/`Grid` → `<div>` + utilitaires Tailwind (voir §6). **Effort : faible.**

---

### Backdrop / Modal / Popover / Popper (overlays)

- **`Backdrop`** (`src/Backdrop/Backdrop.js`) : scrim plein écran. → **(b)** `<div>` plein écran `background: var(--md-sys-color-scrim); opacity: 0.32; transition: opacity var(--md-sys-motion-duration-medium2) var(--md-sys-motion-easing-emphasized)`. Souvent inutile : `md-dialog` (`material-web/dialog/internal/dialog.ts`) gère déjà son scrim et son `<dialog>` natif.
- **`Modal`** (`src/Modal/Modal.js`) : conteneur d'overlay bas niveau (focus trap, scroll lock, backdrop). → **(b)** `<dialog>` natif (`showModal()` = scrim + focus trap + `inert` gratuits) **ou** `md-dialog` quand le contenu est une vraie boîte de dialogue M3. **Effort : moyen.**
- **`Popover`** (`src/Popover/Popover.js`) : surface ancrée à un élément. → **(b)** **Popover API** (`popover` attr + `popovertarget` ou `showPopover()`) combinée à **CSS Anchor Positioning** (`anchor-name`/`position-anchor`). Le top-layer du navigateur remplace le `Portal` MUI.
- **`Popper`** (`src/Popper/Popper.js`) : moteur de positionnement (Popper.js/floating-ui) sans surface. → **(b)** CSS Anchor Positioning natif quand supporté ; sinon garder `@floating-ui/dom` comme primitive (material-web n'expose pas son positionneur interne). **Effort : moyen.**

Esquisse Popover API + anchor :

```html
<md-icon-button id="btn" popovertarget="pop"><md-icon>more_vert</md-icon></md-icon-button>
<div
  id="pop"
  popover
  style="
  position-anchor: --btn;
  inset: auto; top: anchor(bottom); left: anchor(left);
  background: var(--md-sys-color-surface-container);
  border-radius: var(--md-sys-shape-corner-medium, 12px);
  --md-elevation-level: 2;"
>
  …
</div>
<style>
  #btn {
    anchor-name: --btn;
  }
</style>
```

Fallback si anchor positioning indisponible : `floating-ui`. **Note** : pour les menus, préférer directement `md-menu` (gère anchor + Popover API en interne).

---

### Transitions : Collapse / Fade / Grow / Slide / Zoom

**Fonction MUI** (`src/Collapse`, `src/Fade`, `src/Grow`, `src/Slide`, `src/Zoom`, moteur `react-transition-group`) : wrappers d'animation d'entrée/sortie.
**Pourquoi pas d'équivalent** : material-web **n'expose pas de composants de transition** ; l'animation y est interne à chaque composant et **les tokens motion existent** (`material-web/tokens/versions/v0_192/_md-sys-motion.scss`).
**Solution recommandée — (b) CSS transitions pilotées par motion tokens**, idéalement via un petit helper React (`useTransition` maison ou `react-transition-group` conservé) appliquant les tokens :

| MUI        | CSS cible                                              | Durée / easing M3 réels                                                                                              |
| ---------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| `Fade`     | `opacity: 0→1`                                         | `--md-sys-motion-duration-medium2` (300ms) / `--md-sys-motion-easing-standard`                                       |
| `Grow`     | `transform: scale(.75)→1` + opacity                    | `--md-sys-motion-duration-medium4` (400ms) / `--md-sys-motion-easing-emphasized`                                     |
| `Zoom`     | `transform: scale(0)→1`                                | `--md-sys-motion-duration-medium2` / `--md-sys-motion-easing-emphasized`                                             |
| `Slide`    | `translate: 100% / 0`                                  | `--md-sys-motion-duration-long1` (450ms) / `--md-sys-motion-easing-emphasized-decelerate` (in) / `-accelerate` (out) |
| `Collapse` | `grid-template-rows: 0fr→1fr` (ou `block-size` mesuré) | `--md-sys-motion-duration-medium4` / `--md-sys-motion-easing-emphasized`                                             |

```css
.md-fade-enter {
  opacity: 0;
}
.md-fade-enter-active {
  opacity: 1;
  transition: opacity var(--md-sys-motion-duration-medium2, 300ms)
    var(--md-sys-motion-easing-standard, cubic-bezier(0.2, 0, 0, 1));
}
.md-collapse {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows var(--md-sys-motion-duration-medium4, 400ms)
    var(--md-sys-motion-easing-emphasized, cubic-bezier(0.2, 0, 0, 1));
}
.md-collapse[data-open] {
  grid-template-rows: 1fr;
}
.md-collapse > * {
  overflow: hidden;
}
@media (prefers-reduced-motion: reduce) {
  [class^="md-"] {
    transition: none;
  }
}
```

Pour `Collapse` spécifiquement, si le contenu est un panneau accordéon, préférer `md-expansion-panel`/`md-accordion` (fork) qui gère déjà l'animation. **Effort : faible** (CSS tokens) ; **moyen** si l'on réimplémente un `useTransition` complet avec gestion unmount.

---

### MobileStepper

**Fonction MUI** (`src/MobileStepper/MobileStepper.js`) : barre compacte de progression d'étapes (dots / progress / text) avec boutons préc./suiv., typiquement pour carrousels mobiles.
**Pourquoi pas d'équivalent** : `md-stepper`/`md-step` (fork) couvrent le stepper desktop mais **pas la variante mobile compacte** ; absent de `md-elements.txt`. Synergie possible avec `md-carousel`.
**Solution recommandée — (a) shim React** : 2 `md-text-button` (back/next) encadrant un indicateur de position (dots, ou `md-linear-progress` pour `variant="progress"`).

```tsx
import "@material/web/button/text-button.js";
import "@material/web/progress/linear-progress.js";
// dots: map(steps) -> <span> coloré primary/outline-variant selon activeStep
// progress: <md-linear-progress value={activeStep/(steps-1)} />
```

**Effort : faible.**

---

### CssBaseline / ScopedCssBaseline / GlobalStyles / InitColorSchemeScript

**Fonction MUI** (`src/CssBaseline`, `src/ScopedCssBaseline`, `src/GlobalStyles`, `src/InitColorSchemeScript`) : reset CSS global / scopé, injection de styles globaux, script anti-FOUC pour le mode sombre.
**Pourquoi pas d'équivalent** : infrastructure de theming, **hors périmètre composant M3**.
**Solution recommandée — (a)** :

- `CssBaseline` → feuille de reset globale (`box-sizing: border-box`, `margin:0`, `font` = `--md-sys-typescale-body-large`, `background: var(--md-sys-color-background)`, `color: var(--md-sys-color-on-surface)`, `color-scheme`) injectée une fois. Les tokens `--md-sys-*` viennent de `material-web/tokens/` (voir `02-theme-token-migration.md`).
- `ScopedCssBaseline` → mêmes règles mais sous une classe racine `.md-baseline { … }` (scope local).
- `GlobalStyles` → simple `<style>` / fichier CSS importé (pas de composant).
- `InitColorSchemeScript` → script inline qui lit `localStorage`/`prefers-color-scheme` et pose `color-scheme` + une classe `dark`/attribut `data-md-theme` sur `<html>` avant peinture (anti-FOUC). Équivalent du toggle de thème décrit en `02-…`.

**Effort : faible** (chacun).

---

### Utilitaires divers

- **`ClickAwayListener`** (`src/ClickAwayListener`) → **(a)** hook React `useClickAway(ref, onAway)` (listener `pointerdown`/`focusin` sur `document`). Souvent superflu : `md-menu`/`md-dialog` ferment déjà sur clic extérieur. **Effort : faible.**
- **`Portal`** (`src/Portal`) → **(b)** `ReactDOM.createPortal` (primitive React, conservée). Pour les overlays md, le top-layer natif (`<dialog>`/Popover API) rend le portal inutile. **Effort : faible.**
- **`NoSsr`** (`src/NoSsr`) → **(a)** garde `useIsClient()` (`useState(false)` + `useEffect(()=>setTrue)`). Pertinent car les éléments custom doivent être upgradés côté client (voir SSR dans `03-react-integration.md`). **Effort : faible.**
- **`Unstable_TrapFocus`** (`src/Unstable_TrapFocus`) → **(b)** focus trap via `inert` sur le reste du document + sentinelles, ou simplement déléguer à `<dialog>.showModal()` / `md-dialog`. **Effort : moyen.**
- **`TextareaAutosize`** (`src/TextareaAutosize`) → **(a)** comportement auto-resize (recalcul `scrollHeight`) appliqué à un `<textarea>`, ou usage de `md-filled-text-field`/`md-outlined-text-field` avec `type="textarea"` + un contrôleur de hauteur. **Effort : faible.**
- **`ImageListItemBar`** (`src/ImageListItemBar`) → **(a)** overlay slotté (label + sous-titre + action icon) positionné en bas/haut de `md-grid-tile` (fork) via `slot`. **Effort : faible.**
- **`SpeedDialIcon`** (`src/SpeedDialIcon`) : icône morphée (open/close) du SpeedDial. Largement couvert par `md-fab-menu`/`md-fab-menu-item` (fork) qui gèrent l'ouverture ; si swap d'icône explicite requis, gérer `icon`/`openIcon` côté wrapper du FAB. **Effort : faible.**
- **`ButtonGroup`** (`src/ButtonGroup`) : groupe de boutons accolés. → **(a)** wrapper layout (flex + rayons d'angle adaptés) autour de `md-*-button`. À noter : `md-button-group` **existe** dans `md-elements.txt` — vérifier sa sémantique avant de shimmer ; sinon segmented buttons (`md-outlined-segmented-button-set`) couvrent le cas « toggle ». **Effort : faible.**
- **`darkScrollbar`** (util, `src/darkScrollbar`) → **(a)** snippet CSS `scrollbar-color: var(--md-sys-color-outline) transparent;` + `color-scheme`. **Effort : faible.**

### Hooks

- **`useMediaQuery`** (`src/useMediaQuery`) → **(b)** hook maison sur `window.matchMedia(query)` + `useSyncExternalStore`. **Effort : faible.**
- **`useScrollTrigger`** (`src/useScrollTrigger`) → **(a)** hook `IntersectionObserver` ou listener `scroll` throttlé (utilisé pour app bar « shrink on scroll »). **Effort : faible.**
- **`usePagination`** (`src/usePagination`) → **(a)** hook pur de calcul des items de pagination (logique reprenable telle quelle), ou s'appuyer sur `md-paginator` (fork) qui encapsule cette logique. **Effort : faible.**
- **`useLazyRipple`** (`src/useLazyRipple`) → **abandonner** : remplacé nativement par `md-ripple` (attachable, `material-web/ripple/internal/ripple.ts`). **Effort : nul.**

---

## Synthèse d'effort

- **Shims web component Lit à construire** (priorité, code esquissé) : `md-avatar`, `md-alert`(+`AlertTitle` slot), `md-skeleton`, `md-rating`. Effort cumulé faible→moyen.
- **Shims React purs** : `AvatarGroup`, `Breadcrumbs`, `Link`, `Paper`, `MobileStepper`, hooks (`useMediaQuery`, `useScrollTrigger`, `usePagination`, `useClickAway`, `useIsClient`). Effort faible.
- **Primitives plateforme** (zéro composant à livrer) : `Modal`→`<dialog>`, `Popover`/`Popper`→Popover API + Anchor Positioning, transitions→CSS + motion tokens, `Backdrop`→scrim token, `Portal`→`createPortal`, focus trap→`<dialog>`/`inert`.
- **Infra theming** : `CssBaseline`/`ScopedCssBaseline`/`GlobalStyles`/`InitColorSchemeScript` → reset + injection tokens (renvoi `02-theme-token-migration.md`).
- **Layout** : `Box`/`Container`/`Stack`/`Grid`/`Paper` → renvoi `06-tailwind-material-web.md`.
- **Abandon** : `useLazyRipple` (couvert par `md-ripple`).
