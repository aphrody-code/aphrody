# Gemini CLI Fork - Known Issues & Roadmap

Ce document recense tous les bugs et problèmes connus de la **Gemini CLI** officielle (`@google/gemini-cli`), en particulier lorsqu'elle est exécutée sur des environnements modernes comme **Node.js 26** ou le runtime **Bun**. 

L'objectif de ce fork interne est de corriger ces limitations pour garantir l'intégration de la CLI avec notre infrastructure `aphrody`.

## 🐛 Bugs Connus : Runtime BUN

La CLI officielle a été conçue historiquement pour Node.js et `npm`. Lorsqu'elle est lancée avec **Bun**, plusieurs problèmes architecturaux émergent :

1. **Path Detection & Updates**
   - *Problème* : L'outil `installationInfo.ts` contient une logique codée en dur qui détecte `npm` par défaut. Si la CLI est installée via `bun add -g`, elle ne parvient pas à se mettre à jour ou crashe, car elle ne trouve pas les chemins globaux de Bun.
2. **Module Resolution (WASM)**
   - *Problème* : Lors de la compilation ou de l'exécution avec Bun (`bun build` ou `bun run`), le resolveur de modules échoue sur les imports WebAssembly spécifiques à Node. (Ex: `import ... from './module.wasm?binary'`).
   - *Contournement actuel* : Patcher les fichiers `dist` pour retirer le suffixe `?binary` ou utiliser le flag de compatibilité.
3. **Syntax Errors & ESM**
   - *Problème* : Des erreurs de syntaxe type `SyntaxError: Export named 'resourceFromAttributes' not found` surviennent à cause des différences de résolution ESM (ECMAScript Modules) entre Bun et Node.js.

## 🐛 Bugs Connus : Node.js 26+

Bien que Node.js soit l'environnement natif de la CLI, le passage aux versions récentes (Node 20 à 26+) expose de nouveaux bugs :

1. **Crash du système de fichiers (ENOENT - scandir)**
   - *Problème* : La CLI tente de scanner le répertoire de connexion IDE (`AppData/Local/Temp/gemini/ide`) sans vérifier son existence au préalable via `fs.existsSync`. Cela lève une exception `ENOENT` fatale au démarrage. *(Fix implémenté localement en forçant la création du dossier).*
2. **Fuites mémoire (JavaScript heap out of memory)**
   - *Problème* : Lors du traitement de larges prompts ou de l'indexation de gros dépôts locaux (MCP), le process Node 26 dépasse la limite du tas V8 (V8 heap limit), entraînant un crash (`#2993`, `#2585`).
3. **Crash sur gros payloads MCP (Blob)**
   - *Problème* : Lorsque la CLI interroge des ressources via le Model Context Protocol (MCP) qui renvoient des données binaires massives (Blobs), le buffer Node crashe (Issue `#16369`).
4. **Interface utilisateur (Curses)**
   - *Problème* : L'affichage dans les terminaux récents sous Windows (Windows Terminal) souffre parfois de problèmes d'indentation lors de la création ou l'édition de fichiers via l'UI TUI (Text User Interface).

## 🛠️ Plan d'action pour le Fork

Pour que l'agent Antigravity et les autres outils puissent fonctionner sans interruption :
- [x] **Fix FS** : S'assurer que les dossiers temporaires existent (Contourné via mkdir).
- [ ] **Fix WASM** : Adapter le bundler pour gérer proprement les `.wasm` sous Bun et Node 26 (En attente de reproduction).
- [x] **Augmentation V8** : Modifier le script de lancement de la CLI (`gemini.cmd`) pour inclure le flag `--max-old-space-size=8192` afin d'éviter les `heap out of memory`.
- [x] **Bun Polyfills** : Remplacer l'appel strict à `npm` par une détection agnostique ou forcer `bun` dans les chunks compilés.

## 🗺️ Alignement avec la Roadmap Officielle

La roadmap officielle du dépôt *Google Gemini CLI* (suivie via l'Issue #4191) met l'accent sur les axes stratégiques suivants que nous devons anticiper dans notre fork :

1. **Background Agents & Autonomie** : L'équipe prévoit d'étendre la CLI avec des tâches autonomes de longue durée (*Background Agents*). Notre intégration sous `aphrody` doit garantir que l'environnement (Bun + Rust) ne tue pas prématurément ces daemons de fond et gère proprement les signaux OS.
2. **Écosystème MCP & Outils** : L'outillage (*Tooling*) et le support du **Model Context Protocol (MCP)** sont au cœur du développement. Nous devons nous assurer que nos serveurs locaux (`packages/google-mcp`) restent compatibles avec les implémentations natives de la CLI.
3. **Qualité & Benchmarks** : Les performances sont mesurées via des standards de l'industrie (SWE Bench, Terminal Bench). L'utilisation de notre fork accéléré par Bun et optimisé pour V8 Nightly devrait considérablement accroître notre score de performance par rapport aux releases officielles.
4. **Authentification** : Gestion sécurisée des API keys et du login *Gemini Code Assist*.
5. **Extensibilité** : Ouverture de la CLI à d'autres surfaces (comme GitHub).
