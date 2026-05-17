# Material Design 3 & Google OS Native Architecture

Ce fichier sert de source de vérité absolue (Single Source of Truth) pour l'écosystème `aphrody` et la conception de l'OS. Il définit l'architecture hybride du God Mode, s'appuyant sur l'accélération matérielle et une implémentation **Material Design 3 (M3)** Desktop-First.

## 1. L'Architecture à 3 Piliers (The God Mode GUI)

L'interface graphique du système d'exploitation Google OS repose sur une fondation triptyque inébranlable, entièrement propulsée par le GPU.

### Pilier I : Rust (Performance & MD3 Natif)
Le socle applicatif et le moteur de rendu `WebView` (`wry` / `tao`) sont écrits en Rust pur. 
- **Objectif** : Vitesse d'exécution maximale, sécurité mémoire, et interopérabilité directe avec l'OS via FFI.
- **Rendu** : Hébergement des conteneurs UI et gestion du cycle de vie de la fenêtre.

### Pilier II : C++ (Terminal Windows Custom)
L'interface en ligne de commande (CLI) de l'OS est une évolution directe d'un **fork C++ de Windows Terminal**.
- **Cible** : `Microsoft.WindowsTerminalCanary_8wekyb3d8bbwe!App`
- **Rendu Extrême** : Utilisation stricte de l'**AtlasEngine** (Direct2D / Direct3D 11) pour un rendu de texte fluide à très haute fréquence d'images.
- **Objectif** : Un shell Google OS surpuissant, customisé au maximum, offrant l'expérience terminale la plus réactive au monde.

### Pilier III : Bun & JSX (Logique UI, CSS & Design Tokens)
La construction de l'interface, le templating, et la gestion dynamique des thèmes sont orchestrés par **Bun** avec du **JSX natif**.
- **Objectif** : Itération ultra-rapide, gestion simplifiée du CSS, et intégration parfaite des Design Tokens.
- **Composants** : `@material/web` natif enveloppé dans des composants JSX sans framework lourd (pas de React/Vue).

## 2. Le Moteur Graphique : D3D12 & WebGPU

Aucun rendu CPU toléré. L'ensemble de la surface graphique de Google OS est accéléré matériellement.

- **DirectX 12 / D3D11** : Backing natif ("dur") des surfaces applicatives via l'OS hôte.
- **WebGPU Natif** : Accélération des WebView et applications M3 via le processus GPU de Chromium.
- **Chrome Canary SxS** : Le backend de rendu pour les interfaces web/JSX s'appuie sur `Chrome SxS\chrome.exe (Canary)`, exploitant le processus GPU Chromium avec un backend **D3D11/12** (WebGPU et WebGL activés).
- **Vulkan** : L'intégration du SDK Vulkan est prévue comme évolution optionnelle future pour le cross-platform absolu.

## 3. Philosophie & Desktop Best Practices (M3)

- **Layout Adaptatif (Three-Pane)** : Grilles fluides et architectures en 3 panneaux (Navigation Rail, Content Area, Side Sheet).
- **Densité "High Density"** : Interface compacte, esthétique "Tooling" (dense, informative, industrielle).
- **Surface Containers** : Utilisation de `lowest`, `low`, `normal`, `high`, `highest`. L'élévation physique `<md-elevation>` est réservée aux modales et tooltips.
- **Survol et Focus** : Clic souris et clavier priorisés. États `:hover`, `:focus-visible` natifs.

## 4. Typographie (Google Sans)

- **UI & Display** : `Google Sans Flex` (Police variable, axes `wght`, `opsz`).
- **Éditeurs** : `Google Sans Text`.
- **CLI & Terminal (Pilier II)** : `Google Sans Mono`. Typographie inaltérable pour le rendu AtlasEngine.

## 5. Design Tokens (Dynamic Color) & Iconographie

- **CSS Variables exclusives** : `var(--md-sys-color-primary)`. Espace HCT (ou `oklch` pour les accents purs). Pas de Tailwind.
- **Icons** : `Material Symbols Rounded` (FILL 0/1, wght 400, opsz 24).

L'intégration de ces trois piliers propulse Aphrody et Google OS au-delà d'un simple wrapper, forgeant un environnement natif, résilient, et visuellement absolu.

## 6. Références Architectures Cibles

- Voir [Architecture OS 2026 : Meilleures Pratiques Unix](docs/google-os-plan/ARCHITECTURE_2026.md) pour les détails sur le noyau, l'I/O et le modèle de processus.
