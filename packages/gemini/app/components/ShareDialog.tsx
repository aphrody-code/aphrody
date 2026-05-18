// SPDX-License-Identifier: Apache-2.0
"use client";

import { useCallback, useState, type JSX } from "react";

interface ShareDialogProps {
  open: boolean;
  onClose: () => void;
}

export function ShareDialog({ open, onClose }: ShareDialogProps): JSX.Element | null {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    // In a real app, we'd persist the conversation and get a real URL.
    // Here we just copy a dummy URL.
    navigator.clipboard.writeText("https://gemini.google.com/share/draft").then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }).catch(() => {
      // ignore
    });
  }, []);

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
        className="share-dialog"
        role="dialog"
        aria-label="Share Conversation"
        style={{
          position: "fixed",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          width: "90%",
          maxWidth: 400,
          background: "var(--md-sys-color-surface-container-high)",
          zIndex: 50,
          padding: "24px",
          display: "flex",
          flexDirection: "column",
          gap: 16,
          boxShadow: "0 8px 24px rgba(0,0,0,0.2)",
          borderRadius: "var(--md-sys-shape-corner-extra-large)"
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <h2 style={{ margin: 0, fontSize: 20, fontWeight: 500 }}>Share conversation</h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close dialog"
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

        <p style={{ margin: 0, fontSize: 14, color: "var(--md-sys-color-on-surface-variant)" }}>
          Create a public link to share this conversation. Anyone with the link will be able to view it.
        </p>

        <div style={{ display: "flex", gap: 12, marginTop: 8 }}>
          <button
            type="button"
            onClick={handleCopy}
            style={{
              flex: 1,
              padding: "10px 16px",
              borderRadius: "var(--md-sys-shape-corner-full)",
              border: "none",
              background: copied ? "var(--md-sys-color-surface-container-highest)" : "var(--md-sys-color-primary)",
              color: copied ? "var(--md-sys-color-primary)" : "var(--md-sys-color-on-primary)",
              fontWeight: 500,
              cursor: "pointer",
              transition: "background 0.2s"
            }}
          >
            {copied ? "Link Copied!" : "Create public link"}
          </button>
        </div>
      </div>
    </>
  );
}
