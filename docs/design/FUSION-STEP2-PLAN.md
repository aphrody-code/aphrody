# Fusion UI — Étape 2 : plan d'implémentation concret

Étape 1 (faite) : `crates/m3-tokens/src/color.rs` expose, sous la feature `std`
(activée par défaut, cf. `crates/m3-tokens/Cargo.toml:21`), les fonctions
publiques suivantes — toutes vérifiées :

- `m3_tokens::color::export_css(theme: &ColorRoles) -> String` (`color.rs:309`)
- `m3_tokens::color::export_shadcn_aliases() -> String` (`color.rs:427`)
- `m3_tokens::color::export_tailwind_theme() -> String` (`color.rs:453`)
- `m3_tokens::color::export_fusion_css(theme: &ColorRoles) -> String` (`color.rs:481`)

Constantes de thème disponibles : `m3_tokens::color::BASELINE` (`color.rs:106`)
et `m3_tokens::color::BASELINE_DARK` (`color.rs:157`). Exemple runnable :
`crates/m3-tokens/examples/fusion.rs` (`cargo run -p m3-tokens --example fusion`).
Le crate `aphrody-design` réexporte déjà `m3_tokens` sous l'alias `tokens`
(`crates/aphrody-design/src/lib.rs:20` : `pub use m3_tokens as tokens;`).

Cette étape 2 couvre les deux pistes de diffusion de `export_fusion_css` : la
CLI `aphrody` (piste A) et le câblage shadcn via un item `registry:theme`
(piste B).

---

## Piste A — Exposer `export_fusion_css` dans la CLI `aphrody`

### Constat vérifié

- Le binaire `aphrody` est le crate `crates/cli` (package `aphrody`,
  cf. `crates/cli/Cargo.toml:2`). Sous-commandes définies via clap *derive* :
  `#[derive(Subcommand)] enum Commands` dans `crates/cli/src/main.rs:100-425`.
- Dispatch dans `async fn dispatch(...)` (`crates/cli/src/main.rs:735-1016`),
  un bras `match` par variante, déléguant à un type implémentant le trait
  `TerminalCommand` (`crates/cli/src/context.rs`) ou à un module `*_cmd`.
- **Aucune sous-commande `design` n'existe** (grep `Design` sur
  `main.rs` : aucun résultat). Le crate `crates/aphrody-design` existe mais
  expose un daemon + sidecar (`bin/daemon.rs`, `bin/sidecar.rs`), pas une
  surface CLI `aphrody design`. Il n'est pas dépendance de `crates/cli`.
- **`crates/cli/Cargo.toml` ne dépend PAS de `m3-tokens`** (vérifié :
  aucune ligne `m3-tokens` ni `aphrody-design` dans la section
  `[dependencies]` / `[target...]`). Il faut donc ajouter la dépendance.
- Modèle de feature opt-in déjà en place pour les commandes host-only
  (`images`, `firefly`, `index`, `forensics` — `crates/cli/Cargo.toml:20-49`,
  modules gated `crates/cli/src/main.rs:16-19`). `m3-tokens` est pur Rust,
  `std`-only, sans I/O réseau ni C : il compile sur wasm aussi, donc la
  commande peut être **inconditionnelle** (pas besoin de feature gate), à
  ceci près que l'écriture fichier `-o` utilise `std::fs` (non-wasm). On
  câble donc la variante sous `#[cfg(not(target_arch = "wasm32"))]` comme les
  commandes `Mcp` / `Re` voisines, par cohérence et pour éviter
  `std::fs::write` sur wasm.

### Fichiers à toucher

1. **`crates/cli/Cargo.toml`** — ajouter la dépendance path.
   Dans `[dependencies]` (section non gated, en-tête `crates/cli/Cargo.toml:58`),
   après `chrono` (`:69`) :

   ```toml
   # `aphrody design tokens` — Material 3 design tokens + fusion stylesheet
   # (M3 system colors + shadcn aliases + Tailwind @theme inline). Pur Rust,
   # std-only, pas d'I/O réseau ni de C → compile sur toutes les cibles.
   m3-tokens     = { path = "../m3-tokens" }
   ```

   Pas de version : le workspace est path-only (cf. les autres deps `{ path =
   "../..." }`). La feature `std` est active par défaut (`m3-tokens` n'a pas
   besoin d'être listée dans `default-features`). `cargo-machete` : si la dep
   est détectée inutilisée par erreur (faux positif possible avant câblage),
   l'ajouter à `[package.metadata.cargo-machete] ignored` (`crates/cli/Cargo.toml:183`).

2. **`crates/cli/src/main.rs`** — déclarer le module + la variante + le dispatch.

   a. Module de commande (après `crates/cli/src/main.rs:26`) :

   ```rust
   #[cfg(not(target_arch = "wasm32"))] mod design_cmd;
   ```

   b. Variante d'enum `Commands` — insérer avant la variante terminale
      `Auto(Vec<String>)` (`crates/cli/src/main.rs:419-424`) :

   ```rust
   /// Material 3 design tokens — CSS custom properties + fusion stylesheet.
   ///
   /// Génère le pont M3 ↔ shadcn ↔ Tailwind v4 calculé par `m3-tokens`.
   /// `aphrody design tokens` émet par défaut les `--md-sys-color-*` ; avec
   /// `--fusion`, ajoute le bloc d'alias shadcn et le bloc Tailwind
   /// `@theme inline`. Cf. docs/design/FUSION-PLAN.md.
   #[cfg(not(target_arch = "wasm32"))]
   Design {
       #[command(subcommand)]
       action: DesignAction,
   },
   ```

   c. Sous-enum `DesignAction` (à placer près de `McpAction` / `ReAction`,
      ex. après `crates/cli/src/main.rs:716`) :

   ```rust
   /// Actions for the `design` subcommand (Material 3 design tokens).
   #[cfg(not(target_arch = "wasm32"))]
   #[derive(clap::Subcommand, Debug, Clone)]
   pub(crate) enum DesignAction {
       /// Emit Material 3 design tokens as CSS custom properties.
       Tokens {
           /// Emit the full fusion sheet (M3 + shadcn aliases + Tailwind
           /// `@theme inline`) instead of only the `--md-sys-color-*` block.
           #[arg(long)]
           fusion: bool,
           /// Use the dark baseline (`BASELINE_DARK`) instead of light.
           #[arg(long)]
           dark: bool,
           /// Write to this file instead of stdout.
           #[arg(long, short)]
           output: Option<PathBuf>,
       },
   }
   ```

   d. Bras de dispatch — dans `match cli.command`, à côté du bras `Re`
      (`crates/cli/src/main.rs:784`) :

   ```rust
   Some(Commands::Design { action }) => match action {
       DesignAction::Tokens { fusion, dark, output } => {
           design_cmd::run_tokens(fusion, dark, output)?;
       },
   },
   ```

   e. (wasm) La variante étant `#[cfg(not(target_arch = "wasm32"))]`, le `match`
      du `fn main()` wasm (`crates/cli/src/main.rs:1062-1093`) reste inchangé
      (il n'énumère que les variantes non gated). Aucun ajout requis côté wasm.

3. **`crates/cli/src/design_cmd.rs`** — nouveau fichier, handler réel.
   Logique 100 % production (zéro stub) :

   ```rust
   // SPDX-License-Identifier: Apache-2.0
   //! `aphrody design tokens` — émet les design tokens Material 3 calculés par
   //! le crate `m3-tokens` (M3 system colors + fusion shadcn/Tailwind).

   use std::path::PathBuf;

   use m3_tokens::color::{BASELINE, BASELINE_DARK, export_css, export_fusion_css};

   /// Génère la feuille de tokens et l'écrit sur stdout ou dans `output`.
   pub(crate) fn run_tokens(
       fusion: bool,
       dark: bool,
       output: Option<PathBuf>,
   ) -> miette::Result<()> {
       let theme = if dark { &BASELINE_DARK } else { &BASELINE };
       let css = if fusion { export_fusion_css(theme) } else { export_css(theme) };

       match output {
           Some(path) => {
               std::fs::write(&path, css.as_bytes())
                   .map_err(|e| miette::miette!("write {}: {e}", path.display()))?;
               eprintln!("wrote {} ({} bytes)", path.display(), css.len());
           },
           None => {
               // print! (pas println!) : pas de newline parasite, sortie
               // identique à l'exemple `m3-tokens --example fusion`.
               print!("{css}");
           },
       }
       Ok(())
   }
   ```

   Note : le handler est synchrone (`fn`, pas `async`) — il n'y a aucune
   I/O réseau. Le bras de dispatch l'appelle sans `.await` (cf. d.). C'est
   cohérent avec d'autres bras synchrones (ex. `Commands::Logo`,
   `crates/cli/src/main.rs:963`).

### Option `--seed <hex>` (palette dynamique) — borne réelle

L'énoncé évoque `--seed <hex>`. Constat vérifié : `m3-tokens` expose
`dynamic::seed_to_palette(seed_argb: u32) -> [Argb; 13]`
(`crates/m3-tokens/src/dynamic.rs:596`), mais **aucune fonction publique ne
convertit un seed en `ColorRoles` complet** (grep `-> ColorRoles` : seules
`export_css`/`export_fusion_css` prennent `&ColorRoles`, et seules `BASELINE`
/ `BASELINE_DARK` produisent un `ColorRoles`). Conséquence :

- **MVP livrable immédiatement** : n'exposer que `--fusion` / `--dark` /
  `--output` (seed = baseline M3 Purple `#6750A4`, déjà la valeur de `BASELINE`).
- **`--seed <hex>`** nécessite d'abord d'ajouter dans `m3-tokens` un
  constructeur `pub fn roles_from_seed(seed_argb: u32) -> ColorRoles` (light)
  + variante dark, qui assemble un `ColorRoles` depuis `seed_to_palette` — ce
  qui est un *prérequis dans `m3-tokens`*, pas dans la CLI. Le câbler dans
  `DesignAction::Tokens` ensuite via un champ `#[arg(long)] seed: Option<String>`
  parsé en `u32` (`u32::from_str_radix(s.trim_start_matches(['#','x']), 16)`).
  À traiter en sous-tâche séparée pour ne pas bloquer le MVP.

### Commandes de vérification (piste A)

```bash
# Compile (cible #1 Linux, puis Windows)
cargo check -p aphrody --target x86_64-unknown-linux-gnu --locked
cargo check -p aphrody --target x86_64-pc-windows-msvc --locked
# wasm : la variante est cfg-stripped, doit toujours compiler
cargo check -p aphrody --target wasm32-unknown-unknown --locked

# Comportement réel (verify strictly — cf. CLAUDE.md §7)
cargo run -p aphrody -- design tokens | head -3
#  attendu : ":root {" puis "--md-sys-color-primary: #6750A4;"
cargo run -p aphrody -- design tokens --fusion | grep -- '--primary: var(--md-sys-color-primary);'
cargo run -p aphrody -- design tokens --dark | grep -- '--md-sys-color-primary:'
cargo run -p aphrody -- design tokens --fusion -o /tmp/tokens.css && wc -c /tmp/tokens.css

# Parité avec la sortie de référence
cargo run -p aphrody -- design tokens --fusion > /tmp/cli.css
cargo run -p m3-tokens --example fusion > /tmp/ex.css
diff /tmp/cli.css /tmp/ex.css   # attendu : identiques

# Lints + tests
cargo ci-offline
cargo xt-offline
```

Test d'intégration recommandé (mêmes outils que `crates/cli/tests/doctor.rs`,
qui utilise déjà `assert_cmd` + `predicates`, cf. dev-deps
`crates/cli/Cargo.toml:164-169`) : ajouter `crates/cli/tests/design.rs` qui
spawn le binaire et asserte que la sortie `design tokens --fusion` contient
`--md-sys-color-primary:`, `--primary: var(--md-sys-color-primary);` et
`@theme inline {`.

---

## Piste B — Câbler la feuille générée dans shadcn via `registry:theme`

### Constat vérifié (structure du registry shadcn)

Le fork shadcn vit dans `packages/ui` ; l'app v4 (la démo + la source du
registry) est sous `packages/ui/apps/v4`.

- **Registry racine** : `packages/ui/apps/v4/registry.json` — objet
  `{ name, homepage, items: [...] }` (`registry.json:1-4`). Les items
  `registry:theme` existent déjà (`theme-stone` à `registry.json:2337`,
  + 4 autres ; 5 occurrences `"type": "registry:theme"` lignes 2339/2412/
  2485/2558/2631). Forme d'un item theme : `{ name, type: "registry:theme",
  cssVars: { light: {...}, dark: {...} } }`.
- **Schéma d'item** : `packages/ui/apps/v4/public/schema/registry-item.json`.
  Le type `registry:theme` est dans l'enum (`registry-item.json:17`). Champs
  pertinents : `cssVars.theme` (variables `@theme`, Tailwind v4 only,
  `:142`), `cssVars.light` (`:149`), `cssVars.dark` (`:156`), et `css` (bloc
  CSS brut arbitraire — at-rules, sélecteurs, layers — `:165-171`,
  récursif via `definitions/cssValue` `:301`).
- **Builds par style** : items materialisés sous
  `packages/ui/apps/v4/public/r/styles/<style>/theme-*.json`
  (ex. `new-york-v4/theme-slate.json` lu intégralement — voir sa forme
  `cssVars.light` / `cssVars.dark` en OKLCH).
- **`components.json`** : `packages/ui/apps/v4/components.json` (config du
  projet : aliases, tailwind css path).
- **`:root` OKLCH actuel** : défini dans `packages/ui/apps/v4/app/globals.css`.
  - `@theme inline { ... }` (`globals.css:25-78`) : mappe les `--color-*`
    Tailwind vers les variables sémantiques shadcn (`--color-primary:
    var(--primary);` `:44`). **C'est exactement le rôle de
    `export_tailwind_theme()`**, mais ici l'indirection passe par `--primary`
    (pas directement par `--md-sys-color-*`).
  - `:root { ... }` (`globals.css:80-122`) : valeurs OKLCH hardcodées
    (`--primary: oklch(0.205 0 0);` `:88`, `--background: oklch(1 0 0);` `:82`…).
  - `.dark { ... }` (`globals.css:124+`) : overrides dark, mêmes clés.

### Comment un `registry:theme` remplace le `:root` OKLCH

Le CLI shadcn (`npx shadcn add <theme>`) **fusionne** les `cssVars.light` /
`cssVars.dark` de l'item dans le `:root` / `.dark` du fichier CSS du projet
(merge clé-à-clé). Un item theme ne *supprime* pas le bloc existant : il
écrase les clés qu'il fournit. Pour la fusion M3, deux stratégies :

- **B1 — `cssVars` valorisés (recommandé pour la diffusion shadcn standard).**
  Émettre les couleurs M3 *résolues* (valeurs hex/oklch concrètes) dans
  `cssVars.light` et `cssVars.dark`, sous les noms sémantiques shadcn. C'est
  le format que le CLI shadcn sait fusionner nativement et que les autres
  `theme-*` utilisent (cf. `theme-slate.json`). Le mapping sémantique
  M3 → shadcn est **déjà encodé** dans `m3-tokens` :
  `FUSION_ALIAS_MAP` (`crates/m3-tokens/src/color.rs:388-408`) — p. ex.
  `background → surface`, `primary → primary`, `border → outline-variant`,
  `destructive → error`. Il faut un petit générateur Rust qui, pour chaque
  paire `(nom_shadcn, role_m3)` de `FUSION_ALIAS_MAP`, lit le champ
  correspondant de `BASELINE` (light) / `BASELINE_DARK` (dark) et l'émet en
  `#RRGGBB`. Cela donne un JSON `registry:theme` autonome (aucune dépendance
  à un `tokens.css` externe).

- **B2 — `css` avec import + alias `var()` (fidèle au FUSION-PLAN).**
  Le FUSION-PLAN décrit l'injection de `tokens.css` (les `--md-sys-color-*`)
  + un bloc d'alias `--primary: var(--md-sys-color-primary);`. C'est ce que
  produit `export_fusion_css()` / `export_shadcn_aliases()`. Un
  `registry:theme` peut porter cela via le champ `css` (CSS brut) plutôt que
  `cssVars`, p. ex. en `css: { ":root": { ... }, ".dark": { ... } }` ou en
  `@layer base`. Avantage : light/dark M3 traversent automatiquement (les
  alias sont des `var()` indépendants du thème, cf. doc de
  `export_shadcn_aliases` `crates/m3-tokens/src/color.rs:410-415`).
  Inconvénient : le bloc `css` brut est moins bien fusionné par le CLI
  shadcn que `cssVars` (il est ajouté, pas mergé clé-à-clé sur `:root`).

Reco intra-piste : **B1 pour publier un `theme-aphrody` propre et installable
via le CLI shadcn**, car il s'aligne sur la mécanique de merge existante et
sur le format des `theme-*.json` du fork. B2 reste l'option si l'on veut
garder la couche `--md-sys-color-*` visible runtime (utile pour les web
components MD3 qui héritent de `--md-sys-color-*`).

### Fichiers à toucher (variante B1)

1. **`packages/ui/apps/v4/registry.json`** — ajouter un item dans `items`
   (à côté des autres `theme-*`, vers `registry.json:2337`) :

   ```json
   {
     "name": "theme-aphrody",
     "type": "registry:theme",
     "title": "Aphrody (Material 3)",
     "author": "aphrody",
     "cssVars": {
       "light": {
         "background": "#FFFBFF",
         "foreground": "#1C1B1F",
         "primary": "#6750A4",
         "primary-foreground": "#FFFFFF",
         "secondary": "#E8DEF8",
         "secondary-foreground": "#1D192B",
         "muted": "#E7E0EC",
         "muted-foreground": "#49454F",
         "accent": "#FFD8E4",
         "accent-foreground": "#31111D",
         "destructive": "#B3261E",
         "border": "#CAC4D0",
         "input": "#CAC4D0",
         "ring": "#6750A4"
       },
       "dark": {
         "background": "#1C1B1F",
         "foreground": "#E6E1E5",
         "primary": "#D0BCFF",
         "primary-foreground": "#381E72"
       }
     }
   }
   ```

   (Les valeurs ci-dessus sont illustratives du mapping `FUSION_ALIAS_MAP` ; la
   liste complète des clés = les 19 entrées de `FUSION_ALIAS_MAP`, valorisées
   depuis `BASELINE` / `BASELINE_DARK`.)

2. **Générateur — réutiliser la CLI de la piste A.** Ne PAS écrire à la main
   les hex : ajouter dans `m3-tokens` une fonction
   `export_shadcn_theme_json(theme: &ColorRoles) -> String` (ou un sous-objet
   `cssVars` sérialisable) qui parcourt `FUSION_ALIAS_MAP` et émet le mapping
   valorisé, puis exposer un drapeau CLI
   `aphrody design tokens --shadcn-registry [--dark]` qui produit le bloc
   `cssVars`. La génération du fichier registry devient alors reproductible
   (cargo en amont du pipeline JS, conforme au FUSION-PLAN §52-54). MVP
   acceptable : générer les deux blocs (light/dark) et les coller dans
   l'item `registry.json`.

3. **(B2, alternatif) `packages/ui/apps/v4/app/globals.css`** — si l'on
   choisit B2 : importer la feuille fusion en amont des blocs `:root` (après
   `globals.css:12`) via `@import "./fusion-tokens.css" layer(base);`, où
   `fusion-tokens.css` est la sortie de
   `aphrody design tokens --fusion -o packages/ui/apps/v4/app/fusion-tokens.css`.
   Les alias `--primary: var(--md-sys-color-primary);` d'`export_shadcn_aliases`
   doivent alors être placés *après* les `:root` OKLCH existants pour les
   écraser (ordre de cascade), ou ces derniers retirés. C'est la voie « tokens,
   pas valeurs » du FUSION-PLAN, mais elle modifie un fichier source partagé du
   fork → préférer B1 pour rester additif.

### Comment le CSS est référencé

- En B1 : l'item `theme-aphrody` est auto-suffisant (valeurs dans `cssVars`).
  Installation : `npx shadcn@latest add theme-aphrody` (résolu via
  `registry.json` une fois servi), qui merge dans le `:root`/`.dark` du projet
  cible. Aucun import CSS additionnel.
- En B2 : référence via `@import` dans `globals.css`, le `registry:theme`
  portant le champ `css`/`files` pointant la feuille `tokens.css` générée.

### Commandes de vérification (piste B)

```bash
# 1. Validité JSON du registry modifié
jq -e '.items[] | select(.name=="theme-aphrody") | .type=="registry:theme"' \
   packages/ui/apps/v4/registry.json

# 2. Conformité au schéma registry-item (draft-07)
#    valider l'item extrait contre public/schema/registry-item.json
jq '.items[] | select(.name=="theme-aphrody")' \
   packages/ui/apps/v4/registry.json > /tmp/item.json
# (vérif schéma via un validateur JSON-Schema draft-07)

# 3. Présence des clés sémantiques shadcn attendues (mapping FUSION_ALIAS_MAP)
jq -e '.items[] | select(.name=="theme-aphrody") | .cssVars.light.primary' \
   packages/ui/apps/v4/registry.json

# 4. Si générateur Rust câblé (piste A étendue) : reproductibilité
cargo run -p aphrody -- design tokens --shadcn-registry > /tmp/gen.json
#  comparer /tmp/gen.json au bloc cssVars de l'item
```

Note plateforme : la piste B touche `packages/ui` (TypeScript/JSON). Aucun
runtime JS n'est invoqué côté aphrody (policy §2 : JS/TS bannis du code
aphrody). La *génération* du JSON reste pure Rust via la CLI de la piste A ;
le `npx shadcn add` est exécuté côté consommateur du registry, hors du build
hermétique aphrody.

---

## Recommandation : ordre d'exécution

**Faire la piste A en premier.** Justifications :

1. **Auto-suffisante et testable headless** : la piste A est 100 % Rust dans
   le workspace, vérifiable par `cargo run` + `assert_cmd` (cf. CLAUDE.md §0.1
   evals headless), sans toolchain JS. Cible #1 Linux respectée d'emblée.
2. **Elle débloque la piste B** : la voie propre de B1/B2 consiste à *générer*
   le bloc `cssVars` / la feuille `tokens.css` depuis la CLI plutôt que de
   coller des hex à la main. La sous-commande `aphrody design tokens` est donc
   le générateur reproductible qt le FUSION-PLAN (§52-54) place en amont du
   pipeline UI. Écrire B avant A reviendrait à figer des valeurs non
   régénérables.
3. **Coût marginal faible** : une dépendance path, une variante d'enum, un
   sous-enum, un bras de dispatch et un fichier handler (~40 lignes), sans
   surface réseau ni feature gate complexe — le risque de régression CI est
   minimal.

Piste B ensuite, en variante **B1** (item `theme-aphrody` autonome dans
`registry.json`, valorisé par la sortie de `aphrody design tokens`), B2 réservé
si l'on veut conserver la couche `--md-sys-color-*` runtime pour les web
components MD3.

Sous-tâche transverse à planifier (prérequis `--seed`) : ajouter dans
`m3-tokens` un `roles_from_seed(u32) -> ColorRoles` (light + dark) au-dessus de
`dynamic::seed_to_palette` (`crates/m3-tokens/src/dynamic.rs:596`), qui ouvre à
la fois `aphrody design tokens --seed <hex>` (A) et un `theme-aphrody`
paramétrable par marque (B).
