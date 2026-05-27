// SPDX-License-Identifier: Apache-2.0
import { Injectable, NgZone, signal } from "@angular/core";

/* Minimal typings for the Web Speech API (not in lib.dom for webkit-prefixed
   builds). Only the surface we use is declared. */
interface SpeechRecognitionResultLike {
    readonly isFinal: boolean;
    readonly length: number;
    item(index: number): { transcript: string };
    [index: number]: { transcript: string };
}
interface SpeechRecognitionEventLike extends Event {
    readonly resultIndex: number;
    readonly results: {
        readonly length: number;
        item(index: number): SpeechRecognitionResultLike;
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

/**
 * fr-FR voice dictation via the browser-native Web Speech API
 * (`webkitSpeechRecognition`), exactly as Gemini's mic does. The recognised
 * transcript is streamed back to the caller; `listening` drives the mic UI.
 */
@Injectable({ providedIn: "root" })
export class SpeechService {
    readonly listening = signal(false);
    readonly supported = signal(false);

    private recognition: SpeechRecognitionLike | null = null;
    private onTranscript: ((text: string, isFinal: boolean) => void) | null = null;

    constructor(private zone: NgZone) {
        const ctor = this.resolveCtor();
        this.supported.set(ctor !== null);
    }

    private resolveCtor(): SpeechRecognitionCtor | null {
        const w = globalThis as unknown as {
            SpeechRecognition?: SpeechRecognitionCtor;
            webkitSpeechRecognition?: SpeechRecognitionCtor;
        };
        return w.SpeechRecognition ?? w.webkitSpeechRecognition ?? null;
    }

    /**
     * Start dictation. `cb(text, isFinal)` fires for interim + final results.
     * Returns false if speech recognition is unsupported.
     */
    start(cb: (text: string, isFinal: boolean) => void): boolean {
        const ctor = this.resolveCtor();
        if (!ctor) {
            return false;
        }
        this.stop();
        this.onTranscript = cb;

        const rec = new ctor();
        rec.lang = "fr-FR";
        rec.continuous = true;
        rec.interimResults = true;

        rec.onresult = (ev: SpeechRecognitionEventLike) => {
            let interim = "";
            let final = "";
            for (let i = ev.resultIndex; i < ev.results.length; i++) {
                const result = ev.results[i];
                const chunk = result[0]?.transcript ?? "";
                if (result.isFinal) {
                    final += chunk;
                } else {
                    interim += chunk;
                }
            }
            this.zone.run(() => {
                if (final) {
                    this.onTranscript?.(final, true);
                } else if (interim) {
                    this.onTranscript?.(interim, false);
                }
            });
        };

        rec.onerror = () => {
            this.zone.run(() => this.listening.set(false));
        };
        rec.onend = () => {
            this.zone.run(() => this.listening.set(false));
        };

        this.recognition = rec;
        try {
            rec.start();
            this.listening.set(true);
            return true;
        } catch {
            this.listening.set(false);
            return false;
        }
    }

    stop(): void {
        if (this.recognition) {
            try {
                this.recognition.stop();
            } catch {
                // already stopped
            }
            this.recognition = null;
        }
        this.listening.set(false);
    }

    toggle(cb: (text: string, isFinal: boolean) => void): void {
        if (this.listening()) {
            this.stop();
        } else {
            this.start(cb);
        }
    }

    // ── Speech synthesis (the "voice" half of voice-to-voice) ────────────────

    /** True when `speechSynthesis` exists (drives the voice-mode availability). */
    readonly canSpeak = signal(
        typeof globalThis !== "undefined" && "speechSynthesis" in globalThis,
    );
    /** True while an utterance is being spoken aloud. */
    readonly speaking = signal(false);

    private get synth(): SpeechSynthesis | null {
        const g = globalThis as unknown as { speechSynthesis?: SpeechSynthesis };
        return g.speechSynthesis ?? null;
    }

    /**
     * Pick the best French voice, preferring a local (offline) one so no network
     * is touched. Voices load asynchronously on some engines; callers should
     * tolerate a null and let the engine fall back to its default.
     */
    private pickFrenchVoice(): SpeechSynthesisVoice | null {
        const synth = this.synth;
        if (!synth) return null;
        const voices = synth.getVoices();
        if (voices.length === 0) return null;
        const fr = voices.filter((v) => v.lang?.toLowerCase().startsWith("fr"));
        return fr.find((v) => v.localService) ?? fr[0] ?? null;
    }

    /**
     * Speak `text` in French. Resolves when the utterance ends (or errors). Any
     * in-flight utterance is cancelled first so replies never overlap.
     */
    speak(text: string): Promise<void> {
        const synth = this.synth;
        const clean = text.trim();
        if (!synth || !clean) {
            return Promise.resolve();
        }
        return new Promise<void>((resolve) => {
            try {
                synth.cancel();
            } catch {
                // ignore — nothing was speaking
            }
            const utter = new SpeechSynthesisUtterance(clean);
            utter.lang = "fr-FR";
            utter.rate = 1;
            utter.pitch = 1;
            const voice = this.pickFrenchVoice();
            if (voice) {
                utter.voice = voice;
                utter.lang = voice.lang;
            }
            const done = () => {
                this.zone.run(() => this.speaking.set(false));
                resolve();
            };
            utter.onend = done;
            utter.onerror = done;
            this.zone.run(() => this.speaking.set(true));
            try {
                synth.speak(utter);
            } catch {
                done();
            }
        });
    }

    /** Stop any current speech immediately. */
    shutUp(): void {
        try {
            this.synth?.cancel();
        } catch {
            // ignore
        }
        this.speaking.set(false);
    }
}
