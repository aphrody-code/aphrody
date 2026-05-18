// SPDX-License-Identifier: Apache-2.0
"use client";

import { useCallback, useId, useState, type JSX } from "react";

export type GeminiModelId = "gemini-2.0-flash" | "gemini-2.5-flash" | "gemini-2.5-pro";

const MODELS: ReadonlyArray<{ id: GeminiModelId; label: string }> = [
  { id: "gemini-2.0-flash", label: "2.0 Flash" },
  { id: "gemini-2.5-flash", label: "2.5 Flash" },
  { id: "gemini-2.5-pro", label: "2.5 Pro" },
];

interface ModelPickerProps {
  value: GeminiModelId;
  onChange(next: GeminiModelId): void;
}

export function ModelPicker({ value, onChange }: ModelPickerProps): JSX.Element {
  const [open, setOpen] = useState(false);
  const menuId = useId();
  const label = MODELS.find((m) => m.id === value)?.label ?? "Pick model";
  const toggle = useCallback(() => setOpen((o) => !o), []);
  const close = useCallback(() => setOpen(false), []);

  return (
    <div style={{ position: "relative" }}>
      <button
        className="model-picker"
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={menuId}
        onClick={toggle}
        onBlur={() => setTimeout(close, 120)}
      >
        {label}
      </button>
      {open ? (
        <ul
          id={menuId}
          role="listbox"
          aria-label="Gemini model"
          style={{
            position: "absolute",
            top: "calc(100% + 6px)",
            left: 0,
            margin: 0,
            padding: "6px 0",
            listStyle: "none",
            background: "var(--md-sys-color-surface-container-high)",
            border: "1px solid var(--md-sys-color-outline-variant)",
            borderRadius: "var(--md-sys-shape-corner-medium)",
            boxShadow: "0 8px 24px rgba(0,0,0,0.12)",
            minWidth: 160,
            zIndex: 8,
          }}
        >
          {MODELS.map((m) => (
            <li key={m.id}>
              <button
                role="option"
                aria-selected={m.id === value}
                type="button"
                onClick={() => {
                  onChange(m.id);
                  close();
                }}
                style={{
                  display: "block",
                  width: "100%",
                  padding: "10px 16px",
                  background:
                    m.id === value
                      ? "color-mix(in srgb, var(--gemini-brand-blue) 14%, transparent)"
                      : "transparent",
                  border: "none",
                  textAlign: "left",
                  font: "inherit",
                  color: "var(--md-sys-color-on-surface)",
                  cursor: "pointer",
                }}
              >
                {m.label}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
