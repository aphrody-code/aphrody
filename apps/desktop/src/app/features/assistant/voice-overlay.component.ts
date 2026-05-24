// SPDX-License-Identifier: Apache-2.0
import { Component, OnDestroy, OnInit, inject, output, signal } from "@angular/core";
import { MatIconModule } from "@angular/material/icon";
import { MatTooltipModule } from "@angular/material/tooltip";

import { AgentService, HostUnavailableError } from "../../core/agent.service";
import { AphrodyService } from "../../core/aphrody.service";
import { SettingsService } from "../../core/settings.service";
import { SpeechService } from "../../core/speech.service";

/** One exchange recorded in the live voice session, surfaced as a transcript. */
interface VoiceTurn {
  role: "user" | "assistant";
  text: string;
}

/** The voice loop's current phase, driving the orb animation + status label. */
type Phase = "idle" | "listening" | "thinking" | "speaking";

/**
 * Full-screen voice-to-voice overlay (Gemini Live style). Continuous fr-FR
 * speech recognition feeds an animated listening orb; on a FINAL utterance the
 * text is sent to `aphrody chat` (through the selected backend) and the reply
 * is spoken aloud.
 *
 * Audio path (two-tier):
 *
 *  STT: MediaRecorder -> base64 -> voice_transcribe (native Whisper / ElevenLabs)
 *       falls back to Web Speech recognition when no API key is configured or
 *       when running in a plain browser (no Tauri host).
 *
 *  TTS: voice_synthesize (native ElevenLabs) -> Audio(<data:audio/mpeg;base64>)
 *       falls back to speechSynthesis.speak() (fr-FR) when no ElevenLabs key
 *       is configured or when running in a plain browser.
 *
 * Existing overlay UX (animated orb, live transcript, mute/stop) is preserved.
 * Native voice is a transparent enhancement when API keys are present.
 */
@Component({
  selector: "app-voice-overlay",
  imports: [MatIconModule, MatTooltipModule],
  template: `
    <div class="voice-scrim" role="dialog" aria-modal="true" aria-label="Mode vocal">
      <button class="close" (click)="close()" matTooltip="Fermer" aria-label="Fermer">
        <mat-icon class="material-symbols-outlined">close</mat-icon>
      </button>

      <div class="stage">
        <div class="orb" [attr.data-phase]="phase()">
          <span class="orb-core"></span>
          <span class="orb-ring ring1"></span>
          <span class="orb-ring ring2"></span>
          <span class="orb-ring ring3"></span>
        </div>

        <p class="status">{{ statusLabel() }}</p>

        @if (nativeVoice()) {
          <span class="voice-badge" matTooltip="Voix native : ElevenLabs / Whisper">
            <mat-icon class="material-symbols-outlined">graphic_eq</mat-icon>
            Voix native
          </span>
        }

        @if (liveText()) {
          <p class="live" [class.interim]="phase() === 'listening'">{{ liveText() }}</p>
        }

        @if (lastReply()) {
          <p class="reply">{{ lastReply() }}</p>
        }

        @if (errorText()) {
          <p class="error">{{ errorText() }}</p>
        }
      </div>

      <div class="controls">
        <button
          class="ctrl"
          [class.active]="muted()"
          (click)="toggleMute()"
          [matTooltip]="muted() ? 'Reprendre' : 'Mettre en pause'"
          [attr.aria-label]="muted() ? 'Reprendre' : 'Pause'"
        >
          <mat-icon class="material-symbols-outlined">{{ muted() ? "mic_off" : "mic" }}</mat-icon>
        </button>
        <button class="ctrl stop" (click)="close()" matTooltip="Terminer" aria-label="Terminer">
          <mat-icon class="material-symbols-outlined">call_end</mat-icon>
        </button>
      </div>

      @if (transcript().length > 0) {
        <div class="transcript" aria-label="Transcription">
          @for (t of transcript(); track $index) {
            <div class="t-line" [class.t-user]="t.role === 'user'">
              <span class="t-who">{{ t.role === "user" ? "Vous" : "aphrody" }}</span>
              <span class="t-text">{{ t.text }}</span>
            </div>
          }
        </div>
      }
    </div>
  `,
  styles: [
    `
      :host {
        position: fixed;
        inset: 0;
        z-index: 1000;
      }
      .voice-scrim {
        position: absolute;
        inset: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        background:
          radial-gradient(circle at 50% 38%, rgba(20, 32, 79, 0.55), transparent 60%),
          color-mix(in srgb, var(--mat-sys-surface) 92%, black);
        backdrop-filter: blur(8px);
      }
      .close {
        position: absolute;
        top: 18px;
        right: 18px;
        width: 44px;
        height: 44px;
        border: none;
        border-radius: 50%;
        background: color-mix(in srgb, var(--mat-sys-on-surface) 8%, transparent);
        color: var(--mat-sys-on-surface-variant);
        cursor: pointer;
        display: grid;
        place-items: center;
      }
      .close:hover {
        background: color-mix(in srgb, var(--mat-sys-on-surface) 14%, transparent);
      }
      .stage {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 18px;
        max-width: 620px;
        padding: 0 28px;
        text-align: center;
      }
      /* ── Animated orb ── */
      .orb {
        position: relative;
        width: 160px;
        height: 160px;
        display: grid;
        place-items: center;
      }
      .orb-core {
        width: 88px;
        height: 88px;
        border-radius: 50%;
        background: radial-gradient(circle at 35% 30%, #5b8dff, #2f63ff 55%, #14204f);
        box-shadow: 0 0 38px 8px rgba(66, 133, 244, 0.55);
        transition: transform 0.3s ease;
      }
      .orb-ring {
        position: absolute;
        border-radius: 50%;
        border: 2px solid rgba(91, 141, 255, 0.4);
        opacity: 0;
      }
      .ring1 {
        width: 110px;
        height: 110px;
      }
      .ring2 {
        width: 134px;
        height: 134px;
      }
      .ring3 {
        width: 158px;
        height: 158px;
      }
      /* Listening: gentle pulse + expanding rings */
      .orb[data-phase="listening"] .orb-core {
        animation: breathe 1.8s ease-in-out infinite;
      }
      .orb[data-phase="listening"] .orb-ring {
        animation: ripple 2.4s ease-out infinite;
      }
      .orb[data-phase="listening"] .ring2 {
        animation-delay: 0.6s;
      }
      .orb[data-phase="listening"] .ring3 {
        animation-delay: 1.2s;
      }
      /* Thinking: faster shimmer */
      .orb[data-phase="thinking"] .orb-core {
        animation: think 0.9s ease-in-out infinite;
      }
      /* Speaking: stronger glow */
      .orb[data-phase="speaking"] .orb-core {
        animation: speak 0.55s ease-in-out infinite alternate;
        box-shadow: 0 0 54px 14px rgba(66, 133, 244, 0.7);
      }
      @keyframes breathe {
        0%,
        100% {
          transform: scale(1);
        }
        50% {
          transform: scale(1.08);
        }
      }
      @keyframes ripple {
        0% {
          opacity: 0.6;
          transform: scale(0.7);
        }
        100% {
          opacity: 0;
          transform: scale(1.1);
        }
      }
      @keyframes think {
        0%,
        100% {
          transform: scale(0.96);
          filter: hue-rotate(0deg);
        }
        50% {
          transform: scale(1.03);
          filter: hue-rotate(25deg);
        }
      }
      @keyframes speak {
        from {
          transform: scale(1);
        }
        to {
          transform: scale(1.12);
        }
      }
      @media (prefers-reduced-motion: reduce) {
        .orb-core,
        .orb-ring {
          animation: none !important;
        }
      }
      .status {
        margin: 0;
        font-size: 18px;
        font-weight: 500;
        color: var(--mat-sys-on-surface);
      }
      .voice-badge {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        font-size: 11px;
        padding: 3px 10px 3px 6px;
        border-radius: 999px;
        background: color-mix(in srgb, var(--mat-sys-tertiary) 16%, transparent);
        color: var(--mat-sys-tertiary);
      }
      .voice-badge mat-icon {
        font-size: 14px;
        width: 14px;
        height: 14px;
      }
      .live {
        margin: 0;
        font-size: 15px;
        line-height: 1.5;
        color: var(--mat-sys-on-surface);
        min-height: 1.2em;
      }
      .live.interim {
        color: var(--mat-sys-on-surface-variant);
        font-style: italic;
      }
      .reply {
        margin: 0;
        font-size: 14px;
        line-height: 1.55;
        color: var(--mat-sys-on-surface-variant);
        max-height: 30vh;
        overflow-y: auto;
      }
      .error {
        margin: 0;
        font-size: 13px;
        color: var(--mat-sys-error);
      }
      .controls {
        display: flex;
        gap: 22px;
        margin-top: 40px;
      }
      .ctrl {
        width: 60px;
        height: 60px;
        border: none;
        border-radius: 50%;
        cursor: pointer;
        display: grid;
        place-items: center;
        background: var(--mat-sys-surface-container-highest);
        color: var(--mat-sys-on-surface);
        transition:
          background 0.15s,
          transform 0.1s;
      }
      .ctrl:hover {
        transform: scale(1.06);
      }
      .ctrl.active {
        background: var(--mat-sys-secondary-container);
        color: var(--mat-sys-on-secondary-container);
      }
      .ctrl.stop {
        background: var(--mat-sys-error);
        color: var(--mat-sys-on-error);
      }
      .ctrl mat-icon {
        font-size: 26px;
        width: 26px;
        height: 26px;
      }
      .transcript {
        position: absolute;
        bottom: 20px;
        left: 50%;
        transform: translateX(-50%);
        width: min(680px, 92%);
        max-height: 22vh;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 12px 16px;
        border-radius: 16px;
        background: color-mix(in srgb, var(--mat-sys-surface-container) 80%, transparent);
      }
      .t-line {
        display: flex;
        gap: 8px;
        font-size: 13px;
        line-height: 1.45;
      }
      .t-who {
        flex: 0 0 auto;
        font-weight: 600;
        color: var(--mat-sys-primary);
      }
      .t-user .t-who {
        color: var(--mat-sys-on-surface-variant);
      }
      .t-text {
        color: var(--mat-sys-on-surface);
        word-break: break-word;
      }
    `,
  ],
})
export class VoiceOverlayComponent implements OnInit, OnDestroy {
  private readonly aphrody = inject(AphrodyService);
  private readonly agent = inject(AgentService);
  private readonly settings = inject(SettingsService);
  readonly speech = inject(SpeechService);

  /** Emitted when the overlay should be torn down by the parent. */
  readonly closed = output<void>();

  readonly phase = signal<Phase>("idle");
  readonly liveText = signal("");
  readonly lastReply = signal("");
  readonly errorText = signal("");
  readonly muted = signal(false);
  readonly transcript = signal<VoiceTurn[]>([]);
  /** True when a native voice provider (ElevenLabs / Whisper) is active. */
  readonly nativeVoice = signal(false);

  private destroyed = false;
  /** Guards against re-entrant sends while a turn is in flight. */
  private processing = false;

  /** Active MediaRecorder for native STT capture path. */
  private mediaRecorder: MediaRecorder | null = null;
  /** Accumulated chunks from the current MediaRecorder recording. */
  private recordingChunks: Blob[] = [];
  /** MIME type reported by the active MediaRecorder. */
  private recordingMime = "audio/webm";

  ngOnInit(): void {
    if (!this.speech.supported()) {
      this.errorText.set("La reconnaissance vocale n'est pas disponible dans cet environnement.");
      this.phase.set("idle");
      return;
    }
    this.beginListening();
  }

  ngOnDestroy(): void {
    this.destroyed = true;
    this.stopMediaRecorder();
    this.speech.stop();
    this.speech.shutUp();
    this.stopNativeAudio();
  }

  statusLabel(): string {
    if (this.errorText()) return "Indisponible";
    switch (this.phase()) {
      case "listening":
        return this.muted() ? "En pause" : "Je vous écoute…";
      case "thinking":
        return "aphrody réfléchit…";
      case "speaking":
        return "aphrody répond…";
      default:
        return this.muted() ? "En pause" : "Prêt";
    }
  }

  /** Start (or restart) continuous listening; ignored when muted/processing. */
  private beginListening(): void {
    if (this.destroyed || this.muted() || this.processing) {
      return;
    }
    this.phase.set("listening");

    // Try native STT capture (MediaRecorder) when the agent service is Tauri.
    if (this.agent.isTauri && typeof navigator !== "undefined" && navigator.mediaDevices) {
      void this.beginNativeListening();
      return;
    }

    // Browser / no-host fallback: Web Speech API.
    this.beginWebSpeechListening();
  }

  // ── Native STT path (MediaRecorder + Whisper/ElevenLabs) ─────────────────────

  private async beginNativeListening(): Promise<void> {
    if (this.destroyed || this.muted() || this.processing) return;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      if (this.destroyed || this.muted()) {
        stream.getTracks().forEach((t) => t.stop());
        return;
      }
      const mime = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
        ? "audio/webm;codecs=opus"
        : MediaRecorder.isTypeSupported("audio/webm")
          ? "audio/webm"
          : "";
      this.recordingMime = mime || "audio/webm";
      const recorder = new MediaRecorder(stream, mime ? { mimeType: mime } : undefined);
      this.mediaRecorder = recorder;
      this.recordingChunks = [];

      recorder.ondataavailable = (ev) => {
        if (ev.data && ev.data.size > 0) {
          this.recordingChunks.push(ev.data);
        }
      };

      recorder.onstop = () => {
        stream.getTracks().forEach((t) => t.stop());
        if (!this.destroyed && !this.muted() && !this.processing) {
          void this.processNativeRecording();
        }
      };

      // Record in 3-second slices so we get a natural utterance boundary.
      recorder.start(3000);
      // Stop after 6 seconds to capture a full utterance and send it.
      setTimeout(() => {
        if (recorder.state === "recording") {
          recorder.stop();
        }
      }, 6000);
    } catch {
      // Mic permission denied or no microphone — fall back to Web Speech.
      this.beginWebSpeechListening();
    }
  }

  private async processNativeRecording(): Promise<void> {
    if (this.processing || this.destroyed) return;

    const chunks = this.recordingChunks;
    this.recordingChunks = [];
    this.mediaRecorder = null;

    if (!chunks.length) {
      // Nothing recorded; loop back.
      this.beginListening();
      return;
    }

    const blob = new Blob(chunks, { type: this.recordingMime });
    // Convert to base64 for the Tauri IPC.
    const b64 = await blobToBase64(blob);

    let utterance = "";
    try {
      utterance = await this.agent.voiceTranscribe(b64, this.recordingMime);
      utterance = utterance.trim();
    } catch (err) {
      const msg = String(err);
      if (msg === "no-stt-key" || err instanceof HostUnavailableError) {
        // No native STT key configured — fall back to Web Speech for this turn.
        this.beginWebSpeechListening();
        return;
      }
      // Transient API error: show briefly, then loop.
      this.errorText.set(`STT indisponible : ${msg}`);
      setTimeout(() => {
        if (!this.destroyed) {
          this.errorText.set("");
          this.beginListening();
        }
      }, 2500);
      return;
    }

    if (!utterance) {
      // Silence / nothing recognisable; loop.
      this.beginListening();
      return;
    }

    this.liveText.set(utterance);
    await this.handleUtterance(utterance);
  }

  private stopMediaRecorder(): void {
    if (this.mediaRecorder && this.mediaRecorder.state !== "inactive") {
      try {
        this.mediaRecorder.stop();
      } catch {
        // already stopped
      }
    }
    this.mediaRecorder = null;
  }

  // ── Web Speech fallback path ──────────────────────────────────────────────────

  private beginWebSpeechListening(): void {
    const ok = this.speech.start((text, isFinal) => {
      if (isFinal) {
        const utterance = text.trim();
        this.liveText.set(utterance);
        if (utterance) {
          void this.handleUtterance(utterance);
        }
      } else {
        this.liveText.set(text);
      }
    });
    if (!ok) {
      this.errorText.set("Impossible de démarrer le micro (permission refusée ?).");
      this.phase.set("idle");
    }
  }

  // ── Current native audio element (TTS playback) ───────────────────────────────

  private currentAudio: HTMLAudioElement | null = null;

  private stopNativeAudio(): void {
    if (this.currentAudio) {
      try {
        this.currentAudio.pause();
      } catch {
        // ignore
      }
      this.currentAudio = null;
    }
  }

  // ── Core turn handler ─────────────────────────────────────────────────────────

  /** Send a final utterance to the agent and speak the reply, then resume. */
  private async handleUtterance(utterance: string): Promise<void> {
    if (this.processing || this.destroyed) {
      return;
    }
    this.processing = true;
    this.stopMediaRecorder();
    this.speech.stop(); // pause the recogniser while we think + speak
    this.phase.set("thinking");
    this.errorText.set("");
    this.transcript.update((t) => [...t, { role: "user", text: utterance }]);

    let reply: string;
    try {
      const res = await this.aphrody.exec([
        "chat",
        "--prompt",
        utterance,
        ...this.settings.extraChatArgs(),
      ]);
      reply = (res.stdout || res.stderr || "").trim() || "(réponse vide)";
      if (res.code !== 0 && !res.stdout) {
        reply = `La commande a échoué (code ${res.code}).`;
      }
    } catch (err) {
      reply = `Erreur lors de l'appel au backend aphrody : ${String(err)}`;
    }

    if (this.destroyed) {
      return;
    }
    this.lastReply.set(reply);
    this.liveText.set("");
    this.transcript.update((t) => [...t, { role: "assistant", text: reply }]);

    // Speak the reply.
    this.phase.set("speaking");
    const spokenNatively = await this.speakNative(reply);
    if (!spokenNatively) {
      // Fallback: browser speechSynthesis (fr-FR).
      await this.speech.speak(reply);
    }

    this.processing = false;
    if (!this.destroyed && !this.muted()) {
      this.beginListening();
    } else if (!this.destroyed) {
      this.phase.set("listening");
    }
  }

  /**
   * Attempt native TTS via ElevenLabs (voice_synthesize).
   * Returns true when audio was played; false when the key is missing or the
   * call fails (-> caller falls back to Web Speech synthesis).
   */
  private async speakNative(text: string): Promise<boolean> {
    if (!this.agent.isTauri) return false;
    try {
      const b64 = await this.agent.voiceSynthesize(text);
      if (this.destroyed) return true; // closed mid-flight
      const audio = new Audio(`data:audio/mpeg;base64,${b64}`);
      this.currentAudio = audio;
      this.nativeVoice.set(true);
      await new Promise<void>((resolve) => {
        audio.onended = () => resolve();
        audio.onerror = () => resolve();
        audio.play().catch(() => resolve());
      });
      this.currentAudio = null;
      return true;
    } catch (err) {
      const msg = String(err);
      // "no-tts-key" or HostUnavailable -> silent fallback, no error shown.
      if (msg !== "no-tts-key" && !(err instanceof HostUnavailableError)) {
        this.errorText.set(`TTS natif indisponible : ${msg}`);
        setTimeout(() => {
          if (!this.destroyed) this.errorText.set("");
        }, 2500);
      }
      this.nativeVoice.set(false);
      return false;
    }
  }

  toggleMute(): void {
    const next = !this.muted();
    this.muted.set(next);
    if (next) {
      this.stopMediaRecorder();
      this.speech.stop();
      this.speech.shutUp();
      this.stopNativeAudio();
      this.phase.set("idle");
    } else if (!this.processing) {
      this.beginListening();
    }
  }

  close(): void {
    this.stopMediaRecorder();
    this.speech.stop();
    this.speech.shutUp();
    this.stopNativeAudio();
    this.closed.emit();
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Convert a Blob to a base64 string (data URL prefix stripped). */
function blobToBase64(blob: Blob): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      // Strip the "data:<mime>;base64," prefix.
      const comma = result.indexOf(",");
      resolve(comma >= 0 ? result.slice(comma + 1) : result);
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}
