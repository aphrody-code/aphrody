// SPDX-License-Identifier: Apache-2.0
"use client";

import type { JSX } from "react";

interface GemsPanelProps {
  open: boolean;
  onClose: () => void;
}

export function GemsPanel({ open, onClose }: GemsPanelProps): JSX.Element | null {
  if (!open) return null;

  return (
    <div
      style={{
        position: "absolute",
        top: 64,
        left: 72,
        bottom: 0,
        width: 300,
        background: "var(--md-sys-color-surface-container)",
        borderRight: "1px solid var(--md-sys-color-outline-variant)",
        zIndex: 10,
        padding: "24px 16px",
        display: "flex",
        flexDirection: "column",
        gap: 16,
        overflowY: "auto",
        boxShadow: "4px 0 16px rgba(0,0,0,0.05)"
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h2 style={{ margin: 0, fontSize: 18, fontWeight: 500 }}>Gems</h2>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close Gems panel"
          style={{ background: "transparent", border: "none", cursor: "pointer", fontSize: 24, color: "var(--md-sys-color-on-surface)" }}
        >
          &times;
        </button>
      </div>

      <p style={{ margin: 0, fontSize: 13, color: "var(--md-sys-color-on-surface-variant)" }}>
        Custom instructions and personas (mock).
      </p>

      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 8 }}>
        {["Pirate Bot", "Code Reviewer", "Translator", "Summarizer"].map((gem) => (
          <li key={gem}>
            <button
              style={{
                width: "100%",
                padding: "12px",
                background: "var(--md-sys-color-surface-container-high)",
                border: "none",
                borderRadius: "var(--md-sys-shape-corner-medium)",
                textAlign: "left",
                cursor: "pointer",
                color: "var(--md-sys-color-on-surface)",
                fontSize: 14,
                fontWeight: 500,
              }}
            >
              {gem}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
