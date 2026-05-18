// SPDX-License-Identifier: Apache-2.0
"use client";

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type JSX,
  type PointerEvent as ReactPointerEvent,
} from "react";

import { useVoiceInput } from "../../hooks/useVoiceInput";
import { VoiceWaveform } from "./VoiceWaveform";

interface MicButtonProps {
  onTranscript(text: string): void;
  disabled?: boolean;
}

const HOLD_THRESHOLD_MS = 280;

export function MicButton({
  onTranscript,
  disabled = false,
}: MicButtonProps): JSX.Element {
  const { recording, analyser, start, stop, error } = useVoiceInput();
  const [held, setHeld] = useState(false);
  const heldRef = useRef(false);
  const downTsRef = useRef(0);

  useEffect(() => {
    heldRef.current = held;
  }, [held]);

  const finalize = useCallback(async () => {
    const transcript = await stop();
    if (transcript) {
      onTranscript(transcript);
    }
  }, [onTranscript, stop]);

  const onPointerDown = useCallback(
    (ev: ReactPointerEvent<HTMLButtonElement>) => {
      if (disabled) return;
      ev.preventDefault();
      downTsRef.current = performance.now();
      setHeld(true);
      void start();
    },
    [disabled, start],
  );

  const onPointerUp = useCallback(
    (ev: ReactPointerEvent<HTMLButtonElement>) => {
      if (disabled) return;
      ev.preventDefault();
      const elapsed = performance.now() - downTsRef.current;
      setHeld(false);
      if (elapsed < HOLD_THRESHOLD_MS) {
        // Treat as click-toggle: if currently recording, stop & transcribe.
        // If not recording (short tap during release), still attempt finalize
        // because start() ran on pointerdown.
        void finalize();
      } else {
        // Push-to-hold release: finalize.
        void finalize();
      }
    },
    [disabled, finalize],
  );

  const onClickToggle = useCallback(() => {
    // Pure-click fallback (no pointer support): toggle.
    if (recording) {
      void finalize();
    } else {
      void start();
    }
  }, [recording, finalize, start]);

  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
      {recording ? <VoiceWaveform analyser={analyser} /> : null}
      <button
        type="button"
        className={`mic-btn${recording || held ? " mic-btn--recording" : ""}`}
        aria-pressed={recording}
        aria-label={recording ? "Stop recording and transcribe" : "Start voice input"}
        disabled={disabled}
        title={error ?? undefined}
        onPointerDown={onPointerDown}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onClick={onClickToggle}
      >
        <MicGlyph />
      </button>
    </span>
  );
}

function MicGlyph(): JSX.Element {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 14a3 3 0 0 0 3-3V5a3 3 0 1 0-6 0v6a3 3 0 0 0 3 3zm5-3a5 5 0 1 1-10 0H5a7 7 0 0 0 6 6.92V21h2v-3.08A7 7 0 0 0 19 11h-2z" />
    </svg>
  );
}
