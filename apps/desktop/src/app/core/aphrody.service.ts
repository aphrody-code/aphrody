// SPDX-License-Identifier: Apache-2.0
import { Injectable } from "@angular/core";

/** Result of an in-process aphrody CLI run, mirroring the Rust `ExecResult`. */
export interface ExecResult {
    code: number;
    stdout: string;
    stderr: string;
}

/** Static shell + host metadata, mirroring the Rust `Meta` command. */
export interface Meta {
    app_version: string;
    target_os: string;
    target_arch: string;
    family: string;
}

type TauriInvoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

/**
 * Bridge to the aphrody CLI. In the Tauri shell every call goes through the
 * `aphrody_exec` `#[tauri::command]`, which runs the CLI IN-PROCESS
 * (Rust -> Rust via `aphrody::run_captured`). In a plain browser (ng serve, no
 * Tauri) it falls back to `POST /api/run` if reachable, else a clearly-labelled
 * offline stub so the UI stays demonstrable.
 */
@Injectable({ providedIn: "root" })
export class AphrodyService {
    private invokeFn: TauriInvoke | null = null;
    private invokeReady: Promise<void>;

    constructor() {
        this.invokeReady = this.resolveInvoke();
    }

    /** True when running inside the Tauri webview (real backend available). */
    get isTauri(): boolean {
        // Tauri v2 marks the global with `__TAURI_INTERNALS__`.
        return (
            typeof (globalThis as Record<string, unknown>)["__TAURI_INTERNALS__"] !== "undefined"
        );
    }

    private async resolveInvoke(): Promise<void> {
        if (!this.isTauri) {
            return;
        }
        try {
            // Dynamic import so the bundle still builds when the package is treated
            // as optional in a pure-web context.
            const mod = (await import("@tauri-apps/api/core")) as { invoke: TauriInvoke };
            this.invokeFn = mod.invoke;
        } catch {
            this.invokeFn = null;
        }
    }

    /**
     * Run an aphrody command. `args` is the argv WITHOUT the program name, e.g.
     * `["chat", "--prompt", "bonjour"]`.
     */
    async exec(args: string[]): Promise<ExecResult> {
        await this.invokeReady;

        if (this.invokeFn) {
            return this.invokeFn<ExecResult>("aphrody_exec", { args });
        }

        // Web/dev fallback: try a local dev API, else return an offline stub.
        return this.execWebFallback(args);
    }

    /** Fetch the shell/host metadata (Tauri only; stubbed on web). */
    async meta(): Promise<Meta> {
        await this.invokeReady;
        if (this.invokeFn) {
            return this.invokeFn<Meta>("aphrody_meta");
        }
        return {
            app_version: "web-dev",
            target_os: this.guessWebOs(),
            target_arch: "wasm/web",
            family: "web",
        };
    }

    private guessWebOs(): string {
        const ua = navigator.userAgent.toLowerCase();
        if (ua.includes("windows")) return "windows";
        if (ua.includes("mac")) return "macos";
        if (ua.includes("linux")) return "linux";
        return "web";
    }

    private async execWebFallback(args: string[]): Promise<ExecResult> {
        try {
            const res = await fetch("/api/run", {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify({ args }),
            });
            if (res.ok) {
                return (await res.json()) as ExecResult;
            }
        } catch {
            // network unavailable -> offline stub below
        }

        // Offline stub: echo the command so the UI flow is fully exercisable
        // without a backend (ng serve / static dist preview).
        const joined = ["aphrody", ...args].join(" ");
        if (args[0] === "chat") {
            const promptIdx = args.indexOf("--prompt");
            const prompt = promptIdx >= 0 ? (args[promptIdx + 1] ?? "") : "";
            return {
                code: 0,
                stdout:
                    `[mode hors-ligne — backend aphrody indisponible dans le navigateur]\n\n` +
                    `Votre message : « ${prompt} »\n\n` +
                    `Dans l'application de bureau (Tauri), ce message serait envoyé à ` +
                    `Gemini 3.5 Flash via la commande « aphrody chat --prompt ». ` +
                    `Lancez l'app empaquetée pour obtenir une vraie réponse.`,
                stderr: "",
            };
        }
        return {
            code: 0,
            stdout: `[hors-ligne] La commande « ${joined} » s'exécuterait via le binaire aphrody dans l'app de bureau.`,
            stderr: "",
        };
    }
}
