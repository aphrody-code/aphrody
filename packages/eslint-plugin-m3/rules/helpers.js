// SPDX-License-Identifier: Apache-2.0
// Helpers AST partagés par les règles m3 (ESLint-compatible, ESTree + JSX).

/**
 * Nom textuel d'un élément JSX ouvrant.
 * <MdButton/> -> "MdButton" ; <md-icon/> -> "md-icon" ; <Foo.Bar/> -> null.
 */
export function jsxTagName(openingElement) {
  const n = openingElement && openingElement.name;
  if (!n) return null;
  if (n.type === "JSXIdentifier") return n.name;
  return null;
}

/** true si le tag est un composant material-web : wrapper React `Md*` OU custom element `md-*`. */
export function isMdTag(name) {
  if (!name) return false;
  return /^Md[A-Z0-9]/.test(name) || /^md-[a-z]/.test(name);
}

/** true si le tag est une icône md (MdIcon / md-icon). */
export function isMdIconTag(name) {
  return name === "MdIcon" || name === "md-icon";
}

/** Récupère un attribut JSX par nom (insensible n/a). Retourne le node JSXAttribute ou null. */
export function getAttr(openingElement, attrName) {
  for (const a of openingElement.attributes || []) {
    if (
      a.type === "JSXAttribute" &&
      a.name &&
      a.name.type === "JSXIdentifier" &&
      a.name.name === attrName
    ) {
      return a;
    }
  }
  return null;
}

/** Valeur string littérale d'un attribut JSX (attr="x" ou attr={"x"}). null sinon. */
export function attrStringValue(attr) {
  const v = attr && attr.value;
  if (!v) return null;
  if (v.type === "Literal" && typeof v.value === "string") return v.value;
  if (v.type === "JSXExpressionContainer") {
    const e = v.expression;
    if (e && e.type === "Literal" && typeof e.value === "string") return e.value;
    if (e && e.type === "TemplateLiteral" && e.quasis.length === 1) return e.quasis[0].value.cooked;
  }
  return null;
}

/** Texte enfant unique d'un élément JSX (si c'est un seul JSXText). null sinon (vide/expr/multiple). */
export function singleTextChild(jsxElement) {
  const kids = (jsxElement.children || []).filter(
    (c) => !(c.type === "JSXText" && c.value.trim() === ""),
  );
  if (kids.length !== 1) return null;
  const only = kids[0];
  if (only.type === "JSXText") return only.value.trim();
  return null;
}

/**
 * Les ~49 rôles de couleur M3 exposés en runtime par material-web sous
 * `--md-sys-color-<role>` (kebab-case). Source unique : la table `ROLES` de
 * `@aphrody-code/m3-tokens` (dynamic-color), kebab-isée par le même algorithme
 * (`role.replace(/([A-Z])/g, "-$1").toLowerCase()`). Sert à valider les
 * `var(--md-sys-color-*)` écrits à la main (typos) — cf. règle valid-color-role.
 */
export const MD_SYS_COLOR_ROLES = new Set([
  "background",
  "on-background",
  "surface",
  "surface-dim",
  "surface-bright",
  "surface-container-lowest",
  "surface-container-low",
  "surface-container",
  "surface-container-high",
  "surface-container-highest",
  "on-surface",
  "surface-variant",
  "on-surface-variant",
  "inverse-surface",
  "inverse-on-surface",
  "outline",
  "outline-variant",
  "shadow",
  "scrim",
  "surface-tint",
  "primary",
  "on-primary",
  "primary-container",
  "on-primary-container",
  "inverse-primary",
  "secondary",
  "on-secondary",
  "secondary-container",
  "on-secondary-container",
  "tertiary",
  "on-tertiary",
  "tertiary-container",
  "on-tertiary-container",
  "error",
  "on-error",
  "error-container",
  "on-error-container",
  "primary-fixed",
  "primary-fixed-dim",
  "on-primary-fixed",
  "on-primary-fixed-variant",
  "secondary-fixed",
  "secondary-fixed-dim",
  "on-secondary-fixed",
  "on-secondary-fixed-variant",
  "tertiary-fixed",
  "tertiary-fixed-dim",
  "on-tertiary-fixed",
  "on-tertiary-fixed-variant",
]);

/** Distance de Levenshtein (itérative, deux lignes) — pour proposer le rôle le plus proche. */
export function levenshtein(a, b) {
  const m = a.length;
  const n = b.length;
  if (m === 0) return n;
  if (n === 0) return m;
  let prev = Array.from({ length: n + 1 }, (_, i) => i);
  let curr = new Array(n + 1);
  for (let i = 1; i <= m; i++) {
    curr[0] = i;
    for (let j = 1; j <= n; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      curr[j] = Math.min(prev[j] + 1, curr[j - 1] + 1, prev[j - 1] + cost);
    }
    [prev, curr] = [curr, prev];
  }
  return prev[n];
}

/** Rôle M3 le plus proche de `name` (distance <= 3), ou null. */
export function closestColorRole(name) {
  let best = null;
  let bestD = Infinity;
  for (const role of MD_SYS_COLOR_ROLES) {
    const d = levenshtein(name, role);
    if (d < bestD) {
      bestD = d;
      best = role;
    }
  }
  return bestD <= 3 ? best : null;
}
