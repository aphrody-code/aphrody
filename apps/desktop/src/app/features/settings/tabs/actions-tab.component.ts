// SPDX-License-Identifier: Apache-2.0
import { Component, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatIconModule } from "@angular/material/icon";

import { AgentService, HostUnavailableError } from "../../../core/agent.service";
import { AphrodyService, ExecResult } from "../../../core/aphrody.service";

/** Where a runnable action sends its argv. */
type Runner = "aphrody" | "skills";

/**
 * Actions tab: "lancer des actions". Each button runs a REAL command — the
 * autonomous loop (agy-loop start/stop/status), doctor, a skill via the sibling
 * binary, a test notification (notify), and a hermes channel check (simulate,
 * so no live token is required). The captured stdout/stderr + exit code render
 * in a shared result pane. Free-text inputs (loop goal, skill name, notify
 * channel/message, hermes channel) feed the argv.
 */
@Component({
    selector: "app-actions-tab",
    imports: [FormsModule, MatIconModule],
    template: `
    <section class="card">
      <h2>Boucle de codage autonome (agy)</h2>
      <p class="hint">
        Arme/désarme la boucle <code>agy-loop</code> qui pilote le CLI Antigravity en autonomie.
        L'objectif est réinjecté à chaque tour jusqu'à la sentinelle ou le plafond d'itérations.
      </p>
      <div class="input-row">
        <input
          class="text-in"
          placeholder="Objectif (ex. implémente l'auth OAuth, tests verts)"
          [value]="goal()"
          (input)="goal.set($any($event.target).value)"
        />
        <input
          class="num-in"
          type="number"
          min="1"
          [value]="maxIter()"
          (input)="maxIter.set($any($event.target).value)"
          aria-label="Plafond d'itérations"
        />
      </div>
      <div class="btn-row">
        <button class="act primary" (click)="startLoop()" [disabled]="busy() || !goal().trim()">
          <mat-icon class="material-symbols-outlined">play_arrow</mat-icon> Démarrer
        </button>
        <button class="act" (click)="run('Arrêt de la boucle', 'aphrody', ['agy-loop', 'stop'])" [disabled]="busy()">
          <mat-icon class="material-symbols-outlined">stop</mat-icon> Arrêter
        </button>
        <button class="act" (click)="run('État de la boucle', 'aphrody', ['agy-loop', 'status'])" [disabled]="busy()">
          <mat-icon class="material-symbols-outlined">info</mat-icon> État
        </button>
      </div>
    </section>

    <section class="card">
      <h2>Diagnostic</h2>
      <p class="hint">Vérifie l'environnement, l'intégration A2A et la chaîne d'approvisionnement.</p>
      <div class="btn-row">
        <button class="act" (click)="run('Diagnostic', 'aphrody', ['doctor'])" [disabled]="busy()">
          <mat-icon class="material-symbols-outlined">monitor_heart</mat-icon> Lancer doctor
        </button>
      </div>
    </section>

    <section class="card">
      <h2>Exécuter un skill</h2>
      <p class="hint">Lit un SKILL.md via le binaire voisin <code>aphrody-skills</code> (info).</p>
      <div class="input-row">
        <input
          class="text-in"
          placeholder="Nom du skill (ex. best-stack-2026)"
          [value]="skillName()"
          (input)="skillName.set($any($event.target).value)"
        />
        <button class="act" (click)="runSkill()" [disabled]="busy() || !skillName().trim()">
          <mat-icon class="material-symbols-outlined">extension</mat-icon> Ouvrir
        </button>
      </div>
    </section>

    <section class="card">
      <h2>Notification de test</h2>
      <p class="hint">
        Envoie un message via <code>aphrody notify</code>. Les identifiants viennent des canaux
        configurés (variables d'environnement).
      </p>
      <div class="input-row">
        <select class="sel" [value]="notifyChannel()" (change)="notifyChannel.set($any($event.target).value)">
          <option value="slack">Slack</option>
          <option value="telegram">Telegram</option>
          <option value="matrix">Matrix</option>
        </select>
        <input
          class="text-in"
          placeholder="Message de test"
          [value]="notifyMsg()"
          (input)="notifyMsg.set($any($event.target).value)"
        />
        <button class="act" (click)="sendNotify()" [disabled]="busy() || !notifyMsg().trim()">
          <mat-icon class="material-symbols-outlined">notifications</mat-icon> Envoyer
        </button>
      </div>
    </section>

    <section class="card">
      <h2>Vérifier hermes (canal)</h2>
      <p class="hint">
        Traite un message synthétique sur le canal choisi (<code>--simulate</code>) — vérification
        sans jeton ni navigateur.
      </p>
      <div class="input-row">
        <select class="sel" [value]="hermesChannel()" (change)="hermesChannel.set($any($event.target).value)">
          <option value="discord">Discord</option>
          <option value="x">X</option>
        </select>
        <input
          class="text-in"
          placeholder="Message simulé"
          [value]="hermesMsg()"
          (input)="hermesMsg.set($any($event.target).value)"
        />
        <button class="act" (click)="checkHermes()" [disabled]="busy() || !hermesMsg().trim()">
          <mat-icon class="material-symbols-outlined">graphic_eq</mat-icon> Simuler
        </button>
      </div>
    </section>

    <!-- Shared result pane -->
    <section class="result">
      <div class="result-head">
        <mat-icon class="material-symbols-outlined">terminal</mat-icon>
        <span class="cmd">{{ ranLabel() || "Aucune action exécutée" }}</span>
        @if (result(); as r) {
          <span class="code" [class.ok]="r.code === 0" [class.err]="r.code !== 0">code {{ r.code }}</span>
        }
      </div>
      <pre class="result-body">@if (busy()) {Exécution en cours…} @else if (result(); as r) {{{ r.stdout }}@if (r.stderr) {
{{ r.stderr }}}} @else {<span class="ph">La sortie de la commande s'affichera ici.</span>}</pre>
    </section>
  `,
    styles: [
        `
            .card {
                background: var(--mat-sys-surface-container);
                border-radius: 18px;
                padding: 18px 22px;
                margin-bottom: 14px;
            }
            .card h2 {
                font-size: 16px;
                font-weight: 500;
                margin: 0 0 8px;
            }
            .hint {
                margin: 0 0 14px;
                font-size: 13px;
                line-height: 1.5;
                color: var(--mat-sys-on-surface-variant);
            }
            code {
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
            }
            .input-row {
                display: flex;
                gap: 8px;
                margin-bottom: 12px;
                flex-wrap: wrap;
            }
            .text-in {
                flex: 1 1 200px;
                min-width: 0;
            }
            .num-in {
                flex: 0 0 80px;
                width: 80px;
            }
            .text-in,
            .num-in,
            .sel {
                border: 1px solid var(--mat-sys-outline-variant);
                border-radius: 10px;
                background: var(--mat-sys-surface-container-low);
                color: var(--mat-sys-on-surface);
                font: inherit;
                font-size: 14px;
                padding: 10px 12px;
                outline: none;
            }
            .text-in:focus,
            .num-in:focus,
            .sel:focus {
                border-color: var(--mat-sys-primary);
            }
            .btn-row {
                display: flex;
                gap: 8px;
                flex-wrap: wrap;
            }
            .act {
                display: inline-flex;
                align-items: center;
                gap: 8px;
                border: none;
                border-radius: 999px;
                padding: 9px 16px;
                font: inherit;
                font-size: 14px;
                cursor: pointer;
                color: var(--mat-sys-on-surface);
                background: var(--mat-sys-surface-container-high);
            }
            .act.primary {
                color: var(--mat-sys-on-primary);
                background: var(--mat-sys-primary);
            }
            .act:disabled {
                opacity: 0.5;
                cursor: default;
            }
            .act mat-icon {
                font-size: 18px;
                width: 18px;
                height: 18px;
            }
            .result {
                margin-top: 6px;
                border-radius: 14px;
                overflow: hidden;
                background: var(--mat-sys-surface-container-low);
            }
            .result-head {
                display: flex;
                align-items: center;
                gap: 8px;
                padding: 10px 14px;
                background: var(--mat-sys-surface-container-high);
                font-size: 13px;
            }
            .result-head mat-icon {
                font-size: 18px;
                width: 18px;
                height: 18px;
                color: var(--mat-sys-on-surface-variant);
            }
            .result-head .cmd {
                flex: 1 1 auto;
                min-width: 0;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                color: var(--mat-sys-on-surface-variant);
            }
            .code {
                flex: 0 0 auto;
                font-size: 11px;
                padding: 2px 8px;
                border-radius: 999px;
            }
            .code.ok {
                background: color-mix(in srgb, var(--mat-sys-tertiary) 16%, transparent);
                color: var(--mat-sys-tertiary);
            }
            .code.err {
                background: color-mix(in srgb, var(--mat-sys-error) 16%, transparent);
                color: var(--mat-sys-error);
            }
            .result-body {
                margin: 0;
                padding: 14px 16px;
                font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
                font-size: 12px;
                line-height: 1.55;
                color: var(--mat-sys-on-surface);
                white-space: pre-wrap;
                word-break: break-word;
                max-height: 320px;
                overflow: auto;
                min-height: 60px;
            }
            .ph {
                color: var(--mat-sys-on-surface-variant);
            }
        `,
    ],
})
export class ActionsTabComponent {
    private readonly aphrody = inject(AphrodyService);
    private readonly agent = inject(AgentService);

    readonly busy = signal(false);
    readonly result = signal<ExecResult | null>(null);
    readonly ranLabel = signal("");

    readonly goal = signal("");
    readonly maxIter = signal("50");
    readonly skillName = signal("");
    readonly notifyChannel = signal("slack");
    readonly notifyMsg = signal("Test depuis aphrody desktop.");
    readonly hermesChannel = signal("discord");
    readonly hermesMsg = signal("Bonjour aphrody");

    /** Run an aphrody or sibling command and show its captured output. */
    async run(label: string, runner: Runner, args: string[]): Promise<void> {
        if (this.busy()) return;
        this.busy.set(true);
        this.result.set(null);
        const prog = runner === "skills" ? "aphrody-skills" : "aphrody";
        this.ranLabel.set([prog, ...args].join(" "));
        try {
            const res =
                runner === "skills"
                    ? await this.agent.siblingExec("aphrody-skills", args)
                    : await this.aphrody.exec(args);
            this.result.set(res);
        } catch (err) {
            const msg =
                err instanceof HostUnavailableError ? err.message : `Échec : ${String(err)}`;
            this.result.set({ code: -1, stdout: "", stderr: msg });
        } finally {
            this.busy.set(false);
        }
    }

    startLoop(): void {
        const max = Number.parseInt(this.maxIter(), 10);
        const args = ["agy-loop", "start", "--goal", this.goal().trim()];
        if (Number.isFinite(max) && max > 0) {
            args.push("--max", String(max));
        }
        void this.run("Démarrage de la boucle", "aphrody", args);
    }

    runSkill(): void {
        void this.run("Lecture du skill", "skills", ["info", this.skillName().trim()]);
    }

    sendNotify(): void {
        void this.run("Notification", "aphrody", [
            "notify",
            "--channel",
            this.notifyChannel(),
            "--message",
            this.notifyMsg().trim(),
        ]);
    }

    checkHermes(): void {
        void this.run("Simulation hermes", "aphrody", [
            "hermes",
            "--channel",
            this.hermesChannel(),
            "--simulate",
            this.hermesMsg().trim(),
        ]);
    }
}
