// SPDX-License-Identifier: Apache-2.0
"use client";

import {
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type FormEvent,
  type JSX,
  forwardRef,
} from "react";

import { MicButton } from "./MicButton";

export interface PromptBarHandle {
  /** Programmatically inject text (used by the suggestion chips). */
  setText(text: string): void;
  /** Focus the textarea. */
  focus(): void;
}

interface PromptBarProps {
  busy: boolean;
  onSubmit(text: string): void;
}

export const PromptBar = forwardRef<PromptBarHandle, PromptBarProps>(
  function PromptBar({ busy, onSubmit }, ref): JSX.Element {
    const [value, setValue] = useState("");
    const taRef = useRef<HTMLTextAreaElement | null>(null);

    const autoGrow = useCallback(() => {
      const ta = taRef.current;
      if (!ta) return;
      ta.style.height = "auto";
      ta.style.height = `${Math.min(200, ta.scrollHeight)}px`;
    }, []);

    useEffect(() => {
      autoGrow();
    }, [autoGrow, value]);

    useImperativeHandle(
      ref,
      () => ({
        setText(text: string): void {
          setValue(text);
          requestAnimationFrame(() => taRef.current?.focus());
        },
        focus(): void {
          taRef.current?.focus();
        },
      }),
      [],
    );

    const handleSubmit = useCallback(
      (ev: FormEvent<HTMLFormElement>) => {
        ev.preventDefault();
        const text = value.trim();
        if (!text || busy) return;
        onSubmit(text);
        setValue("");
      },
      [busy, onSubmit, value],
    );

    const onTranscript = useCallback(
      (text: string) => {
        const trimmed = text.trim();
        if (!trimmed) return;
        setValue(trimmed);
        if (!busy) {
          onSubmit(trimmed);
          setValue("");
        }
      },
      [busy, onSubmit],
    );

    const enabled = value.trim().length > 0 && !busy;

    return (
      <div className="prompt-wrap">
        <form className="prompt-bar" role="search" onSubmit={handleSubmit}>
          <button
            type="button"
            className="prompt-bar__icon-btn"
            aria-label="Attach file"
            title="Attach (not wired in this build)"
            disabled
          >
            +
          </button>
          <textarea
            ref={taRef}
            className="prompt-bar__textarea"
            rows={1}
            placeholder="Ask Gemini"
            autoFocus
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                if (enabled) {
                  onSubmit(value.trim());
                  setValue("");
                }
              }
            }}
            disabled={busy}
          />
          <MicButton onTranscript={onTranscript} disabled={busy} />
          <button
            type="submit"
            className={`prompt-bar__send${enabled ? " prompt-bar__send--enabled" : ""}`}
            aria-label={busy ? "Sending" : "Send message"}
            disabled={!enabled}
          >
            <span style={{ display: "inline-block" }}>{"↑"}</span>
          </button>
        </form>
        <p className="prompt-foot">
          Gemini may display inaccurate info, including about people. Double-check responses.
        </p>
      </div>
    );
  },
);
