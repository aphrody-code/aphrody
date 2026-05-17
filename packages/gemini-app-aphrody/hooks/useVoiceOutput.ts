// SPDX-License-Identifier: Apache-2.0
"use client";

import { useCallback, useEffect, useRef, useState } from "react";

export interface UseVoiceOutputResult {
  playing: boolean;
  error: string | null;
  /** POST `text` to /api/tts and play the returned audio. */
  speak(text: string): Promise<void>;
  /** Barge-in: cancel ongoing playback immediately. */
  cancel(): void;
}

export function useVoiceOutput(): UseVoiceOutputResult {
  const [playing, setPlaying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const urlRef = useRef<string | null>(null);

  const cancel = useCallback(() => {
    const audio = audioRef.current;
    if (audio) {
      audio.pause();
      audio.src = "";
      audioRef.current = null;
    }
    const url = urlRef.current;
    if (url) {
      URL.revokeObjectURL(url);
      urlRef.current = null;
    }
    setPlaying(false);
  }, []);

  useEffect(() => () => cancel(), [cancel]);

  const speak = useCallback(
    async (text: string) => {
      const trimmed = text.trim();
      if (!trimmed) return;
      cancel();
      setError(null);
      try {
        const response = await fetch("/api/tts", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ text: trimmed }),
        });
        if (!response.ok) {
          const detail = (await response.json().catch(() => ({}))) as {
            error?: string;
          };
          setError(detail.error ?? `HTTP ${response.status}`);
          return;
        }
        const blob = await response.blob();
        const url = URL.createObjectURL(blob);
        urlRef.current = url;
        const audio = new Audio(url);
        audioRef.current = audio;
        audio.addEventListener("ended", () => {
          setPlaying(false);
          cancel();
        });
        audio.addEventListener("error", () => {
          setError("audio playback failed");
          setPlaying(false);
        });
        setPlaying(true);
        await audio.play();
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setPlaying(false);
      }
    },
    [cancel],
  );

  return { playing, error, speak, cancel };
}
