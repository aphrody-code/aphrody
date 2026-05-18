// SPDX-License-Identifier: Apache-2.0
"use client";

import type { JSX } from "react";

type RailKey = "new" | "recent" | "gems" | "settings";

interface LeftRailProps {
  active: RailKey;
  onSelect(next: RailKey): void;
}

const ITEMS: ReadonlyArray<{ key: RailKey; label: string }> = [
  { key: "new", label: "New" },
  { key: "recent", label: "Recent" },
  { key: "gems", label: "Gems" },
  { key: "settings", label: "Settings" },
];

export function LeftRail({ active, onSelect }: LeftRailProps): JSX.Element {
  return (
    <nav className="rail" aria-label="Primary navigation">
      {ITEMS.map((item) => (
        <button
          key={item.key}
          type="button"
          className={`rail__btn${item.key === active ? " rail__btn--active" : ""}`}
          aria-current={item.key === active ? "page" : undefined}
          onClick={() => onSelect(item.key)}
        >
          <RailGlyph kind={item.key} />
          <span className="rail__btn-label">{item.label}</span>
        </button>
      ))}
    </nav>
  );
}

function RailGlyph({ kind }: { kind: RailKey }): JSX.Element {
  switch (kind) {
    case "new":
      return (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path d="M3 17.46v3.04c0 .28.22.5.5.5h3.04c.13 0 .26-.05.35-.15L17.81 9.94l-3.75-3.75L3.15 17.1c-.1.1-.15.22-.15.36zM20.71 7.04a1 1 0 0 0 0-1.41l-2.34-2.34a1 1 0 0 0-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z" />
        </svg>
      );
    case "recent":
      return (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path d="M13 3a9 9 0 1 0 8.94 10H20a7 7 0 1 1-7-7v3l4-4-4-4v2zm-1 5v5l4.25 2.52.75-1.23-3.5-2.08V8H12z" />
        </svg>
      );
    case "gems":
      return (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm-1 14.93A7 7 0 0 1 5 12h6zm0-9.86A7 7 0 0 0 5 12h6zm2 9.86V12h6a7 7 0 0 1-6 4.93z" />
        </svg>
      );
    case "settings":
      return (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path d="M19.43 12.98c.04-.32.07-.65.07-.98s-.03-.66-.07-.98l2.11-1.65a.5.5 0 0 0 .12-.64l-2-3.46a.5.5 0 0 0-.61-.22l-2.49 1a7.03 7.03 0 0 0-1.69-.98l-.38-2.65A.5.5 0 0 0 14 2h-4a.5.5 0 0 0-.49.42l-.38 2.65c-.61.25-1.18.57-1.69.98l-2.49-1a.5.5 0 0 0-.61.22l-2 3.46a.5.5 0 0 0 .12.64l2.11 1.65c-.04.32-.07.66-.07.98s.03.65.07.98L2.46 14.63a.5.5 0 0 0-.12.64l2 3.46a.5.5 0 0 0 .61.22l2.49-1c.51.41 1.08.73 1.69.98l.38 2.65A.5.5 0 0 0 10 22h4a.5.5 0 0 0 .49-.42l.38-2.65c.61-.25 1.18-.57 1.69-.98l2.49 1a.5.5 0 0 0 .61-.22l2-3.46a.5.5 0 0 0-.12-.64l-2.11-1.65zM12 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7z" />
        </svg>
      );
  }
}

export type { RailKey };
