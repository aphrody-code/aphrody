// SPDX-License-Identifier: Apache-2.0
import { Component, inject, OnInit, signal } from "@angular/core";
import { MatIconModule } from "@angular/material/icon";
import { AphrodyService, Meta } from "../../core/aphrody.service";

/** About view — the aphrody logo, shell metadata, and a short description. */
@Component({
  selector: "app-about",
  imports: [MatIconModule],
  template: `
    <div class="about">
      <img class="logo" src="assets/aphrody.webp" alt="aphrody" />
      <h1>aphrody</h1>
      <p class="tagline">Le CLI cross-platform ultime — assistant, reverse engineering et forensics.</p>
      @if (meta(); as m) {
        <div class="grid">
          <div class="row"><span>Version de l'app</span><b>{{ m.app_version }}</b></div>
          <div class="row"><span>Système</span><b>{{ m.target_os }}</b></div>
          <div class="row"><span>Architecture</span><b>{{ m.target_arch }}</b></div>
          <div class="row"><span>Famille</span><b>{{ m.family }}</b></div>
        </div>
      }
      <p class="note">
        Interface bâtie avec Angular 21 + Angular Material 21, dans une coque Tauri,
        propulsée en local par le binaire aphrody. Apparence inspirée de l'app Gemini.
      </p>
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
        height: 100%;
        overflow-y: auto;
      }
      .about {
        max-width: 560px;
        margin: 0 auto;
        padding: 56px 28px;
        text-align: center;
      }
      .logo {
        width: 120px;
        height: auto;
        object-fit: contain;
        filter: drop-shadow(0 6px 24px rgba(20, 32, 79, 0.55));
      }
      h1 {
        margin: 18px 0 4px;
        font-size: 34px;
        font-weight: 300;
        letter-spacing: -0.5px;
      }
      .tagline {
        margin: 0 0 28px;
        color: var(--mat-sys-on-surface-variant);
        font-size: 15px;
      }
      .grid {
        display: flex;
        flex-direction: column;
        gap: 1px;
        border-radius: 16px;
        overflow: hidden;
        background: var(--mat-sys-outline-variant);
        text-align: left;
      }
      .row {
        display: flex;
        justify-content: space-between;
        padding: 14px 18px;
        background: var(--mat-sys-surface-container);
        font-size: 14px;
      }
      .row span {
        color: var(--mat-sys-on-surface-variant);
      }
      .note {
        margin-top: 24px;
        font-size: 13px;
        line-height: 1.6;
        color: var(--mat-sys-on-surface-variant);
      }
    `,
  ],
})
export class AboutComponent implements OnInit {
  private readonly aphrody = inject(AphrodyService);
  readonly meta = signal<Meta | null>(null);

  async ngOnInit(): Promise<void> {
    try {
      this.meta.set(await this.aphrody.meta());
    } catch {
      // non-fatal
    }
  }
}
