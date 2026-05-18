// SPDX-License-Identifier: Apache-2.0
"use client";

import type { JSX } from "react";
import { ModelPicker, type GeminiModelId } from "./ModelPicker";

interface AppBarProps {
  model: GeminiModelId;
  onModelChange(next: GeminiModelId): void;
  onOpenSettings?: () => void;
  onOpenShare?: () => void;
}

export function AppBar({ model, onModelChange, onOpenSettings, onOpenShare }: AppBarProps): JSX.Element {
  return (
    <header className="appbar" role="banner">
      <button
        className="appbar__icon-btn"
        type="button"
        aria-label="Open navigation menu"
      >
        <MenuGlyph />
      </button>
      <span className="appbar__brand">Gemini</span>
      <ModelPicker value={model} onChange={onModelChange} />
      <span className="appbar__spacer" />
      <button
        className="appbar__icon-btn"
        type="button"
        aria-label="Share conversation"
        onClick={onOpenShare}
      >
        <ShareGlyph />
      </button>
      <button
        className="appbar__icon-btn"
        type="button"
        aria-label="Open settings"
        onClick={onOpenSettings}
      >
        <SettingsGlyph />
      </button>
      <button
        className="appbar__avatar"
        type="button"
        aria-label="Account menu"
      >
        <span>A</span>
      </button>
    </header>
  );
}

function MenuGlyph(): JSX.Element {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M3 6h18v2H3zm0 5h18v2H3zm0 5h18v2H3z" />
    </svg>
  );
}

function ShareGlyph(): JSX.Element {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M18 16a3 3 0 0 0-2.4 1.2l-7.05-4.11a3 3 0 0 0 0-2.18l7.05-4.11A3 3 0 1 0 15 5.03l-7.05 4.11a3 3 0 1 0 0 5.72l7.05 4.11A3 3 0 1 0 18 16z" />
    </svg>
  );
}

function SettingsGlyph(): JSX.Element {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M19.43 12.98c.04-.32.07-.65.07-.98s-.03-.66-.07-.98l2.11-1.65a.5.5 0 0 0 .12-.64l-2-3.46a.5.5 0 0 0-.61-.22l-2.49 1a7.03 7.03 0 0 0-1.69-.98l-.38-2.65A.5.5 0 0 0 14 2h-4a.5.5 0 0 0-.49.42l-.38 2.65c-.61.25-1.18.57-1.69.98l-2.49-1a.5.5 0 0 0-.61.22l-2 3.46a.5.5 0 0 0 .12.64l2.11 1.65c-.04.32-.07.66-.07.98s.03.65.07.98L2.46 14.63a.5.5 0 0 0-.12.64l2 3.46a.5.5 0 0 0 .61.22l2.49-1c.51.41 1.08.73 1.69.98l.38 2.65A.5.5 0 0 0 10 22h4a.5.5 0 0 0 .49-.42l.38-2.65c.61-.25 1.18-.57 1.69-.98l2.49 1a.5.5 0 0 0 .61-.22l2-3.46a.5.5 0 0 0-.12-.64l-2.11-1.65zM12 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7z" />
    </svg>
  );
}
