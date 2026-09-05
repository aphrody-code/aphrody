// SPDX-License-Identifier: Apache-2.0

/**
 * Signale les imports MUI résiduels quand on cible material-web : mélanger les
 * deux systèmes double le bundle (Emotion + Lit) et casse la cohérence M3.
 * - @mui/material        -> @aphrody/m3-react (codemod orchestrator.ts)
 * - @mui/icons-material  -> <md-icon> + Material Symbols (codemod icons.ts)
 * - @mui/x-*             -> md-table / md-*-chart / md-*-picker / md-scheduler
 */
const SUGGEST = [
  [
    /^@mui\/icons-material(\/|$)/,
    "<md-icon> + Material Symbols (codemod migration/codemods/transforms/icons.ts)",
  ],
  [/^@mui\/material(\/|$)/, "@aphrody/m3-react (codemod orchestrator.ts)"],
  [/^@mui\/x-data-grid/, "md-table (@aphrody/m3-react)"],
  [/^@mui\/x-charts/, "md-*-chart (@aphrody/m3-react)"],
  [/^@mui\/x-date-pickers/, "md-date-picker / md-time-picker (@aphrody/m3-react)"],
  [/^@mui\/(lab|system|material-nextjs)/, "material-web (pas d'Emotion/cache requis)"],
];

export default {
  meta: {
    type: "suggestion",
    docs: {
      description: "Disallow MUI imports when targeting material-web (M3).",
    },
    schema: [],
  },
  create(context) {
    function check(node, source) {
      if (typeof source !== "string") return;
      for (const [re, target] of SUGGEST) {
        if (re.test(source)) {
          context.report({
            node,
            message: `Import MUI "${source}" : migrer vers ${target}.`,
          });
          return;
        }
      }
    }
    return {
      ImportDeclaration(node) {
        check(node, node.source && node.source.value);
      },
      // import("@mui/...") dynamique
      ImportExpression(node) {
        if (node.source && node.source.type === "Literal") check(node, node.source.value);
      },
    };
  },
};
