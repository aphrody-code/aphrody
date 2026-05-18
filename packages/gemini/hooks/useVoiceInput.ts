// SPDX-License-Identifier: Apache-2.0
"use client";

import { useCallback, useRef, useState } from "react";

export interface UseVoiceInputResult {
  recording: boolean;
  analyser: AnalyserNode | null;
  error: string | null;
  start(): Promise<void>;
  /**
   * Stop the recorder, POST the captured audio to /api/stt, return the
   * transcript text. Returns null if no audio was captured / the request
   * failed.
   */
  stop(): Promise<string | null>;
}

interface RecorderState {
  stream: MediaStream;
  recorder: MediaRecorder;
  audioCtx: AudioContext;
  analyser: AnalyserNode;
  chunks: Blob[];
}

export function useVoiceInput(): UseVoiceInputResult {
  const [recording, setRecording] = useState(false);
  const [analyser, setAnalyser] = useState<AnalyserNode | null>(null);
  const [error, setError] = useState<string | null>(null);
  const stateRef = useRef<RecorderState | null>(null);

  const start = useCallback(async () => {
    if (stateRef.current) return;
    setError(null);
    if (
      typeof navigator === "undefined" ||
      !navigator.mediaDevices?.getUserMedia
    ) {
      setError("getUserMedia is unavailable in this browser");
      return;
    }
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const AudioCtxCtor =
        window.AudioContext ??
        (window as unknown as { webkitAudioContext?: typeof AudioContext })
          .webkitAudioContext;
      if (!AudioCtxCtor) {
        throw new Error("AudioContext unavailable");
      }
      const audioCtx = new AudioCtxCtor();
      const src = audioCtx.createMediaStreamSource(stream);
      const an = audioCtx.createAnalyser();
      an.fftSize = 256;
      src.connect(an);

      const recorder = new MediaRecorder(stream);
      const chunks: Blob[] = [];
      recorder.addEventListener("dataavailable", (ev) => {
        if (ev.data && ev.data.size > 0) chunks.push(ev.data);
      });
      stateRef.current = { stream, recorder, audioCtx, analyser: an, chunks };
      setAnalyser(an);
      recorder.start();
      setRecording(true);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    }
  }, []);

  const stop = useCallback(async (): Promise<string | null> => {
    const state = stateRef.current;
    if (!state) return null;
    stateRef.current = null;

    const done = new Promise<void>((resolve) => {
      state.recorder.addEventListener(
        "stop",
        () => {
          resolve();
        },
        { once: true },
      );
    });
    if (state.recorder.state !== "inactive") {
      state.recorder.stop();
    } else {
      // Already stopped — resolve immediately.
    }
    await done;

    state.stream.getTracks().forEach((t) => t.stop());
    void state.audioCtx.close().catch(() => {});
    setAnalyser(null);
    setRecording(false);

    if (state.chunks.length === 0) return null;
    const mimeType = state.recorder.mimeType || "audio/webm";
    const blob = new Blob(state.chunks, { type: mimeType });
    const form = new FormData();
    form.append("audio", blob, `mic.${blob.type.split("/")[1] ?? "webm"}`);
    try {
      const response = await fetch("/api/stt", {
        method: "POST",
        body: form,
      });
      const payload = (await response.json()) as
        | { text: string }
        | { error: string };
      if (!response.ok || "error" in payload) {
        const message = "error" in payload ? payload.error : `HTTP ${response.status}`;
        setError(message);
        return null;
      }
      return (payload as { text: string }).text;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      return null;
    }
  }, []);

  return { recording, analyser, error, start, stop };
}
