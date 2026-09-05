# Gestionnaire des CLI Aphrody

`aphrody package` gère les outils officiels avec une interface identique sur
Linux, macOS et Windows. Le catalogue embarqué connaît le dépôt, le moteur de
construction, le paquet Cargo, les scripts Bun, les binaires produits et les
variantes d'architecture de chaque outil.

```bash
aphrody package catalog
aphrody package doctor
aphrody package status --json
aphrody package install n2b
aphrody package update all
aphrody package uninstall bxc --dry-run
aphrody package uninstall bxc --purge --yes
```

Les sources gérées sont isolées dans le profil utilisateur sous
`.local/share/aphrody/sources` sur Unix et `LocalAppData/Aphrody/sources` sur
Windows. Les binaires sont installés dans `.local/bin` sur Unix et dans le
dossier `bin` Aphrody sur Windows. La désinstallation ne supprime jamais les
sources sans `--purge`; toute mutation exige `--yes`, sauf simulation.

## Catalogue officiel

| Identifiant | Dépôt | Moteur | Produit principal |
|---|---|---|---|
| `aphrody` | `aphrody-code/aphrody` | Cargo | `aphrody` |
| `bxc` | `aphrody-code/bxc` | Bun + pont Rust | `bxc`, `bxc-mcp` |
| `n2b` | `aphrody-code/n2b` | Cargo | `n2b` |
| `nie` | `aphrody-code/nie` | Cargo | `niers` |

## Avantages mis en commun

- Aphrody fournit l'orchestration multiplateforme, l'installation et les diagnostics.
- BXC apporte la distribution de binaires Bun autonomes multi-cibles et son pont Rust.
- N2B apporte les contrats schema-first, les registres déclaratifs et les migrations transactionnelles.
- Niers apporte le workspace multi-binaires avec une façade unique et le diagnostic des backends.

La prochaine évolution du catalogue lira le manifeste
`package.metadata.aphrody-package` publié par chaque paquet. Le catalogue
embarqué reste le repli signé et empêche qu'un dépôt distant choisisse seul une
commande arbitraire à exécuter.
