# `vendor/terminal` — Procédure de build

Document écrit le 2026-05-16, valide pour le commit
`8fe6c21ef88a73a7985b5968ee18936928ccac69` (cf. `README.md`).

## 1. Pré-requis effectifs sur la machine

| Composant                       | Version utilisée localement      | Source                            |
|---------------------------------|----------------------------------|-----------------------------------|
| OS                              | Windows 11 Home 10.0.28020       | machine de dev                    |
| Visual Studio                   | 2026 Community Insiders 18.7     | preview                           |
| MSVC toolset                    | v145 (cl 14.51)                  | composant VS Insiders             |
| Windows SDK                     | 10.0.26100.0                     | inclus avec VS Insiders           |
| .NET                            | 10.0.300                         | required pour `Microsoft.Taef`    |
| PowerShell                      | 7.6.1                            | requis par `OpenConsole.psm1`     |
| NuGet                           | 7.6.0.59 via `dep/nuget/nuget-latest.exe` | téléchargé par le wrapper ; le 4.1 embarqué dans le repo ne parse pas `.slnx` |
| MSBuild                         | 18.7.1 (VS 2026 Insiders)        | requis pour parser `.slnx` (cf. note ci-dessous) |

Pré-requis upstream officiels (cf. `vendor/terminal/README.md` §
*Prerequisites* et `vendor/terminal/doc/building.md`) :

- Windows 10 2004 (build >= 19041) ou ultérieur ;
- Developer Mode activé pour pouvoir déployer `CascadiaPackage` ;
- PowerShell 7+ ;
- Windows 11 SDK 10.0.22621.0 ;
- VS 2022 minimum ;
- workload « Desktop Development with C++ » + « Universal Windows
  Platform Development » ;
- composant individuel « C++ (v143) Universal Windows Platform Tools » ;
- .NET Framework Targeting Pack pour les projets de tests managés.

Le repo embarque ses dépendances natives via NuGet
(`vendor/terminal/dep/nuget/packages.config`) et vcpkg (manifest
`vendor/terminal/vcpkg.json`, baseline figée `15e5f3820f0370f1ba…`).
Les paquets vcpkg utilisés : `fmt 12.1.0`, `ms-gsl 3.1.0`, plus dans la
feature `terminal` : `jsoncpp 1.9.6`, `cli11 2.6.1`, `cmark 0.31.1`.

## 2. Procédure officielle Microsoft

Depuis PowerShell :

```powershell
Import-Module .\tools\OpenConsole.psm1
Set-MsbuildDevEnvironment
Invoke-OpenConsoleBuild
```

`Set-MsbuildDevEnvironment` (`vendor/terminal/tools/OpenConsole.psm1`)
utilise `VSSetup` + `Microsoft.VisualStudio.DevShell.dll` pour exporter
les variables d'environnement de `vcvarsall.bat` dans le shell courant.
Sans le flag `-Prerelease`, seules les installs stables de VS sont
considérées : VS 2026 Insiders est ignoré, d'où le wrapper local.

`Invoke-OpenConsoleBuild` (même fichier) appelle :

1. `nuget.exe restore OpenConsole.slnx` ;
2. `nuget.exe restore dep\nuget\packages.config` ;
3. `msbuild.exe OpenConsole.slnx @args` (où `@args` reçoit tout ce qu'on
   passe à la fonction PowerShell).

Depuis `cmd.exe` :

```cmd
.\tools\razzle.cmd
bcz
```

`razzle.cmd` est l'équivalent `cmd` de `Set-MsbuildDevEnvironment`. `bcz`
(`tools/bcz.cmd`) est l'alias clean + build.

## 3. Procédure utilisée chez nous

Le repo `microsoft/terminal` pin :

```xml
<PlatformToolset>v143</PlatformToolset>
<WindowsTargetPlatformVersion>10.0.22621.0</WindowsTargetPlatformVersion>
```

(`vendor/terminal/src/common.build.pre.props`, lignes 78 et 98). Or
notre machine ne possède **ni** le toolset v143, **ni** le SDK 22621 :
elle est equipée du toolset v145 et du SDK 10.0.26100.0 livrés avec VS
2026 Insiders. Il faut donc surcharger les deux variables au moment de
l'invocation MSBuild, sans modifier les `.props` upstream (lecture
seule). C'est exactement le rôle de
`scripts/terminal/build.ps1` (à la racine du repo, hors sous-module).

### 3.1 Anatomie de `scripts/terminal/build.ps1`

Le script (90 lignes) fait :

1. `Import-Module ./tools/OpenConsole.psm1 -Force` ;
2. `Set-MsbuildDevEnvironment -Prerelease` : ce flag pousse `VSSetup` à
   inclure les builds Insiders (le `-Prerelease` n'est utilisé que par
   ce wrapper, jamais par le script upstream) ;
3. force `$env:PlatformToolset = 'v145'` et
   `$env:WindowsTargetPlatformVersion = '10.0.26100.0'` ;
4. télécharge `nuget-latest.exe` depuis
   `https://dist.nuget.org/win-x86-commandline/latest/nuget.exe` s'il
   est absent (version 7.6.0.59 au moment de l'écriture). Raison : le
   `dep/nuget/nuget.exe` embarqué dans le repo est en version 4.1.x,
   antérieure au format `.slnx`, et plante avec « Invalid input
   'OpenConsole.slnx'. The file type was not recognized. » ;
5. exécute deux `nuget restore` (slnx + packages.config). Le restore
   `.slnx` est passé avec `-MSBuildPath "$env:VSINSTALLDIR\MSBuild\Current\Bin"` :
   `nuget.exe` n'a toujours pas de parser `.slnx` natif (issue
   NuGet/Home #14034 ouverte), mais quand on lui passe `-MSBuildPath`
   vers MSBuild 17.13+ (ici 18.7 / VS 2026), MSBuild s'occupe du parse ;
6. lance MSBuild avec :

```text
msbuild.exe OpenConsole.slnx
    /p:Configuration=<Debug|Release|AuditMode|Fuzzing>
    /p:Platform=<x64|x86|ARM64>
    /p:PlatformToolset=v145
    /p:WindowsTargetPlatformVersion=10.0.26100.0
    /p:AppxSymbolPackageEnabled=false
    /m /nologo /v:minimal
    [/t:<Project>]
```

Paramètres du wrapper :

| Paramètre        | Défaut             | Valeurs acceptées                    |
|------------------|--------------------|--------------------------------------|
| `-Project`       | `Conhost\Host_EXE` | nom de cible MSBuild, ou `""` (tout) |
| `-Configuration` | `Release`          | `Debug`, `Release`, `AuditMode`, `Fuzzing` |
| `-Platform`      | `x64`              | `x64`, `x86`, `ARM64`                |

La cible `Conhost\Host_EXE` est un *fast smoke test* qui ne reconstruit
que `OpenConsole.exe` (le `conhost.exe` local) et ses dépendances
directes. Pour tout construire :

```powershell
pwsh -File scripts/terminal/build.ps1 -Project ""
```

`/p:AppxSymbolPackageEnabled=false` désactive la génération du
`.appxsym` du packaging MSIX, qui exige des certificats que nous
n'avons pas.

### 3.2 Pourquoi écrire un wrapper plutôt que patcher les `.props`

`vendor/terminal/src/common.build.pre.props` est imposé par toutes les
`.vcxproj` du repo via :

```xml
<Import Project="$(SolutionDir)src\common.build.pre.props" />
```

Le modifier reviendrait à committer dans le sous-module et à diverger
de l'upstream. MSBuild laisse heureusement gagner toute variable passée
en `/p:` sur la valeur définie dans une `<PropertyGroup Label="Configuration">`,
ce qui est exactement le cas ici. La méthode est documentée par
Microsoft : voir
<https://learn.microsoft.com/en-us/cpp/build/reference/setting-additional-msbuild-properties>.

## 4. Configurations disponibles

Définies dans `vendor/terminal/src/common.build.pre.props` (lignes 187 →
269) et listées dans `vendor/terminal/OpenConsole.slnx` (`<BuildType>` :
`AuditMode`, `Debug`, `Fuzzing`, `Release`).

| Configuration | Particularités                                                                                       |
|---------------|------------------------------------------------------------------------------------------------------|
| `Debug`       | `_DEBUG;DBG`, optimisations désactivées (`/Od`), CRT debug, link incrémental, `DebugFastLink`        |
| `Release`     | `NDEBUG`, `/O2 /Ot /GL`, WPO, COMDAT folding, `/OPT:REF`, full PDB                                   |
| `AuditMode`   | identique à Release plus `CppCoreCheck` + `PREfast` (`/analyze`) via `src/StaticAnalysis.ruleset`    |
| `Fuzzing`     | `/fsanitize=address /fsanitize-coverage=…`, CRT statique, `libsancov.lib` + `clang_rt.asan_dynamic`, désactive HybridCRT |

Plateformes : `x64`, `x86` (= `Win32` côté MSBuild), `ARM64`. La
plate-forme `Any CPU` n'est pas supportée pour les projets C++ (voir
README upstream).

HybridCRT (`EnableHybridCRT`) est activé par défaut sauf en `Fuzzing` :
il fait disparaître la dépendance `vcruntime140.dll` en linkant
statiquement la STL et en réimportant les symboles `vcruntime` depuis
`ucrtbase.dll`. C'est pour ça que ConPTY peut tourner dans
`kernelbase.dll` sans dépendances DLL exotiques.

Commande type :

```powershell
# Release x64, tout le monde
pwsh -File scripts/terminal/build.ps1 -Project "" -Configuration Release -Platform x64

# Debug x64, juste OpenConsole.exe + ses libs (rapide)
pwsh -File scripts/terminal/build.ps1 -Configuration Debug

# Mode Fuzzing pour ASAN (utile sur le parser VT)
pwsh -File scripts/terminal/build.ps1 -Project "TerminalParser_FT_Fuzzer" -Configuration Fuzzing
```

## 5. Sorties

`vendor/terminal/src/common.build.pre.props` (lignes 5 → 24) impose :

```text
OutDir = $(SolutionDir)\bin\$(Platform)\$(Configuration)\
IntDir = $(SolutionDir)\obj\$(Platform)\$(Configuration)\$(ProjectName)\
```

Pour C++/WinRT, `OutDir` reçoit un suffixe `\$(ProjectName)\` pour ne
pas écraser les `.winmd` entre projets. Tous les exécutables et DLL
atterrissent donc dans `vendor/terminal/bin/<Platform>/<Configuration>/`.

Cibles installées pour `Conhost\Host_EXE` (chaîne typique) :

```
bin/x64/Release/OpenConsole.exe        (= conhost local)
bin/x64/Release/conptylib.lib          (statique, namespace winconpty.LIB)
bin/x64/Release/conpty.dll             (= winconpty.DLL)
bin/x64/Release/OpenConsoleProxy.dll   (interface IDL Console/Terminal Handoff)
bin/x64/Release/ConhostV2Lib.lib       (statique, hostlib)
bin/x64/Release/ConBufferOut.lib       (statique, bufferout)
bin/x64/Release/ConTermParser.lib      (statique, terminal/parser)
bin/x64/Release/ConTermAdapt.lib       (statique, terminal/adapter)
bin/x64/Release/TerminalInput.lib      (statique, terminal/input)
bin/x64/Release/ConTypes.lib           (statique, types)
bin/x64/Release/ConRenderBase.lib      (statique, renderer/base)
bin/x64/Release/ConRenderAtlas.lib     (statique, renderer/atlas, dépend de D3D11/D2D)
bin/x64/Release/ConRenderGdi.lib       (statique, renderer/gdi)
bin/x64/Release/ConRenderUia.lib       (statique, renderer/uia)
bin/x64/Release/ConServer.lib          (statique, server, IPC ConDrv)
bin/x64/Release/ConTSF.lib             (statique, Text Services Framework)
bin/x64/Release/ConInteractivityBaseLib.lib  (statique, interactivity/base)
bin/x64/Release/ConInteractivityWin32Lib.lib (statique, interactivity/win32)
bin/x64/Release/MidiAudio.lib          (statique, audio/midi)
bin/x64/Release/console.dll            (propsheet)
bin/x64/Release/ConProps.lib           (propslib)
```

Pour `CascadiaPackage` (target « tout Terminal moderne »), s'ajoutent :

```
bin/x64/Release/WindowsTerminal/WindowsTerminal.exe
bin/x64/Release/wt.exe                       (shim de redirection)
bin/x64/Release/wtd.exe                      (variante Dev branding)
bin/x64/Release/CascadiaPackage_*.msix       (paquet MSIX signé/non signé — requiert UAP patch sur wap-common.build.pre.props, cf. PATCHES.diff)
bin/x64/Release/WindowsTerminalShellExt.dll
bin/x64/Release/Microsoft.Terminal.Control.dll
bin/x64/Release/Microsoft.Terminal.Settings.Model.dll
bin/x64/Release/Microsoft.Terminal.Settings.Editor.dll
bin/x64/Release/TerminalApp.dll
bin/x64/Release/elevate-shim.exe
bin/x64/Release/UIHelpers.dll, UIMarkdown.dll, WinRTUtils.dll
```

## 6. Tests

Voir `vendor/terminal/doc/TAEF.md` et `vendor/terminal/tools/tests.xml`.

Lancer les tests unitaires depuis PowerShell après un build :

```powershell
Import-Module vendor/terminal/tools/OpenConsole.psm1
Invoke-OpenConsoleTests                # tous les unit tests x64 Debug
Invoke-OpenConsoleTests -Test til      # juste les unit tests TIL
Invoke-OpenConsoleTests -FTOnly        # tous les feature tests
Invoke-OpenConsoleTests -Test uia      # UI automation (déplace la souris)
```

`Invoke-OpenConsoleTests` charge `tools/tests.xml`, qui décrit chaque
binaire de test (`Conhost.Unit.Tests.dll`, `TextBuffer.Unit.Tests.dll`,
`til.unit.tests.dll`, `ConParser.Unit.Tests.dll`,
`ConAdapter.Unit.Tests.dll`, `Types.Unit.Tests.dll`,
`Terminal.Core.Unit.Tests.dll`, etc.). Chaque suite tourne via
`te.exe`, fourni par le NuGet `Microsoft.Taef 10.100.251104001`.

Variantes ligne de commande :

```cmd
.\tools\runut.cmd           :: unit tests
.\tools\runft.cmd           :: feature tests
.\tools\runuia.cmd          :: UIA tests
```

## 7. Troubleshooting (erreurs rencontrées)

### 7.1 « MSB8020 : The build tools for v143 cannot be found »

Cause : `common.build.pre.props` pin v143, mais la machine n'a que v145.
Solution : ajouter `/p:PlatformToolset=v145` (déjà géré par le wrapper).

### 7.2 « MSB4019 : The imported project … 10.0.22621.0 was not found »

Cause : SDK 22621 absent. Solution : `/p:WindowsTargetPlatformVersion=10.0.26100.0`
(géré par le wrapper).

### 7.3 « Unable to parse solution file 'OpenConsole.slnx' »

Cause : NuGet 4.1 embarqué dans `dep/nuget/nuget.exe` ne comprend pas
`.slnx`. **Subtilité** : `nuget.exe` lui-même n'a pas de parser `.slnx`
natif à ce jour (cf. issue NuGet/Home #14034, toujours ouverte mai
2026). Ce qui fait fonctionner notre pipeline, c'est l'argument
`-MSBuildPath "$env:VSINSTALLDIR\MSBuild\Current\Bin"` passé à
`nuget-latest.exe` : NuGet délègue alors le parse de `.slnx` à MSBuild
18.7 (VS 2026), qui le supporte nativement depuis MSBuild 17.13.

Solutions :
1. **Notre wrapper** : télécharge `nuget-latest.exe` (7.6.0.59) +
   `-MSBuildPath` vers MSBuild 18.7 — fonctionne.
2. **Alternative officielle .NET** (non utilisée ici car le restore
   doit aussi traiter `dep/nuget/packages.config` qui est l'ancien
   format) : `dotnet restore OpenConsole.slnx`, supporté depuis
   .NET SDK 9.0.200.

### 7.4 Warning bénin `vswhere.exe not found in PATH` au démarrage du shell

`vswhere` n'est nécessaire qu'à `Invoke-CodeFormat` (clang-format),
pas au build. Ignorable.

### 7.5 `DEP0700 : Registration of the app failed` au déploiement
`CascadiaPackage`

Cf. `vendor/terminal/doc/building.md` § *Are you seeing DEP0700* : le
`OpenConsoleProxy.dll` est verrouillé par une instance Terminal Dev
restée ouverte. Tuer les processus `WindowsTerminalDev.exe` puis
relancer le deploy.

### 7.6 vcpkg : `error: failed to download cmark`

Cause : `vcpkg.json` impose un baseline figé
(`15e5f3820f0370f1ba7150853762cec0688cd396`) qui peut bouger côté
upstream. Solution : `set VCPKG_BINARY_SOURCES=clear;` puis relancer.

### 7.7 `error LNK2019 unresolved external symbol __imp_CreatePseudoConsole`

Cause : on link `conptylib.lib` mais on inclut `<consoleapi.h>` qui
déclare les symboles comme `dllimport`. Solution : utiliser
`vendor/terminal/src/inc/conpty-static.h` qui redéclare les symboles
sans `dllimport`, ou linker `conpty.dll` à la place.

### 7.8 `error APPX3217 : UAP 10.0.22621.0 introuvable`

Cause : `vendor/terminal/src/wap-common.build.pre.props` hard-code
`TargetPlatformVersion=10.0.22621.0` sans condition, donc l'override
MSBuild `/p:TargetPlatformVersion=...` est écrasé. Solution : appliquer
le patch local qui ajoute `Condition="'$(TargetPlatformVersion)' == ''"`
sur cette ligne (cf. `PATCHES.diff`). Le script `scripts/terminal/build.ps1`
passe ensuite `/p:TargetPlatformVersion=10.0.26100.0`.

## 7bis. État du build vérifié (2026-05-16, machine de référence)

Après application de `PATCHES.diff` et exécution de
`scripts/terminal/build.ps1 -Project "" -Configuration Release -Platform x64`,
les artefacts suivants sont produits dans
`vendor/terminal/bin/x64/Release/` :

**Exécutables vérifiés**

| Binaire                                 | Origine                          |
|-----------------------------------------|----------------------------------|
| `OpenConsole.exe`                       | `src/host/exe/Host.EXE.vcxproj` (= `conhost.exe` local) |
| `OpenConsoleProxy.dll`                  | `src/host/proxy/Host.Proxy.vcxproj` |
| `conpty.dll`                            | `src/winconpty/dll/winconptydll.vcxproj` |
| `wt.exe`                                | shim de redirection vers WindowsTerminal |
| `WindowsTerminal/WindowsTerminal.exe`   | `src/cascadia/WindowsTerminal/WindowsTerminal.vcxproj` |
| 263 DLL + 56 EXE au total (incluant tests) | |

**Bibliothèques statiques utiles pour l'intégration `google_os`**

| Lib                              | Source                                  |
|----------------------------------|-----------------------------------------|
| `ConTermParser.lib`              | `src/terminal/parser/lib/`              |
| `ConBufferOut.lib`               | `src/buffer/out/lib/`                   |
| `conptylib.lib`                  | `src/winconpty/lib/`                    |
| `ConTermAdapt.lib`               | `src/terminal/adapter/lib/`             |
| `ConTypes.lib`                   | `src/types/lib/`                        |
| `ConServer.lib`                  | `src/server/lib/`                       |
| `ConhostV2Lib.lib`               | `src/host/lib/` (référence seulement)   |
| `ConRenderAtlas.lib`             | `src/renderer/atlas/`                   |
| `ConRenderBase.lib`              | `src/renderer/base/lib/`                |
| `ConInteractivityWin32Lib.lib`   | `src/interactivity/win32/lib/`          |
| `ConInt.lib`                     | `src/internal/`                         |

**Erreur résiduelle (non bloquante pour notre cas d'usage)**

`CascadiaPackage.wapproj` produit un `.msix` signé seulement si :
- patch UAP de `PATCHES.diff` appliqué (déjà fait sur la machine de référence) ;
- ET un certificat de code signing valide est installé.

Sans cert, le MSIX échoue mais `WindowsTerminal.exe` lui-même est
opérationnel et lançable directement (pas via Store).

## 8. Recettes utiles

### Recompiler uniquement un sous-projet

Tout `.vcxproj` du repo est buildable via son chemin solution :

```powershell
pwsh -File scripts/terminal/build.ps1 -Project "Conhost\TerminalParser"
pwsh -File scripts/terminal/build.ps1 -Project "Conhost\BufferOut"
pwsh -File scripts/terminal/build.ps1 -Project "Conpty\winconpty_LIB"
```

(Les noms exacts viennent des attributs `ProjectName` dans chaque
`.vcxproj`. La hiérarchie dans la sln correspond aux `<Folder Name="…">`
de `OpenConsole.slnx`.)

### Profile-Guided Optimization

Désactivée par défaut sur build externe. Activable via
`/p:PgoBuildType=Instrument` ou `Optimize`, en présence du NuGet
`Microsoft.Internal.PGO-Helpers.Cpp 0.2.34`. Ce paquet est interne
Microsoft et n'est pas redistribuable : le feed
`pkgs.dev.azure.com/shine-oss/terminal/_packaging/TerminalDependencies`
fait office de mirror public, mais sa pérennité n'est pas garantie pour
nous.

### Reformater le code

Depuis `Set-MsbuildDevEnvironment` :

```powershell
Invoke-CodeFormat        # clang-format + xstyler sur tout le repo
```

(Inutile dans notre setup : on ne committe pas dans le sous-module.)
</content>
</invoke>
