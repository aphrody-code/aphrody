# V8 Nightly / Canary (v15.0+) 🐤

Dans le cadre de la **Canary Channel Policy** (définie dans la gouvernance de `aphrody`), notre projet cible exclusivement **V8 Nightly / Canary**.

## 🎯 Pourquoi cibler V8 Nightly ?

Node.js standard accuse systématiquement un retard de plusieurs versions majeures (par exemple, Node 26.1 est sur V8 14.6, tandis que Canary est sur V8 15.0).
Ce retard signifie l'absence de :
1. Les dernières instructions JIT de **Maglev** et **TurboFan**.
2. L'implémentation native des brouillons ECMAScript (Stage 3/4) sans polyfills.
3. Les dernières optimisations WebAssembly (WasmGC, Memory64).

En nous calant sur **Electron Nightly** et **Chrome Canary**, nous utilisons le code `src/v8/` mis à jour quotidiennement.

## 🔬 Benchmark : Electron Nightly vs Rust + Bun

Le développement de notre solution Rust/Bun nécessite des benchmarks rigoureux.
**L'adversaire de notre stack Rust n'est pas le Node.js classique, mais bien le moteur V8 15.0 sous stéroïdes.**

### L'axe V8 (Electron)
*   **Moteur** : V8 v15.0 (compilation dynamique, Maglev ultra-rapide).
*   **Liaison C++** : Fortement couplé, mais les passages de frontières (boundary crossings) entre C++ et JS (via `v8::Isolate` et `v8::Context`) ont toujours un coût de marshaling.
*   **Forces** : Le pic de performance en exécution continue (throughput) grâce à TurboFan.

### L'axe JavaScriptCore (Bun + Rust)
*   **Moteur** : JavaScriptCore (JSC) de WebKit.
*   **Liaison Rust (FFI)** : Utilisation de primitives FFI brutes (C ABI) sans passer par l'API N-API massive de Node.
*   **Forces** : Démarrage à froid (cold-start) extrêmement rapide, consommation mémoire (RSS) bien inférieure, et appels natifs (FFI) quasi gratuits (quelques nanosecondes).

## 🚀 Récupérer la documentation V8 Nightly C++

Si vous développez des ponts natifs ou des add-ons C++ pour l'écosystème Canary :
Les APIs C++ (`v8::Value`, `v8::ObjectTemplate`, etc.) changent fréquemment dans la branche Nightly. 
Il est critique de générer ou de consulter les en-têtes directement depuis le checkout de Chromium :
```bash
# Depuis la racine de Chromium (sur le VPS ou machine locale)
cd src/v8/
# Les headers se trouvent ici :
ls include/
```
