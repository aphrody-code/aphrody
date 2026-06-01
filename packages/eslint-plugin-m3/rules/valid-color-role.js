// SPDX-License-Identifier: Apache-2.0
import { MD_SYS_COLOR_ROLES, closestColorRole } from "./helpers.js";

// Capture le nom de rôle dans un `var(--md-sys-color-<role>[, fallback])`.
const VAR_RE = /var\(\s*--md-sys-color-([a-z0-9-]+)/gi;

/**
 * Valide les rôles référencés via `var(--md-sys-color-<role>)`. material-web
 * n'émet en runtime que ~49 rôles `--md-sys-color-*` (cf. la table `ROLES` de
 * `@aphrody-code/m3-tokens`) ; un nom inconnu (typo, rôle MUI/Tailwind, casse)
 * résout en `unset` silencieux — la couleur disparaît sans erreur. Cette règle
 * scanne TOUTES les chaînes du fichier (style/sx inline, `className` arbitraire
 * Tailwind `[var(--md-sys-color-x)]`, CSS-in-JS) et signale les rôles absents,
 * en proposant le plus proche (distance d'édition <= 3).
 */
export default {
  meta: {
    type: "problem",
    docs: {
      description:
        "var(--md-sys-color-*) must reference a real M3 color role emitted by material-web.",
    },
    schema: [],
  },
  create(context) {
    /** Vérifie une chaîne arbitraire et signale chaque rôle inconnu. */
    function checkString(node, raw) {
      if (typeof raw !== "string" || raw.indexOf("--md-sys-color-") === -1) return;
      VAR_RE.lastIndex = 0;
      const seen = new Set();
      let m;
      while ((m = VAR_RE.exec(raw)) !== null) {
        const role = m[1].toLowerCase();
        if (MD_SYS_COLOR_ROLES.has(role) || seen.has(role)) continue;
        seen.add(role);
        const suggestion = closestColorRole(role);
        context.report({
          node,
          message: suggestion
            ? `Rôle de couleur M3 inconnu : --md-sys-color-${role}. Vouliez-vous --md-sys-color-${suggestion} ?`
            : `Rôle de couleur M3 inconnu : --md-sys-color-${role}. material-web n'émet que les ~49 rôles --md-sys-color-* (primary, surface, on-surface, outline, error, ...).`,
        });
      }
    }
    return {
      Literal(node) {
        if (typeof node.value === "string") checkString(node, node.value);
      },
      TemplateElement(node) {
        const v = node.value && node.value.cooked;
        if (typeof v === "string") checkString(node, v);
      },
    };
  },
};
