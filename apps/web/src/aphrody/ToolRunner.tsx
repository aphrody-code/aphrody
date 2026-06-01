// Generic tool surface (port of Angular ToolRunnerComponent): a header, a column
// of runnable aphrody commands (each calls `run`), and a terminal-style output
// pane with captured stdout/stderr + exit code. Many feature panels are just a
// title + a list of ToolActions fed to this component.

import { useState } from "react";
import {
  MdCircularProgress,
  MdFilledButton,
  MdIcon,
  MdOutlinedTextField,
} from "@aphrody-code/m3-react";
import { run } from "./client.ts";
import type { ExecResult } from "./types.ts";

/** A single runnable aphrody command exposed in a tool view. */
export interface ToolAction {
  label: string;
  icon: string;
  /** argv passed to the runner (without program name). */
  args: string[];
  /** Optional free-text input appended to args when present. */
  prompt?: { placeholder: string; flag?: string };
  hint?: string;
}

export function ToolRunner({
  title,
  subtitle,
  icon = "build",
  actions,
}: {
  title: string;
  subtitle?: string;
  icon?: string;
  actions: ToolAction[];
}) {
  const [running, setRunning] = useState<string | null>(null);
  const [result, setResult] = useState<ExecResult | null>(null);
  const [ranLabel, setRanLabel] = useState("");
  const [inputs, setInputs] = useState<Record<string, string>>({});

  const exec = async (action: ToolAction) => {
    if (running) return;
    const args = [...action.args];
    if (action.prompt) {
      const val = (inputs[action.label] ?? "").trim();
      if (val) {
        if (action.prompt.flag) args.push(action.prompt.flag, val);
        else args.push(val);
      }
    }
    setRunning(action.label);
    setResult(null);
    setRanLabel(["aphrody", ...args].join(" "));
    const res = await run(args);
    setResult(res);
    setRunning(null);
  };

  return (
    <div className="aph-tool">
      <header className="aph-tool__head">
        <span className="aph-tool__glyph">
          <MdIcon>{icon}</MdIcon>
        </span>
        <div>
          <h1 className="aph-pagehead__title">{title}</h1>
          {subtitle && <p className="aph-muted">{subtitle}</p>}
        </div>
      </header>

      <div className="aph-tool__body">
        <section className="aph-actions">
          {actions.map((a) => (
            <div key={a.label} className="aph-action">
              <div className="aph-row aph-action__main">
                <MdIcon className="aph-action__glyph">{a.icon}</MdIcon>
                <div className="aph-action__text">
                  <span className="aph-action__label">{a.label}</span>
                  {a.hint && <span className="aph-muted aph-action__hint">{a.hint}</span>}
                </div>
              </div>
              <div className="aph-action__controls">
                {a.prompt && (
                  <MdOutlinedTextField
                    className="aph-action__input"
                    placeholder={a.prompt.placeholder}
                    value={inputs[a.label] ?? ""}
                    onInput={(e) =>
                      setInputs((m) => ({ ...m, [a.label]: (e.target as HTMLInputElement).value }))
                    }
                  />
                )}
                <MdFilledButton disabled={running !== null} onClick={() => void exec(a)}>
                  {running === a.label ? (
                    <MdCircularProgress indeterminate slot="icon" />
                  ) : (
                    <MdIcon slot="icon">play_arrow</MdIcon>
                  )}
                  Exécuter
                </MdFilledButton>
              </div>
            </div>
          ))}
        </section>

        <section className="aph-output aph-output--tool">
          <div className="aph-output__head">
            <MdIcon>terminal</MdIcon>
            <span className="aph-output__cmd">{ranLabel || "Aucune commande exécutée"}</span>
            {result && (
              <span className={`aph-code ${result.code === 0 ? "aph-code--ok" : "aph-code--err"}`}>
                code {result.code}
              </span>
            )}
          </div>
          <pre className="aph-output__body">
            {running
              ? "Exécution en cours…"
              : result
                ? `${result.stdout}${result.stderr ? `\n${result.stderr}` : ""}`
                : "La sortie de la commande s'affichera ici."}
          </pre>
        </section>
      </div>
    </div>
  );
}
