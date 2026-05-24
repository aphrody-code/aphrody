// SPDX-License-Identifier: Apache-2.0
import { Component, inject, input, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { MatButtonModule } from "@angular/material/button";
import { MatIconModule } from "@angular/material/icon";
import { MatProgressSpinnerModule } from "@angular/material/progress-spinner";

import { AphrodyService, ExecResult } from "../../core/aphrody.service";

/** A single runnable aphrody command exposed in a tool view. */
export interface ToolAction {
  /** Human label shown on the button. */
  label: string;
  /** Material Symbol name. */
  icon: string;
  /** argv passed to `aphrody_exec` (without program name). */
  args: string[];
  /** Optional free-text input appended to args when present. */
  prompt?: { placeholder: string; flag?: string };
  /** Longer help text. */
  hint?: string;
}

/**
 * Generic tool surface: a titled header, a column of runnable actions (each
 * calls `aphrody_exec`), and a terminal-style output pane with the captured
 * stdout/stderr + exit code.
 */
@Component({
  selector: "app-tool-runner",
  imports: [FormsModule, MatButtonModule, MatIconModule, MatProgressSpinnerModule],
  templateUrl: "./tool-runner.component.html",
  styleUrl: "./tool-runner.component.scss",
})
export class ToolRunnerComponent {
  private readonly aphrody = inject(AphrodyService);

  readonly title = input.required<string>();
  readonly subtitle = input<string>("");
  readonly icon = input<string>("build");
  readonly actions = input.required<ToolAction[]>();

  readonly running = signal<string | null>(null);
  readonly result = signal<ExecResult | null>(null);
  readonly ranLabel = signal<string>("");
  readonly inputs = signal<Record<string, string>>({});

  setInput(label: string, value: string): void {
    this.inputs.update((m) => ({ ...m, [label]: value }));
  }

  /** Current input value for an action label (empty string when unset). */
  inputValue(label: string): string {
    return this.inputs()[label] ?? "";
  }

  async run(action: ToolAction): Promise<void> {
    if (this.running()) {
      return;
    }
    const args = [...action.args];
    if (action.prompt) {
      const val = (this.inputs()[action.label] ?? "").trim();
      if (val) {
        if (action.prompt.flag) {
          args.push(action.prompt.flag, val);
        } else {
          args.push(val);
        }
      }
    }
    this.running.set(action.label);
    this.result.set(null);
    this.ranLabel.set(["aphrody", ...args].join(" "));
    try {
      const res = await this.aphrody.exec(args);
      this.result.set(res);
    } catch (err) {
      this.result.set({ code: -1, stdout: "", stderr: String(err) });
    } finally {
      this.running.set(null);
    }
  }
}
