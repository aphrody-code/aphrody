# `vendor/terminal` — Stratégie d'intégration avec `aphrody` / `google_os`

Document écrit le 2026-05-16, valide pour le commit
`8fe6c21ef88a73a7985b5968ee18936928ccac69`.

Le but : énumérer quels composants de Microsoft Terminal sont
réellement reprenables dans le contexte de `aphrody` (monorepo Rust
qui porte un userland POSIX sur Windows via la crate `google_os` →
syscalls NT natifs via `windows-rs`), et lesquels sont à laisser de
côté. Politique du projet : **zéro stub, zéro mock, 100 % production**
(`CLAUDE.md`).

## 1. Matrice détaillée des composants

Légende :

- **Sortie** : `.lib` statique, `.dll` dynamique, `.exe`, ou en-têtes.
- **Couplage Win32** : oui (utilise `kernel32`/`user32`/`gdi32`/`d3d11`/`d2d`/`dwrite`/`comctl32`/`WinUI`), partiel (Win32
  uniquement par RAII WIL, portable en théorie), non (header-only C++20 STL).
- **Intérêt** : raison concrète pour l'intégrer dans notre stack.
- **État** : `gardé`, `optionnel`, `écarté`.

| Composant Terminal                | Chemin                                       | Sortie                  | Couplage Win32 | Intérêt pour aphrody                                                                  | État        |
|-----------------------------------|----------------------------------------------|-------------------------|---------------|-------------------------------------------------------------------------------------------|-------------|
| `til` (Terminal Implementation Library) | `vendor/terminal/src/inc/til/` + `src/til/` | en-têtes + lib unit-test | partiel (WIL via headers) | utilitaires C++20 : `til::small_vector`, `til::flat_set`, `til::generational`, `til::rect`, `til::point`, `til::color`, `til::env`, `til::rle`, `til::throttled_func`, `til::ticket_lock`, `til::winrt` | **gardé** : utiles si on a un module C++ FFI, *non* utiles côté pur Rust |
| `vtparser` (`TerminalParser`)     | `vendor/terminal/src/terminal/parser/lib/`   | `ConTermParser.lib`     | non           | state machine xterm / ECMA-48 / DEC complète, séquences CSI/OSC/DCS/SS3, supporte Win32-Input-Mode | **gardé** : référence n°1 pour notre futur émulateur de PTY Linux côté `google_os` |
| `bufferout` (`BufferOut`)         | `vendor/terminal/src/buffer/out/lib/`        | `ConBufferOut.lib`      | partiel       | modèle de text buffer UTF-16 / UTF-8 avec attributs SGR, surrogate pairs, lignes DECDHL/DECDWL, recherche, double-width CJK | **gardé** : modèle de référence pour le buffer de nos consoles |
| `winconpty` LIB                   | `vendor/terminal/src/winconpty/lib/`         | `conptylib.lib`         | oui           | implémentation production du ConPTY (handles `\Device\ConDrv\Server` + `\Reference` + pipe de signal, `CreateProcessAsUserW` + `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`) | **gardé** : référence n°2 pour notre crate ConPTY syscall dans `google_os` |
| `winconpty` DLL                   | `vendor/terminal/src/winconpty/dll/`         | `conpty.dll` + `.def`   | oui           | équivalent open-source de `kernel32!CreatePseudoConsole` ; exporte `Conpty*` + alias compat `CreatePseudoConsole` | **optionnel** : utilisable directement par P/Invoke depuis nos crates Rust |
| `terminal/adapter` (`TerminalAdapter`) | `vendor/terminal/src/terminal/adapter/lib/` | `ConTermAdapt.lib` | partiel | mapping verbes VT → calls API console, gère SGR, modes DEC, polices, macros (DECDMAC), Sixel | **gardé** : essentiel si on émule le serveur console côté `google_os` |
| `terminal/input` (`TerminalInput`) | `vendor/terminal/src/terminal/input/lib/`   | `TerminalInput.lib`     | partiel       | encodage clavier→VT (xterm modifyOtherKeys, win32-input-mode, mouse SGR `\e[<…`) | **gardé** : utile pour synthétiser de l'input VT depuis `google_os` |
| `types` (`Types`)                 | `vendor/terminal/src/types/lib/`             | `ConTypes.lib`          | partiel       | `Viewport`, `CodepointWidthDetector` (largeur Unicode + grapheme clusters), `convert.cpp` (UTF-8↔UTF-16), `colorTable`, `sgrStack`, `Cluster`, `IInputEvent`, palettes UIA | **gardé** : `CodepointWidthDetector` est portable, utile au pur layout |
| `renderer/base` (`RendererBase`)  | `vendor/terminal/src/renderer/base/lib/`    | `ConRenderBase.lib`     | partiel       | abstraction `IRenderEngine` + `Renderer` (transforme `IRenderData` en primitives `DrawString`/`DrawCursor`) | **optionnel** : utile uniquement si on garde une UI Win32 ; sinon ré-écrire en Rust |
| `renderer/atlas` (`RendererAtlas`) | `vendor/terminal/src/renderer/atlas/`       | `ConRenderAtlas.lib`    | oui           | moteur DirectWrite + Direct2D + Direct3D 11 + cache de glyphes GPU + custom HLSL shaders | **écarté** : trop spécifique Win32/DirectX, non portable |
| `renderer/gdi` (`RendererGdi`)    | `vendor/terminal/src/renderer/gdi/lib/`    | `ConRenderGdi.lib`      | oui           | rendu GDI (utilisé par `conhost.exe` historique)                                          | **écarté** : Win32 only, perf < Atlas |
| `renderer/uia` (`RendererUia`)    | `vendor/terminal/src/renderer/uia/lib/`    | `ConRenderUia.lib`      | oui           | « rendu » virtuel pour UI Automation                                                       | **écarté** : Win32-only, hors scope |
| `renderer/wddmcon`                | `vendor/terminal/src/renderer/wddmcon/lib/` | `wddmcon.lib`           | oui           | rendu DXGK pour environnement de boot (avant que le compositeur ne tourne)                  | **écarté** : usage interne kernel |
| `server` (`Server`)               | `vendor/terminal/src/server/lib/`           | `ConServer.lib`         | oui (ConDrv)  | implémente côté user-mode le protocole ALPC `\Device\ConDrv\Server` (`ApiDispatchers`, `IoDispatchers`, `WaitBlock`, `ProcessHandle`) | **gardé** : référence d'implémentation du serveur console NT |
| `interactivity/base`              | `vendor/terminal/src/interactivity/base/lib/` | `ConInteractivityBaseLib.lib` | partiel | abstraction `IConsoleControl`, `IConsoleWindow`, `IInteractivityFactory` ; `ServiceLocator` ; `VtApiRedirection` ; `RemoteConsoleControl` | **optionnel** : si on rebondit sur conhost à distance |
| `interactivity/win32`             | `vendor/terminal/src/interactivity/win32/lib/` | `ConInteractivityWin32Lib.lib` | oui | clipboard, dpi, window proc, IME, UIA, sélection, fenêtre `conhost` | **écarté** |
| `interactivity/onecore`           | `vendor/terminal/src/interactivity/onecore/lib/` | `onecore.lib` | oui | équivalent OneCore (sans `user32`)                                                          | **écarté** : niche |
| `propslib` (`PropertiesLibrary`)  | `vendor/terminal/src/propslib/`             | `ConProps.lib`          | oui (registry/LNK) | lecture/écriture des prefs console depuis HKCU + `.lnk`                                  | **écarté** : registry Windows only |
| `propsheet`                       | `vendor/terminal/src/propsheet/`            | `console.dll`           | oui           | property sheet « clic droit propriétés »                                                  | **écarté** |
| `tsf` (`TextServicesFramework`)   | `vendor/terminal/src/tsf/`                  | `ConTSF.lib`            | oui           | bridge IME (CJK, pen, touch)                                                              | **écarté** : nécessite TextServicesFramework Windows |
| `audio/midi`                      | `vendor/terminal/src/audio/midi/lib/`       | `MidiAudio.lib`         | oui (winmm)   | implémentation du DECPSO (commande de musique VT)                                          | **écarté** : usage curiosité |
| `host` (`Host`)                   | `vendor/terminal/src/host/lib/`             | `ConhostV2Lib.lib`      | oui           | tout `conhost.exe` (boucle d'événements, dispatch API, clipboard, fenêtre)                  | **gardé en référence** uniquement |
| `host/exe`                        | `vendor/terminal/src/host/exe/`             | `OpenConsole.exe`       | oui           | `conhost` rebuildable en dev                                                              | **gardé en référence** |
| `host/proxy`                      | `vendor/terminal/src/host/proxy/`           | `OpenConsoleProxy.dll`  | oui           | proxy COM IDL (`IConsoleHandoff`, `ITerminalHandoff`)                                       | **écarté** |
| `cascadia/TerminalConnection`     | `vendor/terminal/src/cascadia/TerminalConnection/` | `Microsoft.Terminal.TerminalConnection.dll` | oui (WinRT) | implémente `ConptyConnection`, `AzureConnection`, `EchoConnection` côté WinRT | **écarté** : WinRT + WinUI 2 obligatoire |
| `cascadia/TerminalCore`           | `vendor/terminal/src/cascadia/TerminalCore/lib/` | `TerminalCore.lib`  | partiel       | classe `Microsoft::Terminal::Core::Terminal` qui composite buffer + parser + adapter + input sans UI ; `ITerminalApi` + `IRenderData` | **optionnel** : c'est une glue C++ propre, utile si on garde une UI |
| `cascadia/TerminalControl`        | `vendor/terminal/src/cascadia/TerminalControl/` | `Microsoft.Terminal.Control.dll` | oui (XAML islands) | `TermControl` (WinUI 2), `HwndTerminal` (Win32 pur via `HwndTerminal.cpp`), automation peer | **optionnel** : `HwndTerminal` est utilisable en C++ pur sans XAML |
| `cascadia/TerminalApp`            | `vendor/terminal/src/cascadia/TerminalApp/` | `TerminalApp.dll`       | oui (WinUI 2) | l'application Terminal (tabs, panes, palette, settings UI)                                  | **écarté** : 100 % WinUI 2 + XAML islands |
| `cascadia/TerminalSettingsModel`  | `vendor/terminal/src/cascadia/TerminalSettingsModel/` | `Microsoft.Terminal.Settings.Model.dll` | oui | parseur JSON5 + héritage de profils, schéma `profiles.schema.json` | **écarté** : couplé WinRT |
| `cascadia/WindowsTerminal`        | `vendor/terminal/src/cascadia/WindowsTerminal/` | `WindowsTerminal.exe` | oui (XAML islands) | hôte Win32 de l'application WinUI                                                          | **écarté** |
| `cascadia/CascadiaPackage`        | `vendor/terminal/src/cascadia/CascadiaPackage/` | `*.msix`              | oui (AppX)    | packaging MSIX, jumplist                                                                  | **écarté** |
| `cascadia/wt`                     | `vendor/terminal/src/cascadia/wt/`          | `wt.exe`, `wtd.exe`     | oui           | shim 36-lignes : redirige `wt args` → `WindowsTerminal.exe wt args` via `CreateProcessW`     | **gardé en référence** : exemple minimal de redirection AppX |
| `cascadia/WpfTerminalControl`     | `vendor/terminal/src/cascadia/WpfTerminalControl/` | `Microsoft.Terminal.Wpf.dll` | oui (WPF .NET) | contrôle terminal pour applications WPF, basé sur `HwndTerminal`                          | **écarté** : .NET WPF |
| `tools/ColorTool`                 | `vendor/terminal/src/tools/ColorTool/`      | `colortool.exe`         | oui           | applique des schemes XTerm dans la palette `conhost`                                      | **écarté** : niche |
| `tools/scratch`, `tools/nihilist`, etc. | `vendor/terminal/src/tools/`           | divers `.exe`           | oui           | bancs d'essai, jouets internes                                                            | **écarté** |

Conclusion résumée :

- **Composants gardés en intégration directe** (link statique) :
  `vtparser`, `bufferout`, `winconptylib`, `terminal/adapter`,
  `terminal/input`, `types`, `server`.
- **Composants utilisés via DLL** : `conpty.dll` (P/Invoke depuis Rust
  est trivial, et c'est aussi ce qu'expose `kernel32` nativement).
- **Composants gardés en référence d'implémentation** : `host`, `wt`,
  `interactivity/base`.
- **Composants écartés** : toute la pile `cascadia/`, `renderer/atlas`,
  `renderer/gdi`, `renderer/uia`, `propsheet`, `propslib`, `tsf`,
  `audio/midi`, `interactivity/win32`.

## 2. Stratégie d'intégration FFI

### 2.1 Cible : crate Rust `crates/terminal_ffi/` (à créer plus tard)

Modèle préconisé :

1. linker les `.lib` statiques C++ depuis `bin/x64/Release/` via une
   `build.rs` qui appelle `cc-rs` (pour les wrappers `extern "C"`) +
   `bindgen` (pour générer les FFI sur les en-têtes `conpty-static.h`,
   `winconpty.h`, et nos propres wrappers) ;
2. exposer une surface C `extern "C"` minimale écrite par nous (côté
   C++) dans un fichier `wrapper.cpp`, pour cacher la machinerie WIL /
   exceptions / RAII C++ ;
3. allouer les tampons via `mimalloc` (cf. `CLAUDE.md`, contrainte zero-copy
   FFI), exposer des pointeurs bruts safe-wrapped côté Rust.

### 2.2 Modèle d'API FFI minimale

Exemple à viser pour un wrapper Rust autour du parser VT, sans
introduire de stub :

```rust
// crates/terminal_ffi/src/parser.rs (à créer plus tard, pas maintenant)
unsafe extern "C" {
    fn gcli_vt_parser_new() -> *mut GcliVtParser;
    fn gcli_vt_parser_free(p: *mut GcliVtParser);
    fn gcli_vt_parser_feed(p: *mut GcliVtParser,
                           data: *const u16, len: usize,
                           callback: GcliVtCallback,
                           user: *mut std::ffi::c_void) -> u32;
}
```

`wrapper.cpp` côté C++ instancie un
`Microsoft::Console::VirtualTerminal::StateMachine` (cf.
`vendor/terminal/src/terminal/parser/stateMachine.hpp`, classes
publiques `StateMachine`, `OutputStateMachineEngine`,
`InputStateMachineEngine`) et redispatche les actions via la callback.

### 2.3 ConPTY direct depuis Rust (sans terminal_ffi)

Le DLL `conpty.dll` exporte des symboles équivalents à
`kernel32!CreatePseudoConsole`. Liaison via `windows-rs` :

```text
use windows::Win32::System::Console::{
    CreatePseudoConsole, ResizePseudoConsole, ClosePseudoConsole, HPCON, COORD,
};
```

Tant qu'on tourne sur Windows 10 19041+, on peut utiliser le ConPTY
système. Pour bénéficier des correctifs récents (notamment GH#12977 sur
le win32-input-mode), il faut charger `conpty.dll` produit par
`winconpty/dll` depuis `bin/`. Cf. `vendor/terminal/src/winconpty/dll/winconpty.def`
pour la liste exacte d'exports.

### 2.4 Lecture du text buffer en zero-copy

`vendor/terminal/src/buffer/out/textBuffer.hpp` expose des itérateurs
(`TextBufferCellIterator`, `TextBufferTextIterator`) qui retournent des
`std::wstring_view` sur les cellules. Pour rester zero-copy entre Rust
et C++ via `mimalloc`, l'approche correcte :

1. wrapper C++ qui prend un `std::function<void(const wchar_t*, size_t, TextAttribute)>`
   et l'appelle pour chaque run ;
2. la callback C `extern "C"` côté Rust reçoit un slice `&[u16]` et un
   `u64` d'attributs SGR encodés ;
3. allocation des buffers de cellules dans l'arène `mimalloc` partagée
   (`mi_malloc` / `mi_free`).

## 3. Sécurité

### 3.1 Modèle C++ Terminal incompatible avec `no_std` Rust

Terminal repose massivement sur :

- **WIL** (`vendor/terminal/dep/wil/`) : macros `RETURN_IF_*`,
  `THROW_IF_*`, smart handles (`wil::unique_handle`,
  `wil::unique_process_information`, etc.). WIL utilise les exceptions
  C++ pour propager des erreurs depuis les helpers
  `THROW_IF_WIN32_BOOL_FALSE`.
- **RAII C++** partout (`std::filesystem::path`, `std::unique_ptr`,
  scope guards).
- **Exceptions C++** internes, encapsulées dans les classes mais
  jamais converties en codes d'erreur pour les fonctions publiques (cf.
  `vendor/terminal/doc/EXCEPTIONS.md`).

Conséquence : exposer une surface `extern "C"` est **obligatoire** pour
toute consommation depuis Rust. Aucun symbole C++ ne doit traverser le
FFI directement. Les exceptions doivent être interceptées dans le
wrapper et converties en codes d'erreur :

```cpp
// wrapper.cpp esquisse (à écrire le moment venu, pas maintenant)
extern "C" int32_t gcli_pty_create(/* args */, HPCON* out) noexcept try
{
    return SUCCEEDED(ConptyCreatePseudoConsole(/*…*/, out)) ? 0 : -1;
}
catch (...)
{
    return -2;
}
```

`noexcept` + try/catch sur la totalité du corps est le seul moyen
sûr d'éviter de laisser remonter une exception C++ jusqu'à l'unwinder
Rust (UB).

### 3.2 Plateformes

`google_os` vise à terme MUSL Linux et WebAssembly (cf. commit
`f986583c` de notre repo : « configure ultra-minimal release profiles
and isolate Windows dependencies to enable Linux MUSL and WebAssembly
cross-compilation »). Tous les composants Terminal listés « gardés »
sont C++ portable **sur le papier** mais en pratique câblés à Win32 :

- `vtparser` n'a pas de dépendance Win32 hors `wchar_t` 16-bit (à
  vérifier précisément avec `nm` après build), c'est le plus
  portable ;
- `bufferout` dépend de `til/u8u16convert.h` qui n'utilise pas Win32 ;
- `winconpty` est intrinsèquement Win32 (utilise `NtCreateFile`,
  `CreateProcessAsUserW`, `\Device\ConDrv`). **Non portable.** Seul
  utilisable en condition d'hôte Windows.

**Décision projet (2026-05-16)** : pour les cibles non-Windows
(MUSL Linux, WebAssembly, BSD à venir), **aucun composant C++ de
Microsoft Terminal n'est porté ni linké**. Tout est ré-implémenté en
**Rust pur** dans `crates/google_os/` :

- VT parser → réécrire en Rust (s'inspirer de `vte` crate ou repartir
  de zéro à partir de `vendor/terminal/src/terminal/parser/lib/` pour
  le comportement, pas pour le code) ;
- TextBuffer → modèle data-structure en Rust (`crates/google_os/src/`
  + éventuel sous-crate `crates/terminal_buffer/`) ;
- ConPTY → ré-implémenter au-dessus de `posix_openpt` / `forkpty` (Linux)
  ou shim WASI (WASM), en s'inspirant de l'**algorithme** documenté
  dans `vendor/terminal/src/winconpty/winconpty.cpp` (séquence
  `CreateServerHandle` + `CreateClientHandle(\Reference)` + signal
  pipe + `CreateProcessAsUserW(--headless)`) — pas du code.

Conséquence : Terminal Microsoft reste **strictement Windows-hôte**
dans notre stack. Le futur `crates/terminal_ffi` (si on le crée) ne
compile que sous `--target x86_64-pc-windows-msvc`, guard par
`#[cfg(target_os = "windows")]`. Tout chemin Linux/WASM est servi par
nos crates Rust natives.

Cette décision tranche le débat `wchar_t` 16-bit vs 32-bit et
`-fshort-wchar` : on n'y touche pas, car aucun code C++ Terminal ne
traverse la frontière OS.

### 3.3 ABI

Microsoft Terminal compile en x64 par défaut. `common.build.pre.props`
définit `_WINDOWS;EXTERNAL_BUILD;_SILENCE_STDEXT_ARR_ITERS_DEPRECATION_WARNING`
et impose `stdcpp20`. ABI MSVC C++20, non compatible MinGW. On linkera
donc nos crates Rust avec la même toolchain MSVC (`stable-x86_64-pc-windows-msvc`).

## 4. Aspects légaux

`vendor/terminal/LICENSE` : MIT. Permet :

- distribution binaire dans `aphrody` (avec mention de copyright) ;
- modification (mais on ne modifie pas l'upstream ; on vendore et on
  surcharge MSBuild via wrapper) ;
- link statique dans nos binaires Rust → OK ;
- relicensing impossible (rester sous MIT pour la portion redistribuée).

`vendor/terminal/NOTICE.md` liste les composants tiers embarqués sous
MIT ou Apache-2.0 compatibles : `jsoncpp`, `chromium/base/numerics`,
`{fmt}`, `interval_tree`, `pcg`, `wyhash`, `stb`, `Oklab`,
`ColorBrewer` (Apache-2.0), `cmark`, `fzf`, GSL, Microsoft-UI-XAML,
VirtualDesktopUtils, WIL. **Si on redistribue un binaire embarquant
ces composants, il faut reproduire les notices.**

Risque : le NuGet `Microsoft.Internal.PGO-Helpers.Cpp` est interne
Microsoft, non MIT. La PGO build doit rester désactivée par défaut
chez nous (déjà le cas — pas dans le wrapper).

### 4.1 Décisions projet 2026-05-16 sur les dépendances tierces

**Feed NuGet privé `pkgs.dev.azure.com/shine-oss/terminal/_packaging/TerminalDependencies`**
(défini dans `vendor/terminal/NuGet.Config`).

- **Décision** : accepté tel quel, pas de mirroir proactif.
- **Plan de repli** : les packages restaurés vivent dans
  `vendor/terminal/packages/` (créé par `nuget restore` au premier
  build) et restent localement disponibles. Si Microsoft restreint
  un jour l'accès au feed, on peut soit (a) committer le dossier
  `packages/` une fois pour toutes (gros mais portable), soit
  (b) push les `.nupkg` vers un feed NuGet interne (ex. GitHub
  Packages, Azure Artifacts).
- **Pourquoi pas mirroir maintenant** : le feed est public en lecture,
  rien ne casse aujourd'hui, et mirroir 200+ MB de NuGet sans raison
  pollue le repo.

**Packages internes Microsoft** : `Microsoft.Internal.PGO-Helpers.Cpp`,
`Microsoft.Internal.Windows.Terminal.ThemeHelpers`.

- **Décision** : acceptés tant qu'ils sont publics depuis le feed
  ci-dessus (ils le sont actuellement, le restore réussit).
- **Si on perd l'accès** : PGO est non activée chez nous
  (`PgoBuildType` non set → `Microsoft.Internal.PGO-Helpers.Cpp` est
  inerte). `ThemeHelpers` est consommé par `CascadiaPackage` ; si on
  ne build pas le UI Terminal (et notre intégration cible parser/buffer/conpty
  uniquement), on peut désactiver `CascadiaPackage` via
  `/p:BuildProjectReferences=false` sur le subgraph.

## 5. Checklist go/no-go pour intégrer Terminal

Avant de tirer le moindre `.lib` Terminal dans un crate :

- [ ] le composant figure-t-il dans la colonne « **gardé** » de la
      matrice § 1 ? Si non → stop.
- [ ] le composant peut-il être enveloppé dans un `extern "C"`
      `noexcept try/catch` ?
- [ ] la cible Rust est-elle compilée avec
      `stable-x86_64-pc-windows-msvc` (toolset compatible MSVC v145) ?
- [ ] le crate qui consomme respecte-t-il la directive `CLAUDE.md` :
      pas de stub, pas de TODO, allocation via `mimalloc` ?
- [ ] les tests TAEF du composant (cf. `tools/tests.xml`) tournent-ils
      proprement en local ?
- [ ] si on redistribue, `NOTICE.md` est-il mis à jour ?
- [ ] si le composant est Win32-only (`winconpty`, `host`,
      `interactivity/win32`), a-t-on un plan B pour Linux/WASM
      (cf. § 3.2) ?

Si toutes les cases sont cochées → go. Sinon, garder le composant en
« référence » et ré-implémenter en Rust dans `google_os`.

## 6. Conclusion

L'intégration ciblée *value* de Terminal sur **Windows hôte** se
réduit à un ensemble clair : **parser VT, adapter, input, buffer,
types, server, conpty**.

Sur **toutes les autres cibles** (Linux MUSL, WebAssembly, BSD…),
décision projet 2026-05-16 : **Rust pur, point final**. Les composants
Terminal ne servent que de référence algorithmique ; ils ne sont ni
portés, ni linkés. La parité fonctionnelle (VT parsing, TextBuffer,
PTY) est livrée par les crates `google_os` et compagnie. Cela libère
le projet de l'ABI MSVC C++, de `wchar_t` 16-bit, de WIL et des
exceptions C++ partout où Windows n'est pas l'hôte.
</content>
</invoke>
