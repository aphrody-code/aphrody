// SPDX-License-Identifier: Apache-2.0
/**
 * QA-only client entry: mounts the REAL Gemini AI Mode visual grammar
 * using pure Web Components without any React.
 */
import "@aphrody/material-web/all.js";
import "@aphrody/material-web/aphrody-components.js";
import { cssFromSeed } from "@aphrody/m3-tokens/dynamic-color";
import "./showcase.css";
import "./theme.css";

// Same baseline-theme bootstrap as app.ts
const baseTheme = document.createElement("style");
baseTheme.id = "m3-base-theme";
baseTheme.textContent = cssFromSeed("#6750a4");
document.head.appendChild(baseTheme);

const root = document.getElementById("root");
if (root) {
  root.innerHTML = `
    <div class="shell">
      <section class="section" id="gemini" style="padding: 24px;">
        <h2 class="section__head">Gemini AI Mode</h2>
        <div class="grid">
          <div class="specimen specimen--full">
            <div class="specimen__title">Gemini AI Mode surface</div>
            <div class="gemini">
              <!-- Left rail -->
              <div class="gemini__rail">
                <md-navigation-rail>
                  <md-navigation-rail-item label="Discover" selected>
                    <md-icon slot="active-icon">auto_awesome</md-icon>
                    <md-icon slot="inactive-icon">auto_awesome</md-icon>
                  </md-navigation-rail-item>
                  <md-navigation-rail-item label="History">
                    <md-icon slot="active-icon">history</md-icon>
                    <md-icon slot="inactive-icon">history</md-icon>
                  </md-navigation-rail-item>
                  <md-navigation-rail-item label="Compose">
                    <md-icon slot="active-icon">edit_note</md-icon>
                    <md-icon slot="inactive-icon">edit_note</md-icon>
                  </md-navigation-rail-item>
                </md-navigation-rail>
              </div>

              <!-- Main Panel -->
              <div class="gemini__main">
                <h3 class="gemini__greeting">Hi. What's on your mind?</h3>
                
                <div class="gemini__pillwrap">
                  <div class="gemini__pill">
                    <md-icon-button aria-label="Add context">
                      <md-icon>add</md-icon>
                    </md-icon-button>
                    <md-outlined-text-field
                      id="gemini-search-field"
                      class="gemini__field"
                      aria-label="Search"
                      placeholder="Search or ask anything"
                      value="material design 3"
                    ></md-outlined-text-field>
                    <md-icon-button id="gemini-clear-btn" aria-label="Clear" style="display: none;">
                      <md-icon>close</md-icon>
                    </md-icon-button>
                    <md-icon-button aria-label="Voice search">
                      <md-icon>mic</md-icon>
                    </md-icon-button>
                    <md-icon-button aria-label="Search by image">
                      <md-icon>photo_camera</md-icon>
                    </md-icon-button>
                    <md-assist-chip id="gemini-mode-chip" class="gemini__chip" label="AI Mode">
                      <md-icon slot="icon" class="gemini__sparkle">auto_awesome</md-icon>
                    </md-assist-chip>
                  </div>

                  <!-- Dropdown -->
                  <md-elevated-card id="gemini-dropdown" class="gemini__dropdown" style="display: none;">
                    <md-list id="gemini-suggestions-list"></md-list>
                    <div class="gemini__dropfoot">
                      <span>Report inappropriate predictions</span>
                      <a href="#inputs">Learn more</a>
                    </div>
                  </md-elevated-card>
                </div>

                <div class="gemini__actions">
                  <md-elevated-button id="gemini-search-btn">Search</md-elevated-button>
                  <md-elevated-button id="gemini-lucky-btn">
                    <md-icon slot="icon">bolt</md-icon>
                    I'm Feeling Lucky
                  </md-elevated-button>
                </div>

                <!-- SERP Results -->
                <div class="gemini__serp" id="gemini-serp">
                  <div class="gemini__results">
                    <div class="gemini__result">
                      <div class="gemini__reshead">
                        <md-avatar class="gemini__resfav" label="M3"></md-avatar>
                        <div class="gemini__resmeta">
                          <span class="gemini__ressite">Material Design</span>
                          <span class="gemini__resurl">m3.material.io &rsaquo; foundations</span>
                        </div>
                      </div>
                      <a href="#" class="gemini__restitle">Material Design 3 &mdash; the design system</a>
                      <p class="gemini__ressnip">
                        Material 3 is the latest version of Google's open-source design system, with updated tokens, dynamic colour and expressive components.
                      </p>
                    </div>
                    
                    <div class="gemini__result">
                      <div class="gemini__reshead">
                        <md-avatar class="gemini__resfav" label="MW"></md-avatar>
                        <div class="gemini__resmeta">
                          <span class="gemini__ressite">material-web</span>
                          <span class="gemini__resurl">github.com &rsaquo; material-components &rsaquo; material-web</span>
                        </div>
                      </div>
                      <a href="#" class="gemini__restitle">Material Web &mdash; Web Components for Material 3</a>
                      <p class="gemini__ressnip">
                        A library of Material Design 3 web components, self-contained on the --md-sys-* system tokens, with first-class React wrappers.
                      </p>
                    </div>
                  </div>

                  <!-- Knowledge Panel -->
                  <md-elevated-card class="gemini__knowledge">
                    <h4 class="gemini__khead">Material Design 3</h4>
                    <span class="gemini__ksub">Design System</span>
                    <md-divider style="margin: 12px 0;"></md-divider>
                    <p class="gemini__kdesc">
                      Material You is the new design language introduced in Android 12, featuring a highly personalized dynamic color theme engine based on a single source seed color.
                    </p>
                    <md-list class="gemini__klist">
                      <md-list-item>
                        <div slot="headline">Developer</div>
                        <div slot="supporting-text">Google LLC</div>
                      </md-list-item>
                      <md-list-item>
                        <div slot="headline">Initial release</div>
                        <div slot="supporting-text">October 2021</div>
                      </md-list-item>
                    </md-list>
                  </md-elevated-card>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  `;

  // Wire up autocomplete suggestion logic
  const searchField = document.getElementById("gemini-search-field") as any;
  const clearBtn = document.getElementById("gemini-clear-btn") as HTMLElement;
  const dropdown = document.getElementById("gemini-dropdown") as HTMLElement;
  const suggestionsList = document.getElementById("gemini-suggestions-list") as any;
  const serp = document.getElementById("gemini-serp") as HTMLElement;

  const SUGGESTIONS = [
    "material design 3 tokens",
    "material you dynamic color",
    "m3 expressive components",
    "material symbols variable axes",
    "adaptive layout window size class",
  ];

  if (suggestionsList) {
    suggestionsList.innerHTML = SUGGESTIONS.map(
      (s) => `
      <md-list-item type="button" class="gemini-suggestion-item" data-val="${s}">
        <md-icon slot="start" class="gemini__sparkle">auto_awesome</md-icon>
        <div slot="headline">${s}</div>
      </md-list-item>
    `,
    ).join("");
  }

  const updateQueryState = (val: string) => {
    searchField.value = val;
    const hasText = val.trim().length > 0;
    if (clearBtn) clearBtn.style.display = hasText ? "" : "none";
    if (dropdown) dropdown.style.display = hasText ? "" : "none";
    if (serp) serp.style.opacity = hasText ? "1" : "0.4";
  };

  searchField?.addEventListener("input", (e: any) => {
    updateQueryState(e.target.value);
  });

  clearBtn?.addEventListener("click", () => {
    updateQueryState("");
    searchField.focus();
  });

  document.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;
    const item = target.closest(".gemini-suggestion-item") as HTMLElement;
    if (item) {
      const val = item.getAttribute("data-val") || "";
      updateQueryState(val);
      if (dropdown) dropdown.style.display = "none";
    } else if (!target.closest(".gemini__pillwrap")) {
      if (dropdown) dropdown.style.display = "none";
    }
  });
}
