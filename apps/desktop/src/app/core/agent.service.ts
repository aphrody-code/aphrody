// SPDX-License-Identifier: Apache-2.0
import { Injectable } from "@angular/core";

/** An allowlisted agent config file (agy/antigravity share the gemini config). */
export type ConfigWhich = "agy" | "antigravity" | "claude" | "aphrody-mcp";
/** An allowlisted persona file. */
export type PersonaWhich = "soul" | "identity";
/** An allowlisted stat target (read-only size/mtime probe). */
export type StatWhich = "memory-sqlite" | "agent-config" | "channels" | "soul" | "identity";
/** An allowlisted sibling binary. */
export type SiblingBin = "aphrody-skills" | "aphrody-mcp";

/** Mirrors the Rust `ConfigDoc` returned by read/write_agent_config. */
export interface ConfigDoc {
    which: string;
    path: string;
    exists: boolean;
    content: string;
}

/** Mirrors the Rust `PersonaDoc` returned by read/write_agent_home. */
export interface PersonaDoc {
    which: string;
    path: string;
    workspace: string;
    exists: boolean;
    content: string;
}

/** Mirrors the Rust `ChannelsState`. Secrets are NEVER present in `values`. */
export interface ChannelsState {
    path: string;
    exists: boolean;
    /** key -> is a non-empty value stored on disk. */
    configured: Record<string, boolean>;
    /** NON-secret handles only (X_HANDLE, channel/room IDs, homeserver…). */
    values: Record<string, string>;
}

/** A single channel field update (empty value clears the key). */
export interface ChannelUpdate {
    key: string;
    value: string;
}

/** Mirrors the Rust `StatResult`. */
export interface StatResult {
    which: string;
    path: string;
    exists: boolean;
    size: number;
    modified_secs: number;
}

/** Mirrors the Rust `ExecResult` (sibling exec). */
export interface SiblingResult {
    code: number;
    stdout: string;
    stderr: string;
}

type TauriInvoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

/** Raised when a host-only command is called in a plain browser (no Tauri). */
export class HostUnavailableError extends Error {
    constructor() {
        super("Cette fonction nécessite l'application de bureau (binaire aphrody local).");
        this.name = "HostUnavailableError";
    }
}

/**
 * Bridge to the desktop shell's scoped, path-allowlisted Tauri commands:
 * agent config (agy/antigravity/claude/aphrody-mcp), the agent-home persona
 * files (soul/identity), the messaging channels store, file stats, and the
 * sibling binaries (aphrody-skills / aphrody-mcp). Every target is resolved and
 * validated in Rust — the UI never passes a raw filesystem path.
 *
 * In a plain browser (no Tauri) each method rejects with
 * {@link HostUnavailableError}; the views render an honest disabled state.
 */
@Injectable({ providedIn: "root" })
export class AgentService {
    private invokeFn: TauriInvoke | null = null;
    private invokeReady: Promise<void>;

    constructor() {
        this.invokeReady = this.resolveInvoke();
    }

    /** True when running inside the Tauri webview (real backend available). */
    get isTauri(): boolean {
        return (
            typeof (globalThis as Record<string, unknown>)["__TAURI_INTERNALS__"] !== "undefined"
        );
    }

    private async resolveInvoke(): Promise<void> {
        if (!this.isTauri) {
            return;
        }
        try {
            const mod = (await import("@tauri-apps/api/core")) as { invoke: TauriInvoke };
            this.invokeFn = mod.invoke;
        } catch {
            this.invokeFn = null;
        }
    }

    private async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
        await this.invokeReady;
        if (!this.invokeFn) {
            throw new HostUnavailableError();
        }
        return this.invokeFn<T>(cmd, args);
    }

    // ── Agent config (JSON editors) ──────────────────────────────────────────

    readAgentConfig(which: ConfigWhich): Promise<ConfigDoc> {
        return this.invoke<ConfigDoc>("read_agent_config", { which });
    }

    writeAgentConfig(which: ConfigWhich, content: string): Promise<ConfigDoc> {
        return this.invoke<ConfigDoc>("write_agent_config", { which, content });
    }

    // ── Agent-home persona (soul / identity) ─────────────────────────────────

    readAgentHome(which: PersonaWhich): Promise<PersonaDoc> {
        return this.invoke<PersonaDoc>("read_agent_home", { which });
    }

    writeAgentHome(which: PersonaWhich, content: string): Promise<PersonaDoc> {
        return this.invoke<PersonaDoc>("write_agent_home", { which, content });
    }

    // ── Channels ─────────────────────────────────────────────────────────────

    readChannels(): Promise<ChannelsState> {
        return this.invoke<ChannelsState>("read_channels");
    }

    writeChannels(updates: ChannelUpdate[]): Promise<ChannelsState> {
        return this.invoke<ChannelsState>("write_channels", { updates });
    }

    // ── Stat ─────────────────────────────────────────────────────────────────

    stat(which: StatWhich): Promise<StatResult> {
        return this.invoke<StatResult>("aphrody_stat", { which });
    }

    // ── Sibling binaries ─────────────────────────────────────────────────────

    siblingExec(bin: SiblingBin, args: string[]): Promise<SiblingResult> {
        return this.invoke<SiblingResult>("aphrody_sibling_exec", { bin, args });
    }

    // ── Voice (native TTS + STT via aphrody-voice) ────────────────────────────

    /**
     * Synthesise `text` via ElevenLabs and return the audio as a base64-encoded
     * MPEG string (play via `new Audio("data:audio/mpeg;base64,<result>")`).
     *
     * Rejects with `"no-tts-key"` when no ElevenLabs key is configured; the
     * caller should fall back to `speechSynthesis.speak()`.
     *
     * Throws {@link HostUnavailableError} in a plain browser.
     */
    voiceSynthesize(text: string, voiceId?: string): Promise<string> {
        return this.invoke<string>("voice_synthesize", {
            text,
            voiceId: voiceId ?? null,
        });
    }

    /**
     * Transcribe base64-encoded audio via OpenAI Whisper or ElevenLabs STT.
     *
     * `audioB64` is the base64-encoded audio blob (from `MediaRecorder`).
     * `mime` is the MIME type string (e.g. `"audio/webm;codecs=opus"`).
     *
     * Rejects with `"no-stt-key"` when no API key is configured; the caller
     * should fall back to the Web Speech recogniser.
     *
     * Throws {@link HostUnavailableError} in a plain browser.
     */
    voiceTranscribe(audioB64: string, mime: string): Promise<string> {
        return this.invoke<string>("voice_transcribe", { audioB64, mime });
    }
}
