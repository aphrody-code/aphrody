// SPDX-License-Identifier: Apache-2.0
import { getAttr, jsxTagName } from "./helpers.js";

/**
 * Détecte les noms de props MUI laissés sur un wrapper material-web après une
 * migration incomplète. Source : migration/00-CONVENTIONS.md §4 (props renommées)
 * + mui-m3-map.json (events.renamedProps).
 *
 * map: tag Md* -> { propMUI: propM3 }
 */
const RENAMES = {
  MdSwitch: { checked: "selected" },
  MdTooltip: { title: "text" },
  MdNavigationDrawer: { open: "opened" },
  MdNavigationDrawerModal: { open: "opened" },
  MdDialog: { open: "open" /* identique : pas d'alerte */ },
  MdTabs: { value: "activeTabIndex" },
};

export default {
  meta: {
    type: "problem",
    fixable: "code",
    docs: {
      description: "Disallow MUI prop names on material-web wrappers (renamed props).",
    },
    schema: [],
  },
  create(context) {
    return {
      JSXOpeningElement(node) {
        const name = jsxTagName(node);
        const map = RENAMES[name];
        if (!map) return;
        for (const [muiProp, m3Prop] of Object.entries(map)) {
          if (muiProp === m3Prop) continue;
          const attr = getAttr(node, muiProp);
          if (attr) {
            context.report({
              node: attr,
              message: `<${name}> : la prop MUI \`${muiProp}\` se nomme \`${m3Prop}\` en material-web.`,
              // Autofix : renomme l'identifiant de l'attribut (la valeur est conservée).
              fix: (fixer) => fixer.replaceText(attr.name, m3Prop),
            });
          }
        }
      },
    };
  },
};
