// SPDX-License-Identifier: Apache-2.0
import { getAttr, isMdIconTag, jsxTagName } from "./helpers.js";

/** Récupère les clés d'un style inline JSX (style={{ ... }}). [] sinon. */
function styleKeys(openingElement) {
  const style = getAttr(openingElement, "style");
  if (!style || !style.value || style.value.type !== "JSXExpressionContainer") return [];
  const expr = style.value.expression;
  if (!expr || expr.type !== "ObjectExpression") return [];
  const keys = [];
  for (const p of expr.properties) {
    if (p.type === "Property" && p.key) {
      if (p.key.type === "Identifier") keys.push(p.key.name);
      else if (p.key.type === "Literal") keys.push(String(p.key.value));
    }
  }
  return keys;
}

/**
 * Sur <md-icon>/<MdIcon>, piloter les axes Material Symbols via les tokens
 * --md-icon-fill / --md-icon-wght / --md-icon-grad / --md-icon-opsz (héritables,
 * animables) plutôt que `fontVariationSettings` inline (fige les axes, casse la
 * cascade). cf. migration/11-material-symbols.md.
 */
export default {
  meta: {
    type: "suggestion",
    docs: {
      description: "Prefer --md-icon-* axis tokens over inline font-variation-settings on md-icon.",
    },
    schema: [],
  },
  create(context) {
    return {
      JSXOpeningElement(node) {
        const name = jsxTagName(node);
        if (!isMdIconTag(name)) return;
        const keys = styleKeys(node);
        if (keys.includes("fontVariationSettings") || keys.includes("font-variation-settings")) {
          context.report({
            node,
            message:
              "Piloter les axes via --md-icon-fill / --md-icon-wght / --md-icon-grad / --md-icon-opsz (héritables, animables) plutôt que fontVariationSettings inline.",
          });
        }
      },
    };
  },
};
