# Monorepo Rust — Web/Desktop partagé + Mobile séparé + UI Discord/VS Code-like

Date : 2026-04-18

---

## 0. Objectif

- **Un seul monorepo** avec code maximal partagé
- **Types + DB + logique métier** communs à toutes les plateformes
- **Web + Desktop = même UI** (shell dense style **Discord / VS Code** : activity rail + sidebar + tabs + status bar)
- **Mobile = UI distincte** (optimisée touch/petit écran)
- **Stack Rust** bout-en-bout

---

## 1. Architecture du monorepo

### 1.1 Arborescence Cargo workspace

```
my-app/
├── Cargo.toml                    # workspace racine
├── crates/
│   ├── core/                     # Logique métier pure (Rust)
│   │   └── src/lib.rs            # - règles business
│   │                             # - use-cases
│   │                             # - pas de dépendance UI ni DB directe
│   │
│   ├── types/                    # DTOs, entities, enums
│   │   └── src/lib.rs            # partagé par TOUS les crates
│   │                             # sérialisable (serde)
│   │
│   ├── db/                       # Couche persistance
│   │   └── src/lib.rs            # sqlx/seaORM + migrations
│   │                             # expose un Repository trait
│   │
│   ├── api-client/               # Client HTTP typé (fetch)
│   │   └── src/lib.rs            # wrap server functions Leptos
│   │                             # ou reqwest sur WASM/native
│   │
│   ├── ui-shared/                # ⭐ Composants UI web+desktop
│   │   └── src/lib.rs            # Dioxus/Leptos components
│   │                             # Canvas Figma-like, panels, toolbars
│   │
│   ├── ui-mobile/                # ⭐ Composants UI mobile
│   │   └── src/lib.rs            # layouts tactiles, bottom nav
│   │                             # peut réutiliser ui-shared ponctuellement
│   │
│   └── design-tokens/            # Tokens partagés
│       └── src/lib.rs            # couleurs, spacing, typo
│                                 # exportés aussi en CSS/JSON
│
├── apps/
│   ├── web/                      # App navigateur (Leptos SSR + Axum)
│   │   ├── Cargo.toml
│   │   └── src/main.rs           # use ui_shared::*;
│   │
│   ├── desktop/                  # App Tauri 2 desktop
│   │   ├── src-tauri/            # backend Rust Tauri
│   │   └── frontend/             # même codebase que web
│   │       └── src/main.rs       # use ui_shared::*;
│   │
│   └── mobile/                   # App Tauri 2 iOS/Android
│       ├── src-tauri/
│       └── frontend/
│           └── src/main.rs       # use ui_mobile::*;
│
└── server/                       # API backend autonome
    └── src/main.rs               # Axum + db + core
```

### 1.2 Dépendances entre crates

```
               ┌──────────┐
               │  types   │  ← sérialisable, zéro dep
               └────┬─────┘
                    │
     ┌──────────────┼──────────────┐
     ▼              ▼              ▼
┌────────┐    ┌─────────┐    ┌─────────────┐
│  core  │    │   db    │    │ api-client  │
└───┬────┘    └────┬────┘    └──────┬──────┘
    │              │                 │
    └──────┬───────┘                 │
           ▼                          │
     ┌──────────┐                    │
     │  server  │                    │
     └──────────┘                    │
                                     │
    ┌────────────────────┬──────────┘
    ▼                    ▼
┌────────────┐    ┌────────────┐
│ ui-shared  │    │ ui-mobile  │
└─────┬──────┘    └─────┬──────┘
      │                  │
  ┌───┴────┐             │
  ▼        ▼             ▼
┌────┐ ┌────────┐ ┌──────────┐
│web │ │desktop │ │  mobile  │
└────┘ └────────┘ └──────────┘
```

Règle : **chaque crate dépend uniquement des étages inférieurs**. Le code métier n'importe jamais de l'UI.

### 1.3 Cargo.toml racine

```toml
[workspace]
resolver = "2"
members = [
  "crates/core",
  "crates/types",
  "crates/db",
  "crates/api-client",
  "crates/ui-shared",
  "crates/ui-mobile",
  "crates/design-tokens",
  "apps/web",
  "apps/desktop/src-tauri",
  "apps/mobile/src-tauri",
  "server",
]

[workspace.dependencies]
# Versions unifiées pour tous les crates
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
dioxus = "0.7"
leptos = "0.8"
axum = "0.8"
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio"] }
tauri = "2"
```

---

## 2. Stack recommandée

### 2.1 Option recommandée : **Dioxus 0.7** (un seul code web + desktop + mobile)

**Avantage décisif** : Dioxus permet d'avoir **le même composant** rendu sur Web (WASM), Desktop (WebView ou Blitz natif) et Mobile (WebView natif iOS/Android) — avec des features conditionnelles `cfg`.

```rust
// crates/ui-shared/src/canvas.rs
use dioxus::prelude::*;

#[component]
pub fn FigmaCanvas(doc: Signal<Document>) -> Element {
    rsx! {
        div {
            class: "canvas-root",
            // Même code pour web + desktop
        }
    }
}
```

```rust
// apps/web/src/main.rs
use ui_shared::FigmaCanvas;
fn main() { dioxus::launch(App) }

// apps/desktop/src-tauri/src/main.rs
// → Tauri 2 charge le même bundle WASM/JS

// apps/mobile/frontend/src/main.rs
use ui_mobile::MobileHome;  // UI différente
fn main() { dioxus::launch(App) }
```

### 2.2 Option alternative : **Leptos** (web + desktop) **+ Dioxus Mobile**

Si tu veux la perf **signals** de Leptos côté web/desktop :
- Web : Leptos SSR + Axum
- Desktop : Tauri 2 + Leptos (même bundle)
- Mobile : séparé (Dioxus Mobile ou React Native)

Inconvénient : deux frameworks UI différents, 2 mental models.

### 2.3 Stack finale (la plus pragmatique)

```
┌────────────────────────────────────────────────────┐
│  Backend                                           │
│  ├─ Axum (API REST/GraphQL)                        │
│  ├─ sqlx + PostgreSQL                              │
│  └─ crates/core + crates/types + crates/db         │
├────────────────────────────────────────────────────┤
│  Web (navigateur)                                  │
│  ├─ Dioxus 0.7 fullstack (SSR + WASM)              │
│  ├─ Tailwind 4                                     │
│  └─ crates/ui-shared                               │
├────────────────────────────────────────────────────┤
│  Desktop (macOS/Windows/Linux)                     │
│  ├─ Tauri 2                                        │
│  ├─ Frontend Dioxus (même que web)                 │
│  ├─ window-vibrancy + tauri-plugin-decorum         │
│  └─ crates/ui-shared                               │
├────────────────────────────────────────────────────┤
│  Mobile (iOS/Android)                              │
│  ├─ Tauri 2 (ou Dioxus Mobile)                     │
│  └─ crates/ui-mobile                               │
└────────────────────────────────────────────────────┘
```

---

## 3. Base de données partagée

### 3.1 Si « typedb » = types + DB partagés (interprétation probable)

```rust
// crates/types/src/lib.rs
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

// crates/db/src/lib.rs
pub trait ProjectRepository {
    async fn get(&self, id: Uuid) -> Result<Project>;
    async fn list(&self) -> Result<Vec<Project>>;
}

pub struct PgProjectRepository(pub PgPool);

impl ProjectRepository for PgProjectRepository {
    async fn get(&self, id: Uuid) -> Result<Project> {
        sqlx::query_as!(Project, "SELECT * FROM projects WHERE id = $1", id)
            .fetch_one(&self.0).await
    }
    // ...
}
```

Les mêmes types `Project` sont utilisés par :
- Le serveur (sqlx)
- Le client web/desktop/mobile (serde + fetch)
- La couche UI (props typés)

### 3.2 Si « TypeDB » = le produit (graph DB)

TypeDB (anciennement Grakn) a un [client Rust officiel](https://github.com/vaticle/typedb-client-rust). Remplace `sqlx` par `typedb_client` dans `crates/db/`.

### 3.3 Offline-first avec SQLite embarqué

Pour desktop + mobile, embarquer **SQLite** (via `bun:sqlite` côté JS ou `sqlx-sqlite` / `rusqlite` côté Rust) pour le cache local + sync avec le serveur. Pattern **CRDT** recommandé pour la collab type Figma.

---

## 4. UI Discord / VS Code-like pour le web (et desktop)

### 4.1 Le pattern commun Discord + VS Code

Les deux apps partagent une **grammaire de layout identique** — c'est devenu le standard de facto pour les outils desktop/pro :

```
┌──────────────────────────────────────────────────────────┐
│ Titlebar (drag zone + window controls)          [_][□][×] │
├──┬────────────┬─────────────────────────────────┬────────┤
│  │            │                                 │        │
│  │            │                                 │        │
│A │     B      │              C                  │   D    │
│  │            │                                 │        │
│  │            │                                 │        │
│  │            │                                 │        │
├──┴────────────┴─────────────────────────────────┴────────┤
│ Status bar (état, utilisateurs connectés, notifications) │
└──────────────────────────────────────────────────────────┘

A = Rail d'icônes (activity bar VS Code / server list Discord)
B = Sidebar contextuelle (explorer, channels list…)
C = Zone principale (éditeur, chat, canvas…) + tabs en haut
D = Panel auxiliaire optionnel (members, outline, preview…)
```

### 4.2 Design tokens Discord / VS Code

| Élément | Discord | VS Code | Reco unifiée |
|---|---|---|---|
| Fond principal | `#313338` | `#1E1E1E` | `#1E1F22` (dark) |
| Fond panels | `#2B2D31` | `#252526` | `#2B2D31` |
| Fond rail (icons) | `#1E1F22` | `#333333` | `#1A1B1E` |
| Hover | `#35373C` | `#2A2D2E` | `#35373C` |
| Sélection active | `#404249` | `#094771` | accent à 20 % alpha |
| Accent | `#5865F2` (Blurple) | `#007ACC` | **à choisir selon marque** |
| Texte primaire | `#F2F3F5` | `#CCCCCC` | `#E3E3E3` |
| Texte secondaire | `#B5BAC1` | `#969696` | `#9B9DA0` |
| Texte muted | `#80848E` | `#6A6A6A` | `#6D6F78` |
| Bordures | `#1E1F22` | `#3C3C3C` | `#2B2D31` |

| Spacing | Valeur |
|---|---|
| Rail d'icônes | 48 px (Discord) / 48 px (VS Code) |
| Sidebar default | 240 px |
| Titlebar | 30–38 px |
| Status bar | 22 px |
| Tab height | 35 px |
| Row compact | 24 px |
| Row confort | 32 px |
| Icon size | 16 px (VS Code) / 18 px (Discord) |
| Gutter | 8 px |

### 4.3 Typographie

| Usage | Font | Taille |
|---|---|---|
| UI labels | Inter / gg sans / Segoe UI Variable | 13 px |
| Body text | Inter | 14 px |
| Titles | Inter 600 | 14–16 px |
| Code / monospace | JetBrains Mono / SF Mono / Fira Code | 12–13 px |
| Status bar | Inter | 12 px |

### 4.4 Layout Dioxus shell Discord/VS Code-like

```rust
// crates/ui-shared/src/shell.rs
#[component]
pub fn AppShell() -> Element {
    rsx! {
        div { class: "h-screen flex flex-col bg-[#1e1f22] text-[#e3e3e3]",
            Titlebar {}                    // drag region + window controls
            div { class: "flex-1 flex overflow-hidden",
                ActivityRail {}            // A — rail icônes verticales
                Sidebar {}                 // B — panel contextuel (redimensionnable)
                MainArea {}                // C — tabs + contenu principal
                AuxPanel {}                // D — optionnel, toggleable
            }
            StatusBar {}                   // état + notifs
        }
    }
}
```

### 4.5 Composants clés à implémenter

**ActivityRail** (A) — rail vertical à gauche
```rust
#[component]
pub fn ActivityRail() -> Element {
    rsx! {
        nav { class: "w-12 bg-[#1a1b1e] flex flex-col items-center py-2 gap-1",
            RailButton { icon: "home",    active: true }
            RailButton { icon: "folder",  active: false }
            RailButton { icon: "search",  active: false }
            RailButton { icon: "git",     active: false }
            div { class: "flex-1" }
            RailButton { icon: "settings", active: false }
        }
    }
}
```

Chaque bouton :
- Icône mono 18–20 px
- Indicateur actif = barre verticale gauche (pill de 3–4 px × 24 px en `--accent`)
- Hover = fond `#35373C`
- Badge notification possible (rouge, coin supérieur droit)

**Sidebar** (B) — resizable avec poignée
```rust
#[component]
pub fn Sidebar(mut width: Signal<u32>) -> Element {
    rsx! {
        aside { class: "bg-[#2b2d31] flex flex-col", style: "width: {width}px;",
            SidebarHeader { title: "EXPLORER" }   // uppercase 11 px tracking
            SidebarContent {}                     // tree / liste
            ResizeHandle { width }                // 4 px bordure droite, cursor col-resize
        }
    }
}
```

**Tabs** (C) — style VS Code
```rust
#[component]
pub fn TabBar(tabs: Vec<Tab>, active: Signal<usize>) -> Element {
    rsx! {
        div { class: "h-9 bg-[#2b2d31] flex items-end overflow-x-auto",
            for (i, tab) in tabs.iter().enumerate() {
                TabItem {
                    tab: tab.clone(),
                    active: *active.read() == i,
                    onclick: move |_| active.set(i),
                }
            }
        }
    }
}
```

Chaque tab :
- Fond actif = `#1e1f22` (= contenu), fond inactif = `#2b2d31`
- Bordure top 2 px `--accent` si actif
- Icône fichier 14 px + nom 13 px + bouton fermer (×) sur hover
- Indicateur "modifié" = point `#f1fa8c`

**StatusBar** (bas)
```rust
#[component]
pub fn StatusBar() -> Element {
    rsx! {
        footer { class: "h-[22px] bg-[#5865f2] text-white text-xs flex items-center px-2 gap-3",
            span { "● Connected" }
            span { "UTF-8" }
            span { "Ln 42, Col 13" }
            div { class: "flex-1" }
            span { "2 issues" }
            span { "v1.0.0" }
        }
    }
}
```

**Command Palette** (`⌘K` / `Ctrl+Shift+P`) — central
- Modal floating en haut, 600 px large
- Fuzzy search immédiat
- Résultats catégorisés (Actions, Fichiers, Commandes…)
- Navigation clavier pure (↑↓ Enter Esc)

### 4.6 Interactions et raccourcis standards

| Raccourci | Action |
|---|---|
| `⌘K` / `Ctrl+K` | Command palette |
| `⌘P` / `Ctrl+P` | Go to file / quick switcher |
| `⌘B` / `Ctrl+B` | Toggle sidebar |
| `⌘J` / `Ctrl+J` | Toggle panel bas |
| `⌘⇧E` / `Ctrl+Shift+E` | Focus explorer |
| `⌘W` / `Ctrl+W` | Fermer tab |
| `⌘T` / `Ctrl+T` | Nouveau tab |
| `⌘1-9` | Aller au tab N |
| `⌘,` / `Ctrl+,` | Préférences |

### 4.7 Details qui font la différence

- **Coins arrondis 4 px** sur les hover/selection (pas plus — garde le look "pro/tool")
- **Pas d'ombres** ou alors très subtiles (`rgba(0,0,0,0.2)` max)
- **Scrollbars fines** (8 px), overlay, thumb `#4a4d53`
- **Icônes** : jeu unifié mono (Lucide, Phosphor, ou Fluent Icons)
- **Animations** : 100–150 ms ease-out, jamais spring ni bounce
- **Tooltips** : apparaissent après 500 ms hover, fond `#1e1f22`, bordure 1 px `#3c3c3c`
- **Context menus** : fond `#2b2d31`, item actif `#404249`, séparateurs `#3c3c3c` 1 px
- **Focus ring** : 1 px accent à 60 % opacité, pas de ring large type Tailwind default

### 4.8 Cas desktop spécifique (Tauri)

Pour un rendu **encore plus natif** :
- **macOS** : titlebar overlay + traffic lights natifs (`tauri-plugin-decorum`), `apply_vibrancy(Sidebar)` sur le rail et sidebar → le fond devient translucide et capte le wallpaper
- **Windows 11** : `apply_mica(dark)` pour un Mica backdrop authentique, titlebar custom avec zone de drag CSS `app-region: drag`
- **Linux** : fallback décorations classiques

```rust
#[cfg(target_os = "macos")]
{
    apply_vibrancy(&main, NSVisualEffectMaterial::Sidebar, None, None)?;
    // Le fond de la sidebar doit être transparent pour voir la vibrancy
}
#[cfg(target_os = "windows")]
apply_mica(&main, Some(true))?;
```

### 4.9 Canvas / zone de contenu principale

Contrairement à Figma, ici la zone C n'est **pas un canvas infini** mais :
- Un **éditeur de texte** (si VS Code-like) — `monaco-editor` ou `codemirror` via web-sys
- Un **flux de messages** (si Discord-like) — liste virtualisée
- Un **tableau de bord** — grille de widgets
- Ou un **canvas** pour sous-parties (diagrammes embarqués, preview…)

Virtualisation des listes **obligatoire** au-delà de 200 éléments (via `dioxus-virtualized` ou équivalent).

### 4.10 Placeholder Figma (si besoin ponctuel)

Le code SVG/wgpu évoqué plus tôt reste pertinent pour des **sous-vues** type "canvas de diagramme intégré" mais ce n'est plus le shell principal.

```rust
rsx! {
    svg {
        width: "100%", height: "100%",
        view_box: "{vb.x} {vb.y} {vb.w} {vb.h}",
        onmousedown: on_pan_start,
        onwheel: on_zoom,
        // layers rendus en <rect>, <path>, <text>
    }
}
```

### 4.11 Tokens CSS unifiés (partagés web + desktop)

```css
/* crates/design-tokens/tokens.css */
:root {
  /* Backgrounds (Discord/VS Code hybrid) */
  --bg-base:        #1e1f22;   /* contenu principal */
  --bg-panel:       #2b2d31;   /* sidebars */
  --bg-rail:        #1a1b1e;   /* activity rail */
  --bg-hover:       #35373c;
  --bg-selected:    #404249;
  --bg-tooltip:     #1e1f22;

  /* Borders */
  --border-subtle:  #2b2d31;
  --border-default: #3c3c3c;

  /* Text */
  --text-primary:   #e3e3e3;
  --text-secondary: #9b9da0;
  --text-muted:     #6d6f78;
  --text-disabled:  #4a4d53;

  /* Accent (Blurple / adaptable à ta marque) */
  --accent:         #5865f2;
  --accent-hover:   #4752c4;
  --accent-soft:    rgba(88, 101, 242, 0.2);

  /* Semantic */
  --success: #23a55a;
  --warning: #f0b232;
  --danger:  #f23f42;

  /* Radius */
  --radius-xs: 2px;
  --radius-sm: 4px;
  --radius-md: 6px;

  /* Spacing (4 px grid) */
  --space-1:  4px;
  --space-2:  8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;

  /* Dimensions shell */
  --titlebar-h:   32px;
  --statusbar-h:  22px;
  --rail-w:       48px;
  --sidebar-w:   240px;
  --tab-h:        35px;

  /* Motion */
  --ease-out:    cubic-bezier(0.2, 0, 0, 1);
  --dur-fast:   100ms;
  --dur-normal: 150ms;

  /* Shadow (minimal) */
  --shadow-popover: 0 4px 12px rgba(0, 0, 0, 0.3);
  --shadow-modal:   0 8px 24px rgba(0, 0, 0, 0.4);
}

/* Light theme override */
[data-theme="light"] {
  --bg-base:        #ffffff;
  --bg-panel:       #f5f5f5;
  --bg-rail:        #ececec;
  --bg-hover:       #e8e8e8;
  --bg-selected:    #e0e7ff;
  --text-primary:   #1f1f1f;
  --text-secondary: #5c5c5c;
  --text-muted:     #808080;
  --border-default: #d4d4d4;
}
```

Tailwind config :
```js
// tailwind.config.js
theme: {
  extend: {
    colors: {
      base:      'var(--bg-base)',
      panel:     'var(--bg-panel)',
      rail:      'var(--bg-rail)',
      hover:     'var(--bg-hover)',
      selected:  'var(--bg-selected)',
      accent:    'var(--accent)',
      'accent-soft': 'var(--accent-soft)',
      'text-1':  'var(--text-primary)',
      'text-2':  'var(--text-secondary)',
      'text-3':  'var(--text-muted)',
    },
    fontFamily: {
      sans: ['Inter', '-apple-system', 'Segoe UI Variable', 'system-ui', 'sans-serif'],
      mono: ['JetBrains Mono', 'SF Mono', 'Fira Code', 'monospace'],
    },
    fontSize: {
      micro: '11px',
      tiny:  '12px',
      ui:    '13px',
      body:  '14px',
    },
    spacing: {
      rail:     'var(--rail-w)',
      sidebar:  'var(--sidebar-w)',
      titlebar: 'var(--titlebar-h)',
      statusbar:'var(--statusbar-h)',
      tab:      'var(--tab-h)',
    },
    transitionDuration: {
      fast:   'var(--dur-fast)',
      normal: 'var(--dur-normal)',
    },
  }
}
```

### 4.12 Scrollbar fine unifiée

```css
/* Réutilisable partout, look VS Code */
.scroll-fine::-webkit-scrollbar { width: 10px; height: 10px; }
.scroll-fine::-webkit-scrollbar-track { background: transparent; }
.scroll-fine::-webkit-scrollbar-thumb {
  background: #4a4d53;
  border-radius: 5px;
  border: 2px solid var(--bg-base);
}
.scroll-fine::-webkit-scrollbar-thumb:hover { background: #5a5d63; }
```

### 4.13 Interactions clés

- **Command palette** (`⌘K` / `Ctrl+Shift+P`) — fuzzy search global
- **Keyboard shortcuts globaux** — registre centralisé, rebindable
- **Drag & drop** pour import fichiers
- **Multi-selection** (click + shift, drag box)
- **Undo/Redo** centralisé (reducer via signals Dioxus)
- **Collaborative cursors** via WebSocket (si multi-user)
- **Virtualisation listes** obligatoire (>200 items)

### 4.6 Différences Web ↔ Desktop (mineures)

Via `cfg(target_family = "wasm")` ou detection runtime :
- **Desktop** : menus natifs (via Tauri `Menu`), fenêtre transparente avec Mica/Vibrancy
- **Web** : menus HTML, fond opaque

```rust
#[cfg(not(target_family = "wasm"))]
fn setup_native() { /* vibrancy, menus Tauri */ }

#[cfg(target_family = "wasm")]
fn setup_native() {}
```

---

## 5. Mobile — UI distincte

`crates/ui-mobile` expose des composants adaptés tactile :

```rust
#[component]
pub fn MobileHome() -> Element {
    rsx! {
        div { class: "h-screen flex flex-col",
            MobileHeader {}              // barre top simple
            ProjectList {}               // liste scrollable, pas de canvas
            BottomNav {                  // tab bar native-like
                tabs: vec!["Home", "Docs", "Profil"]
            }
        }
    }
}
```

Principes :
- **Pas de canvas Figma** (inadapté mobile)
- **Listes verticales** + cards
- **Bottom navigation** (iOS) ou **tab bar** (Android)
- **Touch targets ≥ 44 pt**
- **Safe areas** respectées (notch, home indicator)
- Réutilise `core`, `types`, `api-client`, `design-tokens` — **tout sauf la couche présentation**

---

## 6. Build et déploiement

### 6.1 Commandes uniques via `just`
```justfile
# justfile à la racine
dev-web:         cd apps/web && dx serve
dev-desktop:     cd apps/desktop && cargo tauri dev
dev-mobile-ios:  cd apps/mobile && cargo tauri ios dev
dev-mobile-and:  cd apps/mobile && cargo tauri android dev
dev-server:      cd server && cargo watch -x run

build-all:
  cd apps/web && dx build --release
  cd apps/desktop && cargo tauri build
  cd apps/mobile && cargo tauri ios build && cargo tauri android build
  cd server && cargo build --release

test:    cargo test --workspace
lint:    cargo clippy --workspace -- -D warnings
fmt:     cargo fmt --all
```

### 6.2 CI (GitHub Actions)
- Matrix build : `ubuntu-latest`, `macos-latest`, `windows-latest`
- Cache Cargo + Tauri via `Swatinem/rust-cache`
- `tauri-action` pour les releases desktop
- Fastlane ou `tauri-action` pour mobile

---

## 7. Récapitulatif des décisions clés

| Décision | Choix | Pourquoi |
|---|---|---|
| Framework UI | **Dioxus 0.7** | Web + Desktop + Mobile avec le même crate |
| Backend | **Axum + sqlx** | Standard Rust, intégration Dioxus fullstack |
| Shell desktop | **Tauri 2** | Binaires légers + Mica/Vibrancy natif |
| Shell mobile | **Tauri 2** | Même écosystème, moins de friction |
| Styling | **Tailwind 4 + tokens CSS** | Cohérence cross-platform |
| Canvas Figma | **wgpu** (ou SVG en MVP) | Perf WebGPU + natif via même code |
| DB | **PostgreSQL + sqlx** | Types partagés via `#[derive(FromRow)]` |
| Monorepo | **Cargo workspace** | Standard, `workspace.dependencies` unifié |
| Tooling | **just + cargo + dx + cargo-tauri** | Minimal, natif Rust |

---

## 8. Structure finale concrète

```
my-app/
├── Cargo.toml                    # workspace
├── justfile                      # tâches
├── crates/
│   ├── types/                    # entities + DTOs (serde)
│   ├── core/                     # use-cases purs
│   ├── db/                       # sqlx repos
│   ├── api-client/               # wrap HTTP
│   ├── design-tokens/            # tokens + CSS
│   ├── ui-shared/                # UI web + desktop (Figma-like)
│   └── ui-mobile/                # UI mobile tactile
├── apps/
│   ├── web/                      # Dioxus web (WASM)
│   ├── desktop/                  # Tauri 2 + frontend Dioxus
│   └── mobile/                   # Tauri 2 iOS/Android
└── server/                       # Axum API
```

---

## 9. Sources

- [Cargo Workspaces — Rust Book](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [Monorepos with Cargo Workspace — Earthly](https://earthly.dev/blog/cargo-workspace-crates/)
- [Dioxus 0.7 — Fullstack crossplatform framework](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop/)
- [Tauri v2 with Next.js: Monorepo Guide](https://melvinoostendorp.nl/blog/tauri-v2-nextjs-monorepo-guide)
- [How to build multiple Tauri2 apps from a monorepo — Discussion](https://github.com/orgs/tauri-apps/discussions/13941)
- [AxonotesCore — Tauri monorepo example (GitHub)](https://github.com/axonotes/AxonotesCore)
- [Shared core and types — Crux (Red Badger)](https://redbadger.github.io/crux/getting_started/core.html)
- [TypeDB Rust client](https://github.com/vaticle/typedb-driver)
- [Figma design system patterns — Figma Community](https://www.figma.com/community)
- [wgpu (WebGPU Rust)](https://wgpu.rs/)
