// SPDX-License-Identifier: Apache-2.0
//
// Text-to-speech for assistant replies, built on the browser Web Speech API
// (`speechSynthesis`). Isolated and minimal: a toggle (persisted) plus a
// `speak()` that reads a single finished assistant message in French (fr-FR),
// after lightly stripping Markdown so punctuation/code fences aren't read aloud.

import { useCallback, useEffect, useRef, useState } from "react";

const TTS_KEY = "aphrody-tts-enabled";

/** Strip the Markdown the bubbles render so speech stays clean. */
export function stripForSpeech(md: string): string {
  return md
    .replace(/```[\s\S]*?```/g, " (bloc de code) ") // fenced code
    .replace(/`([^`]+)`/g, "$1") // inline code
    .replace(/!\[[^\]]*\]\([^)]*\)/g, "") // images
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1") // links → text
    .replace(/[*_#>~]/g, "") // emphasis / headings / quotes
    .replace(/\s+/g, " ")
    .trim();
}

export interface TtsController {
  supported: boolean;
  enabled: boolean;
  toggle: () => void;
  speak: (text: string) => void;
  cancel: () => void;
}

export function useTts(): TtsController {
  const supported =
    typeof window !== "undefined" && "speechSynthesis" in window;

  const [enabled, setEnabled] = useState<boolean>(() => {
    if (typeof localStorage === "undefined") return false;
    return localStorage.getItem(TTS_KEY) === "1";
  });

  const voiceRef = useRef<SpeechSynthesisVoice | null>(null);

  useEffect(() => {
    if (!supported) return;
    const pick = () => {
      const voices = window.speechSynthesis.getVoices();
      voiceRef.current =
        voices.find((v) => v.lang?.toLowerCase().startsWith("fr")) ?? voices[0] ?? null;
    };
    pick();
    window.speechSynthesis.addEventListener("voiceschanged", pick);
    return () => window.speechSynthesis.removeEventListener("voiceschanged", pick);
  }, [supported]);

  const cancel = useCallback(() => {
    if (supported) window.speechSynthesis.cancel();
  }, [supported]);

  const speak = useCallback(
    (text: string) => {
      if (!supported || !enabled) return;
      const clean = stripForSpeech(text);
      if (!clean) return;
      window.speechSynthesis.cancel();
      const utter = new SpeechSynthesisUtterance(clean);
      if (voiceRef.current) utter.voice = voiceRef.current;
      utter.lang = "fr-FR";
      utter.rate = 1;
      window.speechSynthesis.speak(utter);
    },
    [supported, enabled],
  );

  const toggle = useCallback(() => {
    setEnabled((prev) => {
      const next = !prev;
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(TTS_KEY, next ? "1" : "0");
      }
      if (!next && supported) window.speechSynthesis.cancel();
      return next;
    });
  }, [supported]);

  return { supported, enabled, toggle, speak, cancel };
}
