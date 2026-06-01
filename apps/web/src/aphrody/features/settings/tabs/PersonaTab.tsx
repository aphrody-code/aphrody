// Persona tab: editor for an agent-home persona file (SOUL.md / IDENTITY.md), loaded + persisted via the aphrody CLI.

import { useEffect, useState } from "react";
import {
  MdFilledButton,
  MdIcon,
  MdOutlinedButton,
  MdOutlinedTextField,
} from "@aphrody-code/m3-react";
import { execText, run } from "../../../client.ts";
import { Hint, Panel, Spinner } from "../../../ui.tsx";

export type PersonaWhich = "soul" | "identity";

const SOUL_SEED = `---
tone: précis et chaleureux
brevity: balanced
humor: dry
bluntness: direct
opinions:
  - Citer les numéros de ligne et vérifier avant d'affirmer le succès.
  - Préférer des changements petits et testés.
boundaries:
  - Ne jamais exécuter de commande destructrice sans confirmation.
---
Je suis un agent d'ingénierie autonome. Je vais droit au but, je vérifie mon
travail, et je signale honnêtement ce qui n'est pas câblé.
`;

const IDENTITY_SEED = `---
name: aphrody
vibe: assistant IA autonome, précis et direct
---
Je suis aphrody, un agent autonome propulsé par Gemini.
`;

export function PersonaTab({ which }: { which: PersonaWhich }) {
  const [content, setContent] = useState("");
  const [original, setOriginal] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [saving, setSaving] = useState(false);
  const [savedAt, setSavedAt] = useState("");
  const [saveError, setSaveError] = useState("");

  const heading =
    which === "soul" ? "Âme de l'agent (SOUL.md)" : "Identité de l'agent (IDENTITY.md)";
  const blurb =
    which === "soul"
      ? "L'âme définit la personnalité, le ton, les opinions et les limites de l'agent — elle est intégrée au prompt système. Markdown avec un en-tête YAML facultatif (tone, brevity, humor, bluntness, opinions, boundaries)."
      : "L'identité indique qui est l'agent : son nom et sa vibe. Elle ouvre le prompt système (« Tu es … »). Markdown avec un en-tête YAML facultatif (name, vibe).";
  const placeholder =
    which === "soul"
      ? "---\ntone: …\n---\nLa personnalité de l'agent…"
      : "---\nname: …\n---\nQui est l'agent…";

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setLoadError("");
    setSavedAt("");
    setSaveError("");
    void run([which, "show"]).then((res) => {
      if (cancelled) return;
      if (res.code === 0) {
        const text = res.stdout;
        setContent(text);
        setOriginal(text);
      } else {
        setLoadError(execText(res) || "Lecture impossible.");
      }
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [which]);

  function useSeed(): void {
    setContent(which === "soul" ? SOUL_SEED : IDENTITY_SEED);
  }

  function revert(): void {
    setContent(original);
    setSaveError("");
  }

  async function save(): Promise<void> {
    setSaving(true);
    setSaveError("");
    setSavedAt("");
    const res = await run([which, "set", content]);
    if (res.code === 0) {
      setOriginal(content);
      setSavedAt(`Enregistré à ${new Date().toLocaleTimeString("fr-FR")}`);
    } else {
      setSaveError(`Enregistrement impossible : ${execText(res)}`);
    }
    setSaving(false);
  }

  const dirty = content !== original;
  const isEmpty = original.trim() === "";

  return (
    <div className="aph-settings-tab">
      <Panel title={heading} icon="psychology">
        <p className="aph-muted">{blurb}</p>

        {loading ? (
          <Spinner label="Chargement…" />
        ) : loadError ? (
          <Hint icon="error" title="Lecture impossible" text={loadError} />
        ) : (
          <>
            {isEmpty && !content && (
              <div className="aph-settings-seed">
                <p className="aph-muted">
                  Ce fichier n'existe pas encore. Vous pouvez partir d'un modèle :
                </p>
                <MdOutlinedButton onClick={useSeed}>
                  <MdIcon slot="icon">auto_fix_high</MdIcon>
                  Insérer un modèle
                </MdOutlinedButton>
              </div>
            )}

            <MdOutlinedTextField
              style={{ width: "100%", minHeight: 280 }}
              type="textarea"
              rows={14}
              label="Contenu du fichier"
              placeholder={placeholder}
              value={content}
              onInput={(e) => setContent((e.target as HTMLInputElement).value)}
            />

            <div className="aph-settings-row aph-settings-actions">
              <MdFilledButton onClick={() => void save()} disabled={saving || !dirty}>
                <MdIcon slot="icon">{saving ? "progress_activity" : "save"}</MdIcon>
                {saving ? "Enregistrement…" : "Enregistrer"}
              </MdFilledButton>
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
