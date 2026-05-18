// SPDX-License-Identifier: Apache-2.0
"use client";

import type { JSX } from "react";

interface SettingsDrawerProps {
  open: boolean;
  onClose: () => void;
  autoTTS: boolean;
  onAutoTTSChange: (value: boolean) => void;
}

export function SettingsDrawer({ open, onClose, autoTTS, onAutoTTSChange }: SettingsDrawerProps): JSX.Element | null {
  if (!open) return null;

  return (
    <>
      <div 
        className="settings-scrim"
        onClick={onClose}
        style={{
          position: "fixed",
          inset: 0,
          background: "rgba(0,0,0,0.5)",
          zIndex: 40,
        }}
        aria-hidden="true"
      />
      <div
        className="settings-drawer"
        role="dialog"
        aria-label="Settings"
        style={{
          position: "fixed",
          top: 0,
          right: 0,
          bottom: 0,
          width: 320,
          background: "var(--md-sys-color-surface-container-high)",
          zIndex: 50,
          padding: "24px 16px",
          display: "flex",
          flexDirection: "column",
          gap: 24,
          boxShadow: "-8px 0 24px rgba(0,0,0,0.1)",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <h2 style={{ margin: 0, fontSize: 20, fontWeight: 500 }}>Settings</h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close settings"
            style={{
              background: "transparent",
              border: "none",
              color: "var(--md-sys-color-on-surface)",
              cursor: "pointer",
              fontSize: 24,
            }}
          >
            &times;
          </button>
        </div>

        <section style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <h3 style={{ margin: 0, fontSize: 14, color: "var(--md-sys-color-primary)", fontWeight: 600 }}>Voice Output</h3>
          <label style={{ display: "flex", alignItems: "center", gap: 12, cursor: "pointer", fontSize: 15 }}>
            <input 
              type="checkbox" 
              checked={autoTTS}
              onChange={(e) => onAutoTTSChange(e.target.checked)}
              style={{ width: 18, height: 18, accentColor: "var(--md-sys-color-primary)" }}
            />
            Auto-play assistant responses (TTS)
          </label>
          <p style={{ margin: 0, fontSize: 13, color: "var(--md-sys-color-on-surface-variant)" }}>
            Automatically read the assistant's reply aloud when generation completes. Default is OFF for privacy.
          </p>
        </section>
      </div>
    </>
  );
}
