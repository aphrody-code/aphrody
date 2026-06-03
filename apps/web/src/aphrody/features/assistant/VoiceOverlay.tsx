// Full-screen voice-to-voice overlay (React port of Angular VoiceOverlayComponent): browser SpeechRecognition feeds `aphrody chat`, replies spoken via speechSynthesis.

import { useCallback, useEffect, useRef, useState } from "react";
import { MdIcon } from "@aphrody/m3-react";
import { run } from "../../client.ts";

interface VoiceTurn {
  role: "user" | "assistant";
  text: string;
}

type Phase = "idle" | "listening" | "thinking" | "speaking";

// ── Minimal Web Speech API typings (webkit-prefixed builds are not in lib.dom) ─

interface SpeechRecognitionAlternative {
  readonly transcript: string;
}
interface SpeechRecognitionResultLike {
  readonly isFinal: boolean;
  readonly length: number;
  [index: number]: SpeechRecognitionAlternative;
}
interface SpeechRecognitionEventLike extends Event {
  readonly resultIndex: number;
  readonly results: {
    readonly length: number;
    [index: number]: SpeechRecognitionResultLike;
  };
}
interface SpeechRecognitionLike extends EventTarget {
  lang: string;
  continuous: boolean;
  interimResults: boolean;
  start(): void;
  stop(): void;
  abort(): void;
  onresult: ((ev: SpeechRecognitionEventLike) => void) | null;
  onerror: ((ev: Event) => void) | null;
  onend: ((ev: Event) => void) | null;
}
type SpeechRecognitionCtor = new () => SpeechRecognitionLike;

function resolveRecognitionCtor(): SpeechRecognitionCtor | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as {
    SpeechRecognition?: SpeechRecognitionCtor;
    webkitSpeechRecognition?: SpeechRecognitionCtor;
  };
  return w.SpeechRecognition ?? w.webkitSpeechRecognition ?? null;
}

function speechSynthesisAvailable(): boolean {
  return typeof window !== "undefined" && "speechSynthesis" in window;
}

export function VoiceOverlay({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [liveText, setLiveText] = useState("");
  const [lastReply, setLastReply] = useState("");
  const [errorText, setErrorText] = useState("");
  const [muted, setMuted] = useState(false);
  const [transcript, setTranscript] = useState<VoiceTurn[]>([]);

  const recognition = useRef<SpeechRecognitionLike | null>(null);
  const processing = useRef(false);
  const mutedRef = useRef(false);
  const destroyed = useRef(false);
  const supported = resolveRecognitionCtor() !== null;

  const stopRecognition = useCallback(() => {
    if (recognition.current) {
      try {
        recognition.current.stop();
      } catch {
        // already stopped
      }
      recognition.current = null;
    }
  }, []);

  const shutUp = useCallback(() => {
    if (speechSynthesisAvailable()) {
      try {
        window.speechSynthesis.cancel();
      } catch {
        // ignore
      }
    }
  }, []);

  const speak = useCallback((text: string): Promise<void> => {
    return new Promise<void>((resolve) => {
      if (!speechSynthesisAvailable() || !text) {
        resolve();
        return;
      }
      try {
        const utter = new SpeechSynthesisUtterance(text);
        utter.lang = "fr-FR";
        utter.onend = () => resolve();
        utter.onerror = () => resolve();
        window.speechSynthesis.cancel();
        window.speechSynthesis.speak(utter);
      } catch {
        resolve();
      }
    });
  }, []);

  // Forward declaration via ref so handleUtterance and beginListening can call
  // each other without a stale-closure cycle.
  const beginListeningRef = useRef<() => void>(() => {});

  const handleUtterance = useCallback(
    async (utterance: string) => {
      if (processing.current || destroyed.current) return;
      processing.current = true;
      stopRecognition();
      shutUp();
      setPhase("thinking");
      setErrorText("");
      setTranscript((t) => [...t, { role: "user", text: utterance }]);

      let reply: string;
      try {
        // `--web` is the default transport (real Gemini 3.5 Flash).
        const res = await run(["chat", "--prompt", utterance, "--web"]);
        reply = (res.stdout || res.stderr || "").trim() || "(réponse vide)";
        if (res.code !== 0 && !res.stdout) {
          reply = `La commande a échoué (code ${res.code}).`;
        }
      } catch (err) {
        reply = `Erreur lors de l'appel au backend aphrody : ${String(err)}`;
      }

      if (destroyed.current) return;
      setLastReply(reply);
      setLiveText("");
      setTranscript((t) => [...t, { role: "assistant", text: reply }]);

      setPhase("speaking");
      await speak(reply);

      processing.current = false;
      if (!destroyed.current && !mutedRef.current) {
        beginListeningRef.current();
      } else if (!destroyed.current) {
        setPhase("listening");
      }
    },
    [shutUp, speak, stopRecognition],
  );

  const beginListening = useCallback(() => {
    if (destroyed.current || mutedRef.current || processing.current) return;
    const ctor = resolveRecognitionCtor();
    if (!ctor) {
      setErrorText(
        "La reconnaissance vocale n'est pas disponible dans cet environnement (navigateur sans Web Speech API).",
      );
      setPhase("idle");
      return;
    }
    setPhase("listening");
    stopRecognition();

    const rec = new ctor();
    rec.lang = "fr-FR";
    rec.continuous = true;
    rec.interimResults = true;

    rec.onresult = (ev) => {
      let interim = "";
      let final = "";
      for (let i = ev.resultIndex; i < ev.results.length; i++) {
        const result = ev.results[i];
        const chunk = result[0]?.transcript ?? "";
        if (result.isFinal) final += chunk;
        else interim += chunk;
      }
      if (final) {
        const utterance = final.trim();
        setLiveText(utterance);
        if (utterance) void handleUtterance(utterance);
      } else if (interim) {
        setLiveText(interim);
      }
    };
    rec.onerror = () => {
      if (!processing.current) setPhase("idle");
    };
    rec.onend = () => {
      // Auto-restart while we are still actively listening (continuous loop).
      if (!destroyed.current && !mutedRef.current && !processing.current) {
        beginListeningRef.current();
      }
    };

    recognition.current = rec;
    try {
      rec.start();
    } catch {
      setErrorText("Impossible de démarrer le micro (permission refusée ?).");
      setPhase("idle");
    }
  }, [handleUtterance, stopRecognition]);

  useEffect(() => {
    beginListeningRef.current = beginListening;
  }, [beginListening]);

  // Mount / unmount the live session with the `open` prop.
  useEffect(() => {
    if (!open) return;
    destroyed.current = false;
    if (!resolveRecognitionCtor()) {
      setErrorText(
        "La reconnaissance vocale n'est pas disponible dans cet environnement (navigateur sans Web Speech API).",
      );
      setPhase("idle");
      return;
    }
    beginListening();
    return () => {
      destroyed.current = true;
      stopRecognition();
      shutUp();
    };
  }, [open, beginListening, stopRecognition, shutUp]);

  const close = useCallback(() => {
    destroyed.current = true;
    stopRecognition();
    shutUp();
    onClose();
  }, [onClose, shutUp, stopRecognition]);

  const toggleMute = () => {
    const next = !muted;
    setMuted(next);
    mutedRef.current = next;
    if (next) {
      stopRecognition();
      shutUp();
      setPhase("idle");
    } else if (!processing.current) {
      beginListening();
    }
  };

  if (!open) return null;

  const statusLabel = (): string => {
    if (errorText) return "Indisponible";
    switch (phase) {
      case "listening":
        return muted ? "En pause" : "Je vous écoute…";
      case "thinking":
        return "aphrody réfléchit…";
      case "speaking":
        return "aphrody répond…";
      default:
        return muted ? "En pause" : "Prêt";
    }
  };

  return (
    <div className="aph-voice-scrim" role="dialog" aria-modal="true" aria-label="Mode vocal">
      <button className="aph-voice__close" onClick={close} title="Fermer" aria-label="Fermer">
        <MdIcon>close</MdIcon>
      </button>

      <div className="aph-voice__stage">
        <div className="aph-voice__orb" data-phase={phase}>
          <span className="aph-voice__orb-core"></span>
          <span className="aph-voice__orb-ring aph-voice__ring1"></span>
          <span className="aph-voice__orb-ring aph-voice__ring2"></span>
          <span className="aph-voice__orb-ring aph-voice__ring3"></span>
        </div>

        <p className="aph-voice__status">{statusLabel()}</p>

        {!supported && (
          <p className="aph-voice__error">
            Voix indisponible : ce navigateur n'expose pas l'API Web Speech. Utilisez le clavier
            dans l'assistant.
          </p>
        )}

        {liveText && (
          <p className={`aph-voice__live${phase === "listening" ? " is-interim" : ""}`}>
            {liveText}
          </p>
        )}

        {lastReply && <p className="aph-voice__reply">{lastReply}</p>}

        {errorText && <p className="aph-voice__error">{errorText}</p>}
      </div>

      <div className="aph-voice__controls">
        <button
          className={`aph-voice__ctrl${muted ? " is-active" : ""}`}
          onClick={toggleMute}
          title={muted ? "Reprendre" : "Mettre en pause"}
          aria-label={muted ? "Reprendre" : "Pause"}
          disabled={!supported}
        >
          <MdIcon>{muted ? "mic_off" : "mic"}</MdIcon>
        </button>
        <button
          className="aph-voice__ctrl aph-voice__ctrl--stop"
          onClick={close}
          title="Terminer"
          aria-label="Terminer"
        >
          <MdIcon>call_end</MdIcon>
        </button>
      </div>

      {transcript.length > 0 && (
        <div className="aph-voice__transcript" aria-label="Transcription">
          {transcript.map((t, i) => (
            <div key={i} className={`aph-voice__t-line${t.role === "user" ? " is-user" : ""}`}>
              <span className="aph-voice__t-who">{t.role === "user" ? "Vous" : "aphrody"}</span>
              <span className="aph-voice__t-text">{t.text}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
