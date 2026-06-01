// SPDX-License-Identifier: Apache-2.0
import { getAttr, jsxTagName } from "./helpers.js";

/** true si le tag est un bouton-icône M3 (md-icon-button & variantes / MdIconButton & variantes). */
function isIconButton(name) {
  if (!name) return false;
  if (/^Md(Filled|FilledTonal|Outlined)?IconButton$/.test(name)) return true;
  return /^md-(filled-|filled-tonal-|outlined-)?icon-button$/.test(name);
}

const LABEL_ATTRS = ["aria-label", "aria-labelledby", "title"];

/**
 * Un bouton-icône M3 n'a pas de texte visible : son nom accessible DOIT venir
 * d'un `aria-label` (ou `aria-labelledby` / `title`). Sans lui, le bouton est
 * muet pour les lecteurs d'écran — violation WCAG 4.1.2 (M3 a11y). On ne
 * signale rien en présence d'un spread `{...props}` (le label peut en venir).
 */
export default {
  meta: {
    type: "problem",
    docs: {
      description:
        "md-*-icon-button needs an accessible name (aria-label / aria-labelledby / title).",
    },
    schema: [],
  },
  create(context) {
    return {
      JSXOpeningElement(node) {
        const name = jsxTagName(node);
        if (!isIconButton(name)) return;

        // Spread => label potentiellement injecté ailleurs : ne pas alerter.
        const hasSpread = (node.attributes || []).some((a) => a.type === "JSXSpreadAttribute");
        if (hasSpread) return;

        for (const a of LABEL_ATTRS) {
          const attr = getAttr(node, a);
          // Présent et non vide (aria-label="" ne compte pas).
          if (attr && !(attr.value && attr.value.type === "Literal" && attr.value.value === "")) {
            return;
          }
        }

        context.report({
          node,
          message: `<${name}> sans nom accessible : ajouter aria-label="..." (a11y — WCAG 4.1.2 ; un bouton-icône n'a pas de texte visible).`,
        });
      },
    };
  },
};
