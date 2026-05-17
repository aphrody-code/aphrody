# `vendor/terminal` — Architecture détaillée

Document écrit le 2026-05-16, valide pour le commit
`8fe6c21ef88a73a7985b5968ee18936928ccac69` du dépôt `microsoft/terminal`.

Sommaire :

1. [Vue d'ensemble](#1-vue-densemble)
2. [Cartographie du dépôt (racine)](#2-cartographie-du-dépôt-racine)
3. [Cartographie de `src/`](#3-cartographie-de-src)
4. [Architecture en couches](#4-architecture-en-couches)
5. [Composants réutilisables clés](#5-composants-réutilisables-clés)
6. [Build et toolchain](#6-build-et-toolchain)
7. [Tests](#7-tests)
8. [Politiques de code](#8-politiques-de-code)
9. [Specs et roadmap](#9-specs-et-roadmap)
10. [Licence et conformité](#10-licence-et-conformité)
11. [Risques d'intégration](#11-risques-dintégration)
12. [Conclusion : ce qu'on garde pour `google_os`](#12-conclusion--ce-quon-garde-pour-google_os)

---

## 1. Vue d'ensemble

`microsoft/terminal` est un monorepo qui produit conjointement
**plusieurs binaires** distincts, tous gouvernés par `OpenConsole.slnx`
à la racine.

### 1.1 Binaires produits

Recensés à partir de `vendor/terminal/OpenConsole.slnx` et de
l'inspection des `.vcxproj` de chaque cible.

| Binaire                                  | Type | Origine `.vcxproj`                                                          | Rôle                                                                                            |
|------------------------------------------|------|-----------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------|
| `OpenConsole.exe`                        | exe  | `vendor/terminal/src/host/exe/Host.EXE.vcxproj` (`TargetName=OpenConsole`)  | Build dev de `conhost.exe`, le serveur de console NT historique de Windows.                     |
| `WindowsTerminal.exe`                    | exe  | `vendor/terminal/src/cascadia/WindowsTerminal/WindowsTerminal.vcxproj`      | Hôte Win32 de l'application Terminal moderne. Ouvre une fenêtre XAML islands + DirectX surface. |
| `wt.exe` / `wtd.exe`                     | exe  | `vendor/terminal/src/cascadia/wt/wt.vcxproj`                                | Shim de 36 lignes qui redirige vers `WindowsTerminal.exe`. `wtd` = branding Dev.                |
| `conpty.dll`                             | dll  | `vendor/terminal/src/winconpty/dll/winconptydll.vcxproj`                    | Implémentation open-source du ConPTY (alias des exports de `kernel32`).                         |
| `conptylib.lib`                          | lib  | `vendor/terminal/src/winconpty/lib/winconptylib.vcxproj`                    | Version statique des mêmes symboles (sans `dllimport`, via `conpty-static.h`).                  |
| `OpenConsoleProxy.dll`                   | dll  | `vendor/terminal/src/host/proxy/Host.Proxy.vcxproj`                         | Proxy/stub MIDL pour les interfaces COM `IConsoleHandoff` et `ITerminalHandoff`.                |
| `console.dll`                            | dll  | `vendor/terminal/src/propsheet/propsheet.vcxproj`                           | Property sheet « clic droit > Propriétés » d'une fenêtre conhost.                               |
| `Microsoft.Terminal.Control.dll`         | dll  | `vendor/terminal/src/cascadia/TerminalControl/dll/TerminalControl.vcxproj`  | Contrôle WinUI 2 réutilisable (le « TermControl »), basé sur la TextBuffer + le renderer Atlas. |
| `Microsoft.Terminal.Settings.Model.dll`  | dll  | `vendor/terminal/src/cascadia/TerminalSettingsModel/dll/Microsoft.Terminal.Settings.Model.vcxproj` | Modèle de configuration JSON5 (héritage de profils, layouts, schemes).        |
| `Microsoft.Terminal.Settings.Editor.dll` | dll  | `vendor/terminal/src/cascadia/TerminalSettingsEditor/Microsoft.Terminal.Settings.Editor.vcxproj` | UI WinUI 2 d'édition des settings.                                              |
| `TerminalApp.dll`                        | dll  | `vendor/terminal/src/cascadia/TerminalApp/dll/TerminalApp.vcxproj`          | Application (tabs, panes, palette de commandes, JIT activation des profils).                    |
| `WindowsTerminalShellExt.dll`            | dll  | `vendor/terminal/src/cascadia/ShellExtension/WindowsTerminalShellExt.vcxproj` | Extension shell « Ouvrir dans Terminal » d'Explorer.                                          |
| `elevate-shim.exe`                       | exe  | `vendor/terminal/src/cascadia/ElevateShim/elevate-shim.vcxproj`             | Shim pour relancer Terminal en élévation UAC.                                                   |
| `UIHelpers.dll`, `UIMarkdown.dll`, `WinRTUtils.dll` | dll | `vendor/terminal/src/cascadia/{UIHelpers,UIMarkdown,WinRTUtils}/*.vcxproj` | Utilitaires WinRT internes.                                                                |
| `Microsoft.Terminal.Wpf.dll`             | dll  | `vendor/terminal/src/cascadia/WpfTerminalControl/WpfTerminalControl.csproj` | Wrapper WPF de `HwndTerminal`.                                                                  |
| `CascadiaPackage_*.msix`                 | msix | `vendor/terminal/src/cascadia/CascadiaPackage/CascadiaPackage.wapproj`      | Paquet MSIX qui agrège `WindowsTerminal.exe` + DLL + `OpenConsole.exe`.                         |
| `colortool.exe`                          | exe  | `vendor/terminal/src/tools/ColorTool/ColorTool.sln`                         | Petit utilitaire .NET pour appliquer des schemes XTerm à la palette conhost.                    |

À cela s'ajoutent une vingtaine d'utilitaires internes
(`vendor/terminal/src/tools/{benchcat,buffersize,ConsoleBench,nihilist,closetest,fontlist,RenderingTests,scratch,vtapp,vtpipeterm,U8U16Test,TerminalStress,…}/*.vcxproj`)
et tous les binaires de tests (`*.Unit.Tests.dll`, `*.Feature.Tests.dll`,
`*.UIA.Tests.dll`). La liste complète des projets est dans
`vendor/terminal/OpenConsole.slnx` (1060 lignes, ~70 projets).

### 1.2 Différence Terminal vs Console Host vs ConPTY

Trois entités à ne pas confondre :

- **Console Host (`conhost.exe`)** : le serveur de console historique
  de Windows. Il implémente le protocole `\Device\ConDrv\Server` côté
  user-mode (ALPC), porte la window proc historique avec rendu GDI et
  expose l'API Win32 Console (`ReadConsoleA`, `WriteConsoleA`,
  `GetConsoleScreenBufferInfo`, etc.). Source officielle du `conhost.exe`
  livré par l'OS = ce repo (`src/host/`). En dev on en produit
  `OpenConsole.exe` (cf. `Host.EXE.vcxproj`, `TargetName=OpenConsole`)
  pour éviter de remplacer celui de `System32`.

- **Windows Terminal (`WindowsTerminal.exe`)** : l'application
  utilisateur moderne. Elle gère les onglets, les panneaux, les profils,
  le rendu DirectX, etc. Elle **n'implémente pas** l'API Console : pour
  ça, elle ouvre un ConPTY et laisse `conhost.exe` (lancé en mode
  `--headless`) parler à l'application cliente. Du point de vue de
  `cmd.exe`, `powershell.exe`, `bash.exe`, le serveur reste donc
  `conhost`, mais le rendu visuel est dans Terminal.

- **ConPTY (`conpty.dll` + `OpenConsole.exe --headless`)** : la
  pseudo-console. C'est l'équivalent Windows de `forkpty()`. Elle
  expose en entrée/sortie deux pipes encodés UTF-8 + VT, et derrière
  spawn `conhost.exe --headless` qui traduit ces flux VT en API
  Console pour les vieux clients qui appellent `WriteConsoleA`. Cf.
  `vendor/terminal/src/winconpty/winconpty.cpp:_CreatePseudoConsole`.

Ces trois entités sont distinctes mais bâties sur les **mêmes libs
statiques** (`bufferout`, `parser`, `adapter`, `server`, etc.).

### 1.3 Versions Windows ciblées

Lecture de `vendor/terminal/src/common.build.pre.props` lignes 77-80 :

```xml
<WindowsTargetPlatformVersion Condition="'$(WindowsTargetPlatformVersion)' == ''">10.0.22621.0</WindowsTargetPlatformVersion>
<WindowsTargetPlatformMinVersion Condition="'$(WindowsTargetPlatformMinVersion)' == ''">10.0.18362.0</WindowsTargetPlatformMinVersion>
```

- **SDK de compilation** : 10.0.22621.0 (Windows 11 21H2). Notre wrapper
  surcharge avec 10.0.26100.0 (Windows 11 24H2).
- **OS min runtime** : 10.0.18362.0 (Windows 10 1903), mais le README
  upstream précise « Windows 10 2004 (build >= 19041) ou plus tard »
  pour Terminal lui-même.

---

## 2. Cartographie du dépôt (racine)

`vendor/terminal/` au niveau supérieur :

| Entrée                            | Rôle                                                                                                    |
|-----------------------------------|---------------------------------------------------------------------------------------------------------|
| `LICENSE`                         | MIT (copyright Microsoft Corporation).                                                                  |
| `NOTICE.md`                       | Notices tierces (jsoncpp, chromium/numerics, {fmt}, interval_tree, pcg, wyhash, stb, Oklab, ColorBrewer, cmark, fzf, GSL, MUX, VirtualDesktopUtils, WIL). |
| `README.md`                       | Présentation, install via Store/winget/Chocolatey/Scoop, build, FAQ.                                    |
| `OpenConsole.slnx`                | Solution unique du repo (format `.slnx` 1060 lignes, 4 `BuildType`, 4 `Platform`, 70 projets dans 15 dossiers). |
| `Scratch.sln`                     | Petite sln pour bidouiller du code expérimental sans charger tout `OpenConsole`.                        |
| `XamlStyler.json`                 | Config de formatting XAML (utilisé par `Invoke-XamlFormat`).                                            |
| `NuGet.Config`                    | Source NuGet **unique** : `https://pkgs.dev.azure.com/shine-oss/terminal/_packaging/TerminalDependencies%40Local/nuget/v3/index.json`. Mirror Azure DevOps Microsoft. |
| `vcpkg.json`                      | Manifest vcpkg : `fmt 12.1.0`, `ms-gsl 3.1.0` + feature `terminal` (jsoncpp 1.9.6, cli11 2.6.1, cmark 0.31.1), baseline `15e5f3820f0370f1ba…`. |
| `Directory.Build.props`           | Optionnellement active MSBuildCache (local/Pipeline).                                                   |
| `Directory.Build.targets`         | Idem côté targets.                                                                                      |
| `common.openconsole.props`        | Définit `$(OpenConsoleDir)` pour les `.wapproj` qui ne reçoivent pas correctement `$(SolutionDir)`.     |
| `custom.props`                    | Read by XES (release pipeline) : `XesBaseYearForStoreVersion=2026`, `VersionMajor=1`, `VersionMinor=26`. |
| `dirs`                            | Ancien marqueur Razzle (`DIRS=src`).                                                                    |
| `consolegit2gitfilters.json`      | Filtres utilisés par les outils internes Microsoft pour synchroniser conhost vers le repo OS.            |

Le wrapper de build aphrody est **hors sous-module**, à
`scripts/terminal/build.ps1`. Les patches locaux appliqués sous
`vendor/terminal/dep/vcpkg-overlay-triplets/*.cmake` et
`vendor/terminal/src/common.build.pre.props` sont archivés dans
`docs/terminal/PATCHES.diff` (cf. `BUILD.md`).

| Dossier                           | Rôle                                                                                                    |
|-----------------------------------|---------------------------------------------------------------------------------------------------------|
| `bin/`                            | Sorties MSBuild (`bin/<Platform>/<Configuration>/`).                                                    |
| `obj/`                            | Intermédiaires MSBuild + vcpkg installé (`obj/<Platform>/vcpkg/`).                                      |
| `packages/`                       | NuGet packages restored.                                                                                |
| `src/`                            | Code source des composants (voir § 3).                                                                  |
| `dep/`                            | Dépendances embarquées : `Console/` (headers internes), `NT/` (structs NT non publiques), `Win32K/` (headers privés window manager), `WinAppDriver/` (UI tests), `nuget/` (nuget.exe + `packages.config`), `telemetry/`, `vcpkg-overlay-ports/`, `vcpkg-overlay-triplets/`. |
| `tools/`                          | Scripts PowerShell (`OpenConsole.psm1`) et cmd (`razzle.cmd`, `bcz.cmd`, `runut.cmd`, `runft.cmd`, `runuia.cmd`, `bcx.cmd`, `bx.cmd`), `tests.xml`, `WindbgExtension.js`, génération de header (`Generate-CodepointWidthsFromUCD.ps1`, `Generate-FeatureStagingHeader.ps1`, `GenerateHeaderForJson.ps1`, `GenerateSettingsIndex.ps1`), profil WPR (`ConsolePerf.wprp`, `Terminal.wprp`), `StaticAnalysis.ruleset`. |
| `doc/`                            | Documentation : `STYLE.md`, `ORGANIZATION.md`, `EXCEPTIONS.md`, `WIL.md`, `Niksa.md`, `virtual-dtors.md`, `TAEF.md`, `feature_flags.md`, `building.md`, `Debugging.md`, `submitting_code.md`, `UniversalTest.md`, `WindowsTestPasses.md`, `bot.md`, `fuzzing.md`, `terminal-{a11y-2023,v1-roadmap,v2-roadmap}.md`, `roadmap-2022.md`, `roadmap-2023.md`, `color_nudging.html`, `creating_a_new_project.md`, `AddASetting.md`, `COOKED_READ_DATA.md`, `ConsoleCtrlEvent.md`, `ConsoleHostSettings.md`. Plus `specs/` (60+ specs détaillées), `cascadia/`, `reference/`, `user-docs/`, `images/`. |
| `samples/`                        | Exemples d'utilisation : `ConPTY/EchoCon` (créer un ConPTY et y lancer `ping localhost`), `ConPTY/MiniTerm` (petit terminal C++), `ConPTY/GUIConsole` (variante WPF/.NET), `PixelShaders` (HLSL custom pour AtlasEngine), `ReadConsoleInputStream`. |
| `oss/`                            | Bibliothèques tierces vendorisées en source : `chromium/` (numerics), `interval_tree/`, `pcg/`, `stb/`, `wyhash/`, `xorg_apps_rgb/` + `README.md`. |
| `build/`                          | Pipeline Azure DevOps : `pipelines/{ci,release}.yml`, `pipelines/templates/`, `scripts/{Create-AppxBundle,Index-Pdbs,Invoke-FormattingCheck,Run-Tests,Test-WindowsTerminalPackage}.ps1`, `rules/Branding.targets`, `rules/CollectWildcardResources.targets`, `config/`, `Fuzz/`, `Helix/`, `StoreSubmission/`, `packages.config`, `pgo/`. |
| `res/`                            | Ressources de branding : `LICENSE`, `README.md`, `console.ico`, `fonts/`, `terminal/`, `terminal.ico`, `truetype.bmp`. |
| `policies/`                       | Templates de stratégies de groupe : `WindowsTerminal.admx`, `en-US/`. |
| `scratch/`                        | Brouillons jetables.                                                                                    |

Le dossier `.config/` (mentionné dans le README pour les
`configuration.winget`) n'est pas présent dans notre checkout (probablement
masqué par `.gitignore` ou non poussé).

---

## 3. Cartographie de `src/`

Listing brut (`vendor/terminal/src/`) avec rôle synthétique. Sources :
inspection des `.vcxproj`, `doc/ORGANIZATION.md`, et lecture des
`README.md` quand ils existent.

### 3.1 Communs

| Sous-dossier             | Rôle                                                                                                              |
|--------------------------|-------------------------------------------------------------------------------------------------------------------|
| `src/inc/`               | Headers publics partagés : `DefaultSettings.h`, `HostAndPropsheetIncludes.h`, `HostSignals.hpp`, `LibraryIncludes.h`, `TestUtils.h`, `WilErrorReporting.h`, `conattrs.hpp`, `conint.h`, `conpty-static.h`, `consoletaeftemplates.hpp`, `cpl_core.h`, `unicode.hpp`, `winrtTaefTemplates.hpp`. Sous-dossier `til/` (37 headers : `at.h`, `atomic.h`, `bit.h`, `bytes.h`, `coalesce.h`, `color.h`, `colorbrewer.h`, `enumset.h`, `env.h`, `flat_set.h`, `generational.h`, `hash.h`, `io.h`, `latch.h`, `math.h`, `mutex.h`, `operators.h`, `pmr.h`, `point.h`, `rand.h`, `rect.h`, `regex.h`, `replace.h`, `rle.h`, `size.h`, `small_vector.h`, `spsc.h`, `static_map.h`, `string.h`, `throttled_func.h`, `ticket_lock.h`, `type_traits.h`, `u8u16convert.h`, `unicode.h`, `winrt.h`) ; `CppCoreCheck/warnings.h` ; `test/CommonState.hpp`.                                                                                                                                                                                                                                                                                                                                                                                  |
| `src/til/`               | Library cible pour `til/` : `precomp.{cpp,h}`, `dirs`, sous-dossier `ut_til/` (TAEF tests). Cible MSBuild = `til.unit.tests.dll`. |
| `src/internal/`          | `Internal.vcxproj` (`TargetName=ConInt`) : stubs pour les symboles internes Microsoft non redistribuables (`stubs.cpp`).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `src/staging/`           | Sous-dossier vide à part `makefile.inc` + `sources` (artefacts Razzle), non lié à la solution.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `src/features.xml`       | Source du système de feature flags : chaque feature génère un `Feature_XXX::IsEnabled()` + `TIL_FEATURE_XXX_ENABLED`. Doc : `vendor/terminal/doc/feature_flags.md`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src/testlist`           | Fichier de configuration utilisé par `TestTableWriter` pour générer la liste des suites TAEF.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `src/common.build.{pre,post,tests}.props`, `src/common.nugetversions.{props,targets}`, `src/cppwinrt.build.{pre,post}.props`, `src/wap-common.build.{pre,post}.props` | Couche commune MSBuild. `pre.props` impose `PlatformToolset=v143`, `LanguageStandard=stdcpp20`, options conformes (`/Zc:__cplusplus /Zc:__STDC__ /Zc:enumTypes /Zc:inline /Zc:templateScope /Zc:throwingNew`), warnings = errors (`TreatWarningAsError`), `EXTERNAL_BUILD`, `HybridCRT`, et configure vcpkg + 4 configs (Debug/Release/AuditMode/Fuzzing) × 3 plates-formes. |
| `src/unit.tests.{x64,x86}.runsettings` | Settings pour vstest.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `src/dirs`               | Marqueur Razzle.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `src/project.inc`, `src/project.unittest.inc` | Defaults pour les sources Razzle.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |

### 3.2 Sous-dossiers fonctionnels

| Dossier                      | `.vcxproj`(s) / cible                                                                                  | Dépendances majeures                                                  | Rôle                                                                                                                                          |
|------------------------------|--------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|
| `src/host/`                  | `lib/hostlib.vcxproj` (`ConhostV2Lib.lib`), `exe/Host.EXE.vcxproj` (`OpenConsole.exe`), `proxy/Host.Proxy.vcxproj` (`OpenConsoleProxy.dll`), `ft_host/`, `ft_uia/`, `ft_integrity/`, `ft_fuzzer/`, `ut_host/`, `ut_lib/`. | Quasi tous les autres modules.                                        | Le cœur de `conhost.exe`. ~80 fichiers `.cpp` (`_output.cpp`, `_stream.cpp`, `cmdline.cpp`, `consoleInformation.cpp`, `srvinit.cpp`, `handle.cpp`, `directio.cpp`, `getset.cpp`, `globals.cpp`, `screenInfo.cpp`, `cursor.cpp`, `selection.cpp`, `inputBuffer.cpp`, `clipboard.cpp` ailleurs, `registry.cpp`, `settings.cpp`, `outputStream.cpp`, `readDataCooked.cpp`, `readDataDirect.cpp`, `readDataRaw.cpp`, `VtIo.cpp`, `VtInputThread.cpp`, `PtySignalInputThread.cpp`, etc.). |
| `src/server/`                | `lib/server.vcxproj` (`ConServer.lib`).                                                                | `host/proxy` (pour les IDL `IConsoleHandoff`/`ITerminalHandoff`).     | Couche IPC user-mode parlant à `\Device\ConDrv\Server` via ALPC. Fichiers : `ApiDispatchers.cpp`, `ApiMessage.cpp`, `ApiSorter.cpp`, `ConDrvDeviceComm.cpp`, `ConsoleShimPolicy.cpp`, `DeviceHandle.cpp`, `Entrypoints.cpp`, `IoDispatchers.cpp`, `IoSorter.cpp`, `ObjectHandle.cpp`, `ObjectHeader.cpp`, `ProcessHandle.cpp`, `ProcessList.cpp`, `ProcessPolicy.cpp`, `WaitBlock.cpp`, `WaitQueue.cpp`, `WinNTControl.cpp` (chargement dynamique de `ntdll.dll`). |
| `src/winconpty/`             | `lib/winconptylib.vcxproj` (`conptylib.lib`), `dll/winconptydll.vcxproj` (`conpty.dll` + `winconpty.def`), `ft_pty/winconpty.FeatureTests.vcxproj`, `package/winconpty.nuspec`. | `server/DeviceHandle.cpp`, `server/WinNTControl.cpp`.                 | ConPTY userspace : `_CreatePseudoConsole` (création serveur ConDrv + pipe signal + spawn `conhost --headless`), `_ResizePseudoConsole`, `_ShowHidePseudoConsole`, `_ReparentPseudoConsole`, `_ClosePseudoConsoleMembers`. Exporte `ConptyCreatePseudoConsole` + alias `CreatePseudoConsole` (cf. `winconpty.def`).                                                                                                                                                                                                                                                                                                                                                            |
| `src/buffer/out/`            | `lib/bufferout.vcxproj` (`ConBufferOut.lib`), `ut_textbuffer/TextBuffer.Unit.Tests.vcxproj`.            | `types`.                                                              | Le **text buffer** : `Row.cpp` (ligne logique avec attributs SGR), `textBuffer.cpp` (buffer circulaire 2D), `textBufferCellIterator.cpp` + `textBufferTextIterator.cpp` (itérateurs zero-copy), `OutputCell.cpp` (cellule unitaire), `OutputCellIterator.cpp`, `OutputCellRect.cpp`, `OutputCellView.cpp`, `cursor.cpp`, `search.cpp`, `TextAttribute.cpp`, `TextColor.cpp` (16+RGB SGR), `ImageSlice.cpp` (rendu d'images Sixel/iTerm), `UTextAdapter.cpp` (passe le buffer à ICU). Headers principaux : `textBuffer.hpp`, `Row.hpp`, `LineRendition.hpp` (DECDHL/DECDWL), `Marks.hpp` (marks pour shell integration GH#11000), `DbcsAttribute.hpp`.                                                                                       |
| `src/terminal/parser/`       | `lib/parser.vcxproj` (`ConTermParser.lib`), `ft_fuzzer/VTCommandFuzzer.vcxproj`, `ft_fuzzwrapper/FuzzWrapper.vcxproj`, `ut_parser/Parser.UnitTests.vcxproj`. | aucune (header-only sur til, std).                                    | State machine ECMA-48 / VT100/220/320/420 + DEC + XTerm. Fichiers : `stateMachine.cpp` (entièrement la machine d'états), `OutputStateMachineEngine.cpp`, `InputStateMachineEngine.cpp`, `tracing.cpp`, `base64.cpp` (pour OSC 52 clipboard). Classe-clé : `Microsoft::Console::VirtualTerminal::StateMachine` (cf. `stateMachine.hpp`, support de `MAX_PARAMETER_VALUE=65535`, `MAX_PARAMETER_COUNT=32`, `MAX_SUBPARAMETER_COUNT=6`, gère C1, ANSI/VT52, OSC, DCS, SS3, mode `AcceptC1`).                                                                                                                                                                                                  |
| `src/terminal/adapter/`      | `lib/adapter.vcxproj` (`ConTermAdapt.lib`), `ut_adapter/Adapter.UnitTests.vcxproj`.                      | `types`, `terminal/input`.                                            | Adaptateur des verbes VT vers les calls API console. `adaptDispatch.cpp` (implémente `ITermDispatch`), `adaptDispatchGraphics.cpp` (SGR), `terminalOutput.cpp` (charsets G0..G3, designations DEC), `FontBuffer.cpp` (DECDLD soft-fonts), `MacroBuffer.cpp` (DECDMAC), `PageManager.cpp` (DECNCMR pages), `SixelParser.cpp` (DECSIXEL pour images). Interfaces : `ITerminalApi.hpp`, `ITermDispatch.hpp`, `IInteractDispatch.hpp`, `InteractDispatch.cpp`. |
| `src/terminal/input/`        | `lib/terminalinput.vcxproj` (`TerminalInput.lib`).                                                      | `types`.                                                              | Encodage clavier vers VT (xterm `modifyOtherKeys`, win32-input-mode = `CSI 9001`, SS3 pour fonctions), encodage souris SGR `\e[<…`. Fichiers : `terminalInput.cpp`, `mouseInput.cpp`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `src/types/`                 | `lib/types.vcxproj` (`ConTypes.lib`), `ut_types/Types.Unit.Tests.vcxproj`.                              | aucune (que til + std).                                               | Types utilitaires partagés : `Viewport.cpp` (rect typé en cells), `convert.cpp` (UTF-8↔UTF-16), `CodepointWidthDetector.cpp` (largeur Unicode + grapheme clusters via `unicode_width_overrides.xml` régénéré par `Generate-CodepointWidthsFromUCD.ps1`), `GlyphWidth.cpp`, `ColorFix.cpp`, `colorTable.cpp` (palettes 16-color + 256 XTerm + custom), `sgrStack.cpp` (XTPUSHSGR/XTPOPSGR), `ThemeUtils.cpp`, UIA helpers (`ScreenInfoUiaProviderBase.cpp`, `TermControlUiaProvider.cpp`, `TermControlUiaTextRange.cpp`, `UiaTextRangeBase.cpp`, `UiaTracing.cpp`).                                                                                                                                                  |
| `src/renderer/`              | `base/lib/base.vcxproj` (`ConRenderBase.lib`), `atlas/atlas.vcxproj` (`ConRenderAtlas.lib`), `gdi/lib/gdi.vcxproj` (`ConRenderGdi.lib`), `uia/lib/uia.vcxproj` (`ConRenderUia.lib`), `wddmcon/lib/wddmcon.vcxproj` (`wddmcon.lib`), `inc/` (`IRenderEngine.hpp`, `IRenderData.hpp`, `RenderSettings.hpp`, `Cluster.hpp`, `CSSLengthPercentage.h`, `FontInfo.hpp` etc.). | `types`, `buffer`.                                                | Pipeline de rendu. `base` est l'abstraction (transforme `IRenderData` en primitives `DrawString`/`DrawCursor`), `atlas` est le moteur DirectWrite/D2D/D3D11 avec cache de glyphes GPU (cf. `vendor/terminal/src/renderer/atlas/README.md`, schémas Mermaid), `gdi` le rendu GDI classique de conhost, `uia` le « rendu » virtuel pour UIA, `wddmcon` un rendu DXGK pour environnement de boot. AtlasEngine inclut des shaders HLSL (`shader_ps.hlsl`, `shader_vs.hlsl`, `custom_shader_{ps,vs}.hlsl`). |
| `src/interactivity/base/`    | `lib/InteractivityBase.vcxproj` (`ConInteractivityBaseLib.lib`).                                        | aucune directe (que les interfaces dans `inc/`).                      | Service locator + interfaces (`IConsoleControl`, `IConsoleInputThread`, `IConsoleWindow`, `IHighDpiApi`, `IInteractivityFactory`, `ISystemConfigurationProvider`, `IWindowMetrics`). Fichiers : `ApiDetector.cpp`, `EventSynthesis.cpp`, `HostSignalInputThread.cpp`, `InteractivityFactory.cpp`, `PseudoConsoleWindowAccessibilityProvider.cpp`, `RemoteConsoleControl.cpp`, `ServiceLocator.cpp`, `VtApiRedirection.cpp`.                                                                                                                                                                                                                                                                  |
| `src/interactivity/win32/`   | `lib/win32.LIB.vcxproj` (`ConInteractivityWin32Lib.lib`), `ut_interactivity_win32/Interactivity.Win32.UnitTests.vcxproj`. | `renderer/atlas`.                                       | Implémentation Win32 des interfaces ci-dessus. Fichiers : `Clipboard.cpp`, `ConsoleControl.cpp`, `ConsoleInputThread.cpp`, `ConsoleKeyInfo.cpp`, `Find.cpp` (popup de recherche), `Icon.cpp`, `Menu.cpp`, `screenInfoUiaProvider.cpp`, `SystemConfigurationProvider.cpp`, `uiaTextRange.cpp`, `Window.cpp`, `WindowDpiApi.cpp`, `WindowIo.cpp`, `WindowMetrics.cpp`, `WindowProc.cpp`, `windowUiaProvider.cpp`.                                                                                                                                                                                                                                                                                  |
| `src/interactivity/onecore/` | `lib/onecore.LIB.vcxproj` (`onecore.lib`).                                                              | `interactivity/base`.                                                 | Variante OneCore (Windows sans user32, IoT/HoloLens, etc.).                                                                                  |
| `src/propsheet/`             | `propsheet.vcxproj` (`console.dll`).                                                                    | `propslib`, `internal`.                                               | Property sheet « clic droit > Propriétés ». Fichiers : `console.cpp`, `globals.cpp`, `dbcs.cpp`, `dll.cpp`, `fontdlg.cpp`, `init.cpp`, `misc.cpp`, `preview.cpp`, `PropSheetHandler.cpp`, `OptionsPage.cpp`, `LayoutPage.cpp`, `ColorsPage.cpp`, `ColorControl.cpp`, `TerminalPropsheetPage.cpp`, `registry.cpp`, `util.cpp`.                                                                                                                                                                                                                                                                                                                                                                |
| `src/propslib/`              | `propslib.vcxproj` (`ConProps.lib`).                                                                    | aucune.                                                              | Sérialisation des prefs console dans HKCU + `.lnk` : `DelegationConfig.cpp`, `RegistrySerialization.cpp`, `ShortcutSerialization.cpp`, `TrueTypeFontList.cpp`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `src/tsf/`                   | `tsf.vcxproj` (`ConTSF.lib`).                                                                           | aucune (que win32 TSF).                                              | Bridge IME via Text Services Framework. Fichiers : `Handle.cpp`, `Implementation.cpp`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src/audio/midi/`            | `lib/midi.vcxproj` (`MidiAudio.lib`).                                                                   | `winmm.lib`.                                                          | DECPSO (commande de musique VT). `MidiAudio.cpp`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `src/cascadia/`              | (voir § 3.3)                                                                                            | (voir § 3.3)                                                          | Tout Terminal moderne (WinUI 2 + WinRT).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `src/tools/`                 | `benchcat`, `buffersize`, `closetest`, `ConsoleBench`, `ConsoleMonitor`, `echokey`, `fontlist`, `nihilist`, `RenderingTests`, `scratch`, `TerminalStress`, `U8U16Test`, `vtapp`, `vtpipeterm`, plus `ColorTool/` (.NET) et `GraphemeTableGen`, `GraphemeTestTableGen`, `ansi-color`, `lnkd`, `pixels`, `schemes-fragment`, `test`, `texttests`, `vttests`, `integrity`. | divers, surtout en console. | Utilitaires internes : bancs perf, générateurs de table Unicode, traceurs VT, tests visuels.                                                                                                                                                                                                                                                                                                                                                                                                                                          |

### 3.3 `src/cascadia/` (Terminal moderne)

Cf. `vendor/terminal/doc/ORGANIZATION.md` § *cascadia*. Sous-dossiers :

| Sous-dossier                              | `.vcxproj` / cible                                                              | Rôle                                                                                                                                       |
|-------------------------------------------|---------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------|
| `TerminalConnection/`                     | `TerminalConnection.vcxproj` (`Microsoft.Terminal.TerminalConnection.dll`)      | Abstractions de connexion : `ITerminalConnection.idl`, `ConptyConnection` (Win32 ConPTY), `AzureConnection` (Azure Cloud Shell), `EchoConnection` (debug), `BaseTerminalConnection.h`. Handoff inbound : `CTerminalHandoff.cpp`. |
| `TerminalCore/`                           | `lib/TerminalCore-lib.vcxproj` (`TerminalCore.lib`), `pch.*`, `terminalcore-common.vcxitems` | Classe `Microsoft::Terminal::Core::Terminal` qui compose buffer + parser + adapter + input sans rendu ni UI. Implémente `ITerminalApi`, `ITerminalInput`, `IRenderData` (cf. `Terminal.hpp`). `ICoreSettings.idl` (WinRT settings). |
| `TerminalControl/`                        | `dll/TerminalControl.vcxproj` (`Microsoft.Terminal.Control.dll`), `TerminalControlLib.vcxproj` | UI WinUI 2 du TermControl : `TermControl.xaml/.cpp/.h/.idl`, `ControlCore.cpp` (composition non-UI réutilisable), `ControlInteractivity.cpp`, `HwndTerminal.cpp` (variante Win32 pure, sans XAML), `SearchBoxControl.xaml`, `TermControlAutomationPeer.cpp`. |
| `TerminalApp/`                            | `TerminalAppLib.vcxproj`, `dll/TerminalApp.vcxproj` (`TerminalApp.dll`)         | L'application Terminal en WinUI 2 : `App.xaml`, `TerminalPage.xaml/.cpp`, `Tab.cpp`, `Pane.cpp`, `CommandPalette.xaml`, `SuggestionsControl.xaml`, `MinMaxCloseControl.xaml`, `MarkdownPaneContent.xaml`, `AboutDialog.xaml`, `AppLogic.cpp`, `AppCommandlineArgs.cpp` (parser CLI `wt`), `Jumplist.cpp`, `Toast.cpp`, `Remoting.cpp` (multi-instance via WinRT). |
| `TerminalSettingsModel/`                  | `Microsoft.Terminal.Settings.ModelLib.vcxproj`, `dll/Microsoft.Terminal.Settings.Model.vcxproj` | Modèle de settings JSON5 (héritage de profils), `profiles.schema.json`. |
| `TerminalSettingsEditor/`                 | `Microsoft.Terminal.Settings.Editor.vcxproj`                                    | Settings UI WinUI 2. |
| `TerminalSettingsAppAdapterLib/`          | `TerminalSettingsAppAdapterLib.vcxproj`                                         | Adaptateur entre l'App et le Settings Model. |
| `WindowsTerminal/`                        | `WindowsTerminal.vcxproj` (`WindowsTerminal.exe`)                               | Hôte Win32 + XAML islands : `AppHost.cpp`, `BaseWindow.h`, `IslandWindow.cpp` (XAML island host), `NonClientIslandWindow.cpp` (titlebar custom), `VirtualDesktopUtils.cpp` (extrait de PowerToys), `WindowEmperor.cpp` (multi-fenêtre), `icon.cpp`, `main.cpp`. |
| `WindowsTerminal_UIATests/`               | `WindowsTerminal.UIA.Tests.csproj`                                              | Tests UIA via Appium WebDriver. |
| `CascadiaPackage/`                        | `CascadiaPackage.wapproj`                                                       | MSIX packaging. `Package*.appxmanifest` (Dev/Preview/Canary/Release branding). |
| `ShellExtension/`                         | `WindowsTerminalShellExt.vcxproj` (`WindowsTerminalShellExt.dll`)               | Extension Explorer « Ouvrir dans Terminal ». |
| `ElevateShim/`                            | `elevate-shim.vcxproj` (`elevate-shim.exe`)                                     | Élévation UAC. |
| `Remoting/`                               | (pas de vcxproj, inclus dans TerminalApp)                                       | Resources WinRT du modèle d'inter-fenêtre. |
| `UIHelpers/`                              | `UIHelpers.vcxproj`                                                             | Utilitaires UI WinRT. |
| `UIMarkdown/`                             | `UIMarkdown.vcxproj`                                                            | Rendu Markdown WinUI (utilise cmark). |
| `WinRTUtils/`                             | `WinRTUtils.vcxproj`                                                            | Utilitaires WinRT divers. |
| `WpfTerminalControl/`                     | `WpfTerminalControl.csproj` (`Microsoft.Terminal.Wpf.dll`)                      | Wrapper WPF de `HwndTerminal`. |
| `WpfTerminalTestNetCore/`                 | `WpfTerminalTestNetCore.csproj`                                                 | Banc test .NET Core WPF. |
| `wt/`                                     | `wt.vcxproj` (`wt.exe`, `wtd.exe`)                                              | Shim de 36 lignes (`shim.cpp`) qui réécrit `argv[0]` et lance `WindowsTerminal.exe` via `CreateProcessW`. Astuce pour que `wt.exe` AppX-aliased apparaisse dans le PATH. |
| `fzf/`                                    | (inclus dans `TerminalApp`)                                                     | Fuzzy finder pour la palette de commandes (`fzf.cpp`, `fzf.h`, MIT). |
| `inc/`                                    | (header-only)                                                                   | `ControlProperties.h`, `cppwinrt_utils.h`. |
| `LocalTests_TerminalApp/`                 | `TerminalApp.LocalTests.vcxproj`, `TestHostApp/TestHostApp.vcxproj`             | TAEF locaux pour TerminalApp. |
| `UnitTests_Control/`                      | `Control.UnitTests.vcxproj`                                                     | Tests unit du TermControl. |
| `UnitTests_SettingsModel/`                | `SettingsModel.UnitTests.vcxproj`                                               | Tests unit du Settings Model. |
| `UnitTests_TerminalCore/`                 | `UnitTests.vcxproj`                                                             | Tests unit du TerminalCore. |
| `ut_app/`                                 | `TerminalApp.UnitTests.vcxproj`                                                 | Tests unit additionnels TerminalApp (`FzfTests.cpp`, `JsonUtilsTests.cpp`). |

Dépendances NuGet majeures du `cascadia/` (cf.
`vendor/terminal/dep/nuget/packages.config`) :

- `Microsoft.Windows.CppWinRT 2.0.250303.1` — code-gen C++/WinRT ;
- `Microsoft.UI.Xaml 2.8.4` — **WinUI 2** (pas 3), c'est-à-dire WinUI
  XAML Islands. Terminal n'est pas porté sur WinUI 3 ;
- `Microsoft.Web.WebView2 1.0.1661.34` ;
- `Microsoft.Windows.ImplementationLibrary 1.0.250325.1` — WIL ;
- `Microsoft.Internal.Windows.Terminal.ThemeHelpers 0.8.250811004` ;
- `Microsoft.Internal.PGO-Helpers.Cpp 0.2.34` (interne Microsoft).

---

## 4. Architecture en couches

```
                ┌──────────────────────────────┐
                │ Apps CLI / wWinMain / WinUI  │
                │  (cmd, bash, pwsh, vim, …    │
                │   ou WindowsTerminal.exe)    │
                └──────────────┬───────────────┘
                               │
                  appel API Win32 Console
                  (WriteConsoleA/W, ReadConsoleA/W,
                   GetConsoleScreenBufferInfo, …)
                               │
                               ▼
                ┌──────────────────────────────┐
                │ kernelbase.dll  (Console API)│
                └──────────────┬───────────────┘
                               │
                               │ NtDeviceIoControlFile
                               │ vers \Device\ConDrv
                               ▼
                ┌──────────────────────────────┐
                │ Driver console : condrv.sys  │
                │ (kernel-mode, hors ce repo)  │
                └──────────────┬───────────────┘
                               │
                               │ ALPC msg ring
                               ▼
   ┌───────────────────────────────────────────────────────┐
   │      conhost.exe  (= OpenConsole.exe en dev)         │
   │  - server/  : décode les API msg                     │
   │  - host/    : ApiDispatchers → ApiRoutines           │
   │  - buffer/  : TextBuffer, Row, OutputCell, …         │
   │  - terminal/parser/  : StateMachine VT (entrée+sortie)│
   │  - terminal/adapter/ : verbes VT → API console        │
   │  - terminal/input/   : clavier/souris → VT            │
   │  - renderer/         : rendu (GDI / Atlas / UIA / …)  │
   │  - interactivity/    : window proc, clipboard, IME    │
   └───────────┬──────────────────────────────┬────────────┘
               │                              │
               │ rendu GDI                    │ pipes UTF-8 + VT
               ▼                              │ (mode --headless)
   ┌─────────────────────┐                    │
   │ fenêtre conhost     │                    │
   │ (USER32 + GDI)      │                    │
   └─────────────────────┘                    │
                                              │
                                              ▼
                            ┌───────────────────────────────────┐
                            │  WindowsTerminal.exe              │
                            │  - cascadia/TerminalConnection :  │
                            │      ConptyConnection ↔ pipes     │
                            │  - cascadia/TerminalCore :        │
                            │      Terminal { Buffer + Parser   │
                            │      + Adapter + Input }          │
                            │  - cascadia/TerminalControl :     │
                            │      TermControl (WinUI 2 XAML)   │
                            │      + AtlasEngine (D3D11/D2D)    │
                            │  - cascadia/TerminalApp :         │
                            │      Tabs, Panes, CommandPalette  │
                            │  - cascadia/WindowsTerminal :     │
                            │      hôte Win32 + XAML islands    │
                            └───────────────────────────────────┘
```

### 4.1 Flux IPC

Trois canaux d'IPC coexistent :

1. **ALPC `\Device\ConDrv\Server`**. Pipe historique entre une
   application console et `conhost.exe`. Le serveur côté user-mode est
   `vendor/terminal/src/server/lib/` (`ApiMessage.cpp`, `ApiSorter.cpp`,
   `IoSorter.cpp`, `IoDispatchers.cpp`). Le « server handle » de
   `\Device\ConDrv` est créé par le driver `condrv.sys` (hors repo) ;
   `winconpty.cpp:CreateServerHandle` est un wrapper qui appelle
   `NtCreateFile` sur ce path.

2. **`\Device\ConDrv\Reference`**. Handle enfant du précédent, hérité
   par le processus client via `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`.
   Quand son refcount tombe à 0, conhost se ferme (voir le bloc de
   commentaire en haut de `vendor/terminal/src/winconpty/winconpty.h:_PseudoConsole`).

3. **Pipe anonyme « signal »** entre Terminal et conhost (créé par
   `_CreatePseudoConsole` via `CreatePipe`). Sert à envoyer des
   `PTY_SIGNAL_RESIZE_WINDOW`, `PTY_SIGNAL_CLEAR_WINDOW`,
   `PTY_SIGNAL_SHOWHIDE_WINDOW`, `PTY_SIGNAL_REPARENT_WINDOW` (cf.
   `vendor/terminal/src/winconpty/winconpty.h` lignes 44-49).

À noter pour `Cascadia` (Terminal moderne) :

- `TerminalConnection/ConptyConnection.cpp` instancie un ConPTY puis
  lit/écrit dans les deux pipes UTF-8 (`hInput`, `hOutput`).
- `TerminalCore/Terminal.cpp` traite ces flux via la `StateMachine`
  (depuis `terminal/parser`) qui appelle l'`AdaptDispatch` (depuis
  `terminal/adapter`) qui modifie le `TextBuffer` (depuis `buffer/out`).

### 4.2 Pipeline de rendu (AtlasEngine)

Cf. `vendor/terminal/src/renderer/atlas/README.md`. Schéma simplifié :

```
TermControl (WinUI 2 XAML)
        │ (DispatcherTimer ou VSync)
        ▼
Renderer (base/renderer.cpp)
        │ casse le buffer en "DrawString" / "FillBackground" / "DrawCursor"
        ▼
AtlasEngine (atlas/AtlasEngine.cpp)
        │ regroupe en DWRITE_GLYPH_RUNs
        │ split api.cpp (sous console lock) / r.cpp (hors lock)
        ▼
  ┌─────────────────────┬────────────────────┐
  ▼                     ▼                    │
BackendD2D           BackendD3D              │
(pur Direct2D,       (Direct3D 11 +          │
 fallback RDP /       glyph atlas GPU +      │
 vieux GPU)           HLSL shaders)          │
  │                     │                    │
  └────────┬────────────┘                    │
           ▼                                 │
     IDXGISwapChain                          │
           │                                 │
           ▼                                 │
     compositeur Windows  ◀──────────────────┘
                          (custom shaders)
```

### 4.3 Threads

`conhost.exe` (résumé d'après `src/host/`):

- **API thread** : reçoit les API messages depuis ConDrv via
  `IoSorter::ServiceIoOperation` ; appelle un `ApiDispatcher` qui
  modifie le buffer.
- **Input thread** : `ConsoleInputThread.cpp` lit les messages
  `WM_KEYDOWN`/`WM_INPUT` depuis la window proc puis pousse dans
  `inputBuffer`.
- **VT Input thread** : `VtInputThread.cpp` (`pVtInputThread`). Lit
  l'`hInput` pipe et délègue à la `StateMachine` (engine input).
- **Pty Signal thread** : `PtySignalInputThread.cpp` lit le signal pipe.
- **Render thread** : alimente le renderer engine actif (GDI ou Atlas).

`WindowsTerminal.exe` :

- **UI thread** (XAML) : WinUI 2 / dispatcher.
- **Render thread** : AtlasEngine.
- **Connection thread** : par TermControl, lit le pipe ConPTY UTF-8.

---

## 5. Composants réutilisables clés

Pour chaque composant : chemin du `.vcxproj`, headers exposés, sortie,
couplage Win32, exemple d'utilisation **issu du code réel** ou esquisse
FFI Rust.

### 5.1 `til` — Terminal Implementation Library

- **Chemin** : `vendor/terminal/src/til/` (lib unit-tests),
  `vendor/terminal/src/inc/til/*.h` (headers publics) + `src/inc/til.h`.
- **Sortie** : header-only (sauf `precomp.cpp` pour les unit tests qui
  produit `til.unit.tests.dll`).
- **Couplage Win32** : partiel. Plusieurs headers utilisent WIL et
  `wchar_t`, mais `at.h`, `small_vector.h`, `flat_set.h`,
  `rle.h`, `hash.h`, `bit.h`, `math.h`, `bytes.h`, `point.h`,
  `rect.h`, `size.h`, `static_map.h`, `enumset.h`, `generational.h`,
  `type_traits.h`, `coalesce.h`, `latch.h`, `mutex.h`, `pmr.h`,
  `ticket_lock.h`, `spsc.h`, `rand.h`, `replace.h`, `regex.h`,
  `u8u16convert.h`, `unicode.h`, `string.h`, `color.h`, `operators.h`
  sont en pratique du C++20 portable.
- **Exemple** : `til::small_vector<Injection, 8>` est utilisé par la
  `StateMachine` pour stocker les injections VT (cf.
  `vendor/terminal/src/terminal/parser/stateMachine.hpp:87`).
- **Idée FFI Rust** : pas nécessaire — équivalents directs en Rust
  (`smallvec`, `hashbrown`, etc.).

### 5.2 `vtparser` (`terminal/parser`)

- **Chemin** : `vendor/terminal/src/terminal/parser/lib/parser.vcxproj`.
- **Sortie** : `ConTermParser.lib` (statique).
- **Headers exposés** : `vendor/terminal/src/terminal/parser/stateMachine.hpp`
  (classe `Microsoft::Console::VirtualTerminal::StateMachine`),
  `IStateMachineEngine.hpp` (interface),
  `OutputStateMachineEngine.hpp`, `InputStateMachineEngine.hpp`,
  `base64.hpp`, `ascii.hpp`, `tracing.hpp`.
- **Couplage Win32** : non (`std::wstring_view`, til). Le seul lien à
  Win32 est via WIL pour les macros d'erreur.
- **API publique condensée** (extrait de `stateMachine.hpp`) :

  ```cpp
  namespace Microsoft::Console::VirtualTerminal {
      class StateMachine final {
      public:
          template<typename T>
          StateMachine(std::unique_ptr<T> engine) noexcept;
          void ProcessCharacter(const wchar_t wch);
          void ProcessString(const std::wstring_view string);
          void SetParserMode(const Mode mode, const bool enabled) noexcept;
          void InjectSequence(InjectionType type);
          const til::small_vector<Injection, 8>& GetInjections() const noexcept;
          void ResetState() noexcept;
          bool FlushToTerminal();
      };
  }
  ```

- **Idée FFI Rust** : wrapper `extern "C"` minimal autour de
  `StateMachine::ProcessString` avec callback C++ qui pousse les
  actions dispatched dans un canal vers Rust. Voir `INTEGRATION.md` §
  2.2.

### 5.3 `bufferout` (text buffer)

- **Chemin** : `vendor/terminal/src/buffer/out/lib/bufferout.vcxproj`.
- **Sortie** : `ConBufferOut.lib` (statique).
- **Headers exposés** : `vendor/terminal/src/buffer/out/textBuffer.hpp`
  (classe `TextBuffer`), `Row.hpp`, `cursor.h`, `OutputCell.hpp`,
  `OutputCellIterator.hpp`, `OutputCellRect.hpp`, `OutputCellView.hpp`,
  `TextAttribute.hpp` (SGR), `TextColor.h` (16/256/RGB),
  `LineRendition.hpp` (DECDHL/DECDWL), `ImageSlice.hpp` (Sixel),
  `Marks.hpp` (shell-integration), `DbcsAttribute.hpp` (CJK), `search.h`,
  `UTextAdapter.h` (intégration ICU), `textBufferCellIterator.hpp`,
  `textBufferTextIterator.hpp`.
- **Couplage Win32** : partiel (utilise WIL).
- **Exemple** : la classe `TextBuffer` (utilisée par toutes les
  consoles) est instanciée par `host/screenInfo.cpp` et par
  `cascadia/TerminalCore/Terminal.cpp`.

### 5.4 `winconpty` (ConPTY)

- **Chemin lib statique** : `vendor/terminal/src/winconpty/lib/winconptylib.vcxproj`
  → `conptylib.lib`.
- **Chemin DLL** : `vendor/terminal/src/winconpty/dll/winconptydll.vcxproj`
  → `conpty.dll` + `winconpty.def` (exports `ConptyCreatePseudoConsole`,
  `ConptyCreatePseudoConsoleAsUser`, `ConptyResizePseudoConsole`,
  `ConptyClosePseudoConsole`, `ConptyClearPseudoConsole`,
  `ConptyShowHidePseudoConsole`, `ConptyReparentPseudoConsole`,
  `ConptyReleasePseudoConsole`, `ConptyPackPseudoConsole` + alias compat
  `CreatePseudoConsole`, `ResizePseudoConsole`, `ClosePseudoConsole`,
  `ClearPseudoConsole`, `ReleasePseudoConsole`).
- **Header public** :
  `vendor/terminal/src/inc/conpty-static.h` (déclare les `Conpty*` sans
  `dllimport` pour usage en static-link), et
  `vendor/terminal/src/winconpty/winconpty.h` (struct interne
  `PseudoConsole { HANDLE hSignal; HANDLE hPtyReference; HANDLE hConPtyProcess; }`).
- **Couplage Win32** : total (`NtCreateFile`, `CreateProcessAsUserW`,
  `\Device\ConDrv\Server`, `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`).
- **Exemple d'utilisation officiel** :
  `vendor/terminal/samples/ConPTY/EchoCon/EchoCon/EchoCon.cpp` (extrait,
  vérifié) :

  ```cpp
  // Crée un ConPTY et y attache "ping localhost"
  HRESULT CreatePseudoConsoleAndPipes(HPCON* phPC, HANDLE* phPipeIn, HANDLE* phPipeOut)
  {
      HANDLE hPipePTYIn{ INVALID_HANDLE_VALUE };
      HANDLE hPipePTYOut{ INVALID_HANDLE_VALUE };
      if (CreatePipe(&hPipePTYIn, phPipeOut, NULL, 0) &&
          CreatePipe(phPipeIn, &hPipePTYOut, NULL, 0))
      {
          COORD consoleSize{};
          CONSOLE_SCREEN_BUFFER_INFO csbi{};
          GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &csbi);
          consoleSize.X = csbi.srWindow.Right - csbi.srWindow.Left + 1;
          consoleSize.Y = csbi.srWindow.Bottom - csbi.srWindow.Top + 1;
          return CreatePseudoConsole(consoleSize, hPipePTYIn, hPipePTYOut, 0, phPC);
      }
      return E_FAIL;
  }
  ```

- **Esquisse Rust** (avec `windows-rs`) :

  ```rust
  use windows::Win32::System::Console::{
      CreatePseudoConsole, ClosePseudoConsole, HPCON, COORD,
  };
  use windows::Win32::System::Pipes::CreatePipe;
  ```

### 5.5 `terminal/adapter`

- **Chemin** : `vendor/terminal/src/terminal/adapter/lib/adapter.vcxproj`.
- **Sortie** : `ConTermAdapt.lib` (statique).
- **Headers exposés** : `adaptDispatch.hpp` (`AdaptDispatch` :
  implémente `ITermDispatch`), `ITermDispatch.hpp`, `ITerminalApi.hpp`,
  `IInteractDispatch.hpp`, `InteractDispatch.hpp`, `DispatchTypes.hpp`,
  `FontBuffer.hpp`, `MacroBuffer.hpp`, `PageManager.hpp`,
  `SixelParser.hpp`, `terminalOutput.hpp`, `charsets.hpp`,
  `termDispatch.hpp`.
- **Couplage Win32** : partiel (via WIL).
- **Dépend** de : `types`, `terminal/input` (cf. `adapter.vcxproj`).

### 5.6 `terminal/input`

- **Chemin** : `vendor/terminal/src/terminal/input/lib/terminalinput.vcxproj`.
- **Sortie** : `TerminalInput.lib` (statique).
- **Headers** : `terminalInput.hpp` (classe `TerminalInput` qui produit
  des séquences VT à partir d'événements clavier/souris).
- **Couplage Win32** : partiel (uses `VK_*` constants).
- **Dépend** de : `types`.

### 5.7 `types`

- **Chemin** : `vendor/terminal/src/types/lib/types.vcxproj`.
- **Sortie** : `ConTypes.lib` (statique).
- **Headers** : `vendor/terminal/src/types/inc/`:
  `CodepointWidthDetector.hpp` (calcul largeur Unicode + graphemes),
  `Viewport.hpp` (rectangle en cellules), `ColorFix.hpp`,
  `colorTable.hpp`, `convert.hpp` (UTF-8 ↔ UTF-16),
  `GlyphWidth.hpp`, `IInputEvent.hpp`, `sgrStack.hpp`, `ThemeUtils.h`,
  `utils.hpp`.
- **Couplage Win32** : partiel.

### 5.8 `renderer` (résumé pour mémoire)

- **`renderer/base`** (`ConRenderBase.lib`) : pur C++, interface
  `IRenderEngine` + `Renderer` qui orchestre. Dépend de `types` et
  `buffer`.
- **`renderer/atlas`** (`ConRenderAtlas.lib`) : Direct3D 11 + Direct2D
  + DirectWrite + shaders HLSL. **Non portable.**
- **`renderer/gdi`** (`ConRenderGdi.lib`) : Win32 GDI. **Non portable.**
- **`renderer/uia`** (`ConRenderUia.lib`) : UI Automation. **Non
  portable.**

### 5.9 `interactivity`

- **`interactivity/base`** : `ServiceLocator` + interfaces. Utile pour
  comprendre l'architecture de plugin du conhost.
- **`interactivity/win32`** et **`interactivity/onecore`** :
  implémentations Win32 / OneCore.

### 5.10 `server` (ConDrv user-mode)

- **Chemin** : `vendor/terminal/src/server/lib/server.vcxproj` →
  `ConServer.lib`.
- **Headers** : `ApiMessage.h`, `ApiDispatchers.h`, `IApiRoutines.h`,
  `IoDispatchers.h`, `ConsoleShimPolicy.h`, `DeviceComm.h`,
  `DeviceHandle.h`, `ObjectHandle.h`, `ObjectHeader.h`, `ProcessHandle.h`,
  `ProcessList.h`, `ProcessPolicy.h`, `WaitBlock.h`, `WaitQueue.h`,
  `WaitTerminationReason.h`, `WinNTControl.h`.
- **Couplage Win32** : total (chargement dynamique de `ntdll.dll` pour
  les fonctions `Nt*` non publiques).
- **Intérêt pour nous** : **référence d'implémentation** du serveur
  console NT pour aider `google_os` à simuler un PTY cohérent côté
  Linux/POSIX.

---

## 6. Build et toolchain

(Détails complets dans `BUILD.md`. Résumé ici.)

### 6.1 Pipeline officiel

```powershell
Import-Module .\tools\OpenConsole.psm1
Set-MsbuildDevEnvironment
Invoke-OpenConsoleBuild
```

Pré-requis : VS 2022 (toolset v143), Windows SDK 10.0.22621.0,
PowerShell 7+, .NET Framework Targeting Pack.

### 6.2 Notre pipeline

```powershell
pwsh -File scripts/terminal/build.ps1
```

Forces `PlatformToolset=v145` + `WindowsTargetPlatformVersion=10.0.26100.0`
parce que la machine n'a que VS 2026 Insiders. Force `nuget-latest.exe`
parce que `dep/nuget/nuget.exe` (4.1) ne comprend pas `.slnx`.

### 6.3 Configurations

Définies dans `vendor/terminal/src/common.build.pre.props` :

- **Debug** : `_DEBUG;DBG`, optimisations off, link incrémental.
- **Release** : `NDEBUG`, `/O2 /Ot /GL` (WPO), COMDAT folding, `OPT:REF`.
- **AuditMode** : Release + CppCoreCheck + PREfast.
- **Fuzzing** : ASAN (`/fsanitize=address`) + coverage tracing
  (`/fsanitize-coverage=…`), CRT statique, ne supporte pas HybridCRT.

Plates-formes : `x64`, `x86` (alias `Win32` côté MSBuild), `ARM64`.
**Pas** de `Any CPU` pour les C++.

---

## 7. Tests

Framework : **TAEF** (Test Authoring and Execution Framework de
Microsoft). NuGet `Microsoft.Taef 10.100.251104001` (cf.
`dep/nuget/packages.config`). Runner : `te.exe`.

Doc upstream : `vendor/terminal/doc/TAEF.md`.

Scripts d'invocation :

- `vendor/terminal/tools/runut.cmd` : unit tests
- `vendor/terminal/tools/runft.cmd` : feature tests
- `vendor/terminal/tools/runuia.cmd` : UIA tests
- depuis PowerShell : `Invoke-OpenConsoleTests` (cf.
  `vendor/terminal/tools/OpenConsole.psm1` ligne 163).

Liste des binaires de tests (depuis `vendor/terminal/tools/tests.xml`) :

| Nom logique         | Type | Binaire                                              | Suites notables                                                                                          |
|---------------------|------|------------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| `host`              | unit | `Conhost.Unit.Tests.dll`                             | `AliasTests`, `ApiRoutinesTests`, `ClipboardTests`, `ConsoleArgumentsTests`, `HistoryTests`, `InitTests`, `InputBufferTests`, `ObjectTests`, `OutputCellIteratorTests`, `ScreenBufferTests`, `SearchTests`, `SelectionTests`, `TextBufferIteratorTests`, `TextBufferTests`, `TitleTests`, `UtilsTests`, `ViewportTests`, `VtIoTests`. |
| `textBuffer`        | unit | `TextBuffer.Unit.Tests.dll`                          | Buffer interne (Row, OutputCellIterator, textBufferTextIterator, search, attributs).                     |
| `terminalCore`      | unit | `UnitTests_TerminalCore\Terminal.Core.Unit.Tests.dll` | Classe `Microsoft::Terminal::Core::Terminal`.                                                            |
| `terminalApp`       | unit | `UnitTests_TerminalApp\Terminal.App.Unit.Tests.dll`  | Logique de l'App Terminal (sans XAML).                                                                   |
| `localTerminalApp`  | unit | `TestHostApp\TerminalApp.LocalTests.dll`             | Tests locaux UI TerminalApp dans un host XAML local.                                                     |
| `unitSettingsModel` | unit | `UnitTests_SettingsModel\SettingsModel.Unit.Tests.dll` (isolated TAEF) | Parser JSON + héritage de profils.                                                            |
| `unitControl`       | unit | `UnitTests_Control\Control.Unit.Tests.dll`           | TermControl + ControlCore.                                                                               |
| `interactivityWin32`| unit | `Conhost.Interactivity.Win32.Unit.Tests.dll`         | Window proc, clipboard.                                                                                  |
| `terminal`          | unit | `ConParser.Unit.Tests.dll`                           | StateMachine VT (`Base64Test`, `InputEngineTest`, `OutputEngineTest`, `StateMachineTest`).               |
| `adapter`           | unit | `ConAdapter.Unit.Tests.dll`                          | `AdaptDispatch`, SGR, charsets.                                                                          |
| `types`             | unit | `Types.Unit.Tests.dll`                               | `CodepointWidthDetector`, `Viewport`, `convert`, `colorTable`.                                           |
| `til`               | unit | `til.unit.tests.dll`                                 | Tous les headers `til/*.h`.                                                                              |
| `feature`           | ft   | `Conhost.Feature.Tests.dll`                          | API tests bout-en-bout (`API_Alias`, `API_Buffer`, `API_Cursor`, `API_Dimensions`, `API_File`, `API_FillOutput`, `API_Font`, `API_Input`, `API_Mode`, `API_MultipleInflightMessage`, `API_Output`, `API_Policy`, `API_RgbColor`, `API_Title`, `CJK_Dbcs`, `Canary`, `Message_KeyPress`). |
| `uia`               | ft   | `Conhost.UIA.Tests.dll` (C#)                         | UI Automation via Appium WebDriver.                                                                      |
| `winconpty`         | ft   | `winconpty.Feature.Tests.dll`                        | Smoke tests bout-en-bout du ConPTY.                                                                      |

Fuzzers : `vtparser/ft_fuzzer/VTCommandFuzzer.vcxproj`,
`host/ft_fuzzer/Host.FuzzWrapper.vcxproj`, ASAN activé en config
Fuzzing.

---

## 8. Politiques de code

Sources : `vendor/terminal/doc/STYLE.md`, `ORGANIZATION.md`,
`EXCEPTIONS.md`, `WIL.md`, `Niksa.md`, `virtual-dtors.md`.

### 8.1 Style

- Modern C++ pour tout code neuf (cf. `STYLE.md` : « Modern C++ … and
  reference the C++ Core Guidelines as much as you possibly can »).
- **WIL obligatoire** pour les appels Win32/NT (`wil::unique_handle`,
  `RETURN_IF_WIN32_BOOL_FALSE`, `THROW_IF_FAILED`, etc.).
- `HRESULT` préféré à `NTSTATUS`. Les fonctions retournant un code
  d'erreur doivent être `noexcept` et `[[nodiscard]]`.
- C++/WinRT : utiliser les `weak_ref` correctement, comprendre la
  concurrence cppwinrt (cf. `STYLE.md`).

### 8.2 Organisation

Règles de `vendor/terminal/doc/ORGANIZATION.md` :

- chaque projet a un sous-dossier `ut_<name>` (unit tests) ;
- les feature tests vont en `ft_<name>` ;
- les scripts de build par type de sortie : `/dll`, `/exe`, `/lib` ;
- les interfaces publiques vont dans `inc/` ;
- groupez les libs liées (ex. `terminal/parser` + `terminal/adapter`).

### 8.3 Exceptions

Cf. `vendor/terminal/doc/EXCEPTIONS.md` :

1. **Ne pas** laisser une exception fuir du code neuf vers le vieux
   code.
2. **Retourner** `HRESULT` (préféré) ou `NTSTATUS`.
3. **Encapsuler** tout comportement d'exception dans la classe qui
   l'utilise.
4. **Ne pas** introduire d'exceptions modernes dans le vieux code.
5. **Utiliser WIL** pour les facilités modernes non-throwing
   (`wil::make_unique_nothrow`, `wistd::unique_ptr`).

### 8.4 WIL — Windows Implementation Library

Cf. `vendor/terminal/doc/WIL.md`. Patterns :

- `wil::unique_handle` (auto-`CloseHandle`), `wil::unique_process_information`,
  `wil::unique_process_heap_string`, `wil::scope_exit` (RAII custom).
- `RETURN_IF_WIN32_BOOL_FALSE(call)` : wrap autour de calls Win32 qui
  retournent `BOOL`. Sur false → `RETURN_HR(HRESULT_FROM_WIN32(GetLastError()))`.
- `LOG_IF_*` : équivalent loggant qui continue.
- `wil::make_unique_nothrow<T>()` : `std::make_unique` sans exception.

### 8.5 Destructeurs virtuels pour interfaces

Cf. `vendor/terminal/doc/virtual-dtors.md`. Pattern strict :

```cpp
class IRenderData {
public:
    virtual ~IRenderData() = 0;
};
inline IRenderData::~IRenderData() {}
```

Définir le destructeur pur virtuel hors de la classe. Sans ça, des
segfaults occasionnels au destructeur (l'interface est appelée à la
place de la classe dérivée).

### 8.6 Niksa.md

Récap de longs commentaires de Dustin Howett et Mike « Niksa » Griese
sur :

- pourquoi on ne touche pas à `cmd.exe` (compat 30+ ans) ;
- pourquoi les perfs typing-to-screen sont exceptionnelles
  (`PolyTextOut` GDI direct, pas de framework) ;
- comment Win32 USER32/GDI32 sont stratifiés ;
- l'histoire « Far East » vs « Western » dans `_stream.cpp` ;
- pourquoi pas de mixed elevated/non-elevated tabs (faille de
  sécurité) ;
- différence shell vs terminal (cf. `Niksa.md#shell-vs-terminal`,
  reproduit dans `INTEGRATION.md` § 1.2).

### 8.7 Linting / formatting

- `clang-format` (config dans `.clang-format` à la racine, fourni par
  VS dans `packages/clang-format.win-x86.10.0.0/`). `Invoke-CodeFormat`
  reformate tout (cf. `OpenConsole.psm1` ligne 411).
- `XamlStyler` (`tools/Test-XamlFormat` + `Invoke-XamlFormat`).
- `clang-format` est imposé par la CI (`build/scripts/Invoke-FormattingCheck.ps1`).
- Treat warnings as errors (`common.build.pre.props` ligne 119).

---

## 9. Specs et roadmap

### 9.1 Specs `doc/specs/`

60+ documents Markdown ; sélection notable :

- `#1043 - Set the initial position of the Terminal`
- `#11000 - Marks` (shell integration)
- `#1142 - Keybinding Arguments`
- `#1235 - Azure cloud shell connector`
- `#12570 - Show Hide operations on GetConsoleWindow via PTY`
- `#13000 - In-process ConPTY`
- `#1337 - Per-Profile Tab Colors`
- `#1502 - Advanced Tab Switcher`
- `#1564 - Settings UI`
- `#1571 - New Tab Menu Customization`
- `#1595 - Suggestions UI`
- `#16599 - Quick Fix`
- `#1790 - Font features and axes-spec`
- `#2046 - Command Palette`, `#2046 - Unified keybindings…`
- `#2325 - Default Profile Settings`
- `#2563 - closeOnExit and TerminalConnection evolution`
- `#2871 - Pane Navigation`
- `#3062 - Appearance configuration object for profiles`
- `#4066 - Theme-controlled color scheme switch`
- `#4191 - Formatted Copy`
- `#492 - Default Terminal`
- `#4993 - Keyboard Selection`
- `#4999 - Improved keyboard handling in Conpty`
- `#5000 - Process Model 2.0`
- `#532 - Panes and Split Windows`
- `#597 - Tab Sizing`
- `#605 - Search`
- `#607 - Commandline Arguments for the Windows Terminal`
- `#653 - Quake Mode`
- `#6899 - Action IDs`, `#6900 - Actions Page`
- `#7335 - Console Allocation Policy`
- `#754 - Cascading Default Settings`
- `#8324 - Application State (TSM)`
- `#885 - Terminal Settings Model`
- `#976 - VT52 escape sequences`
- `#980 - SnapOnOutput`
- `Keybindings-spec.md`
- `Proto extensions-spec.md`
- `TerminalSettings-spec.md`
- `portable-mode-spec.md`
- `settings-spec-template.md`, `spec-template.md`

Brouillons dans `doc/specs/drafts/` :
`#1256 - Tab tearoff`, `#2634 - Broadcast Input`,
`#3327 - Application Theming`, `#642 - Buffer Exporting and Logging`,
`#997 Non-Terminal-Panes.md`, `576-ProfilesJumplistSpec.md`.

### 9.2 Roadmaps

- `doc/terminal-v1-roadmap.md`, `doc/terminal-v2-roadmap.md` :
  feuilles de route historiques (v1 = 2019-2020, v2 = 2020-2021).
- `doc/roadmap-2022.md` : milestones 1.13 → 1.18, planning des
  semesters 22H1 / 22H2.
- `doc/roadmap-2023.md` : feuille de route active la plus récente
  (la 2024+ semble ne pas avoir été poussée publiquement).

### 9.3 Feature flags (`doc/feature_flags.md`)

Système `til::feature` : `src/features.xml` génère, via
`tools/Generate-FeatureStagingHeader.ps1`, un header avec :

```cpp
class Feature_Xxx {
public:
    static bool IsEnabled();
};
#define TIL_FEATURE_XXX_ENABLED 1   // ou 0 selon la cible
```

Stages : `AlwaysEnabled` / `AlwaysDisabled`. Filtres par branche
(`alwaysDisabledBranchTokens`, `alwaysEnabledBranchTokens`) et par
branding (`Dev`, `Preview`, `Release`, `WindowsInbox`). Précédence :
`alwaysDisabledReleaseTokens` > branches enabled > branches disabled
(plus longue match gagne) > brandings enabled > brandings disabled >
défaut.

---

## 10. Licence et conformité

- **Licence** : MIT (`vendor/terminal/LICENSE`). Copyright Microsoft.
- **Notices** : `vendor/terminal/NOTICE.md`. Composants tiers :

  - `jsoncpp` (MIT) ;
  - `chromium/base/numerics` (BSD-3) ;
  - `{fmt}` (MIT + exception optionnelle) ;
  - `interval_tree` (MIT) ;
  - `pcg-cpp` (MIT) ;
  - `wyhash` (public domain) ;
  - `stb` (public domain) ;
  - `Oklab` (MIT) ;
  - `ColorBrewer` (Apache-2.0) ;
  - `cmark` (BSD-2 + parties MIT) ;
  - `fzf` (MIT) ;
  - `GSL` (MIT) ;
  - `Microsoft-UI-XAML` (MIT) ;
  - `VirtualDesktopUtils` (extrait de PowerToys, MIT) ;
  - `wil` (MIT).

- **Notice spéciale** : « Notwithstanding any other terms, you may
  reverse engineer this software to the extent required to debug
  changes to any libraries licensed under the GNU Lesser General Public
  License » (NOTICE.md). Aucun composant LGPL embarqué actuellement
  mais la clause est défensive.

---

## 11. Risques d'intégration

### 11.1 Portabilité

- `cascadia/*` → entièrement Win32 + WinUI 2 + DirectX. Portabilité
  Linux = **0**.
- `host/`, `interactivity/win32/`, `propsheet/`, `propslib/`, `tsf/`,
  `audio/midi/` → Win32 only.
- `renderer/atlas/`, `renderer/gdi/`, `renderer/uia/`,
  `renderer/wddmcon/` → Win32/DirectX/GDI only.
- `winconpty/` → utilise `\Device\ConDrv` qui n'existe pas hors
  Windows. À ré-implémenter sur `posix_openpt`/`forkpty` côté
  `google_os`.
- `vtparser`, `bufferout`, `types`, `terminal/adapter`,
  `terminal/input` → C++ portable en théorie (uses `wchar_t` 16-bit
  toutefois, ce qui pose problème sur Linux où `wchar_t` est 32-bit).

### 11.2 WinUI 2 (pas 3)

`Microsoft.UI.Xaml 2.8.4`. WinUI 2 est en **maintenance** et ne reçoit
plus de nouvelles features. Microsoft ne migrera pas Terminal sur WinUI
3 à court terme (cf. issues GitHub) car ça impliquerait de réécrire le
host XAML islands. Pour nous : pas d'avenir à investir dans une
intégration directe Cascadia.

### 11.3 Feed NuGet privé

`vendor/terminal/NuGet.Config` :

```xml
<add key="TerminalDependencies"
     value="https://pkgs.dev.azure.com/shine-oss/terminal/_packaging/TerminalDependencies%40Local/nuget/v3/index.json" />
```

C'est l'unique feed. Tous les paquets sont récupérés là, y compris :

- `Microsoft.UI.Xaml` (public mais épinglé) ;
- `Microsoft.Internal.PGO-Helpers.Cpp` (interne Microsoft) ;
- `Microsoft.Internal.Windows.Terminal.ThemeHelpers` (interne Microsoft) ;
- `Microsoft.MSBuildCache.*` (preview public).

**Risque** : si ce feed disparaît ou bascule en privé restreint, le
build casse. Le projet pourrait avoir besoin d'un mirror local
(`dep/packages/` activable via `NuGet.Config` § « Static Package
Dependencies »).

### 11.4 Poids du build

- Toolchain VS 2026 + SDK 26100 + .NET 10 ≈ 25 Go disque.
- vcpkg installed (`obj/x64/vcpkg/`) ≈ 1-2 Go par config.
- `packages/` NuGet ≈ 500 Mo.
- `bin/x64/Release/` ≈ 200 Mo.
- Premier build complet (`-Project ""`) ≈ 30-60 minutes sur 8 cores ;
  build incremental d'un module ≈ 10-60 s.

### 11.5 PGO

`Microsoft.Internal.PGO-Helpers.Cpp 0.2.34` est interne Microsoft. Tant
qu'on désactive PGO (`-Project Conhost\Host_EXE` n'active pas PGO ; la
prop `PgoTarget=true` n'est utilisée que dans le pipeline Microsoft
officiel), pas de blocage.

### 11.6 XAML islands + multi-monitor DPI

`WindowsTerminal.exe` utilise `Microsoft.UI.Xaml.Hosting.WindowsXamlManager`
pour héberger du XAML 2 dans un HWND Win32 classique. Cette stack
exige Windows 10 1903+ et impose des contraintes DPI fortes
(`NonClientIslandWindow.cpp`). Non transposable.

---

## 12. Conclusion : ce qu'on garde pour `google_os`

Mapping explicite **Terminal → aphrody** :

| Composant Terminal              | Use-case `aphrody`                                                                                  | Action concrète                                                                                                          |
|---------------------------------|--------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
| `vtparser` (`ConTermParser.lib`)| Émulateur de terminal pour notre futur PTY POSIX dans `google_os` (côté Linux) et bridge interactif dans `crates/cli/`. | Linker en static, exposer via `extern "C"` minimal dans une future crate `terminal_ffi`.                                  |
| `bufferout` (`ConBufferOut.lib`)| Modèle de référence pour `crates/cli` (buffer interactif type REPL). On peut **soit** linker, **soit** ré-implémenter en Rust sur le même modèle. | Étudier le modèle `Row`/`TextBuffer`/`OutputCell` pour s'inspirer. Implémentation full-Rust préférable à terme.            |
| `winconpty` (`conptylib.lib` ou `conpty.dll`) | Couche ConPTY pour spawn de `cmd`/`bash`/`pwsh` depuis nos crates Rust. | Sur Windows : utiliser `kernel32!CreatePseudoConsole` directement (équivalent), ou linker `conpty.dll` open-source pour bénéficier des fixes récents. |
| `terminal/adapter` (`ConTermAdapt.lib`) | Référence pour traduire VT → API console lors de l'émulation conhost dans `google_os`. | Référence + tests TAEF à étudier ; ré-implémentation Rust à terme.                                                       |
| `terminal/input` (`TerminalInput.lib`) | Encodage clavier/souris VT pour notre REPL Rust (envoyer du « \e[A » sur flèche haut, par exemple). | Réimplémenter en Rust (algorithme trivial) ou wrapper FFI minimal.                                                       |
| `types` (`ConTypes.lib`)        | `CodepointWidthDetector` (largeur Unicode) très utile. | Soit linker `ConTypes.lib`, soit utiliser la crate Rust `unicode-width` + grapheme. La crate Rust est probablement suffisante. |
| `server` (`ConServer.lib`)      | **Référence** pour notre émulateur de protocole ConDrv si on porte un userland POSIX qui s'attend à parler à conhost (peu probable mais possible). | Référence d'algorithme uniquement.                                                                                       |
| `til` (header-only)             | Patterns C++20 (`small_vector`, `flat_set`, `generational`, etc.). | Pas nécessaire en Rust pur (équivalents `smallvec`, `hashbrown`).                                                        |
| `host` (`ConhostV2Lib.lib`) + `OpenConsole.exe` | Référence d'implémentation complète de conhost. Très précieux pour l'équivalent côté `google_os`. | Référence pure. Pas de link.                                                                                             |
| `cascadia/wt/` (`wt.exe` shim)  | Modèle minimal pour notre propre alias AppX si on package un jour. | Référence (36 lignes de C++).                                                                                            |

### Ce qu'on **n'utilisera pas** :

- toute la pile **Cascadia/WinUI** (`TerminalApp`, `TerminalControl`,
  `TerminalSettingsModel`, `TerminalSettingsEditor`, `WindowsTerminal`,
  `CascadiaPackage`, `ShellExtension`, `WpfTerminalControl`,
  `Remoting`) : ce sont des couches UI Win32-only et WinUI 2 que nous
  n'avons pas vocation à reproduire dans un userland POSIX ;
- les **renderers** (`atlas`, `gdi`, `uia`, `wddmcon`) : trop liés à
  DirectX/GDI ;
- `propsheet`, `propslib`, `tsf`, `audio/midi`, `interactivity/win32`,
  `interactivity/onecore` : Win32-only et hors scope ;
- `colortool` (.NET), tous les `tools/*` (benchcat, scratch, etc.).

### Verdict global

L'unique investissement direct rentable est dans **un crate
`terminal_ffi` à venir** qui linke statiquement `vtparser.lib`,
`bufferout.lib`, `terminal/adapter.lib`, `terminal/input.lib`,
`types.lib`, plus éventuellement `winconpty.lib` côté hôte Windows.

Le reste est consommé en **lecture** : ce dépôt sert de référence
canonique pour comprendre comment Microsoft a résolu les problèmes
d'émulation VT, de buffer de texte, de codepage CJK, de Unicode width,
de Sixel, de signal pipe et de ConPTY. Cette connaissance alimentera
directement les modules `google_os::libc::io`, `google_os::libc::ipc`
et `google_os::libc::process` quand on devra émuler un PTY côté Linux
hôte.
</content>
</invoke>
