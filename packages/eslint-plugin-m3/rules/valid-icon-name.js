// SPDX-License-Identifier: Apache-2.0
import { MATERIAL_SYMBOLS } from "../data/material-symbols-names.js";
import { isMdIconTag, jsxTagName, singleTextChild } from "./helpers.js";

/** PascalCase -> snake_case (frontières acronymes + chiffres), même algo que le codemod. */
function pascalToSnake(s) {
  return s
    .replace(/([a-z])([A-Z])/g, "$1_$2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1_$2")
    .replace(/([A-Za-z])([0-9])/g, "$1_$2")
    .replace(/([0-9])([A-Za-z])/g, "$1_$2")
    .replace(/_+/g, "_")
    .toLowerCase()
    .replace(/^_|_$/g, "");
}

/**
 * Le contenu d'un <md-icon>/<MdIcon> est le NOM DU GLYPHE Material Symbols
 * (snake_case). Cette règle valide ce nom contre la liste officielle (4253) et
 * détecte les restes de migration MUI (PascalCase, ex. "Delete").
 */
export default {
  meta: {
    type: "problem",
    fixable: "code",
    docs: {
      description: "md-icon child must be a valid Material Symbols glyph name (snake_case).",
    },
    schema: [],
  },
  create(context) {
    /** Fixer : remplace le texte enfant du <md-icon> par `replacement`. */
    function fixChild(node, replacement) {
      const kids = (node.children || []).filter(
        (c) => !(c.type === "JSXText" && c.value.trim() === ""),
      );
      const only = kids[0];
      if (!only || only.type !== "JSXText") return undefined;
      return (fixer) => fixer.replaceText(only, replacement);
    }
    return {
      JSXElement(node) {
        const name = jsxTagName(node.openingElement);
        if (!isMdIconTag(name)) return;
        const text = singleTextChild(node);
        if (text === null || text === "") return; // dynamique/vide : non vérifiable
        if (MATERIAL_SYMBOLS.has(text)) return; // OK

        // PascalCase (reste MUI) ou camelCase -> proposer le snake_case validé
        if (/[A-Z]/.test(text)) {
          const snake = pascalToSnake(text);
          const valid = MATERIAL_SYMBOLS.has(snake);
          context.report({
            node,
            message: valid
              ? `Material Symbols utilise snake_case : remplacer "${text}" par "${snake}".`
              : `"${text}" n'est pas un glyphe Material Symbols (essai "${snake}" : introuvable). Vérifier sur fonts.google.com/icons.`,
            // Autofix uniquement quand le snake_case est un glyphe connu.
            fix: valid ? fixChild(node, snake) : undefined,
          });
          return;
        }
        // déjà snake_case mais introuvable -> typo / nom legacy
        context.report({
          node,
          message: `"${text}" n'est pas un glyphe Material Symbols connu. Vérifier sur fonts.google.com/icons (4253 glyphes).`,
        });
      },
    };
  },
};
