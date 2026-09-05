// SPDX-License-Identifier: Apache-2.0
import { getAttr, isMdTag, jsxTagName } from "./helpers.js";

/**
 * Le prop `sx` (MUI/Emotion) n'a AUCUN effet sur un composant material-web : il
 * n'y a pas de moteur Emotion runtime. Styler via `className`/Tailwind (host) +
 * tokens `--md-sys-*` (l'intérieur du shadow DOM n'est pas atteignable de
 * l'extérieur — cf. migration/06-tailwind-material-web.md).
 */
export default {
  meta: {
    type: "problem",
    docs: {
      description: "`sx` prop has no effect on material-web (md-*) components.",
    },
    schema: [],
  },
  create(context) {
    return {
      JSXOpeningElement(node) {
        const name = jsxTagName(node);
        if (!isMdTag(name)) return;
        const sx = getAttr(node, "sx");
        if (sx) {
          context.report({
            node: sx,
            message: `\`sx\` est sans effet sur <${name}> (pas d'Emotion runtime) : utiliser className/Tailwind sur l'hôte + tokens --md-sys-*.`,
          });
        }
      },
    };
  },
};
