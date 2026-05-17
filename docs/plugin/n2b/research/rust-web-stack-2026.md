# Stack Rust web 2026 + Web app avec look desktop natif — recherche approfondie

Date : 2026-04-18 (réflexion v2)

---

## 0. Clarification du problème

« Web app avec un style d'app desktop natif » peut vouloir dire **trois choses différentes** — le bon choix de stack dépend de laquelle :

| Scénario | Exemple | Approche |
|---|---|---|
| **A.** App **dans le navigateur** qui ressemble à une app desktop | Linear, Notion web | Leptos/Dioxus web + design system « desktop » |
| **B.** App **desktop native** qui réutilise le web stack | Discord, Slack (web shell), 1Password | **Tauri 2** + Leptos/React |
| **C.** App **100% native** avec UI déclarative type web | Zed, Raycast | **GPUI / Dioxus Native (Blitz) / Slint** |

Le bon choix dépend de :
- **Perf** (C > B > A)
- **Équipe** (familiarité Rust vs JS)
- **Distribution** (web seul = A, binaires = B/C)
- **Mobile** (B avec Tauri 2 ou React Native)

---

## 1. Stack Rust web 2026 — analyse complète

### 1.1 Backend (serveur)

| Framework | Points forts | Limites | Verdict 2026 |
|---|---|---|---|
| **Axum** | Async Tokio, ergonomique, SSR/WS/middleware, intégration Leptos native | — | **Défaut recommandé** |
| **Actix Web** | Perf brutes, actor model | API moins moderne | Cas de perf extrême |
| **Rocket** | DX top, macros | Moins d'écosystème async | Bon pour REST pur |
| **Salvo** | Simple, middleware composable | Plus jeune | Alternative niche |
| **Loco** | Rails-like full-stack | Opinioné | Rapid prototyping |

### 1.2 Frontend WASM

| Framework | Modèle de rendu | Full-stack | Cibles | Courbe |
|---|---|---|---|---|
| **Leptos** | Signals fine-grained (pas de VDOM) | ✅ SSR + server functions | Web, SSR | Moyenne |
| **Dioxus 0.7** | VDOM React-like + hot-patching Rust | ✅ fullstack WS | Web, Desktop, Mobile, TUI, **Native (Blitz)** | Douce (si React) |
| **Yew** | VDOM classique | Limité | Web | Douce |
| **Sycamore** | Signals (comme Leptos) | SSR | Web | Moyenne |

**Nouveauté critique 2026 : Dioxus 0.7**
- **Dioxus Native** — rendu 100% GPU via WGPU + moteur HTML/CSS **Blitz**
- **Hot-patching Rust** au runtime (édition code sans perdre l'état)
- **WASM bundle splitting**
- **Fullstack WebSockets** en 1 ligne
- Taille binaire desktop **< 5 Mo** (WebView système)

**Leptos 2026**
- Signals > VDOM pour perf DOM
- Server functions **type-safe bout-en-bout**
- `cargo-leptos` = outil officiel (Tailwind intégré, pas de Node requis)
- Écosystème plus mûr côté SSR

### 1.3 Moteur de rendu natif (emerging)

| Projet | État | Approche |
|---|---|---|
| **Blitz** (Dioxus) | Beta fin 2025, **prod 2026** | Composants Servo (stylo, html5ever, taffy, parley, vello, wgpu) — HTML/CSS **sans JS ni WebView** |
| **Verso** (Tauri) | Expérimental | Wrapper Servo haut-niveau, runtime Tauri alternatif à wry |
| **GPUI** (Zed) | Pre-1.0 | Hybrid immediate/retained, 120 FPS, game-engine-like |
| **Freya** | Actif | Dioxus logic + Skia rendering (no webview) |

Ces projets signalent une **convergence 2026** : **rendre HTML/CSS en pur Rust via GPU**, sans dépendre du WebView OS ni d'Electron.

---

## 2. Web app avec look desktop natif — trois voies

### Voie A — App web qui *ressemble* à du desktop

**Stack** : Leptos + Axum + Tailwind 4 + design tokens custom

Pas de shell natif. On reste dans le browser. On obtient le look via :
- Typographie système (`font-family: -apple-system, "Segoe UI Variable", Inter`)
- Design tokens qui imitent macOS Tahoe / Windows 11 Fluent
- Animations courtes (150–200 ms ease-out)
- Keyboard-first navigation, raccourcis `⌘K` etc.
- Dense information layout (tableaux, listes)

**Exemples** : Linear, Notion, Cron (web), Height.

**Limites** : pas de vraies API OS (notifications, menus, fichiers).

### Voie B — Tauri 2 (WebView système + Rust)

**Recommandé pour 95% des cas.**

```
┌──────────────────────────────────────────┐
│ Tauri 2 (shell natif Rust)               │
│  ├─ Backend : commandes Rust + IPC       │
│  └─ Frontend : WebView OS natif          │
│      └─ Leptos | Dioxus | React          │
└──────────────────────────────────────────┘
```

**Chiffres 2026 vs Electron** :
- **Binaire** : ~2 Mo vs 60+ Mo
- **Cold start** : <500 ms vs 1–2 s
- **RAM** : 30–80 Mo vs 200–400 Mo
- **Sécurité** : permissions default-locked vs opt-out
- **Mobile** : **iOS + Android supportés** (Tauri 2)

**Techniques clés pour un rendu vraiment natif** :

#### 2.B.1 Fenêtre sans décorations + transparente
```json
// tauri.conf.json
{
  "windows": [{
    "decorations": false,
    "transparent": true,
    "titleBarStyle": "Overlay"
  }],
  "macOSPrivateApi": true
}
```
```css
html, body { background: transparent; }
```

#### 2.B.2 Effets natifs avec `window-vibrancy`
| OS | Effet | API |
|---|---|---|
| macOS 10.10+ | NSVisualEffectView | `apply_vibrancy(HudWindow / Sidebar / Menu)` |
| Windows 7/10 | Blur-behind | `apply_blur()` |
| Windows 10/11 | Acrylic | `apply_acrylic()` |
| Windows 11 | **Mica** | `apply_mica(dark_mode)` |
| Linux | Dépend compositor | — |

```rust
#[cfg(target_os = "macos")]
apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None).unwrap();
#[cfg(target_os = "windows")]
apply_mica(&window, Some(true)).unwrap();
```

#### 2.B.3 Titlebar overlay cross-platform — `tauri-plugin-decorum`
- Conserve Snap Layout (Windows) + traffic lights (macOS) natifs
- Titlebar transparente overlay sans réinventer les controls
- Hauteur configurable

#### 2.B.4 Menus natifs
```rust
use tauri::menu::{Menu, MenuItem, Submenu};
// Menus système (pas HTML) → rendu 100% OS
```

#### 2.B.5 Raccourcis globaux, notifications, tray
Tous via plugins officiels : `@tauri-apps/plugin-{global-shortcut, notification, tray, autostart, updater, dialog, fs, sql, stronghold}`.

**Production apps Tauri connues** : 1Password (supporter), Cap, Pot, de nombreux downloaders, apps AI desktop.

### Voie C — Natif pur (le « Raycast pattern »)

Pour un feel **vraiment** natif, sans compromis :

| Stack | Quand |
|---|---|
| **Dioxus Native** (Blitz) | Web devs qui veulent HTML/CSS + pas de WebView + Rust |
| **Slint** | DSL dédié, meilleur outillage, embarqué possible |
| **GPUI** (Zed) | Projets ambitieux perf-first, UI custom (120 FPS) |
| **Floem** | Signals-based, mental model proche Leptos |
| **Iced** | Elm architecture, ergonomique |
| **egui** | Outils internes, debug UIs, itération rapide |
| **Freya** | Dioxus + Skia (no webview) |

**Le « Raycast pattern »** (intéressant à étudier) :
- Shell **100% natif** (Swift/SwiftUI sur macOS)
- Extensions tierces en **React + TypeScript + Node**
- IPC via **JSON-RPC** sur stdin/stdout (`DispatchIO`)
- Chaque extension **rend du natif** via un mapping vers composants Swift
- Résultat : extensibilité web + rendu 100% natif

Équivalent Rust possible : shell en GPUI/Slint + extensions en WASM (Leptos/Dioxus compilant vers composants natifs).

---

## 3. Recommandations concrètes selon le projet

### 3.1 SaaS web classique (Linear/Notion-like)
```
Axum + Leptos (SSR) + Tailwind 4 + PostgreSQL (sqlx) + cargo-leptos
```
+ design tokens desktop-like dans Tailwind config.

### 3.2 App desktop prioritaire + web en bonus
```
Tauri 2
  ├─ Frontend : Leptos (ou React si équipe JS)
  ├─ window-vibrancy (Mica + NSVisualEffect)
  ├─ tauri-plugin-decorum (titlebar cross-platform)
  └─ Backend Rust : Axum si besoin d'un serveur à part
```

### 3.3 Desktop premium perf-first
```
Dioxus 0.7 + Dioxus Native (Blitz)  // si prêt à être early adopter
ou
GPUI (si tu peux vivre avec pre-1.0)
ou
Slint (si DSL ne pose pas de problème)
```

### 3.4 Mobile + desktop depuis un seul code
```
Tauri 2 (iOS + Android + Win + macOS + Linux)
  └─ Leptos ou Dioxus frontend
```

---

## 4. Matrice de décision

| Critère | Tauri 2 | Dioxus Native | GPUI | Leptos web | Slint |
|---|---|---|---|---|---|
| Taille binaire | 2–10 Mo | ~5 Mo | 10–30 Mo | — (web) | 5–15 Mo |
| Démarrage | <500 ms | <200 ms | <100 ms | — | <100 ms |
| Look natif | Via CSS + vibrancy | Via CSS | 100% custom | Limité au browser | DSL natif |
| Stack JS réutilisable | ✅ | ❌ (Rust only) | ❌ | ✅ | ❌ |
| Mobile | ✅ iOS/Android | 🟡 en cours | ❌ | ❌ | ✅ |
| Prod-ready | ✅ | 🟡 2026 | 🟡 pre-1.0 | ✅ | ✅ |
| Accessibilité | ✅ (webview OS) | ✅ (accesskit) | 🟡 | ✅ | ✅ |
| Hot reload | ✅ | ✅ (Rust patching) | 🟡 | ✅ | ✅ |

---

## 5. Stack finale recommandée — 2026

### Pragmatique et production-ready **aujourd'hui**
```
Tauri 2 (shell)
  ├─ Frontend : Leptos 0.8+ (Rust → WASM)
  ├─ Styling : Tailwind 4 (via cargo-leptos, sans Node)
  ├─ Effets natifs : window-vibrancy
  ├─ Titlebar : tauri-plugin-decorum
  ├─ Menus/tray/shortcuts : plugins officiels Tauri
  └─ Backend : Axum (si API externe) + sqlx
```

### Audacieux, early-adopter **fin 2026**
```
Dioxus 0.7 + Dioxus Native (Blitz, pur Rust, pas de WebView)
  └─ Même code → Web (WASM) + Desktop (Blitz) + Mobile
```

### Le meilleur des deux mondes
Commencer en **Voie B (Tauri 2 + Leptos)** puis migrer progressivement vers **Voie C (Blitz)** quand Blitz sera GA — l'API Dioxus est la même, seul le renderer change.

---

## 6. Checklist "look natif" (à appliquer quelle que soit la voie)

### Design tokens
- [ ] Font stack : `-apple-system, BlinkMacSystemFont, "Segoe UI Variable", Inter, system-ui`
- [ ] Base size : 13 px macOS / 14 px Windows / 14 px Linux
- [ ] Line-height : 1.4 body, 1.2 UI dense
- [ ] Radius : 8–12 px (macOS), 4–8 px (Windows 11)
- [ ] Shadows : multi-couches douces, jamais dures
- [ ] Touch targets : ≥ 44 pt (HIG) / 32 px (Win11)

### Couleurs
- [ ] Palette adaptative light/dark via `prefers-color-scheme`
- [ ] Accent color depuis OS (`accentcolor` CSS ou API Tauri)
- [ ] Vibrancy/blur uniquement sur sidebars et popovers

### Mouvement
- [ ] Animations 150–250 ms
- [ ] Ease-out court ou spring (évite linear)
- [ ] Respect `prefers-reduced-motion`

### Patterns
- [ ] Menu bar natif (macOS) / system tray (Windows)
- [ ] Keyboard shortcuts standards (`⌘,` préférences, `⌘K` palette, `⌘F` recherche)
- [ ] Scrollbars fines overlay
- [ ] Focus ring visible mais discret
- [ ] Drag & drop natif pour fichiers

### Accessibilité
- [ ] ARIA labels sur tous les composants custom
- [ ] AccessKit si Dioxus Native / Slint / etc.
- [ ] Contraste AA minimum

---

## 7. Sources

### Frameworks web
- [Leptos — Home](https://leptos.dev/)
- [leptos-rs/leptos (GitHub)](https://github.com/leptos-rs/leptos)
- [Building Web Applications with Leptos 2026 — Reintech](https://reintech.io/blog/building-web-applications-with-leptos-complete-guide-2026)
- [Leptos vs Dioxus 2026 — Rustify](https://rustify.rs/articles/leptos-vs-dioxus-rust-frontend-2026)
- [Leptos + Tailwind setup — 8vi.cat](https://8vi.cat/full-stack-with-rust-axum-leptos-tailwind-css/)
- [cargo-leptos Tailwind without Node — Medium](https://medium.com/@edgedoval/leptos-can-use-tailwind-css-and-daisyui-without-node-js-8b174e222c60)

### Dioxus / Blitz (nouveautés 2026)
- [Dioxus v0.7.0 Release Notes](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.0)
- [Dioxus 0.7: full-stack everywhere — Medium](https://medium.com/@trivajay259/dioxus-0-7-the-rust-ui-release-that-finally-feels-full-stack-everywhere-89f482ee97e3)
- [DioxusLabs/blitz (GitHub)](https://github.com/DioxusLabs/blitz)
- [Blitz — About](https://blitz.is/about)
- [dioxus-native (crates.io)](https://crates.io/crates/dioxus-native)

### Tauri 2
- [Tauri 2.0](https://tauri.app/)
- [Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/)
- [Tauri vs Electron 2026 — Tech Insider](https://tech-insider.org/tauri-vs-electron-2026/)
- [Window Customization — Tauri docs](https://v2.tauri.app/learn/window-customization/)
- [tauri-apps/window-vibrancy](https://github.com/tauri-apps/window-vibrancy)
- [clearlysid/tauri-plugin-decorum](https://github.com/clearlysid/tauri-plugin-decorum)
- [Experimental Tauri Verso Integration](https://v2.tauri.app/blog/tauri-verso-integration/)
- [versotile-org/tauri-runtime-verso](https://github.com/versotile-org/tauri-runtime-verso)
- [awesome-tauri (GitHub)](https://github.com/tauri-apps/awesome-tauri)

### GUI natifs Rust
- [GPUI — Home](https://www.gpui.rs/)
- [Zed/GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md)
- [Leveraging Rust and the GPU to render UIs at 120 FPS — Zed Blog](https://zed.dev/blog/videogame)
- [2025 Survey of Rust GUI Libraries — boringcactus](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html)
- [The State of Rust GUI — Rust Bytes](https://weeklyrust.substack.com/p/the-state-of-rust-gui-the-good-and)

### Design natif
- [Apple Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines)
- [WWDC25: Get to know the new design system](https://developer.apple.com/videos/play/wwdc2025/356/)
- [How the Raycast API and extensions work — Raycast Blog](https://www.raycast.com/blog/how-raycast-api-extensions-work)
- [Designing desktop apps for cross-platform UX — ToDesktop](https://www.todesktop.com/blog/posts/designing-desktop-apps-cross-platform-ux)
- [Window Vibrancy Effects — ToDesktop](https://www.todesktop.com/docs/windows/window-vibrancy-effects)
