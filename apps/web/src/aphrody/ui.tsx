// Shared M3 building blocks for the ported aphrody feature panels.
// Every feature composes these instead of raw Angular Material.

import type { ReactNode } from "react";
import { MdCircularProgress, MdIcon, MdIconButton, MdOutlinedCard } from "@aphrody-code/m3-react";

/** Titled section card (replaces the Angular `mat-card` panels). */
export function Panel({
  title,
  icon,
  actions,
  children,
}: {
  title: string;
  icon?: string;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <MdOutlinedCard className="aph-panel">
      <header className="aph-panel__head">
        <span className="aph-row">
          {icon && <MdIcon className="aph-panel__icon">{icon}</MdIcon>}
          <h2 className="aph-panel__title">{title}</h2>
        </span>
        {actions && <span className="aph-row">{actions}</span>}
      </header>
      <div className="aph-panel__body">{children}</div>
    </MdOutlinedCard>
  );
}

/** A page header row (title + optional trailing controls). */
export function PageHead({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="aph-pagehead">
      <div>
        <h1 className="aph-pagehead__title">{title}</h1>
        {subtitle && <p className="aph-muted">{subtitle}</p>}
      </div>
      {actions && <span className="aph-row">{actions}</span>}
    </div>
  );
}

/** Monospace command output with a copy button (the `ExecResult` viewer). */
export function CodeOutput({ text, empty = "No output yet." }: { text: string; empty?: string }) {
  return (
    <div className="aph-output">
      <MdIconButton
        className="aph-output__copy"
        aria-label="Copy output"
        onClick={() => void navigator.clipboard?.writeText(text)}
      >
        <MdIcon>content_copy</MdIcon>
      </MdIconButton>
      <pre>{text || empty}</pre>
    </div>
  );
}

/** Dashboard stat tile. */
export function StatTile({
  icon,
  label,
  value,
  tone,
}: {
  icon: string;
  label: string;
  value: ReactNode;
  tone?: string;
}) {
  return (
    <MdOutlinedCard className="aph-stat">
      <MdIcon className="aph-stat__icon" style={tone ? { color: tone } : undefined}>
        {icon}
      </MdIcon>
      <div>
        <div className="aph-stat__value">{value}</div>
        <div className="aph-muted aph-stat__label">{label}</div>
      </div>
    </MdOutlinedCard>
  );
}

/** Empty / hint state. */
export function Hint({ icon, title, text }: { icon: string; title: string; text?: string }) {
  return (
    <div className="aph-hint">
      <MdIcon className="aph-hint__icon">{icon}</MdIcon>
      <p className="aph-hint__title">{title}</p>
      {text && <p className="aph-muted">{text}</p>}
    </div>
  );
}

export function Spinner({ label }: { label?: string }) {
  return (
    <div className="aph-spinner">
      <MdCircularProgress indeterminate />
      {label && <span className="aph-muted">{label}</span>}
    </div>
  );
}
