// SPDX-License-Identifier: Apache-2.0
import { AfterViewChecked, Component, ElementRef, ViewChild, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatButtonModule } from "@angular/material/button";
import { MatIconModule } from "@angular/material/icon";
import { MatMenuModule } from "@angular/material/menu";
import { MatTooltipModule } from "@angular/material/tooltip";

import { AphrodyService } from "../../core/aphrody.service";
import { SettingsService } from "../../core/settings.service";
import { SpeechService } from "../../core/speech.service";
import {
    Attachment,
    AttachmentKind,
    MAX_INLINE_CHARS,
    formatBytes,
    githubRawCandidates,
    isTextFile,
    kindIcon,
    kindLabel,
} from "./attachment";
import { VoiceOverlayComponent } from "./voice-overlay.component";

interface Turn {
    role: "user" | "assistant";
    text: string;
    /** assistant turn is still streaming/awaiting the reply */
    pending?: boolean;
}

interface Suggestion {
    icon: string;
    label: string;
    prompt: string;
}

// Gemini 3.5 Flash is the chat model. It is served by the keyless Gemini web
// transport (the default backend, see SettingsService) -- the agy/Cloud Code
// token path does NOT serve gemini-3.5-flash (404), so the GUI defaults to
// `--web`. The id below is passed to `aphrody chat --model`; the web backend
// maps it to its "3.5 Flash" model, and on the agy fallback it is ignored in
// favour of a served model.
const MODELS = [{ id: "gemini-3.5-flash", label: "Gemini 3.5 Flash" }] as const;

/** Accept filters for the native file picker, per media menu entry. */
const ACCEPT: Record<"image" | "video" | "audio", string> = {
    image: "image/*",
    video: "video/*",
    audio: "audio/*",
};

/**
 * The Assistant view = the Gemini empty-state hero + composer + conversation.
 * On submit the prompt (plus any attached context) becomes a user turn and
 * `aphrody chat --prompt` streams the reply, revealed character by character.
 * The composer mic opens a full voice-to-voice overlay; a "+" menu attaches
 * media / links / files whose textual content is honestly inlined as context.
 */
@Component({
    selector: "app-assistant",
    imports: [
        FormsModule,
        MatButtonModule,
        MatIconModule,
        MatMenuModule,
        MatTooltipModule,
        VoiceOverlayComponent,
    ],
    templateUrl: "./assistant.component.html",
    styleUrl: "./assistant.component.scss",
})
export class AssistantComponent implements AfterViewChecked {
    private readonly aphrody = inject(AphrodyService);
    private readonly settings = inject(SettingsService);
    readonly speech = inject(SpeechService);

    @ViewChild("scrollPane") private scrollPane?: ElementRef<HTMLElement>;
    @ViewChild("promptInput") private promptInput?: ElementRef<HTMLTextAreaElement>;
    @ViewChild("fileInput") private fileInput?: ElementRef<HTMLInputElement>;

    readonly turns = signal<Turn[]>([]);
    readonly draft = signal("");
    readonly busy = signal(false);
    readonly models = MODELS;
    readonly model = signal<(typeof MODELS)[number]>(MODELS[0]);

    /** Composer attachments (chips above the textarea). */
    readonly attachments = signal<Attachment[]>([]);
    /** True while the voice-to-voice overlay is mounted. */
    readonly voiceMode = signal(false);
    /** Which inline link-entry panel is open (null = none). */
    readonly linkPanel = signal<"github" | "url" | "drive" | null>(null);

    /** Pending native-picker accept filter + kind for the hidden <input>. */
    private pickerKind: AttachmentKind = "file";

    private shouldScroll = false;
    private idCounter = 0;

    // Re-export helpers for the template.
    readonly kindIcon = kindIcon;
    readonly kindLabel = kindLabel;

    readonly suggestions: Suggestion[] = [
        {
            icon: "edit",
            label: "Rédiger un message",
            prompt: "Aide-moi à rédiger un e-mail professionnel et clair.",
        },
        {
            icon: "school",
            label: "Expliquer un sujet",
            prompt: "Explique-moi un concept compliqué avec des mots simples.",
        },
        {
            icon: "lightbulb",
            label: "Trouver des idées",
            prompt: "Propose-moi des idées originales pour un projet.",
        },
        {
            icon: "summarize",
            label: "Résumer un texte",
            prompt: "Résume ce texte en quelques points clés : ",
        },
    ];

    get hasConversation(): boolean {
        return this.turns().length > 0;
    }

    ngAfterViewChecked(): void {
        if (this.shouldScroll && this.scrollPane) {
            this.scrollPane.nativeElement.scrollTop = this.scrollPane.nativeElement.scrollHeight;
            this.shouldScroll = false;
        }
    }

    onInput(value: string): void {
        this.draft.set(value);
        this.autoGrow();
    }

    private autoGrow(): void {
        const el = this.promptInput?.nativeElement;
        if (!el) return;
        el.style.height = "auto";
        el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
    }

    onKeydown(ev: KeyboardEvent): void {
        if (ev.key === "Enter" && !ev.shiftKey) {
            ev.preventDefault();
            void this.send();
        }
    }

    useSuggestion(s: Suggestion): void {
        this.draft.set(s.prompt);
        void this.send();
    }

    // ── Voice mode ─────────────────────────────────────────────────────────

    /** Open the full voice-to-voice overlay. */
    openVoice(): void {
        if (!this.speech.supported()) {
            return;
        }
        this.speech.stop();
        this.voiceMode.set(true);
    }

    closeVoice(): void {
        this.voiceMode.set(false);
    }

    /** Short dictation into the textarea (keeps the classic mic affordance). */
    dictate(): void {
        this.speech.toggle((text, isFinal) => {
            const base = this.draft();
            if (isFinal) {
                this.draft.set(`${base}${text} `);
            } else {
                this.draft.set(`${base}${text}`);
            }
            this.autoGrow();
        });
    }

    // ── Attachments ──────────────────────────────────────────────────────────

    private nextId(): string {
        this.idCounter += 1;
        return `att-${this.idCounter}`;
    }

    /** Trigger the native file picker for a media kind. */
    pickMedia(kind: "image" | "video" | "audio" | "file"): void {
        this.pickerKind = kind;
        const input = this.fileInput?.nativeElement;
        if (!input) return;
        input.accept = kind === "file" ? "" : ACCEPT[kind];
        input.value = "";
        input.click();
    }

    /** Handle the chosen file(s): inline text/code, reference binary media. */
    async onFilesChosen(ev: Event): Promise<void> {
        const input = ev.target as HTMLInputElement;
        const files = Array.from(input.files ?? []);
        for (const file of files) {
            const textual = this.pickerKind === "file" && isTextFile(file.name, file.type);
            const att: Attachment = {
                id: this.nextId(),
                kind: this.pickerKind,
                name: file.name,
                detail: `${formatBytes(file.size)}${file.type ? ` · ${file.type}` : ""}`,
                loading: textual,
            };
            this.attachments.update((a) => [...a, att]);
            if (textual) {
                try {
                    const raw = await file.text();
                    const text =
                        raw.length > MAX_INLINE_CHARS ? raw.slice(0, MAX_INLINE_CHARS) : raw;
                    const truncated = raw.length > MAX_INLINE_CHARS;
                    this.patchAttachment(att.id, {
                        loading: false,
                        text,
                        note: truncated ? "contenu tronqué (trop long)" : undefined,
                    });
                } catch (err) {
                    this.patchAttachment(att.id, {
                        loading: false,
                        note: `lecture impossible : ${String(err)}`,
                    });
                }
            } else {
                // Binary media: referenced honestly, NOT decoded as vision input.
                this.patchAttachment(att.id, {
                    note: "média binaire référencé (chat texte uniquement — pas d'analyse multimodale)",
                });
            }
        }
    }

    /** Add a GitHub link: fetch the raw file / README and inline its text. */
    async addGithub(url: string): Promise<void> {
        const trimmed = url.trim();
        if (!trimmed) return;
        const candidates = githubRawCandidates(trimmed);
        const att: Attachment = {
            id: this.nextId(),
            kind: "github",
            name: trimmed,
            detail: "GitHub",
            loading: candidates.length > 0,
        };
        this.attachments.update((a) => [...a, att]);
        if (candidates.length === 0) {
            this.patchAttachment(att.id, {
                loading: false,
                note: "URL GitHub non reconnue (attendu : lien blob ou dépôt)",
            });
            return;
        }
        await this.fetchInto(att.id, candidates);
    }

    /** Add an arbitrary URL: fetch the page and inline its visible text. */
    async addUrl(url: string): Promise<void> {
        const trimmed = url.trim();
        if (!trimmed) return;
        let normalized = trimmed;
        if (!/^https?:\/\//i.test(normalized)) {
            normalized = `https://${normalized}`;
        }
        let host = normalized;
        try {
            host = new URL(normalized).hostname;
        } catch {
            // keep the raw string as detail
        }
        const att: Attachment = {
            id: this.nextId(),
            kind: "url",
            name: trimmed,
            detail: host,
            loading: true,
        };
        this.attachments.update((a) => [...a, att]);
        await this.fetchInto(att.id, [normalized], true);
    }

    // ── Inline link-entry panel ──────────────────────────────────────────────

    openLinkPanel(kind: "github" | "url" | "drive"): void {
        this.linkPanel.set(kind);
    }

    closeLinkPanel(): void {
        this.linkPanel.set(null);
    }

    linkPlaceholder(): string {
        switch (this.linkPanel()) {
            case "github":
                return "https://github.com/owner/repo ou .../blob/main/fichier.rs";
            case "drive":
                return "https://drive.google.com/file/d/…";
            default:
                return "https://exemple.com/article";
        }
    }

    /** Commit the inline link panel input to the right attachment handler. */
    submitLink(value: string): void {
        const kind = this.linkPanel();
        const v = value.trim();
        if (kind && v) {
            if (kind === "github") {
                void this.addGithub(v);
            } else if (kind === "url") {
                void this.addUrl(v);
            } else {
                this.addDrive(v);
            }
        }
        this.closeLinkPanel();
    }

    /** Add a Google Drive link (paste-only; no OAuth, no fetch). */
    addDrive(link: string): void {
        const trimmed = link.trim();
        if (!trimmed) return;
        this.attachments.update((a) => [
            ...a,
            {
                id: this.nextId(),
                kind: "drive",
                name: trimmed,
                detail: "Google Drive (lien)",
                note: "lien référencé tel quel (aucune authentification Drive)",
            },
        ]);
    }

    /**
     * Fetch the first reachable candidate URL and inline its text. `stripHtml`
     * converts an HTML page to rough visible text. Failure is non-fatal: the chip
     * keeps a clear note and the link is still referenced in the prompt.
     */
    private async fetchInto(id: string, urls: string[], stripHtml = false): Promise<void> {
        for (const u of urls) {
            try {
                const res = await fetch(u, { redirect: "follow" });
                if (!res.ok) continue;
                let body = await res.text();
                const ct = res.headers.get("content-type") ?? "";
                if (stripHtml || ct.includes("text/html")) {
                    body = this.htmlToText(body);
                }
                body = body.trim();
                if (!body) continue;
                const truncated = body.length > MAX_INLINE_CHARS;
                this.patchAttachment(id, {
                    loading: false,
                    text: truncated ? body.slice(0, MAX_INLINE_CHARS) : body,
                    detail: `${this.attachmentById(id)?.detail ?? ""} · ${formatBytes(body.length)}`,
                    note: truncated ? "contenu tronqué (trop long)" : undefined,
                });
                return;
            } catch {
                // try the next candidate
            }
        }
        this.patchAttachment(id, {
            loading: false,
            note: "contenu non récupérable (réseau bloqué ou ressource privée) — lien tout de même référencé",
        });
    }

    /** Strip tags/scripts from HTML, returning collapsed visible text. */
    private htmlToText(html: string): string {
        const noScript = html
            .replace(/<script[\s\S]*?<\/script>/gi, " ")
            .replace(/<style[\s\S]*?<\/style>/gi, " ")
            .replace(/<!--[\s\S]*?-->/g, " ");
        const text = noScript.replace(/<[^>]+>/g, " ");
        const decoded = text
            .replace(/&nbsp;/g, " ")
            .replace(/&amp;/g, "&")
            .replace(/&lt;/g, "<")
            .replace(/&gt;/g, ">")
            .replace(/&quot;/g, '"')
            .replace(/&#39;/g, "'");
        return decoded
            .replace(/[ \t]+/g, " ")
            .replace(/\n\s*\n\s*\n+/g, "\n\n")
            .trim();
    }

    private attachmentById(id: string): Attachment | undefined {
        return this.attachments().find((a) => a.id === id);
    }

    private patchAttachment(id: string, patch: Partial<Attachment>): void {
        this.attachments.update((list) => list.map((a) => (a.id === id ? { ...a, ...patch } : a)));
    }

    removeAttachment(id: string): void {
        this.attachments.update((a) => a.filter((x) => x.id !== id));
    }

    /** Build the context block honestly folded into the prompt. */
    private buildContext(): string {
        const items = this.attachments();
        if (items.length === 0) return "";
        const blocks: string[] = [];
        for (const a of items) {
            const header = `[${kindLabel(a.kind)}] ${a.name}`;
            if (a.text) {
                blocks.push(`${header}\n\`\`\`\n${a.text}\n\`\`\``);
            } else {
                const reason = a.note ?? "référencé sans contenu textuel";
                blocks.push(`${header} (${reason})`);
            }
        }
        return `Contexte fourni par l'utilisateur :\n\n${blocks.join("\n\n")}`;
    }

    // ── Send ───────────────────────────────────────────────────────────────

    async send(): Promise<void> {
        const prompt = this.draft().trim();
        const hasAtt = this.attachments().length > 0;
        if ((!prompt && !hasAtt) || this.busy()) {
            return;
        }
        this.speech.stop();
        this.busy.set(true);

        const context = this.buildContext();
        const userVisible = prompt || "(voir le contexte joint)";
        const fullPrompt = context ? `${context}\n\n---\n\n${prompt}` : prompt;

        this.draft.set("");
        this.attachments.set([]);
        if (this.promptInput) {
            this.promptInput.nativeElement.style.height = "auto";
        }

        // Push the user turn (visible text only) + a pending assistant turn.
        this.turns.update((t) => [
            ...t,
            { role: "user", text: userVisible },
            { role: "assistant", text: "", pending: true },
        ]);
        this.shouldScroll = true;

        let reply: string;
        try {
            const res = await this.aphrody.exec([
                "chat",
                "--prompt",
                fullPrompt,
                "--model",
                this.model().id,
                ...this.settings.extraChatArgs(),
            ]);
            reply = (res.stdout || res.stderr || "").trim() || "(réponse vide)";
            if (res.code !== 0 && !res.stdout) {
                reply = `La commande a échoué (code ${res.code}).\n\n${res.stderr}`.trim();
            }
        } catch (err) {
            reply = `Erreur lors de l'appel au backend aphrody : ${String(err)}`;
        }

        await this.revealReply(reply);
        this.busy.set(false);
    }

    /** Reveal the assistant reply Gemini-style, a few chars per frame. */
    private async revealReply(full: string): Promise<void> {
        const idx = this.turns().length - 1;
        const step = Math.max(1, Math.round(full.length / 240));
        for (let i = 0; i <= full.length; i += step) {
            const slice = full.slice(0, i);
            this.turns.update((t) => {
                const copy = [...t];
                copy[idx] = { role: "assistant", text: slice, pending: i < full.length };
                return copy;
            });
            this.shouldScroll = true;
            await new Promise((r) => setTimeout(r, 12));
        }
        this.turns.update((t) => {
            const copy = [...t];
            copy[idx] = { role: "assistant", text: full, pending: false };
            return copy;
        });
        this.shouldScroll = true;
    }

    newChat(): void {
        this.speech.stop();
        this.turns.set([]);
        this.draft.set("");
        this.attachments.set([]);
        this.busy.set(false);
    }
}
