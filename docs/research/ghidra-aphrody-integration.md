# Intégrer le ghidra-suite à la pipeline RE d'aphrody

Recherche d'intégration entre **`C:\src\ghidra`** (plugin Claude Code « ghidra-suite » : MCP live + wrapper headless + agents RE, posé sur une install Ghidra 12.x) et la pipeline RE native d'aphrody (`aphrody re`, crate `aphrody-re`, pur Rust).

## Cartographie des deux surfaces

| | aphrody `re` (natif) | ghidra-suite |
|---|---|---|
| Techno | Rust pur (goblin, iced-x86), aucun GPL, dans le binaire | Java 21 + Ghidra, externe (install `C:\src\ghidra`) |
| Vitesse | instantané (ms) | lent (JVM + auto-analysis, secondes→minutes) |
| Profondeur | triage, sections+entropie, strings, disasm linéaire, OAuth/endpoints (`re google`), magika | décompilation C, CFG, types, xrefs, symboles, decomp par fonction |
| Surfaces | `re {triage,strings,sections,disasm,google,classify}` | **headless** (`support/analyzeHeadless` + 6 scripts `Export*.java`) ; **live MCP** (`ghidra` → `http://localhost:8080/mcp`, 38 outils GUI) |

Conclusion : **complémentaires, pas concurrents.** `aphrody re` reste le premier passage rapide ; Ghidra fournit la décompilation profonde que le Rust pur ne fait pas. L'intégration doit déléguer à Ghidra en sous-processus (Java non embarquable, install externe) — exactement le modèle déjà utilisé par `aphrody agy` / `aphrody gemini` / `aphrody ide launch`.

## Reco — bridge en couches

### A. Headless : `aphrody re ghidra <binary>` (RECOMMANDÉ en premier — valeur max)
Nouvelle variante `ReAction::Ghidra` qui prolonge la pipeline `re` du triage vers la décompilation, en réutilisant le wrapper que le ghidra-suite ship déjà.
- **Résolution Ghidra** (cross-platform, première trouvée) : `$GHIDRA_INSTALL_DIR` > `$GHIDRA_HOME` > détection self-relative du plugin (cf. `find-ghidra.ps1` durci ce jour : walk-up vers `support/analyzeHeadless[.bat]`) > PATH. Sur cette machine → `C:\src\ghidra`.
- **Exécution** : spawn `analyzeHeadless <projDir> <projName> -import <bin> -scriptPath <ghidra_scripts> -postScript ExportDecompiled.java -deleteProject` (+ `ExportFunctions/ExportSymbols/ExportStrings/ExportCalls` selon flags). Les 6 scripts Java sont à `C:\src\ghidra\skills\ghidra-headless\scripts\ghidra_scripts/`.
- **Sortie** : JSON sur stdout (contrat scriptable §0.1) — fonctions décompilées, symboles, xrefs.
- **Latence (directive user)** : Ghidra headless est lent → cette commande est **opt-in explicite**, jamais dans un chemin par défaut. `re triage` (pur Rust, instantané) reste le défaut. Flags `--no-analysis`, `--timeout`, `--scripts a,b` forwardés ; un seul lancement JVM par binaire.
- **Coût** : ~120 lignes Rust dans un module `re_ghidra.rs` (spawn + résolution + parse), zéro nouvelle dep, zéro GPL (Ghidra = Apache-2.0, et on ne fait que l'invoquer).

### B. Live MCP : enregistrer `ghidra` dans la config MCP d'aphrody (trivial, complémentaire)
`aphrody mcp` lit `~/.config/aphrody/mcp.json`. Ajouter l'entrée :
```json
{ "mcpServers": { "ghidra": { "type": "http", "url": "http://localhost:8080/mcp" } } }
```
→ `aphrody mcp list` / `aphrody mcp call ghidra <tool>` pilotent les 38 outils du GUI Ghidra vivant (édition persistée, decomp interactive). **Zéro code Rust** — juste un doc + l'entrée de config. Couvre le cas interactif (binaire déjà chargé) là où A couvre le batch.

### C. Surface agents (optionnel, packaging)
Le plugin aphrody peut référencer les 8 agents RE du ghidra-suite (binary-triage, malware-analyst, vulnerability-hunter, decompile-cleaner, firmware-analyst, ctf-solver, ds-reverse, 3ds-reverse) pour délégation depuis `aphrody re`/`ide re`. Priorité basse.

## Chaînage cible dans la pipeline
1. `aphrody re triage <bin>` → format/sections/entropie (instantané) — déjà là.
2. `aphrody re google <bin>` / `ide re` → endpoints/OAuth (sidecars Go inclus) — déjà là.
3. **`aphrody re ghidra <bin>` → décompilation C + symboles (A, nouveau)** — quand le triage justifie l'analyse profonde.
4. `aphrody mcp call ghidra <tool>` → itération interactive sur le GUI (B).

## Décision
Implémenter **A** (haute valeur, prolonge `re`) puis **B** (config, trivial). C plus tard. A et B sont non-destructifs, opt-in, et respectent l'invariant latence (le défaut reste le Rust instantané).
