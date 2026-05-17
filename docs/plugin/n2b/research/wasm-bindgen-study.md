# Étude de `wasm-bindgen` — /home/ubuntu/rsbun/wasm/wasm-bindgen/

Date : 2026-04-18
Branche : `perf` (3 commits au-dessus de `main`, dont le patch Bun-native)

---

## 0. Chiffres clés

| Métrique | Valeur |
|---|---|
| Version | **0.2.118** (SCHEMA_VERSION identique) |
| LOC Rust totales | **303 237** |
| Crates dans le workspace | 14 |
| Examples | 21 (hello_world, canvas, fetch, dom, raytrace-parallel…) |
| Bindings web-sys générés | **~19 761 fn publiques** depuis **26 316 lignes de WebIDL** |
| Téléchargements cumulés (crates.io) | **326 M** |

---

## 1. Architecture du workspace

```
wasm-bindgen/                        ← crate racine (runtime côté Rust)
├── src/lib.rs                       (1835 lignes) — JsValue, conversions, intrinsèques
├── crates/
│   ├── macro/                       (75 lignes) — proc-macro `#[wasm_bindgen]` (wrapper)
│   ├── macro-support/               (2492 lignes) — impl des macros + parser
│   ├── shared/                      (307 lignes) — schema partagé macro↔CLI
│   ├── cli/                         — binaires `wasm-bindgen` + `wasm-bindgen-test-runner`
│   ├── cli-support/                 (888 lignes lib.rs) — moteur de génération
│   │   ├── descriptor.rs            — type descriptors encodés dans le .wasm
│   │   ├── descriptors.rs           — extraction depuis le module
│   │   ├── interpreter/             — mini-VM pour exécuter les descriptors
│   │   ├── js/                      — génération JS
│   │   ├── wit/                     — Interface Types (standard + nonstandard)
│   │   ├── transforms/              — externref, multi-value, threads, catch
│   │   ├── externref.rs             — support WebAssembly reference types
│   │   └── multivalue.rs            — WebAssembly multi-value proposal
│   ├── js-sys/                      — bindings JS globals (Array, Promise, Object…)
│   ├── web-sys/                     — bindings Web APIs (auto-générés)
│   │   └── webidls/enabled/         — 26 K lignes WebIDL source
│   ├── webidl/                      — parseur/codegen WebIDL → Rust
│   ├── webidl-tests/                — tests du générateur
│   ├── futures/                     — wasm-bindgen-futures (async ↔ Promise)
│   ├── test/ + test-macro/ + test-shared/ — framework `wasm-bindgen-test`
│   ├── typescript-tests/            — tests TS d'intégration
│   └── msrv/                        — garde la MSRV
```

---

## 2. Flow de compilation Rust → WASM → JS

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Code Rust                                                    │
│    #[wasm_bindgen] pub fn add(a: i32, b: i32) -> i32 { a+b }    │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. Macro expansion (wasm_bindgen_macro_support::expand)         │
│    - Parse signatures → AST wasm_bindgen                        │
│    - Génère :                                                   │
│      • fonction exportée __wbg_add                              │
│      • descriptor statique dans section custom                  │
│      • shim TypeScript                                          │
│    - Encode le `Program` dans `shared::SCHEMA_VERSION`          │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. rustc compile → .wasm (avec sections custom `__wasm_bindgen`)│
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. CLI `wasm-bindgen` (crates/cli + cli-support)                │
│    - walrus::Module::from_file() — parse .wasm                  │
│    - descriptors.rs — extrait les descriptors via interpreter   │
│    - wit/ — convertit en WebAssembly Interface Types            │
│    - js/ — génère le bridge JS/TS (bundler|web|nodejs|deno|…)   │
│    - transforms/ — applique externref, multi-value, threads     │
│    - emit → module.js + module.d.ts + module_bg.wasm            │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. wasm-pack (externe) — wrap en package npm                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Concepts centraux

### 3.1 `JsValue` — handle opaque côté Rust
Toute valeur JS manipulée depuis Rust est un **handle** vers une table côté JS (pas une copie). Cela permet :
- Zéro copie pour objets, arrays, fonctions
- GC géré par JS
- Support `no_std` + `alloc` only

### 3.2 Descriptors encodés dans le `.wasm`
Chaque `#[wasm_bindgen]` fn injecte un **descriptor** (suite d'octets u32) dans une section custom. Le CLI :
1. Lit la section custom
2. Lance un **mini-interpréteur** (`interpreter/`) qui exécute le descriptor
3. Reconstitue la signature exacte (types, optionals, refs, closures…)
4. Génère le bridge JS correspondant

Cette approche évite que le CLI ait à connaître le système de types Rust — c'est Rust qui **émet** sa propre spec.

### 3.3 WebAssembly Interface Types (WIT)
Le dossier `wit/` est le **modèle intermédiaire** :
- `standard.rs` — conforme à la proposition WIT officielle
- `nonstandard.rs` — extensions wasm-bindgen-only (closures, externref, etc.)
- `incoming.rs` / `outgoing.rs` — codegen pour chaque direction

### 3.4 Modes de sortie (`OutputMode`)
```rust
enum OutputMode {
    Bundler { browser_only: bool },  // ESM pour Webpack/Vite/Rollup
    Web,                             // ESM natif <script type="module">
    NoModules { global: String },    // UMD legacy
    Node { module: bool },           // CJS Node.js (Bun-compatible)
    Deno,                            // ESM Deno
    Module,                          // WebAssembly Module sans wrapper
    Emscripten,                      // Compat emscripten
}
```

### 3.5 Le schéma partagé (`crates/shared/`)
Source **canonique** des types échangés macro↔CLI. Le macro et le CLI **doivent avoir exactement la même version** (`SCHEMA_VERSION = "0.2.118"` — `const` enforced). Si mismatch → erreur à la génération.

---

## 4. Les trois crates compagnes

### 4.1 `js-sys` (303 M DL)
Bindings **stables** de tous les globals JS.
- ~43 `#[wasm_bindgen]` top-level dans lib.rs (trompe-l'œil : chaque bloc `extern "C"` contient des dizaines d'items)
- `Array`, `Promise`, `Object`, `Map`, `Set`, `Date`, `RegExp`, `Reflect`, `JSON`, `BigInt`, `Atomics`, `WebAssembly.*`
- **Pas de Web API** — seulement ECMAScript standard

### 4.2 `web-sys` (240 M DL)
**Auto-généré** depuis les WebIDL sources.
- 26 316 lignes WebIDL en entrée → 19 761 `pub fn` en sortie
- Couvre DOM, Canvas, WebGL, WebGPU, Fetch, WebRTC, Web Audio, etc.
- Chaque API est **feature-gated** (`features = ["HtmlCanvasElement", "CanvasRenderingContext2d"]`) — sinon le binaire exploserait
- Le générateur vit dans `crates/webidl/`

### 4.3 `wasm-bindgen-futures` (201 M DL)
Bridge Rust `Future` ↔ JS `Promise` :
- `spawn_local(future)` — exécute un future sans attendre
- `JsFuture::from(promise)` — attend une Promise depuis Rust
- Fondation d'`async fn` côté WASM (Leptos, Dioxus, fetch…)

---

## 5. Le patch Bun local (branche `perf`)

Commits au-delà de `main` :

```
3019280 feat(test-runner): add Bun-aware runtime detection for nodejs target
5a81fba perf: eliminate per-iteration String alloc in write_class inspectable fold
6a589a1 perf: pre-size String buffers in shared name-building functions
```

### 5.1 Détection runtime Bun
`crates/cli/src/wasm_bindgen_test_runner/runtime.rs` (nouveau) :
```rust
enum JsRuntime { Bun { bin: String }, Node { bin: String } }

fn detect_runtime() -> JsRuntime {
    // 1. Honor WASM_BINDGEN_TEST_RUNTIME=bun|node|auto
    // 2. Sinon probe $PATH pour `bun`
    // 3. Fallback node
}
```

Avant : `Command::new("node")` en dur → imposait Node.js pour `cargo test`.
Après : exécute via Bun si dispo (plus rapide, pas de `--expose-gc` car Bun n'expose pas les GC V8 APIs).

Détails : voir `/home/ubuntu/rsbun/wasm/WASM_BINDGEN_BUN_PATCH.md`.

### 5.2 Optimisations perf
- **Pré-sizing de String buffers** dans les fonctions de génération de noms partagées — évite réallocations.
- **Élimination d'allocations per-iteration** dans `write_class` (fold inspectable) — réduit pression GC Rust lors des gros projets (Leptos workspaces).

Ces optimisations sont **upstream-ready** (pure Rust, pas de deps, pas de breaking changes). Candidat évident pour un PR.

---

## 6. Exemples utiles (`examples/`)

| Exemple | Sujet |
|---|---|
| **hello_world** | Minimal bundler — `greet(name)` qui appelle `alert()` |
| **canvas** | Draw primitive sur `<canvas>` depuis Rust |
| **dom** | Manipulation DOM via web-sys |
| **fetch** | `JsFuture` + `fetch()` browser |
| **closures** | Callbacks Rust passés à JS |
| **console_log** | Wrapping `console.log` via `#[wasm_bindgen]` extern |
| **julia_set** | Calcul lourd + rendu canvas |
| **raytrace-parallel** | **Threads WASM** via `atomics` feature |
| **todomvc** | Framework complet (sans JS framework) |
| **wasm-audio-worklet** | Audio worklet Rust |
| **wasm-in-wasm** | Instancier un `WebAssembly.Module` depuis WASM |
| **explicit-resource-management** | `using` / `disposable` JS récent |
| **nodejs-threads** | Support threads en Node/Bun |
| **synchronous-instantiation** | Pattern zero-async-setup |

---

## 7. Relations avec notre écosystème

### 7.1 Ce qui **utilise** wasm-bindgen
Tout ce qui tourne dans un navigateur côté Rust :
- **Leptos** — cible WASM exclusivement via wasm-bindgen
- **Dioxus web** — pareil
- **Yew, Sycamore, Seed** — pareil
- **wgpu** — cible WebGPU via web-sys
- **bevy** (web build) — cible WASM
- **egui** (web backend) — pareil
- **trunk, wasm-pack** — wrappent la CLI `wasm-bindgen`

### 7.2 Lien avec notre stack locale
- `/home/ubuntu/rsbun/wasm/wasm-bindgen/` = ce clone (branche `perf` avec patches locaux)
- `/home/ubuntu/rsbun/wasm/wasm-pack/` = clone de wasm-pack qui **invoque** le CLI wasm-bindgen
- `n2b wasm build` = wrapper maison qui appelle **wasm-pack**, lui-même utilise wasm-bindgen
- `n2b wasm doctor` vérifie la présence de **wasm-bindgen-cli** installé via cargo

### 7.3 Pipeline concret ce VPS
```
Code Rust (#[wasm_bindgen])
   ↓  cargo (rustc + linker wasm32-unknown-unknown)
fichier.wasm (avec sections custom)
   ↓  wasm-pack (wrap)
     ↓  wasm-bindgen (génère .js + .d.ts)
       ↓  wasm-opt (optimise — repo binaryen/ local)
         ↓  wasm-snip (strip panics)
package npm prêt pour Bun/bundler
```

---

## 8. Points clés à retenir

1. **wasm-bindgen est un protocole** autant qu'un outil. Le runtime Rust émet des descriptors, le CLI les lit.
2. Le **SCHEMA_VERSION strict** force la cohérence macro↔CLI — toujours versions identiques.
3. **Zéro magie côté JS** — le bridge généré est lisible, débogable.
4. **`web-sys` est artisanal** (auto-généré depuis WebIDL) — impossible à écrire à la main (20 K fonctions).
5. **`nodejs` target fonctionne verbatim avec Bun** — pas besoin d'un target `bun` séparé (déjà validé par le patch local).
6. **Le patch Bun** ajoute uniquement la détection runtime pour `wasm-bindgen-test-runner` — minimal, upstream-friendly.
7. Le **`interpreter/`** dans cli-support est l'astuce centrale : Rust encode sa propre spec dans du WASM, le CLI l'exécute.
8. Les **proposals WebAssembly** (externref, multi-value, threads, reference-types) sont **déjà supportés** via `transforms/`.

---

## 9. Ressources officielles

- **Repo** : https://github.com/rustwasm/wasm-bindgen
- **Guide** : https://rustwasm.github.io/wasm-bindgen/
- **API `wasm-bindgen`** : https://docs.rs/wasm-bindgen
- **API `js-sys`** : https://rustwasm.github.io/wasm-bindgen/api/js_sys/
- **API `web-sys`** : https://rustwasm.github.io/wasm-bindgen/api/web_sys/
- **Rust+WASM book** : https://rustwasm.github.io/book/
- **MDN** : https://developer.mozilla.org/docs/WebAssembly/Guides/Rust_to_Wasm
- **Attributs** : https://rustwasm.github.io/wasm-bindgen/reference/attributes/index.html
- **Patch Bun local** : /home/ubuntu/rsbun/wasm/WASM_BINDGEN_BUN_PATCH.md
