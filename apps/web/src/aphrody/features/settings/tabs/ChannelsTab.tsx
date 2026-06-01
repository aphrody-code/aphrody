// Channels tab: configure messaging integration credentials. Secrets are masked; only changed fields are persisted.

import { useEffect, useState } from "react";
import { MdFilledButton, MdIcon, MdOutlinedTextField } from "@aphrody-code/m3-react";
import { execJson, run } from "../../../client.ts";
import { Hint, Panel, Spinner } from "../../../ui.tsx";

interface Field {
  key: string;
  label: string;
  secret: boolean;
  placeholder: string;
}

interface Group {
  title: string;
  icon: string;
  blurb: string;
  fields: Field[];
}

/** Secret-free channels state returned by `aphrody channels show`. */
interface ChannelsState {
  path: string;
  exists: boolean;
  configured: Record<string, boolean>;
  values: Record<string, string>;
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
    fields: [{ key: "X_HANDLE", label: "Identifiant X", secret: false, placeholder: "monhandle" }],
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
      { key: "SLACK_BOT_TOKEN", label: "Jeton du bot Slack", secret: true, placeholder: "xoxb-…" },
      { key: "SLACK_CHANNEL", label: "Salon par défaut", secret: false, placeholder: "C012AB3CD" },
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
      { key: "MATRIX_USER_ID", label: "User ID", secret: false, placeholder: "@moi:matrix.org" },
      {
        key: "MATRIX_ROOM_ID",
        label: "Room ID par défaut",
        secret: false,
        placeholder: "!abc:matrix.org",
      },
    ],
  },
];

export function ChannelsTab() {
  const [state, setState] = useState<ChannelsState | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [savedAt, setSavedAt] = useState("");
  const [saveError, setSaveError] = useState("");

  async function reload(): Promise<void> {
    setLoading(true);
    setLoadError("");
    const res = await run(["channels", "show"]);
    if (res.code === 0) {
      const parsed = execJson<ChannelsState>(res);
      setState(
        parsed ?? { path: "~/.aphrody/channels.json", exists: false, configured: {}, values: {} },
      );
      setEdits({});
    } else {
      setLoadError("La configuration des canaux nécessite le binaire aphrody local.");
    }
    setLoading(false);
  }

  useEffect(() => {
    void reload();
  }, []);

  function isSet(key: string): boolean {
    return state?.configured[key] === true;
  }

  function value(f: Field): string {
    const edit = edits[f.key];
    if (edit !== undefined) return edit;
    if (f.secret) return "";
    return state?.values[f.key] ?? "";
  }

  function edit(key: string, v: string): void {
    setEdits((m) => ({ ...m, [key]: v }));
  }

  function groupConfigured(g: Group): boolean {
    return g.fields.some((f) => isSet(f.key));
  }

  const dirty = Object.keys(edits).length > 0;

  async function save(): Promise<void> {
    setSaving(true);
    setSaveError("");
    setSavedAt("");
    let failed = "";
    for (const [key, v] of Object.entries(edits)) {
      const res = await run(["channels", "set", key, v]);
      if (res.code !== 0) {
        failed = `Enregistrement impossible (${key}).`;
        break;
      }
    }
    if (failed) {
      setSaveError(failed);
    } else {
      setSavedAt(`Enregistré à ${new Date().toLocaleTimeString("fr-FR")}`);
      await reload();
    }
    setSaving(false);
  }

  return (
    <div className="aph-settings-tab">
      <Panel title="Canaux de messagerie" icon="hub">
        <p className="aph-muted">
          Identifiants des intégrations (hermes, notifications). Stockés en local dans
          <code> ~/.aphrody/channels.json</code>, hors du dépôt. Les secrets sont masqués : laissez
          un champ secret vide pour conserver la valeur existante.
        </p>
        {state?.path && <p className="aph-muted aph-settings-mono">{state.path}</p>}
      </Panel>

      {loading ? (
        <Spinner label="Lecture des canaux…" />
      ) : loadError ? (
        <Hint icon="info" title="Configuration indisponible" text={loadError} />
      ) : (
        <>
          {GROUPS.map((g) => (
            <Panel
              key={g.title}
              title={g.title}
              icon={g.icon}
              actions={
                groupConfigured(g) ? <span className="aph-settings-badge">configuré</span> : null
              }
            >
              <p className="aph-muted">{g.blurb}</p>
              {g.fields.map((f) => (
                <div key={f.key} className="aph-settings-field">
                  <MdOutlinedTextField
                    style={{ width: "100%" }}
                    type={f.secret ? "password" : "text"}
                    label={f.label + (isSet(f.key) ? " · défini" : "")}
                    placeholder={
                      f.secret && isSet(f.key)
                        ? "••• défini (laisser vide pour conserver)"
                        : f.placeholder
                    }
                    value={value(f)}
                    onInput={(e) => edit(f.key, (e.target as HTMLInputElement).value)}
                  />
                </div>
              ))}
            </Panel>
          ))}

          <div className="aph-settings-row aph-settings-actions">
            <MdFilledButton onClick={() => void save()} disabled={saving || !dirty}>
              <MdIcon slot="icon">{saving ? "progress_activity" : "save"}</MdIcon>
              {saving ? "Enregistrement…" : "Enregistrer les canaux"}
            </MdFilledButton>
            {savedAt && (
              <span className="aph-row aph-settings-saved">
                <MdIcon>check_circle</MdIcon>
                {savedAt}
              </span>
            )}
            {saveError && <span className="aph-settings-warn">{saveError}</span>}
          </div>
        </>
      )}
    </div>
  );
}
