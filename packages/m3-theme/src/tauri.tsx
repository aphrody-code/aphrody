// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import { useM3Theme } from "./react.js";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke: (cmd: string, args?: any) => Promise<any>;
      transformCallback: (callback?: (response: any) => void, once?: boolean) => number;
      unregisterCallback: (id: number) => void;
    };
  }
}

/**
 * A custom React hook that automatically synchronizes the M3 theme mode
 * (light/dark) back to the Tauri native window context.
 */
export function useM3TauriThemeSync() {
  const { resolvedTheme } = useM3Theme();
  const [isTauri, setIsTauri] = useState(false);

  useEffect(() => {
    if (typeof window !== "undefined" && window.__TAURI_INTERNALS__) {
      setIsTauri(true);
      
      // Update the Tauri native window theme
      const syncTheme = async () => {
        try {
          // Tauri 2 window set_theme accepts the parameter 'value' rather than 'theme'
          await window.__TAURI_INTERNALS__?.invoke("plugin:window|set_theme", {
            value: resolvedTheme,
          });
        } catch (err) {
          // Fallback or ignore if the theme plugin/API is restricted
          console.debug("Tauri window theme sync bypassed:", err);
        }
      };
      
      syncTheme();
    }
  }, [resolvedTheme]);

  return { isTauri };
}

export interface M3TauriTitlebarProps {
  /** The title to display in the titlebar. */
  title?: string;
  /** Custom logo or icon component to display on the left. */
  logo?: React.ReactNode;
  /** Force show even if not running inside Tauri (useful for preview/styling). */
  forceShow?: boolean;
  /** Custom style overrides. */
  style?: React.CSSProperties;
  /** Right-aligned custom actions/elements. */
  extraActions?: React.ReactNode;
}

/**
 * A highly polished, M3-themed custom titlebar component for Tauri 2 windows.
 * Hides automatically when running in a normal web browser.
 */
export function M3TauriTitlebar({
  title,
  logo,
  forceShow = false,
  style,
  extraActions,
}: M3TauriTitlebarProps) {
  const { isTauri } = useM3TauriThemeSync();
  const [isMaximized, setIsMaximized] = useState(false);

  const shouldRender = forceShow || isTauri;

  useEffect(() => {
    if (!shouldRender || typeof window === "undefined" || !window.__TAURI_INTERNALS__) return;

    // Track window maximize state
    const checkMaximized = async () => {
      try {
        const maximized = await window.__TAURI_INTERNALS__?.invoke("plugin:window|is_maximized");
        setIsMaximized(!!maximized);
      } catch (err) {
        // Ignored
      }
    };

    checkMaximized();

    // Resize triggers when window maximizing or resizing operations occur
    window.addEventListener("resize", checkMaximized);
    return () => {
      window.removeEventListener("resize", checkMaximized);
    };
  }, [shouldRender]);

  if (!shouldRender) return null;

  const handleMinimize = () => {
    window.__TAURI_INTERNALS__?.invoke("plugin:window|minimize").catch(() => {});
  };

  const handleMaximize = async () => {
    try {
      if (isMaximized) {
        await window.__TAURI_INTERNALS__?.invoke("plugin:window|unmaximize");
        setIsMaximized(false);
      } else {
        await window.__TAURI_INTERNALS__?.invoke("plugin:window|maximize");
        setIsMaximized(true);
      }
    } catch (err) {
      // Fallback
    }
  };

  const handleClose = () => {
    window.__TAURI_INTERNALS__?.invoke("plugin:window|close").catch(() => {});
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        height: "36px",
        background: "var(--md-sys-color-surface-container, #f3f3fa)",
        color: "var(--md-sys-color-on-surface, #1b1b22)",
        userSelect: "none",
        fontFamily: "system-ui, -apple-system, sans-serif",
        fontSize: "12px",
        fontWeight: 500,
        borderBottom: "1px solid var(--md-sys-color-outline-variant, #c7c6cf)",
        boxSizing: "border-box",
        ...style,
      }}
    >
      {/* Drag region covering the titlebar */}
      <div
        data-tauri-drag-region
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          right: 0,
          height: "36px",
          zIndex: 1,
        }}
      />

      {/* Left items: Logo & Title */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          paddingLeft: "12px",
          zIndex: 2,
        }}
      >
        {logo}
        <span>{title}</span>
      </div>

      {/* Right items: Controls */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          height: "100%",
          zIndex: 2,
        }}
      >
        {extraActions}

        {/* Minimize Button */}
        <button
          onClick={handleMinimize}
          title="Minimize"
          style={{
            background: "transparent",
            border: "none",
            color: "inherit",
            cursor: "pointer",
            width: "46px",
            height: "100%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            transition: "background-color 150ms ease",
          }}
          onMouseOver={(e) => {
            e.currentTarget.style.backgroundColor = "rgba(0, 0, 0, 0.08)";
          }}
          onMouseOut={(e) => {
            e.currentTarget.style.backgroundColor = "transparent";
          }}
        >
          <svg width="10" height="1" viewBox="0 0 10 1">
            <line x1="0" y1="0.5" x2="10" y2="0.5" stroke="currentColor" strokeWidth="1" />
          </svg>
        </button>

        {/* Maximize / Restore Button */}
        <button
          onClick={handleMaximize}
          title={isMaximized ? "Restore" : "Maximize"}
          style={{
            background: "transparent",
            border: "none",
            color: "inherit",
            cursor: "pointer",
            width: "46px",
            height: "100%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            transition: "background-color 150ms ease",
          }}
          onMouseOver={(e) => {
            e.currentTarget.style.backgroundColor = "rgba(0, 0, 0, 0.08)";
          }}
          onMouseOut={(e) => {
            e.currentTarget.style.backgroundColor = "transparent";
          }}
        >
          {isMaximized ? (
            <svg width="10" height="10" viewBox="0 0 10 10">
              <path
                d="M2.5,1.5 L2.5,2.5 L1.5,2.5 L1.5,8.5 L7.5,8.5 L7.5,7.5 L8.5,7.5 L8.5,1.5 Z M7.5,2.5 L7.5,7.5 L2.5,7.5 L2.5,2.5 Z"
                fill="none"
                stroke="currentColor"
                strokeWidth="1"
              />
            </svg>
          ) : (
            <svg width="10" height="10" viewBox="0 0 10 10">
              <rect x="1.5" y="1.5" width="7" height="7" fill="none" stroke="currentColor" strokeWidth="1" />
            </svg>
          )}
        </button>

        {/* Close Button */}
        <button
          onClick={handleClose}
          title="Close"
          style={{
            background: "transparent",
            border: "none",
            color: "inherit",
            cursor: "pointer",
            width: "46px",
            height: "100%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            transition: "background-color 150ms ease, color 150ms ease",
          }}
          onMouseOver={(e) => {
            e.currentTarget.style.backgroundColor = "var(--md-sys-color-error, #ba1a1a)";
            e.currentTarget.style.color = "var(--md-sys-color-on-error, #ffffff)";
          }}
          onMouseOut={(e) => {
            e.currentTarget.style.backgroundColor = "transparent";
            e.currentTarget.style.color = "inherit";
          }}
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M1.5,1.5 L8.5,8.5 M8.5,1.5 L1.5,8.5" stroke="currentColor" strokeWidth="1" />
          </svg>
        </button>
      </div>
    </div>
  );
}
