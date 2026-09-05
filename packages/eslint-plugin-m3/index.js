// SPDX-License-Identifier: Apache-2.0
/**
 * @aphrody/eslint-plugin-m3 — règles de lint pour les sites consommant
 * material-web (@aphrody/m3-react + Material Symbols).
 *
 * API ESLint-compatible : fonctionne tel quel sous **oxlint** (`jsPlugins`) ET
 * sous **ESLint** (`plugins`). Les règles ciblent les composants `Md*` (wrappers
 * React) et `md-*` (custom elements).
 *
 * oxlint (.oxlintrc.json) :
 *   { "jsPlugins": ["./node_modules/@aphrody/eslint-plugin-m3/index.js"],
 *     "rules": { "m3/no-sx-prop": "error", "m3/valid-icon-name": "error" } }
 *
 * ESLint (flat config) :
 *   import m3 from "@aphrody/eslint-plugin-m3";
 *   export default [ m3.configs.recommended ];
 */
import noHardcodedColor from "./rules/no-hardcoded-color.js";
import noMuiImport from "./rules/no-mui-import.js";
import noMuiPropOnMd from "./rules/no-mui-prop-on-md.js";
import noSxProp from "./rules/no-sx-prop.js";
import preferIconToken from "./rules/prefer-icon-token.js";
import requireIconButtonLabel from "./rules/require-icon-button-label.js";
import validColorRole from "./rules/valid-color-role.js";
import validIconName from "./rules/valid-icon-name.js";

const rules = {
  "valid-icon-name": validIconName,
  "valid-color-role": validColorRole,
  "no-sx-prop": noSxProp,
  "no-mui-import": noMuiImport,
  "no-mui-prop-on-md": noMuiPropOnMd,
  "prefer-icon-token": preferIconToken,
  "require-icon-button-label": requireIconButtonLabel,
  "no-hardcoded-color": noHardcodedColor,
};

const plugin = {
  meta: { name: "m3", version: "0.1.0" },
  rules,
};

// Presets ESLint flat-config. (oxlint référence les règles directement par nom.)
plugin.configs = {
  /** Tout en erreur sauf les suggestions, qui restent en warn. */
  recommended: {
    plugins: { m3: plugin },
    rules: {
      "m3/valid-icon-name": "error",
      "m3/valid-color-role": "error",
      "m3/no-sx-prop": "error",
      "m3/no-mui-prop-on-md": "error",
      "m3/no-mui-import": "warn",
      "m3/prefer-icon-token": "warn",
      "m3/require-icon-button-label": "warn",
      "m3/no-hardcoded-color": "warn",
    },
  },
  /** Migration stricte : tout en erreur (utile pour finir un port MUI -> M3). */
  strict: {
    plugins: { m3: plugin },
    rules: {
      "m3/valid-icon-name": "error",
      "m3/valid-color-role": "error",
      "m3/no-sx-prop": "error",
      "m3/no-mui-prop-on-md": "error",
      "m3/no-mui-import": "error",
      "m3/prefer-icon-token": "error",
      "m3/require-icon-button-label": "error",
      "m3/no-hardcoded-color": "error",
    },
  },
};

export default plugin;
