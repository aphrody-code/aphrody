# Paysage outillage RE — Rust / Python / décompilation / inspection backend / Go

Recherche d'écosystème pour étoffer la pipeline RE d'aphrody (`aphrody re`, crate `aphrody-re`, pur Rust). Organisé par axe demandé. **Drapeau licence systématique** : aphrody est Apache-2.0 et bannit la GPL pour l'embarqué (cf. CLAUDE.md §7 — `unicorn-engine` GPL déjà banni). « EMBARQUABLE » = licence permissive (MIT/Apache/BSD), peut entrer dans le binaire ; « EXTERNE-ONLY » = GPL ou outil lourd → invoquer en sous-processus seulement ; « FFI » = pont C/C++.

## 1. « Ghidra full-CLI en Rust » — le plus proche
Il n'existe pas de portage complet de Ghidra en Rust, mais 3 niveaux d'approche :

| Projet | Quoi | Licence / usage |
|---|---|---|
| **`rsleigh`** (ShaneBreazeale/rsleigh) | **Le plus proche** : workspace pure-Rust SLEIGH — decoder/lifter multi-arch → P-code, **décompilateur C-like expérimental** + heuristiques malware. `rsleigh-cli` : list/disassemble/**decompile** des fonctions depuis PE/ELF/… | pure-Rust, EMBARQUABLE — candidat #1 pour une decomp native sans Java |
| **`jingle_sleigh`** / **`libsla`** (mnemonikr) | Bindings Rust idiomatiques à la lib **SLEIGH de Ghidra** (C++) pour disasm P-code ; `sleigh-compiler` compile les `.slaspec`. | FFI vers SLEIGH (Ghidra Apache-2.0) → propre, mais build C++ |
| **`sleigh-rs`** (rbran) | Parser SLEIGH 100% Rust (P-code). | pure-Rust mais **inachevé**, pas prod |
| **`rizin-rs`** | Bindings au framework **Rizin** (fork r2) — qui a un décompilateur (`rz-ghidra`/`jsdec`). | FFI/EXTERNE — Rizin est LGPL |
| **`falcon`** (crates.io/falcon) | Framework d'analyse binaire Rust : lifters ELF/PE (via goblin), IL Falcon, moteur point-fixe (dataflow/abstract interp), exécuteur concret. **Pas un décompilateur** mais la base pour en écrire un. | EMBARQUABLE |

**Reco aphrody** : `rsleigh` pour une voie decomp **native pure-Rust** (à évaluer/benchmarker) ; sinon déléguer à Ghidra headless (cf. `ghidra-aphrody-integration.md`). `falcon` pour l'analyse (CFG/dataflow/IL) si on veut dépasser le triage linéaire actuel.

## 2. Outils RE en Rust (EMBARQUABLES — déjà alignés avec aphrody-re)
- **`goblin`** (m4b) — parsing ELF/Mach-O/PE cross-platform. *(déjà utilisé par aphrody-re.)*
- **`yaxpeax-*`** (yaxpeax-arch/x86/arm/…) — décodeurs désasm multi-arch génériques, perfs ≈ capstone/bad64, ajout d'arch trivial. `yaxpeax-dis` = CLI désasm.
- **`iced-x86`** — désasm/encodeur x86-64 (formatter Intel). *(déjà utilisé par aphrody-re::disasm.)*
- **`falcon`** — IL + dataflow + symbolic (cf. §1).
- **`object`** + **`gimli`**/**`addr2line`** — formats objets + DWARF (symboles/lignes).
- **`symbolic`** (Sentry) — symbolication, debug-id, demangling.
- **`pdb`** (crate) — PDB Windows. **`rustc-demangle`/`cpp_demangle`/`msvc-demangler`** — démangling.
- À ÉVITER : `unicorn-engine` (émulation, **GPL banni**).

## 3. Outils RE en Python
- **`angr`** (BSD) — exécution symbolique/concolique, CFG, path exploration, vuln discovery. EMBARQUABLE côté Python.
- **`LIEF`** (Apache-2.0) — parse/modifie/assemble PE/ELF/Mach-O ; bindings Python solides.
- **`Triton`** (Apache-2.0) — DBA : moteur DSE symbolique, taint engine, AST.
- **`qiling`** (GPLv3 — EXTERNE-ONLY) — émulation/sandbox cross-OS (sur Unicorn).
- **`miasm`** (cea-sec, **GPLv2 — EXTERNE-ONLY**) — framework RE (asm/disasm multi-arch, IL, JIT emul, simplification/déobfuscation).
- **`capstone`/`unicorn`/`keystone`** bindings — disasm/emul/asm (Unicorn = GPL → externe).
- **`pwntools`** — exploitation/CTF. **`pyelftools`**, **`binwalk`** (firmware).
- **Go-specific** : `GoReSym` (CLI, cf. §5). 
- **Note pipeline aphrody** : le package `python/aphrody` peut wrapper angr/LIEF/Triton (permissifs) pour des passes que le Rust ne fait pas (symbolic, déobfuscation), sans contaminer le binaire Rust.

## 4. Décompilateurs / parsers pseudo-code
- **RetDec** (Avast, **MIT**) — décompilateur machine-code retargetable sur LLVM, sortie **C** + langage type-Python. Bindings : `retdec` (crate Rust, via REST), `retdec-python`. EMBARQUABLE (lib) ou via service.
- **Reko** (**GPLv2 — EXTERNE-ONLY**) — lit le binaire, infère les types, émet du **C structuré**.
- **dewolf** (Fraunhofer, en Python, sur Ghidra) — décompilateur orienté lisibilité.
- **Ghidra / Hex-Rays / Binary Ninja** — décompilateurs de référence (externes/commerciaux ; Binary Ninja a une API Rust).
- **Parsing du pseudo-code produit** : grammaires **tree-sitter C** (`tree-sitter`, `tree-sitter-c` — crates Rust EMBARQUABLES) pour parser/normaliser la sortie C de Ghidra/RetDec en AST exploitable.
- **`dogbolt.org`** (Decompiler Explorer) — compare dewolf/Ghidra/Hex-Rays/Reko/RetDec/Relyze sur un même binaire ; utile comme **oracle de référence** et API de comparaison.
- **Reco aphrody** : sortie decomp (Ghidra headless ou rsleigh) → parse via **tree-sitter-c** → AST structuré JSON exposé par `aphrody re`. RetDec (MIT) est l'option de décompilation embarquable la plus propre côté licence.

## 5. Inspection de backend via appels réseau
Directement pertinent pour l'axe Antigravity (le `language_server` parle **Connect/gRPC** vers `cloudcode-pa.googleapis.com` — cf. `antigravity-re-findings`).
- **`grpcurl`** (fullstorydev) — cURL pour gRPC : invoque des méthodes, browse le schéma via **server reflection**, fichiers proto, ou protosets.
- **`grpcui`** — UI web interactive sur reflection. **`buf curl`** (Buf) — client gRPC/Connect moderne. 
- **`connectrpc/grpcreflect-go`** + `bufbuild/connect-grpcreflect-go` — exposent la reflection compatible gRPC sur tout serveur Connect → `grpcurl`/`grpcui` marchent dessus. (NB : les backends Google ne l'exposent généralement PAS publiquement → s'appuyer sur les `.proto`/descripteurs extraits du binaire Go.)
- **`mitmproxy`** + **`mitmproxy-grpc`** — interception/MITM HTTP2+gRPC ; **Burp Suite** idem.
- **`protoscope`** (Google) — décode un wire-format protobuf brut sans schéma. **`protobuf-inspector`**.
- **`frida`** — instrumentation dynamique (hook des appels au runtime, dump des messages avant chiffrement TLS).
- **Reco aphrody** : un outil `inspect`/`probe` qui (a) reconstruit les descripteurs proto depuis le binaire Go (déjà à moitié fait via redress, cf. `go/antigravity-langserver-re`), (b) rejoue/observe les RPC Connect via `grpcurl`/`buf` + le token keyless (réutilise `antigravity-sdk`/`aphrody.vertex`), (c) `mitmproxy`/`frida` pour le dynamique. Pur-Rust possible côté client : `tonic` (gRPC) + `prost` (protobuf) + `prost-reflect` (reflection/descripteurs dynamiques) — tous EMBARQUABLES.

## 6. Patterns Go à exploiter
Structures internes du runtime Go récupérables sans symboles (cf. `antigravity-re-findings` : le `language_server` est un binaire Go strippé google3) :
- **`pclntab`** (Program Counter Line Table) — mappe adresse→nom de fonction + fichier + bornes ; pierre angulaire de la récupération de symboles. Layouts versionnés (pré-1.2, 1.2, 1.16, 1.18, 1.20, 1.2x).
- **`moduledata`** — table runtime : layout du fichier + GC + reflection ; point d'entrée vers `typelinks`.
- **`typelinks` / `rtype`** — récupération des **types** (structs/interfaces) et de leurs noms.
- **`itab`** — tables de méthodes interface↔type concret (résout les appels dynamiques).
- **Outils** : **`GoReSym`** (Mandiant, Apache-2.0) — parse pclntab/types/moduledata, basé sur la source runtime Go, multi-versions ; **`gore`** (`github.com/goretk/gore`, lib Go) + **`redress`** (CLI au-dessus de gore) — packages/types/source projection. *(redress a tout porté sur le language_server ; GoReSym a échoué sur le go1.27 interne non publié — cf. findings.)*
- **Patterns d'exploitation RE** : (1) localiser le magic pclntab → fonctions ; (2) suivre moduledata → typelinks → reconstruire structs/interfaces ; (3) `go version -m` → graphe de modules (vide si build blaze/google3) ; (4) recouvrement de strings Go (longueur-préfixées, pas NUL-terminées → `GoStringUngarbler`/heuristiques) ; (5) itab → dévirtualisation des appels d'interface.
- **Reco aphrody** : un détecteur Go natif dans `aphrody-re` (parse pclntab/moduledata en Rust pur via `goblin` — pas de dep GPL), exposé `aphrody re go <bin>` ; pour la reconstruction lourde, déléguer à `redress` (sous-processus) comme on délègue à Ghidra. Le module `go/antigravity-langserver-re` est le premier consommateur.

## Synthèse — quoi adopter pour la pipeline aphrody
1. **Decomp native** : évaluer **`rsleigh`** (pure-Rust SLEIGH+decomp) ; fallback Ghidra headless (doc séparé) ; **RetDec** (MIT) si decomp embarquée voulue.
2. **Analyse** : **`falcon`** (IL/CFG/dataflow) pour dépasser le triage linéaire ; **`yaxpeax`** pour le multi-arch.
3. **Pseudo-code** : parser la sortie C via **`tree-sitter-c`** → AST JSON.
4. **Backend** : client Connect/gRPC pur-Rust **`tonic`+`prost`+`prost-reflect`** + `grpcurl`/`mitmproxy`/`frida` en externe ; descripteurs proto issus du binaire Go.
5. **Go** : détecteur pclntab/moduledata pur-Rust dans `aphrody-re` (`aphrody re go`) + `redress` externe.
6. **Python** : wrappers `angr`/`LIEF`/`Triton` (permissifs) dans `python/aphrody` pour symbolic/déobfuscation.
**Garde-fou licence** : exclure de l'embarqué `miasm`, `qiling`, `unicorn`, `Reko` (GPL) — invocation sous-processus uniquement.

## Sources
- Rust BA / SLEIGH : rsleigh (github.com/ShaneBreazeale/rsleigh), jingle_sleigh/libsla (github.com/mnemonikr/libsla), sleigh-rs (github.com/rbran/sleigh-rs), falcon (crates.io/crates/falcon), yaxpeax (crates.io/crates/yaxpeax-arch), goblin (github.com/m4b/goblin), rizin-rs (crates.io/crates/rizin-rs), GhidRust (github.com/DMaroo/GhidRust), ReOxide (nlnet.nl/project/ReOxide).
- Décompilateurs : RetDec (github.com/avast/retdec) + retdec-rust (github.com/s3rvac/retdec-rust), Reko, dewolf, Decompiler Explorer (dogbolt.org).
- Python : angr, miasm (github.com/cea-sec/miasm), LIEF, Triton, qiling ; re-list (github.com/extremecoders-re/re-list).
- gRPC/backend : grpcurl (github.com/fullstorydev/grpcurl), connectrpc/grpcreflect-go, mitmproxy-grpc (github.com/aarnaut/mitmproxy-grpc), grpc.io/docs/guides/reflection.
- Go RE : GoReSym (github.com/mandiant/GoReSym), gore (pkg.go.dev/github.com/goretk/gore), Mandiant "Golang Internals and Symbol Recovery" (cloud.google.com/blog), CUJO AI "Reverse Engineering Go Binaries with Ghidra".
