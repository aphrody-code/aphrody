// =============================================================================
// SHIM — M3Alert  (gap MUI `Alert`/`AlertTitle` → aucun équivalent md)
// =============================================================================
// MUI `Alert` n'a PAS d'élément <md-*> correspondant (contrat §3 "Gaps connus",
// détaillé dans 05-gap-analysis.md). Interdiction d'inventer un <md-alert>
// (contrat §7.2). On compose donc une surface tokenisée + une md-icon.
//
// La surface est un simple <div> hôte (pas de shadow DOM) : on peut donc la
// styliser au layout via Tailwind (contrat §6) tout en tirant les COULEURS des
// tokens --md-sys-* (single source of truth, comme les vrais composants md).
//
// Limites assumées (à documenter, contrat §5) : pas de variante `filled`/
// `outlined`/`standard` de MUI, pas d'action de fermeture intégrée. Pour un
// rendu plus riche, voir la piste `md-banner` dans 05-gap-analysis.md.

import * as React from "react";
import { MdIcon } from "../../../wrappers";

type Severity = "success" | "info" | "warning" | "error";

// Mapping severity → (rôle de couleur M3, glyphe Material Symbols).
const SEVERITY: Record<Severity, { bg: string; fg: string; icon: string }> = {
  success: {
    bg: "var(--md-sys-color-secondary-container)",
    fg: "var(--md-sys-color-on-secondary-container)",
    icon: "check_circle",
  },
  info: {
    bg: "var(--md-sys-color-primary-container)",
    fg: "var(--md-sys-color-on-primary-container)",
    icon: "info",
  },
  warning: {
    bg: "var(--md-sys-color-tertiary-container, var(--md-sys-color-secondary-container))",
    fg: "var(--md-sys-color-on-tertiary-container, var(--md-sys-color-on-secondary-container))",
    icon: "warning",
  },
  error: {
    bg: "var(--md-sys-color-error-container)",
    fg: "var(--md-sys-color-on-error-container)",
    icon: "error",
  },
};

export function M3Alert({
  severity = "info",
  children,
}: {
  severity?: Severity;
  children: React.ReactNode;
}) {
  const s = SEVERITY[severity];
  return (
    <div
      role="alert"
      className="flex items-center gap-3 rounded-[var(--md-sys-shape-corner-medium)] px-4 py-3"
      // sx MUI supprimé → style inline tirant les tokens (contrat §4/§6).
      style={{ backgroundColor: s.bg, color: s.fg }}
    >
      <MdIcon aria-hidden="true">{s.icon}</MdIcon>
      <span>{children}</span>
    </div>
  );
}
