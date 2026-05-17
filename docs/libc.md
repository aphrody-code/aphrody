# Architecture Libc : Rust, C et Windows MSVC

Ce document détaille l'état de l'art de l'interopérabilité entre le langage C et Rust, les spécifications de la `libc`, et comment le noyau `google_os` utilise ces concepts pour compiler nativement du code source Linux historique (ex: 1998) directement sous Windows.

---

## 1. La Spécification Libc Officielle

La `libc` (C standard library) est le composant central de tout système d'exploitation de type Unix/Linux. Elle définit l'Application Binary Interface (ABI) qui permet aux programmes écrits en C (et C++) de dialoguer avec le noyau de l'OS.

*   **Rôle :** Elle fournit les wrappers pour les appels systèmes (`open`, `read`, `fork`, `mmap`) et les fonctions utilitaires de base (`printf`, `malloc`, `strlen`).
*   **Implémentations historiques :** `glibc` (GNU C Library, le standard Linux), `musl` (légère, statique, utilisée par Alpine Linux), `msvcrt` / `ucrt` (Microsoft C Runtime).
*   **Le défi `google_os` :** Pour compiler du vieux code Linux sur Windows *sans modification*, nous ne pouvons pas utiliser la libc de Microsoft (`ucrt`), car elle ne possède pas de `fork()` ou de sémantique POSIX stricte. Nous devons fournir notre propre couche ABI compatible.

---

## 2. Communication entre C et Rust (L'ABI C)

Rust n'a pas d'ABI stable par défaut. Pour qu'un programme C puisse appeler une fonction écrite en Rust (ou vice-versa), les deux langages doivent s'accorder sur un format binaire universel : **l'ABI C**.

### La norme d'interopérabilité
Pour exposer notre noyau Rust (`google_os`) comme une `libc` valide aux yeux du compilateur C (`gcc` ou `clang`), nous devons utiliser deux directives clés :

1.  **`#[no_mangle]`** : Empêche le compilateur Rust de modifier le nom de la fonction dans le binaire final. La fonction `fork` en Rust restera `fork` dans le `.dll` ou `.so`, permettant au lieur C (linker) de la trouver.
2.  **`extern "C"`** : Force la fonction Rust à utiliser les conventions d'appel du langage C (la façon dont les registres CPU et la pile sont utilisés pour passer les arguments).

```rust
// Exemple dans google_os/src/libc.rs
#[no_mangle]
pub unsafe extern "C" fn fork() -> pid_t {
    // Implémentation NT native
}
```

Grâce à cela, un fichier `main.c` de 1998 contenant `pid = fork();` appellera de manière transparente notre code Rust compilé, sans se rendre compte qu'il tourne sous Windows.

---

## 3. L'Écosystème des Libc en Rust

Dans notre quête pour remplacer l'infrastructure C vieillissante (comme MSYS2 ou Cygwin), l'écosystème Rust propose plusieurs approches de pointe :

### A. relibc (Redox OS)
*   **Concept :** C'est une véritable `libc` complète, écrite à 100% en Rust, qui exporte l'ABI C.
*   **Objectif :** Remplacer totalement `glibc` ou `musl`. C'est le cœur de Redox OS, mais elle tourne aussi sous Linux.
*   **Utilité pour nous :** `google_os` agit spirituellement comme `relibc`. Nous fournissons une implémentation `libc` en Rust, mais notre backend n'est pas le noyau Linux, c'est le noyau Windows NT.

### B. rustix
*   **Concept :** Ce n'est *pas* une libc. C'est une surcouche de sécurité (I/O Safe, Memory Safe) pour les appels systèmes. Sur Linux, `rustix` peut complètement contourner la `libc` (`glibc`) en émettant directement les instructions assembleur `syscall` vers le noyau.
*   **Utilité pour nous :** Bien que brillant pour écrire des applications "Pure Rust", notre but est de faire tourner du *vieux code C*. Nous avons donc l'obligation contractuelle d'exposer une ABI C, ce que `rustix` s'efforce justement d'éviter.

---

## 4. Cross-Compilation MSVC : Le Pipeline Microsoft

L'un des plus grands défis de `google_os` est de compiler des outils POSIX en ciblant l'architecture native de Microsoft : **MSVC** (`x86_64-pc-windows-msvc`).

### Pourquoi MSVC et pas GNU (`-gnu`) ?
Les outils comme MSYS2 utilisent la cible GNU (`x86_64-pc-windows-gnu`) via `mingw-w64`. Cela ajoute une lourde dépendance d'exécution et complique l'interaction avec les API profondes de Windows (comme COM, IOCP, ou DPAPI).
En forçant la cible MSVC, `google_os` produit un binaire Windows natif pur, capable d'être injecté, débogué avec Visual Studio, et de s'interfacer sans friction avec le Ring 0 de NT.

### La solution : `cargo-xwin`
Pour compiler vers la cible MSVC depuis n'importe quel OS (ou sans installer les 15 Go de Visual Studio sur Windows), l'outil de référence est **`cargo-xwin`**.

1.  **Mécanique :** `cargo-xwin` télécharge automatiquement les SDK Windows et le C Runtime (CRT) directement depuis les serveurs de Microsoft, les met en cache localement, et configure le linker LLVM (`lld`) pour les utiliser.
2.  **Exécution :** `cargo xwin build --target x86_64-pc-windows-msvc`

### Alternative : `cargo-zigbuild`
Pour les projets mixtes C/Rust extrêmement complexes, `cargo-zigbuild` utilise le compilateur `Zig` comme lieur, car Zig embarque d'excellentes toolchains de cross-compilation natives pour Windows MSVC.

---

## 5. La Synthèse pour Google OS

Notre architecture accomplit l'exploit suivant :

1.  Nous récupérons du **vieux code source C** (ex: Bash 3.0, Coreutils de 1998).
2.  Nous le compilons avec un compilateur C (Clang/GCC) en lui indiquant de se lier (link) non pas à `glibc` ou MSYS2, mais à notre **`google_os.dll`**.
3.  Le compilateur C voit notre module `libc.rs` (grâce au `#[no_mangle] extern "C"`) et génère les appels.
4.  À l'exécution, lorsque le code C appelle `mmap()`, notre fonction Rust intercepte l'appel et invoque les API MSVC de bas niveau (`CreateFileMappingW` via `windows-rs`) pour exécuter l'action sur le noyau Windows.

Nous obtenons ainsi un écosystème hybride parfait : la compatibilité totale du code historique Linux, avec les performances, la sécurité mémoire (Rust) et l'intégration profonde de Windows 11.
