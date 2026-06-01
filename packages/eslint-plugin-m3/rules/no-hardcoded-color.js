// SPDX-License-Identifier: Apache-2.0
import { getAttr, isMdTag, jsxTagName } from "./helpers.js";

// hex (#abc / #aabbcc / #aabbccdd) ou rgb()/hsl() littéral
const COLOR_RE = /#[0-9a-fA-F]{3,8}\b|\b(?:rgb|rgba|hsl|hsla)\s*\(/;

/** Collecte les valeurs string d'un style inline JSX (style={{ color: "#fff" }}). */
function styleStringValues(openingElement) {
  const out = [];
  for (const attrName of ["style", "sx"]) {
    const attr = getAttr(openingElement, attrName);
    if (!attr || !attr.value || attr.value.type !== "JSXExpressionContainer") continue;
    const expr = attr.value.expression;
    if (!expr || expr.type !== "ObjectExpression") continue;
    for (const p of expr.properties) {
      if (
        p.type === "Property" &&
        p.value &&
        p.value.type === "Literal" &&
        typeof p.value.value === "string"
      ) {
        out.push(p.value.value);
      }
    }
  }
  return out;
}

/**
 * Couleur codée en dur (#hex / rgb()) dans un style inline sur un composant
 * material-web : préférer un rôle de couleur M3 `var(--md-sys-color-*)` (sinon
 * le composant ne suit pas le thème ni le dynamic-color / dark mode).
 */
export default {
  meta: {
    type: "suggestion",
    docs: {
      description: "Use var(--md-sys-color-*) instead of hardcoded colors on md-* components.",
    },
    schema: [],
  },
  create(context) {
    return {
      JSXOpeningElement(node) {
        const name = jsxTagName(node);
        if (!isMdTag(name)) return;
        for (const val of styleStringValues(node)) {
          if (COLOR_RE.test(val)) {
            context.report({
              node,
              message: `Couleur en dur "${val}" sur <${name}> : utiliser un rôle M3 var(--md-sys-color-*) pour suivre le thème (dark / dynamic-color).`,
            });
            break;
          }
        }
      },
    };
  },
};
