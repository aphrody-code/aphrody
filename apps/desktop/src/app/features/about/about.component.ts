// SPDX-License-Identifier: Apache-2.0
import { Component } from "@angular/core";

import { VersionCardComponent } from "../diagnostic/version-card.component";

/** About view — the aphrody logo, a typed version/system card, and a blurb. */
@Component({
  selector: "app-about",
  imports: [VersionCardComponent],
  template: `
    <div class="about">
      <img class="logo" src="assets/aphrody.webp" alt="aphrody" />
      <h1>aphrody</h1>
      <p class="tagline">Le CLI cross-platform ultime — assistant, reverse engineering et forensics.</p>
      <div class="card-wrap">
        <app-version-card />
      </div>
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
      .card-wrap {
        text-align: left;
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
export class AboutComponent {}
