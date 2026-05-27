// SPDX-License-Identifier: Apache-2.0
import { Component, OnInit, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatIconModule } from "@angular/material/icon";

import {
    AgentService,
    ChannelsState,
    ChannelUpdate,
    HostUnavailableError,
} from "../../../core/agent.service";

/** A single editable channel field descriptor. */
interface Field {
    key: string;
    label: string;
    /** True = masked password input (never echoed back in full). */
    secret: boolean;
    placeholder: string;
}

/** Channels grouped by the integration they belong to. */
interface Group {
    title: string;
    icon: string;
    blurb: string;
    fields: Field[];
}

const GROUPS: Group[] = [
    {
        title: "Discord",
        icon: "forum",
        blurb: "Agent hermes sur Discord (REST bot v10).",
        fields: [
            {
                key: "DISCORD_BOT_TOKEN",
                label: "Jeton du bot Discord",
                secret: true,
                placeholder: "Bot token",
            },
        ],
    },
    {
        title: "X (Twitter)",
        icon: "alternate_email",
        blurb: "Agent hermes sur X (authentification par cookie).",
        fields: [
            { key: "X_HANDLE", label: "Identifiant X", secret: false, placeholder: "monhandle" },
        ],
    },
    {
        title: "Voix (hermes voice-to-voice)",
        icon: "graphic_eq",
        blurb: "Transcription (Whisper) et synthèse (ElevenLabs) pour la boucle voix.",
        fields: [
            { key: "OPENAI_API_KEY", label: "Clé OpenAI (STT)", secret: true, placeholder: "sk-…" },
            {
                key: "ELEVENLABS_API_KEY",
                label: "Clé ElevenLabs (TTS)",
                secret: true,
                placeholder: "Clé API",
            },
        ],
    },
    {
        title: "Slack",
        icon: "tag",
        blurb: "Notifications via aphrody notify (chat.postMessage).",
        fields: [
            {
                key: "SLACK_BOT_TOKEN",
                label: "Jeton du bot Slack",
                secret: true,
                placeholder: "xoxb-…",
            },
            {
                key: "SLACK_CHANNEL",
                label: "Salon par défaut",
                secret: false,
                placeholder: "C012AB3CD",
            },
        ],
    },
    {
        title: "Telegram",
        icon: "send",
        blurb: "Notifications via aphrody notify (sendMessage).",
        fields: [
            {
                key: "TELEGRAM_BOT_TOKEN",
                label: "Jeton du bot Telegram",
                secret: true,
                placeholder: "123456:ABC…",
            },
            {
                key: "TELEGRAM_CHAT_ID",
                label: "Chat ID par défaut",
                secret: false,
                placeholder: "-1001234567890",
            },
        ],
    },
    {
        title: "Matrix",
        icon: "chat",
        blurb: "Notifications via aphrody notify (Client-Server API v3).",
        fields: [
            {
                key: "MATRIX_HOMESERVER",
                label: "Homeserver",
                secret: false,
                placeholder: "https://matrix.org",
            },
            {
                key: "MATRIX_ACCESS_TOKEN",
                label: "Jeton d'accès",
                secret: true,
                placeholder: "Access token",
            },
            {
                key: "MATRIX_USER_ID",
                label: "User ID",
                secret: false,
                placeholder: "@moi:matrix.org",
            },
            {
                key: "MATRIX_ROOM_ID",
                label: "Room ID par défaut",
                secret: false,
                placeholder: "!abc:matrix.org",
            },
        ],
    },
];

/**
 * Channels tab: configure messaging integration credentials, persisted to
 * `~/.aphrody/channels.json` (outside the repo) via the scoped write command.
 * Secrets are masked — the UI only ever learns whether a secret is set (the Rust
 * side returns booleans, never the values), and a secret left untouched is
 * preserved on save. Non-secret handles (X handle, channel/room IDs) are shown
 * in full.
 */
@Component({
    selector: "app-channels-tab",
    imports: [FormsModule, MatIconModule],
    template: `
        <section class="card intro">
            <h2>Canaux de messagerie</h2>
            <p class="hint">
                Identifiants des intégrations (hermes, notifications). Stockés en local dans
                <code>~/.aphrody/channels.json</code>, hors du dépôt. Les secrets sont masqués :
                laissez un champ secret vide pour conserver la valeur existante.
            </p>
            @if (state(); as s) {
                <p class="path mono">{{ s.path }}</p>
            }
        </section>

        @if (hostUnavailable()) {
            <section class="card">
                <p class="warn">La configuration des canaux nécessite l'application de bureau.</p>
            </section>
        } @else if (loadError()) {
            <section class="card">
                <p class="warn">{{ loadError() }}</p>
            </section>
        } @else if (state()) {
            @for (g of groups; track g.title) {
                <section class="card">
                    <div class="group-head">
                        <mat-icon class="material-symbols-outlined">{{ g.icon }}</mat-icon>
                        <div>
                            <h3>{{ g.title }}</h3>
                            <p>{{ g.blurb }}</p>
                        </div>
                        @if (groupConfigured(g)) {
                            <span class="tag ok">configuré</span>
                        }
                    </div>
                    @for (f of g.fields; track f.key) {
                        <label class="field">
                            <span class="f-label">
                                {{ f.label }}
                                @if (isSet(f.key)) {
                                    <span class="set-badge">défini</span>
                                }
                            </span>
                            <input
                                [type]="f.secret ? 'password' : 'text'"
                                [placeholder]="
                                    f.secret && isSet(f.key)
                                        ? '••• défini (laisser vide pour conserver)'
                                        : f.placeholder
                                "
                                [value]="value(f)"
                                (input)="edit(f.key, $any($event.target).value)"
                                [attr.autocomplete]="f.secret ? 'new-password' : 'off'"
                                spellcheck="false"
                            />
                        </label>
                    }
                </section>
            }

            <div class="save-bar">
                <button class="btn" (click)="save()" [disabled]="saving() || !dirty()">
                    <mat-icon class="material-symbols-outlined">{{
                        saving() ? "progress_activity" : "save"
                    }}</mat-icon>
                    {{ saving() ? "Enregistrement…" : "Enregistrer les canaux" }}
                </button>
                @if (savedAt()) {
                    <span class="saved"
                        ><mat-icon class="material-symbols-outlined">check_circle</mat-icon
                        >{{ savedAt() }}</span
                    >
                }
                @if (saveError()) {
                    <span class="warn">{{ saveError() }}</span>
                }
            </div>
        }
    `,
    styles: [
        `
            .card {
                background: var(--mat-sys-surface-container);
                border-radius: 20px;
                padding: 18px 22px;
                margin-bottom: 14px;
            }
            .card h2 {
                font-size: 16px;
                font-weight: 500;
                margin: 0 0 10px;
            }
            .hint {
                margin: 0;
                font-size: 13px;
                line-height: 1.5;
                color: var(--mat-sys-on-surface-variant);
            }
            code {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
            }
            .path {
                margin: 12px 0 0;
                font-size: 11px;
                color: var(--mat-sys-on-surface-variant);
            }
            .mono {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                word-break: break-all;
            }
            .warn {
                margin: 0;
                font-size: 13px;
                color: var(--mat-sys-error);
            }
            .group-head {
                display: flex;
                align-items: flex-start;
                gap: 12px;
                margin-bottom: 14px;
            }
            .group-head > mat-icon {
                color: var(--mat-sys-primary);
                flex: 0 0 auto;
            }
            .group-head h3 {
                margin: 0;
                font-size: 15px;
                font-weight: 500;
            }
            .group-head p {
                margin: 2px 0 0;
                font-size: 12px;
                color: var(--mat-sys-on-surface-variant);
            }
            .tag {
                margin-left: auto;
                flex: 0 0 auto;
                font-size: 10px;
                padding: 3px 9px;
                border-radius: 999px;
            }
            .tag.ok {
                background: color-mix(in srgb, var(--mat-sys-tertiary) 16%, transparent);
                color: var(--mat-sys-tertiary);
            }
            .field {
                display: flex;
                flex-direction: column;
                gap: 6px;
                margin-bottom: 12px;
            }
            .field:last-child {
                margin-bottom: 0;
            }
            .f-label {
                display: flex;
                align-items: center;
                gap: 8px;
                font-size: 13px;
                color: var(--mat-sys-on-surface-variant);
            }
            .set-badge {
                font-size: 10px;
                padding: 1px 7px;
                border-radius: 999px;
                background: color-mix(in srgb, var(--mat-sys-tertiary) 16%, transparent);
                color: var(--mat-sys-tertiary);
            }
            .field input {
                border: 1px solid var(--mat-sys-outline-variant);
                border-radius: 10px;
                background: var(--mat-sys-surface-container-low);
                color: var(--mat-sys-on-surface);
                font: inherit;
                font-size: 14px;
                padding: 10px 12px;
                outline: none;
            }
            .field input:focus {
                border-color: var(--mat-sys-primary);
            }
            .save-bar {
                display: flex;
                align-items: center;
                gap: 12px;
                padding: 4px 2px 8px;
            }
            .btn {
                display: inline-flex;
                align-items: center;
                gap: 8px;
                border: none;
                border-radius: 999px;
                padding: 10px 20px;
                font: inherit;
                font-size: 14px;
                cursor: pointer;
                color: var(--mat-sys-on-primary);
                background: var(--mat-sys-primary);
            }
            .btn:disabled {
                opacity: 0.55;
                cursor: default;
            }
            .btn mat-icon {
                font-size: 18px;
                width: 18px;
                height: 18px;
            }
            .saved {
                display: inline-flex;
                align-items: center;
                gap: 4px;
                font-size: 13px;
                color: var(--mat-sys-tertiary);
            }
            .saved mat-icon {
                font-size: 16px;
                width: 16px;
                height: 16px;
            }
        `,
    ],
})
export class ChannelsTabComponent implements OnInit {
    private readonly agent = inject(AgentService);

    readonly groups = GROUPS;
    readonly state = signal<ChannelsState | null>(null);
    readonly saving = signal(false);
    readonly savedAt = signal("");
    readonly loadError = signal("");
    readonly saveError = signal("");
    readonly hostUnavailable = signal(false);

    /** Pending edits keyed by channel key (only these are sent on save). */
    private readonly edits = signal<Record<string, string>>({});

    async ngOnInit(): Promise<void> {
        await this.reload();
    }

    private async reload(): Promise<void> {
        try {
            this.state.set(await this.agent.readChannels());
            this.edits.set({});
        } catch (err) {
            if (err instanceof HostUnavailableError) {
                this.hostUnavailable.set(true);
            } else {
                this.loadError.set(`Lecture impossible : ${String(err)}`);
            }
        }
    }

    /** Whether a key has a value stored on disk (from the secret-free state). */
    isSet(key: string): boolean {
        return this.state()?.configured[key] === true;
    }

    /** Current input value: a pending edit, else the plain non-secret value. */
    value(f: Field): string {
        const edit = this.edits()[f.key];
        if (edit !== undefined) return edit;
        if (f.secret) return ""; // secrets are never pre-filled
        return this.state()?.values[f.key] ?? "";
    }

    edit(key: string, value: string): void {
        this.edits.update((m) => ({ ...m, [key]: value }));
    }

    groupConfigured(g: Group): boolean {
        return g.fields.some((f) => this.isSet(f.key));
    }

    dirty(): boolean {
        return Object.keys(this.edits()).length > 0;
    }

    async save(): Promise<void> {
        this.saving.set(true);
        this.saveError.set("");
        this.savedAt.set("");
        const updates: ChannelUpdate[] = Object.entries(this.edits()).map(([key, value]) => ({
            key,
            value,
        }));
        try {
            this.state.set(await this.agent.writeChannels(updates));
            this.edits.set({});
            this.savedAt.set(`Enregistré à ${new Date().toLocaleTimeString("fr-FR")}`);
        } catch (err) {
            this.saveError.set(`Enregistrement impossible : ${String(err)}`);
        } finally {
            this.saving.set(false);
        }
    }
}
