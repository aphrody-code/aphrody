// Actions tab: run real aphrody commands (autonomous loop, doctor, skill, notify, hermes) and show captured output.

import { useState } from "react";
import {
  MdCircularProgress,
  MdFilledButton,
  MdIcon,
  MdOutlinedButton,
  MdOutlinedSelect,
  MdOutlinedTextField,
  MdSelectOption,
} from "@aphrody/m3-react";
import { run } from "../../../client.ts";
import type { ExecResult } from "../../../types.ts";

export function ActionsTab() {
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ExecResult | null>(null);
  const [ranLabel, setRanLabel] = useState("");

  const [goal, setGoal] = useState("");
  const [maxIter, setMaxIter] = useState("50");
  const [skillName, setSkillName] = useState("");
  const [notifyChannel, setNotifyChannel] = useState("slack");
  const [notifyMsg, setNotifyMsg] = useState("Test depuis aphrody desktop.");
  const [hermesChannel, setHermesChannel] = useState("discord");
  const [hermesMsg, setHermesMsg] = useState("Bonjour aphrody");

  async function exec(args: string[]): Promise<void> {
    if (busy) return;
    setBusy(true);
    setResult(null);
    setRanLabel(["aphrody", ...args].join(" "));
    const res = await run(args);
    setResult(res);
    setBusy(false);
  }

  function startLoop(): void {
    const max = Number.parseInt(maxIter, 10);
    const args = ["agy-loop", "start", "--goal", goal.trim()];
    if (Number.isFinite(max) && max > 0) {
      args.push("--max", String(max));
    }
    void exec(args);
  }

  function runSkill(): void {
    void exec(["skills", "info", skillName.trim()]);
  }

  function sendNotify(): void {
    void exec(["notify", "--channel", notifyChannel, "--message", notifyMsg.trim()]);
  }

  function checkHermes(): void {
    void exec(["hermes", "--channel", hermesChannel, "--simulate", hermesMsg.trim()]);
  }

  return (
    <div className="aph-settings-tab">
      <section className="aph-panel aph-settings-card">
        <h2 className="aph-settings-card__title">Boucle de codage autonome (agy)</h2>
        <p className="aph-muted">
          Arme/désarme la boucle <code>agy-loop</code> qui pilote le CLI Antigravity en autonomie.
          L'objectif est réinjecté à chaque tour jusqu'à la sentinelle ou le plafond d'itérations.
        </p>
        <div className="aph-settings-row">
          <MdOutlinedTextField
            style={{ flex: "1 1 200px" }}
            label="Objectif (ex. implémente l'auth OAuth, tests verts)"
            value={goal}
            onInput={(e) => setGoal((e.target as HTMLInputElement).value)}
          />
          <MdOutlinedTextField
            style={{ flex: "0 0 110px" }}
            type="number"
            min="1"
            label="Itérations"
            value={maxIter}
            onInput={(e) => setMaxIter((e.target as HTMLInputElement).value)}
          />
        </div>
        <div className="aph-settings-row">
          <MdFilledButton onClick={startLoop} disabled={busy || !goal.trim()}>
            <MdIcon slot="icon">play_arrow</MdIcon>
            Démarrer
          </MdFilledButton>
          <MdOutlinedButton onClick={() => void exec(["agy-loop", "stop"])} disabled={busy}>
            <MdIcon slot="icon">stop</MdIcon>
            Arrêter
          </MdOutlinedButton>
          <MdOutlinedButton onClick={() => void exec(["agy-loop", "status"])} disabled={busy}>
            <MdIcon slot="icon">info</MdIcon>
            État
          </MdOutlinedButton>
        </div>
      </section>

      <section className="aph-panel aph-settings-card">
        <h2 className="aph-settings-card__title">Diagnostic</h2>
        <p className="aph-muted">
          Vérifie l'environnement, l'intégration A2A et la chaîne d'approvisionnement.
        </p>
        <div className="aph-settings-row">
          <MdFilledButton onClick={() => void exec(["doctor"])} disabled={busy}>
            <MdIcon slot="icon">monitor_heart</MdIcon>
            Lancer doctor
          </MdFilledButton>
        </div>
      </section>

      <section className="aph-panel aph-settings-card">
        <h2 className="aph-settings-card__title">Exécuter un skill</h2>
        <p className="aph-muted">
          Lit un SKILL.md via la commande <code>aphrody skills info</code>.
        </p>
        <div className="aph-settings-row">
          <MdOutlinedTextField
            style={{ flex: "1 1 200px" }}
            label="Nom du skill (ex. best-stack-2026)"
            value={skillName}
            onInput={(e) => setSkillName((e.target as HTMLInputElement).value)}
          />
          <MdOutlinedButton onClick={runSkill} disabled={busy || !skillName.trim()}>
            <MdIcon slot="icon">extension</MdIcon>
            Ouvrir
          </MdOutlinedButton>
        </div>
      </section>

      <section className="aph-panel aph-settings-card">
        <h2 className="aph-settings-card__title">Notification de test</h2>
        <p className="aph-muted">
          Envoie un message via <code>aphrody notify</code>. Les identifiants viennent des canaux
          configurés (variables d'environnement).
        </p>
        <div className="aph-settings-row">
          <MdOutlinedSelect
            style={{ flex: "0 0 160px" }}
            label="Canal"
            value={notifyChannel}
            onChange={(e) => setNotifyChannel((e.target as HTMLInputElement).value)}
          >
            <MdSelectOption value="slack">
              <span slot="headline">Slack</span>
            </MdSelectOption>
            <MdSelectOption value="telegram">
              <span slot="headline">Telegram</span>
            </MdSelectOption>
            <MdSelectOption value="matrix">
              <span slot="headline">Matrix</span>
            </MdSelectOption>
          </MdOutlinedSelect>
          <MdOutlinedTextField
            style={{ flex: "1 1 160px" }}
            label="Message de test"
            value={notifyMsg}
            onInput={(e) => setNotifyMsg((e.target as HTMLInputElement).value)}
          />
          <MdOutlinedButton onClick={sendNotify} disabled={busy || !notifyMsg.trim()}>
            <MdIcon slot="icon">notifications</MdIcon>
            Envoyer
          </MdOutlinedButton>
        </div>
      </section>

      <section className="aph-panel aph-settings-card">
        <h2 className="aph-settings-card__title">Vérifier hermes (canal)</h2>
        <p className="aph-muted">
          Traite un message synthétique sur le canal choisi (<code>--simulate</code>) — vérification
          sans jeton ni navigateur.
        </p>
        <div className="aph-settings-row">
          <MdOutlinedSelect
            style={{ flex: "0 0 160px" }}
            label="Canal"
            value={hermesChannel}
            onChange={(e) => setHermesChannel((e.target as HTMLInputElement).value)}
          >
            <MdSelectOption value="discord">
              <span slot="headline">Discord</span>
            </MdSelectOption>
            <MdSelectOption value="x">
              <span slot="headline">X</span>
            </MdSelectOption>
          </MdOutlinedSelect>
          <MdOutlinedTextField
            style={{ flex: "1 1 160px" }}
            label="Message simulé"
            value={hermesMsg}
            onInput={(e) => setHermesMsg((e.target as HTMLInputElement).value)}
          />
          <MdOutlinedButton onClick={checkHermes} disabled={busy || !hermesMsg.trim()}>
            <MdIcon slot="icon">graphic_eq</MdIcon>
            Simuler
          </MdOutlinedButton>
        </div>
      </section>

      <section className="aph-output aph-output--tool">
        <div className="aph-output__head">
          <MdIcon>terminal</MdIcon>
          <span className="aph-output__cmd">{ranLabel || "Aucune action exécutée"}</span>
          {result && (
            <span className={`aph-code ${result.code === 0 ? "aph-code--ok" : "aph-code--err"}`}>
              code {result.code}
            </span>
          )}
        </div>
        <pre className="aph-output__body">
          {busy ? (
            <span className="aph-row">
              <MdCircularProgress indeterminate /> Exécution en cours…
            </span>
          ) : result ? (
            `${result.stdout}${result.stderr ? `\n${result.stderr}` : ""}`
          ) : (
            "La sortie de la commande s'affichera ici."
          )}
        </pre>
      </section>
    </div>
  );
}
