<!-- SPDX-License-Identifier: Apache-2.0 -->
# Supply-chain — Google-grade 2026

> Réf. : `supply-chain/config.toml`, `supply-chain/audits.toml`, `deny.toml`, `Cargo.lock`.
> Migration : `cargo vendor` (legacy) → `Cargo.lock + cargo-vet + cargo-deny` (Phase 1, 2026-05-16).

## Pourquoi pas `cargo vendor` ?

L'article de référence **« Lockfiles Killed Vendoring »** (Andrew Nesbitt, fév. 2026) :

> *Once lockfiles recorded exact versions and integrity hashes, you got reproducible builds without storing the code. The lockfile records what to fetch, the registry serves it, and the hash proves nothing changed in transit.*

`Cargo.lock` contient les **SHA-256 d'intégrité** depuis Cargo 1.41 → la reproductibilité n'a plus besoin du code source physique dans le repo.

| Critère | Vendor (legacy) | Cargo.lock + cargo-vet (actuel) |
|---|---|---|
| Hermeticité au build | 100 % | 99 % (1 fetch initial via sparse registry) |
| Taille du repo git | **+1.2 Go (mesuré)** | 0 Mo |
| Clone CI | Lent | Rapide |
| Audit visibilité | Code lisible | Via `cargo-vet` (signé cryptographiquement) |
| Maintenance | **Lourde** (churn à chaque `cargo update`) | Triviale |
| Risque CVE oubliée | Élevé | Faible (audit automatisable) |

**Pratiques industrielles :**
- **Google Fuchsia** : `cargo vendor` + `cargo-vet`, mais build avec GN/Ninja (pas cargo). Le vendor sert l'audit, pas le build.
- **Facebook** : `cargo` pour download, build avec Buck.
- **Kubernetes** : vendor (mais ils ont une équipe dédiée).
- **Tout le reste** (et nous) : `Cargo.lock` + `cargo-vet`.

## Stack actuel

```
Cargo.lock (SHA-256 pinning)            ← source de vérité reproductibilité
├── Sparse registry (déjà actif)         ← 10-100× plus rapide que git protocol
├── sccache (déjà actif)                 ← cache de compilation partagé
├── cargo-vet (supply-chain/)            ← audits signés cryptographiquement
│   └── imports.lock                     ← pins des feeds (Google, Mozilla, Fuchsia...)
├── cargo-deny (deny.toml)               ← CVE + licences + bans + sources
└── CI : cargo --locked --offline        ← build hermétique
```

## `cargo-vet` — Audits signés

### Configuration (`supply-chain/config.toml`)

7 feeds d'audits trustés :

```toml
[imports.google]
url = "https://raw.githubusercontent.com/google/rust-crate-audits/main/audits.toml"

[imports.mozilla]
url = "https://raw.githubusercontent.com/mozilla/supply-chain/main/audits.toml"

[imports.fuchsia]
url = "https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/third_party/rust_crates/supply-chain/audits.toml?format=TEXT"

[imports.chromeos]
url = "https://chromium.googlesource.com/chromiumos/third_party/rust_crates/+/refs/heads/main/audits.toml?format=TEXT"

[imports.bytecode-alliance]
url = "https://raw.githubusercontent.com/bytecodealliance/wasmtime/main/supply-chain/audits.toml"

[imports.embark-studios]
url = "https://raw.githubusercontent.com/EmbarkStudios/rust-ecosystem/main/audits.toml"

[imports.zcash]
url = "https://raw.githubusercontent.com/zcash/rust-ecosystem/main/supply-chain/audits.toml"
```

### Workflow `cargo vet`

```bash
# Vérifier que toutes les deps sont auditées (ou exemptées)
cargo vet

# Suggérer des audits manquants à faire localement
cargo vet suggest

# Marquer une crate comme auditée localement (safe-to-deploy)
cargo vet certify <crate> <version> --criteria safe-to-deploy

# Importer les audits frais des feeds (refresh imports.lock)
cargo vet fetch-imports

# Voir le diff entre deux versions auditées
cargo vet diff <crate> <old-version> <new-version>
```

### Criteria standards

- `safe-to-run` — la crate ne peut pas casser le build / introduire une supply chain attack ; mais peut avoir des bugs subtils
- `safe-to-deploy` — auditée pour production ; pas de comportement malicieux suspecté
- Custom criteria définis dans `supply-chain/audits.toml` si besoin (ex: `does-not-implement-crypto`, `crypto-safe`)

## `cargo-deny` — CVE / licences / bans / sources

### Policy actuelle (`deny.toml`)

**4 axes vérifiés :**

```bash
cargo deny check                 # tous les axes
cargo deny check advisories      # CVE + yanked + unmaintained
cargo deny check licenses        # licences whitelistées
cargo deny check bans            # version dedup + denied crates
cargo deny check sources         # registry / git origin
```

### Advisories (CVE)
- Base : `https://github.com/rustsec/advisory-db`
- `yanked = "deny"` — pas de version yankée
- `version = 2` — schema moderne
- Ignores justifiés : 12 (cf. `deny.toml § [advisories.ignore]`) couvrant GTK3, pyo3 0.21, aws-lc CVE, Marvin Attack, etc.

### Licences whitelist
```toml
allow = [
    "MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause", "BSD-3-Clause", "BSL-1.0", "CC0-1.0",
    "ISC", "MPL-2.0", "Unicode-3.0", "Unicode-DFS-2016",
    "Zlib", "OpenSSL", "0BSD",
    "CDLA-Permissive-2.0",  # webpki-roots Mozilla
]

# Private/in-repo crates skipped (have publish = false)
private = { ignore = true }
```

### Bans
```toml
multiple-versions    = "warn"   # workspace has legitimate dupes
wildcards            = "deny"   # version = "*" forbidden
allow-wildcard-paths = true     # ...but allowed for path deps

deny = [
    { crate = "git2", reason = "Pulls libgit2 + OpenSSL; use gix instead." },
]
```

### Sources (origin)
```toml
unknown-registry = "deny"
unknown-git      = "deny"
allow-registry   = ["https://github.com/rust-lang/crates.io-index"]
allow-git        = ["https://github.com/modelcontextprotocol/rust-sdk.git"]
```

## Workflow : ajouter une nouvelle dep

1. **Ajouter** dans `[workspace.dependencies]` du root :
   ```toml
   nouvelle-dep = { version = "1.5", default-features = false, features = [...] }
   ```
2. **Inheriter** dans la crate consommatrice :
   ```toml
   [dependencies]
   nouvelle-dep = { workspace = true }
   ```
3. **Update lockfile** :
   ```bash
   cargo update -p nouvelle-dep
   ```
4. **Audit obligatoire** :
   ```bash
   cargo deny check       # bloque sur CVE / licence / bans
   cargo vet              # signale si pas d'audit Google/Mozilla/etc.
   ```
5. **Si CVE ignoré nécessaire** : ajouter dans `deny.toml § [advisories.ignore]` avec justification + horizon.
6. **Si pas d'audit upstream** : `cargo vet suggest` puis `cargo vet certify` localement.

## Workflow : importer un nouveau feed d'audits

Ajouter dans `supply-chain/config.toml` :

```toml
[imports.nouvel-org]
url = "https://raw.githubusercontent.com/nouvel-org/audits/main/audits.toml"
```

Puis :
```bash
cargo vet fetch-imports     # met à jour supply-chain/imports.lock
cargo vet                   # re-valide avec les nouveaux audits
```

## CI hermétique

Alias `cargo ci-offline` dans `.cargo/config.toml` :
```bash
cargo clippy --workspace --all-targets --all-features \
             --locked --offline -- -D warnings
```

- `--locked` : fail si `Cargo.lock` doit être modifié → drift détecté
- `--offline` : aucune connexion réseau autorisée → garantit que toutes les deps sont cachées localement (`~/.cargo/registry/cache/`)
- `-D warnings` : zéro warning toléré

## Validation finale (avant tout commit)

```bash
cargo ci-offline                 # 0 warning
cargo deny check                 # advisories ok | bans ok | licenses ok | sources ok
cargo vet                        # audits ok
cargo audit-machete              # zéro unused dep
```

## Roadmap

- **P7** : convertir les `[[exemptions.*]]` auto-générées par `cargo vet init` en audits signés réels.
- **P8** : publier nos audits sur un repo `aphrody/rust-crate-audits` pour les partager.
- **P9** : ajouter `cargo-supply-chain` pour la transparence des contributeurs upstream.

## Références

- [Lockfiles Killed Vendoring — Andrew Nesbitt, fév. 2026](https://nesbitt.io/2026/02/10/lockfiles-killed-vendoring.html)
- [cargo-vet docs](https://mozilla.github.io/cargo-vet/)
- [cargo-deny docs](https://embarkstudios.github.io/cargo-deny/)
- [Google Rust crate audits](https://github.com/google/rust-crate-audits)
- [Fuchsia supply-chain](https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/third_party/rust_crates/supply-chain/)
- [Mozilla supply-chain](https://github.com/mozilla/supply-chain)
- [RustSec advisory DB](https://github.com/rustsec/advisory-db)
