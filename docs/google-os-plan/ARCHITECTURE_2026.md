# Architecture OS 2026 : Meilleures Pratiques Unix

Ce document définit les standards et les meilleures pratiques pour la conception d'un OS de type Unix en 2026, aligné avec les directives de Google OS.

## 1. Modèle de Noyau (Kernel)

- **Safe-by-default (Rust)** : Le noyau et les modules critiques doivent être développés en Rust pour garantir la sécurité mémoire sans garbage collector. L'initiative *Rust for Linux* est le standard de facto.
- **Hybride / Microkernel** : Isolation forte des pilotes (drivers) dans l'espace utilisateur (userspace) via eBPF ou des modules WebAssembly (Wasm) précompilés. Seul le code critique de l'ordonnanceur et de la gestion de la mémoire (MMU) reste dans l'espace noyau (ring 0).
- **Capability-based Security** : Abandon des privilèges globaux (type `root` monolithique). Chaque processus reçoit des capacités explicites et minimales (via un modèle de jetons/capabilities).

## 2. Gestion des Entrées/Sorties (I/O)

- **Asynchronisme Universel (`io_uring`)** : Abandon des appels système classiques `read/write` bloquants au profit exclusif de files de soumission/complétion asynchrones (`io_uring`).
- **Zero-Copy Architecture** : Utilisation de buffers partagés entre l'espace noyau et l'espace utilisateur (via `memfd` et `io_uring`) pour éviter les recopies inutiles, crucial pour les performances réseau et le stockage NVMe PCIe Gen 5/6.
- **Polling vs Interrupts** : Basculement dynamique du mode interruption au mode *polling* en cas de forte charge réseau ou disque pour minimiser la latence (NAPI).

## 3. Modèle de Processus et d'Exécution

- **Conteneurisation Native (Sandboxing)** : Chaque processus est, par définition, isolé (Namespaces stricts, Cgroups v2+). Il n'y a pas de processus "non conteneurisé".
- **Composants Immuables (Immutable OS)** : Le système de fichiers racine (rootfs) est en lecture seule. Les mises à jour s'effectuent de manière atomique (A/B partitions, modèle de type OSTree / NixOS).
- **IPC (Inter-Process Communication)** : Utilisation de bus de messages locaux ultra-rapides, basés sur des segments de mémoire partagée et des primitives de synchronisation (futex) gérées par le noyau.
- **Exécution WebAssembly (Wasm)** : Support natif en Ring 3 pour l'exécution d'applications portables sécurisées (WASI), avec le runtime Bun/V8 comme exécuteur privilégié.

## 4. Observabilité et Sécurité

- **eBPF (Extended Berkeley Packet Filter)** : Standard absolu pour la sécurité réseau, le profilage (tracing), et l'observabilité. Tout pare-feu ou analyseur de performances s'exécute dans la VM eBPF du noyau.
- **Attestation Matérielle** : Intégration par défaut de l'amorçage sécurisé (Secure Boot) et des modules TPM/Pluton avec signature cryptographique continue (measured boot).

## Synthèse pour Google OS

Google OS adopte cette architecture en mariant le paradigme Windows Terminal Canary (Pilier II) pour un rendu local sans latence, avec un backend d'exécution Rust (Pilier I) qui implémentera progressivement ces primitives asynchrones et sécurisées, remplaçant la couche legacy C/C++.

## Statut réel vs cible (2026-05-17)

| Primitive | Cible Unix 2026 | Statut actuel `google_os` |
|---|---|---|
| Capability-based security | Tokens granulaires | ❌ Privilèges Windows monolithiques (`SeDebugPrivilege`) |
| `io_uring` async universel | IoRing API natif | ⏸ Module présent, branchement IOCP/IoRing en cours (cf. PLAN P6) |
| Zero-copy I/O | `memfd` + buffers partagés | ✅ Via `bun_ffi` mimalloc (Rust ↔ Bun) |
| Sandboxing namespace | cgroups v2+ | ❌ Pas implémenté (Windows = Job Objects) |
| Immutable rootfs | A/B partitions OSTree | ❌ Hors scope user-mode emulator |
| IPC ultra-rapide | futex + SHM | ⏸ Présent dans `google_os::kernel::ipc` (à valider) |
| WebAssembly Ring 3 | WASI runtime | ⏸ Bun/V8 supporte WASM, pas encore exposé via `cli` |
| eBPF observability | VM kernel | ⏸ Module `kernel::ebpf` présent (émulation user-mode) |
| Secure Boot + TPM | Pluton/TPM measured boot | ❌ Hors scope |

**Légende** : ✅ Implémenté · ⏸ En cours / partiel · ❌ Non implémenté

## Axe principal cross-platform (2026)

Depuis 2026-05-17, l'axe principal est un **binaire `cli` cross-platform** Windows/Linux/macOS/wasm/Android. Le crate `google_os` reste **Windows-only** (kernel hybride NT). Voir [`docs/cargo/CROSS_PLATFORM.md`](../cargo/CROSS_PLATFORM.md).
