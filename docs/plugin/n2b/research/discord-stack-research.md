# Discord — Stack technique complète (recherche)

Date : 2026-04-18
Source : recherches web Claude Code

---

## 1. Rust chez Discord

### 1.1 Read States (migration Go → Rust)
- Service qui traque messages/canaux lus — sollicité à chaque connexion, envoi, lecture.
- **Problème Go** : GC toutes les 2 min scannait tout le cache LRU → pics de latence ~40 ms.
- **Après Rust** : libération immédiate à l'éviction LRU, pas de GC.
- Cache porté à **8 M entrées**, latence moyenne en microsecondes.
- Rust bat Go sur **CPU, mémoire ET latence**.

### 1.2 SortedSet NIF (Elixir + Rust, open source)
- Repo : `discord/sorted_set_nif`, crate **Rustler**.
- Structure : **Vector of Vectors** (skip-list-like), bucket par défaut 500.
- Types Elixir non supportés : `reference`, `pid`, `port`, `function`, `float`.
- Gains : **×6,5 best-case**, **×160 worst-case** vs Elixir pur.
- Usage : **Member List** (scaling jusqu'à 11 M utilisateurs concurrents).

### 1.3 Pipeline vidéo Go Live
- Rust côté client pour **capture + encodage** du streaming.

### 1.4 Migration Cassandra → ScyllaDB
- **Services de données intermédiaires en Rust** (gRPC, un endpoint par requête, zéro business logic).
- Feature clé : **request coalescing** (déduplication requêtes simultanées).
- **Migrator custom en Rust** : lecture token ranges → checkpoint local SQLite → firehose ScyllaDB.
- Résultat : **3,2 M messages/sec**, migration totale en **9 jours**.
- Latences : p99 read **15 ms** (vs 40–125 ms), p99 write **5 ms** (vs 5–70 ms).

### 1.5 Indexation recherche (trillions de messages)
- Architecture **Rust + Kubernetes + Elasticsearch multi-cluster** pour full-text search.

### 1.6 Game SDK
- Officiel en C++, bindings Rust officiels `discord_game_sdk`.
- Implémentation alternative open-source : `EmbarkStudios/discord-sdk`.

### 1.7 Discord Social SDK (2025, GDC)
- Sortie GDC 2025, intégré notamment dans le jeu **Rust** (Facepunch).

### Synthèse Rust
Rust = **langage par défaut pour tout nouveau service critique**. Justification : **absence de GC** + **prédictibilité latence** pour système temps réel.

---

## 2. Stack Frontend

### 2.1 Web / Desktop
| Couche | Techno |
|---|---|
| Framework | **React** (depuis ~2015) |
| State | **Redux** + custom hooks |
| Langage | **TypeScript** (migration progressive) |
| Styling | **CSS-in-JS** (styled-components / patterns similaires) |
| Temps réel | **WebSocket** streams → re-renders React minimisés |
| Desktop shell | **Electron** (Chromium + Node.js, main/renderer/preload) |
| Perf-critique | **C/C++** (voix, vidéo, codecs) + **WebRTC** |
| SolidJS | Testé en interne (features isolées), pas de migration globale |

### 2.2 Mobile
| Plateforme | Stack |
|---|---|
| iOS | **React Native** (depuis le lancement) |
| Android | Historiquement Kotlin natif → migration **React Native** (annoncée 2022, confirmée 2025/2026) |
| Code partagé | **90%+** React idiomatique iOS/Android |
| Natif ponctuel | Swift (iOS) / Kotlin (Android) pour décodage vidéo, animations complexes, clavier — bridge RN |
| Futur | **New Architecture** (Fabric/TurboModules), **Hermes statique**, migration core stores vers **Rust** |

Principe d'équipe : « tout le monde est mobile engineer », plus de silos iOS/Android.

---

## 3. Design System

### 3.1 Identité de marque

**Couleurs**
- **Blurple** `#5865F2` (RGB 88/101/242, PANTONE 2716 C) — signature
- **Nouveau Blurple 2024** `#161CBB` (rebrand "Group Chat That's All Fun and Games")
- Palette : Green `#57F287`, Yellow `#FEE75C`, Fuchsia `#EB459E`, Red `#ED4245`, White, Black
- Palette unifiée **produit ↔ marketing**

**Typographie**
- **gg sans** — fonte propriétaire custom par **Colophon Foundry**
- En production depuis **1er décembre 2022** (remplace Whitney)
- "gg" = good game (clin d'œil gaming)

**Mascotte & logo**
- **Clyde** redesigné septembre 2023 : sorti de sa bulle, antennes → "épaules", expressions faciales multiples officialisées
- Wordmark refondu septembre 2023

### 3.2 Outillage design (Figma-first, open source)

Pas de nom public type "Material" ou "Polaris". Discord publie une **suite de plugins Figma** :

| Plugin | Rôle |
|---|---|
| **Auto Theme** | Swap auto clair↔sombre via mapping de tokens (composants type status bar iOS auto-remappés) |
| **Design Lint** | Détecte éléments hors design system (typo, couleurs, tokens manquants) |
| **Inspector** | Inspection composants |
| **Table of Contents** | Navigation fichiers design |
| **Discord Project Scaffold** | Template démarrage projet |

Tous **open source** (`destefanis/auto-theme` etc.).

### 3.3 Composants produit (Components v2 — API publique)

Système tripartite exposé aux devs/bots :
- **Layout** : Action Row, Section, Container, Separator, Label
- **Content** : Text Display, Thumbnail, Media Gallery, File
- **Interactive** : Button, Select Menus, Text Input, File Upload

En interne : référencé comme **"CX"** (syntaxe JSX-like dans le codebase).

### 3.4 Ressources externes
- `discord.com/branding` — guidelines publiques (logos, couleurs, espacements)
- Discord Social SDK — Branding Guidelines (jeux intégrant Discord)
- Behance — "Discord Brand Design System & Guidelines"

### Synthèse Design System
1. **Figma = source de vérité**, outillée par plugins maison open-source
2. **Design tokens** → mapping centralisé clair/sombre
3. **Brand system** public (`/branding`) séparé du **system produit** (Components v2, interne "CX")
4. Cohérence via **lint automatique** plutôt que lib React documentée publiquement

---

## 4. Sources

### Rust
- [Why Discord is switching from Go to Rust](https://discord.com/blog/why-discord-is-switching-from-go-to-rust)
- [Using Rust to Scale Elixir for 11 Million Concurrent Users](https://discord.com/blog/using-rust-to-scale-elixir-for-11-million-concurrent-users)
- [discord/sorted_set_nif (GitHub)](https://github.com/discord/sorted_set_nif)
- [How Discord Stores Trillions of Messages](https://discord.com/blog/how-discord-stores-trillions-of-messages)
- [How Discord Migrated Trillions of Messages to ScyllaDB — The New Stack](https://thenewstack.io/how-discord-migrated-trillions-of-messages-to-scylladb/)
- [How Discord Indexes Trillions of Messages — ScyllaDB](https://www.scylladb.com/tech-talk/how-discord-indexes-trillions-of-messages-scaling-search-infrastructure/)
- [Discord Social SDK Updates & Integrations](https://discord.com/blog/discord-social-sdk-updates-integrations)
- [EmbarkStudios/discord-sdk (GitHub)](https://github.com/EmbarkStudios/discord-sdk)

### Frontend
- [Does Discord Use React or SolidJS? Deep Dive](https://medium.com/@bhagyarana80/does-discord-use-react-or-solidjs-a-deep-dive-into-their-frontend-stack-7e2874c50198)
- [Why Discord is Sticking with React Native](https://discord.com/blog/why-discord-is-sticking-with-react-native)
- [Supercharging Discord Mobile: Our Journey to a Faster App](https://discord.com/blog/supercharging-discord-mobile-our-journey-to-a-faster-app)
- [discord/react-native-sandbox-app (GitHub)](https://github.com/discord/react-native-sandbox-app)
- [Understanding Discord: Is Discord an Electron App?](https://www.dhiwise.com/post/discord-controversy-is-discord-an-electron-app)

### Design System
- [Discord's Brand Guidelines](https://discord.com/branding)
- [Building open-source design tools to improve Discord's design workflow](https://discord.com/blog/building-open-source-design-tools-to-improve-discords-design-workflow)
- [destefanis/auto-theme (GitHub)](https://github.com/destefanis/auto-theme)
- [Discord Component Reference — Docs](https://docs.discord.com/developers/components/reference)
- [Discord Social SDK — Branding Guidelines](https://discord.com/developers/docs/discord-social-sdk/design-guidelines/branding-guidelines)
- [Discord Brand Design System & Guidelines — Behance](https://www.behance.net/gallery/149619231/Discord-Brand-Design-System-Guidelines)
- [What Font Does Discord Use — FontsArena](https://fontsarena.com/blog/what-font-does-discord-use/)
- [Discord Brand Color Palette — Mobbin](https://mobbin.com/colors/brand/discord)
