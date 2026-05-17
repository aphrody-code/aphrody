# Migration C++ → Rust

> Réf. : `CLAUDE.md`, `GEMINI.md`, `docs/PLAN.md`.
> Stratégie : incrémentale, par sous-système, sans régression fonctionnelle.

## Principes

1. **Tout nouveau code en Rust** (ou C ISO C11 si nécessaire). Pas de nouveau C++.
2. **Le C++ existant est retiré progressivement** par sous-système, jamais en bloc.
3. **Toujours par paire** : (a) nouveau code Rust qui implémente la fonctionnalité, (b) suppression du code C++ équivalent.
4. **Test parité obligatoire** : avant de retirer le C++, prouver que le Rust produit le même output sur les mêmes inputs (snapshot tests).

## État actuel (2026-05-16)

### Migré
- Forensics Chromium : `src/CryptoHelper.cpp`, `src/ChromiumParser.cpp` → `crates/backend/src/chromium.rs`
- DPAPI wrappers : `crates/base/src/lib.rs` (AES-GCM via aes-gcm crate, DPAPI via windows-rs)
- Process management : `crates/google_os/src/kernel/process.rs` (NtOpenProcess, TerminateProcess via windows-rs)
- Network DNS : `crates/backend/src/dns.rs`

### En cours
- `injector/injector.cpp` → `crates/base/src/injector.rs` (déjà entamé)
- ProcessManager kernel-side → `crates/google_os/src/kernel/process.rs`
- io_uring émulation → `crates/google_os/src/kernel/io_uring.rs` (à brancher sur Win11 IoRing API)

### À faire
- DxEngine renderer (C++) → Rust pur via windows-rs `Win32_Graphics_*`
- VFS mounts : `/etc`, `/var` → fichiers Windows réels
- Tests intégration exhaustifs pour chaque sous-système migré

## Workflow recommandé

### 1. Identifier la frontière

Le sous-système C++ à migrer doit avoir :
- Une **API claire** (header `.h` documenté).
- Des **tests** (ou être assez simple pour qu'on en écrive).
- **Aucune dépendance** vers un autre C++ non encore migré (sinon, migrer ce dernier d'abord).

### 2. Créer / étendre une crate Rust

```bash
# Si nouveau sous-système → nouvelle crate
mkdir -p crates/nouvelle-feature/src
# Cargo.toml minimal :
cat > crates/nouvelle-feature/Cargo.toml <<EOF
[package]
name                  = "nouvelle-feature"
publish               = false
version.workspace     = true
edition.workspace     = true
rust-version.workspace = true
authors.workspace     = true
license.workspace     = true

[lints]
workspace = true

[dependencies]
windows = { workspace = true, features = [...] }
EOF
```

Puis ajouter `"crates/nouvelle-feature"` dans `members` du root `Cargo.toml`.

### 3. Implémenter avec windows-rs (pas winapi)

```rust
// AVANT (C++ legacy)
// HANDLE hProcess = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid);

// APRÈS (Rust avec windows-rs)
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};

let handle = unsafe {
    OpenProcess(PROCESS_QUERY_INFORMATION, false, pid)
        .map_err(|e| std::io::Error::other(format!("OpenProcess failed: {e}")))?
};
```

**Pourquoi pas `winapi` ?** windows-rs est l'écosystème officiel Microsoft, généré depuis les metadata WinMD. Bindings plus précis et plus à jour.

### 4. Test parité

Si le C++ avait un test, l'adapter en Rust. Sinon, écrire un test snapshot :

```rust
#[test]
fn parity_with_cpp_implementation() {
    let input = b"sample binary blob";
    let rust_output = nouvelle_feature::process(input);
    // Le C++ avait produit ce hash sur le même input
    assert_eq!(sha256(&rust_output), "abc123...");
}
```

### 5. Retirer le C++

Une fois que :
- Le Rust passe tous les tests.
- Aucun autre code C++ ne dépend du C++ migré.
- La CI valide la nouvelle crate (`cargo ci-offline` vert).

→ Supprimer les fichiers `.cpp` / `.h` correspondants en **un commit dédié** :

```bash
git rm src/ChromiumParser.cpp src/ChromiumParser.h
git commit -m "refactor(forensics)!: drop C++ ChromiumParser, replaced by crates/backend"
```

## Conventions

- **Conventional Commits** avec `!` pour breaking change.
- **Scope précis** : `feat(google_os)`, `refactor(ffi)`, `perf(crypto)`.
- **Lien vers le PR** qui a introduit le code Rust dans le commit qui supprime le C++.

## Pièges connus

### a) Erreurs Win32 vs Errno
Le C++ utilisait souvent `GetLastError()` retournant un `DWORD`. Le Rust mappe vers `std::io::Error::last_os_error()` ou `windows::core::Error`. **Ne pas mélanger** : choisir un type d'erreur cohérent dans la nouvelle crate (préférer `thiserror`).

### b) Buffer ownership
Les API Win32 prennent souvent `LPVOID` (write-into-buffer). Le pattern Rust idiomatique :

```rust
let mut buf = vec![0u8; size];
let mut bytes_read: u32 = 0;
unsafe {
    ReadFile(handle, Some(buf.as_mut_ptr().cast()), buf.len() as u32,
             Some(&mut bytes_read), None)?;
}
buf.truncate(bytes_read as usize);
```

### c) Wide strings (UTF-16)
- Win32 W-functions attendent `*const u16` (UTF-16). Rust strings sont UTF-8.
- Helper : `windows::core::w!("path")` macro (compile-time UTF-16) ou `OsStr::encode_wide()` runtime.

### d) HANDLE leaks
Les `HANDLE` doivent être fermés explicitement avec `CloseHandle`. Pattern RAII :

```rust
struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { CloseHandle(self.0).ok(); }
        }
    }
}
```

### e) Tests cross-platform
Beaucoup de code C++ legacy est Windows-only. En Rust, gater avec `#[cfg(windows)]` :

```rust
#[cfg(windows)]
#[test]
fn test_windows_only() { ... }
```

## Validation

Après chaque migration de sous-système :

```bash
cargo ci-offline -p <crate-migrée>
cargo nextest run -p <crate-migrée>
cargo deny check
git status   # vérifier qu'aucun fichier C++ orphelin ne reste
```
