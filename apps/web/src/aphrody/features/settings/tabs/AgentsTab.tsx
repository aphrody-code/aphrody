// Agents tab: edit the selected agent's JSON config (agy / Claude Code / aphrody MCP), validated before save.

import { useEffect, useState } from "react";
import {
  MdFilledButton,
  MdIcon,
  MdMenuItem,
  MdOutlinedButton,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { execText, run } from "../../../client.ts";
import { Hint, Panel, Spinner } from "../../../ui.tsx";
import { Menu } from "../../../../components/ui/Menu.tsx";

export type ConfigWhich = "agy" | "claude" | "aphrody-mcp";

interface AgentChoice {
  which: ConfigWhich;
  label: string;
  icon: string;
  blurb: string;
}

const CHOICES: AgentChoice[] = [
  {
    which: "agy",
    label: "agy CLI / Antigravity 2.0",
    icon: "rocket",
    blurb: "Configuration partagée du CLI Antigravity (agy) — ~/.gemini/config/config.json.",
  },
  {
    which: "claude",
    label: "Claude Code",
    icon: "smart_toy",
    blurb: "Réglages de Claude Code — ~/.claude/settings.json.",
  },
  {
    which: "aphrody-mcp",
    label: "Serveur MCP (aphrody)",
    icon: "hub",
    blurb: "Serveurs MCP déclarés — ~/.config/aphrody/mcp.json.",
  },
];

export function AgentsTab() {
  const [which, setWhich] = useState<ConfigWhich>("agy");
  const [content, setContent] = useState("");
  const [original, setOriginal] = useState("");
  const [jsonError, setJsonError] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [saving, setSaving] = useState(false);
  const [savedAt, setSavedAt] = useState("");
  const [saveError, setSaveError] = useState("");

  const current = CHOICES.find((c) => c.which === which) ?? CHOICES[0];

  function validate(value: string): void {
    try {
      JSON.parse(value);
      setJsonError("");
    } catch (err) {
      setJsonError(`JSON invalide : ${(err as Error).message}`);
    }
  }

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setLoadError("");
    setSaveError("");
    setSavedAt("");
    setJsonError("");
    void run([which, "config", "show"]).then((res) => {
      if (cancelled) return;
      if (res.code === 0) {
        const initial = res.stdout.trim() ? res.stdout : "{}";
        setContent(initial);
        setOriginal(initial);
        validate(initial);
      } else {
        setLoadError(`Lecture impossible : ${execText(res)}`);
      }
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [which]);

  function onEdit(value: string): void {
    setContent(value);
    validate(value);
  }

  function format(): void {
    try {
      const pretty = JSON.stringify(JSON.parse(content), null, 2);
      setContent(pretty);
      setJsonError("");
    } catch {
      // validate() already surfaced the error
    }
  }

  function revert(): void {
    setContent(original);
    validate(original);
    setSaveError("");
  }

  async function save(): Promise<void> {
    if (jsonError) return;
    setSaving(true);
    setSaveError("");
    setSavedAt("");
    const res = await run([which, "config", "set", content]);
    if (res.code === 0) {
      setOriginal(content);
      setSavedAt(`Enregistré à ${new Date().toLocaleTimeString("fr-FR")}`);
    } else {
      setSaveError(`Enregistrement impossible : ${execText(res)}`);
    }
    setSaving(false);
  }

  const dirty = content !== original;

  return (
    <div className="aph-settings-tab">
      <Panel title="Configuration des agents" icon="manage_accounts">
        <p className="aph-muted">
          Éditez la configuration JSON de l'agent sélectionné. La validation JSON s'effectue avant
          l'enregistrement — un JSON invalide est refusé et le fichier existant reste intact.
        </p>

        <div className="aph-settings-row">
          <Menu
            trigger={({ toggle }) => (
              <MdOutlinedButton onClick={toggle}>
                <MdIcon slot="icon">{current.icon}</MdIcon>
                {current.label}
                <MdIcon slot="trailing-icon">arrow_drop_down</MdIcon>
              </MdOutlinedButton>
            )}
          >
            {CHOICES.map((c) => (
              <MdMenuItem
                key={c.which}
                selected={c.which === which}
                onClick={() => setWhich(c.which)}
              >
                <MdIcon slot="start">{c.icon}</MdIcon>
                <div slot="headline">{c.label}</div>
              </MdMenuItem>
            ))}
          </Menu>
        </div>
        <p className="aph-muted aph-settings-blurb">{current.blurb}</p>
      </Panel>

      <Panel title={`Fichier de configuration · ${current.label}`} icon="description">
        {loading ? (
          <Spinner label="Chargement…" />
        ) : loadError ? (
          <Hint icon="error" title="Lecture impossible" text={loadError} />
        ) : (
          <>
            <MdOutlinedTextField
              style={{ width: "100%", minHeight: 300, fontFamily: "ui-monospace, monospace" }}
              type="textarea"
              rows={16}
              label="Configuration JSON"
              placeholder="{}"
              value={content}
              onInput={(e) => onEdit((e.target as HTMLInputElement).value)}
            />

            <div className="aph-settings-status">
              {jsonError ? (
                <span className="aph-row aph-settings-warn">
                  <MdIcon>error</MdIcon>
                  {jsonError}
                </span>
              ) : (
                <span className="aph-row aph-settings-ok">
                  <MdIcon>check_circle</MdIcon>
                  JSON valide
                </span>
              )}
            </div>

            <div className="aph-settings-row aph-settings-actions">
              <MdFilledButton
                onClick={() => void save()}
                disabled={saving || jsonError !== "" || !dirty}
              >
                <MdIcon slot="icon">{saving ? "progress_activity" : "save"}</MdIcon>
                {saving ? "Enregistrement…" : "Enregistrer"}
              </MdFilledButton>
              <MdOutlinedButton onClick={format} disabled={jsonError !== ""}>
                <MdIcon slot="icon">format_align_left</MdIcon>
                Formater
              </MdOutlinedButton>
              {dirty && (
                <MdOutlinedButton onClick={revert} disabled={saving}>
                  <MdIcon slot="icon">undo</MdIcon>
                  Annuler
                </MdOutlinedButton>
              )}
              {savedAt && (
                <span className="aph-row aph-settings-saved">
                  <MdIcon>check_circle</MdIcon>
                  {savedAt}
                </span>
              )}
            </div>
            {saveError && <p className="aph-settings-warn">{saveError}</p>}
          </>
        )}
      </Panel>
    </div>
  );
}
