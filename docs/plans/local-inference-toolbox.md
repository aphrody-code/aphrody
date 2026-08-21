<!-- SPDX-License-Identifier: Apache-2.0 -->
# Plan — Toolbox d'inférence et de gestion de modèles locaux

> Cap défini le **2026-08-21**. Remplace le cap « Apex Autonomous Agent »
> ([`../PLAN.md`](../PLAN.md), révision 2026-05-19) comme priorité #1.
> Contexte d'ensemble : [`../SOURCE_OF_TRUTH.md`](../SOURCE_OF_TRUTH.md).

---

## 1. Cap

**`aphrody` devient un client et une toolbox pour l'inférence et la gestion de
modèles locaux**, orientés usages **programmatiques** — pas conversationnels :

| Usage cible | Entrée | Sortie |
|---|---|---|
| **OCR** | image, page scannée, PDF | texte structuré + boîtes |
| **Transcription visuelle** | screenshot, capture d'écran, image | description exploitable, VQA |
| **Transcription audio** | fichier / flux audio | texte + segments horodatés |
| **Tâches répétées** | planification (cron / intervalle / DAG) | jobs de fond, reprise, statut |

Le fil conducteur : **un agent ou un script demande un modèle par son nom, et
obtient un résultat** — sans clé d'API, sans réseau au moment de l'inférence,
sans intervention humaine (cf. [`../../CLAUDE.md`](../../CLAUDE.md) §0.1).

### Ce qui ne change pas

- **Rust primaire**, Edition 2024, nightly pinné (§2 de `CLAUDE.md`).
- **Ordre des plateformes** : Linux Ubuntu 26.04 → Windows 11 → WebAssembly.
- **Zéro stub** (§1) : chaque crate livre une fonctionnalité observable.
- Les surfaces existantes (A2A, MCP, terminal, chat, skills) restent en place ;
  elles deviennent des **consommatrices** de la toolbox plutôt que le cœur.

### Ce qui recule

Le cap précédent visait le meilleur **agent autonome conversationnel** (router
3-providers cloud, turn loop multi-canal, voice-to-voice). Ces briques restent
compilées et testées, mais ne sont plus le vecteur de valeur : la priorité passe
à l'exécution **locale** et **offline**.

---

## 2. Ce qui existait déjà (audit 2026-08-21)

| Brique | Crate | État |
|---|---|---|
| Embeddings locaux ONNX | `aphrody-embed` | ✅ shippé (fastembed / ONNX Runtime, cache `~/.aphrody/models`) |
| STT whisper.cpp | `aphrody-voice::stt::local_whisper` | 🚧 feature-gated `local-whisper`, sans feature → `NotImplemented` |
| Capture d'écran | `aphrody-capture` | ✅ shippé (GDI, Windows) |
| Planification | `aphrody-cron` | ✅ shippé (cron / intervalle / daily, store JSON) |
| Exécution DAG | `aphrody-task-runner` | ✅ shippé (dépendances, retries, timeouts, NDJSON) |
| Classification ONNX | `aphrody-re` (feature `magika`) | ✅ shippé |

**Le trou** : rien ne gérait le **cycle de vie des poids**. Chaque brique
téléchargeait à sa façon (ou pas du tout), sans catalogue, sans vérification
d'intégrité, sans éviction, et sans savoir sur quel accélérateur tourner.

---

## 3. Étape 1 — `aphrody-models` ✅ livré (2026-08-21)

Crate fondation : **résoudre, télécharger, vérifier, inspecter, classer,
évincer** les poids locaux. Aucun runtime d'inférence n'y est lié — les
backends demandent à ce crate *où sont les octets* et *sur quoi tourner*.

### Surface

| Module | Rôle | wasm32 |
|---|---|---|
| `id` | grammaire `ModelRef` : `hf:owner/repo/file@rev`, `https://…`, `file:…` + dérivation du chemin de cache | ✅ |
| `catalog` | catalogue curé embarqué (`catalog.json`), ids courts → artefacts épinglés + digests + profil d'accélération | ✅ |
| `accel` | sonde matérielle (GPU / VRAM / CUDA) et **classement du catalogue** pour cette machine | partiel |
| `inspect` | détection de format + parsing **manuel** des en-têtes GGUF / GGML whisper / safetensors / ONNX | ✅ |
| `render` | rendu d'un même modèle de table en **texte / JSON / Markdown / HTML / CSV** | ✅ |
| `digest` | SHA-256 one-shot, streaming, fichier ; normalisation `sha256:` | ✅ (partiel) |
| `store` | layout disque, `registry.json`, `verify`, `reconcile`, GC LRU | ❌ host-only |
| `fetch` | téléchargement streaming, reprise `Range:`, refus des octets au mauvais digest | ❌ host-only |

### Décisions de conception

- **L'unité suivie est le fichier, pas le dépôt.** Un backend charge un
  `.gguf`/`.onnx` concret ; c'est donc ce qui est haché, indexé et évincé.
- **Écriture atomique partout** : `.part` → `rename` seulement après le contrôle
  de digest, `registry.json` réécrit par fichier temporaire + `rename`. Un
  process tué ne laisse jamais un fichier tronqué qui a l'air installé.
- **La reprise reste vérifiable** : le préfixe déjà sur disque est ré-haché dans
  le digest courant avant l'ouverture de la socket, donc une reprise produit un
  SHA-256 de fichier entier. Un serveur qui ignore `Range` (200 au lieu de 206)
  redémarre le transfert au lieu d'ajouter sur un préfixe périmé.
- **Les en-têtes sont parsés à la main** sur curseur borné : aucune donnée de
  modèle n'est exécutée, chaque longueur lue est validée contre le buffer
  restant. Un fichier tronqué rend ce qui a été décodé + un `warning`.
  Cas réel traité : un `ModelProto` ONNX embarque tout son graphe dans le champ
  7, donc sur un préfixe de 1 MiB ce champ est **toujours** tronqué ; le parseur
  lit quand même l'en-tête du graphe visible et pose `graph_truncated`, pour que
  `opsets: []` se lise « hors préfixe » et non « absent du fichier ».
- **Le catalogue épingle des commits**, pas des branches : c'est ce qui rend les
  digests stables. Un test refuse toute révision qui n'est pas un sha de 40
  caractères hexadécimaux.
- **Les artefacts adoptés (`file:`) ne sont jamais supprimés** — ni par `rm`, ni
  par le GC : ces octets appartiennent à l'utilisateur, le store les référence.
- **Le token Hugging Face n'est envoyé qu'à `https://huggingface.co/`**, jamais
  à un hôte nommé dans une référence `url:`.
- **La sonde GPU shelle `nvidia-smi`** plutôt que de lier NVML : quatre nombres
  ne justifient pas une dépendance FFI versionnée sur trois plateformes, et un
  binaire absent signifie exactement « pas de CUDA ici ».

### Catalogue livré — 11 entrées

Épinglées et vérifiées contre l'API Hugging Face le 2026-08-21.

| Id | Tâche | Backend | Tier | Taille |
|---|---|---|---|---|
| `ppocr-v5-mobile` | ocr | ONNX Runtime | fast | **20.5 MiB** |
| `ppocr-v5-server` | ocr | ONNX Runtime | balanced | 164.8 MiB |
| `trocr-base-printed` | ocr | ONNX Runtime | balanced | 324.1 MiB |
| `dots-ocr` | ocr | llama.cpp | quality | 4.12 GiB |
| `florence2-base-ft` | visual-transcription | ONNX Runtime | balanced | 320 MiB |
| `granite-docling-258m` | visual-transcription | llama.cpp | balanced | 351 MiB |
| `smolvlm-500m-q8` | visual-transcription | llama.cpp | balanced | 520 MiB |
| `whisper-base-en` | speech-to-text | whisper.cpp | fast | 141 MiB |
| `whisper-small` | speech-to-text | whisper.cpp | balanced | 465 MiB |
| `whisper-large-v3-turbo-q5` | speech-to-text | whisper.cpp | quality | 547 MiB |
| `bge-small-en-v1.5` | text-embedding | ONNX Runtime | fast | 127 MiB |

### CLI livrée

```
aphrody model accel                           # GPU / VRAM / CUDA détectés
aphrody model recommend --task ocr --prefer fast [--pull]
aphrody model catalog [--task <t>]
aphrody model pull <spec> [--force]           # télécharge, reprend, vérifie
aphrody model list [--task <t>]
aphrody model info <spec>                     # en-tête décodé + digest
aphrody model verify <spec>                   # re-hash (exit != 0 si corrompu)
aphrody model rm <spec>
aphrody model gc --budget 4GiB                # éviction LRU + sweep des .part
aphrody model doctor                          # drift registre / disque
aphrody model adopt <path>                    # suit un fichier déjà présent
aphrody model path [<spec>]
```

`<spec>` est un id de catalogue **ou** une référence brute — même chemin de code.

**Contrat de sortie**, uniforme sur toutes les sous-commandes :
`--format text|json|markdown|html|csv` (alias `md`, `txt`, `htm`), `--json` en
raccourci, `--out <path>` pour écrire dans un fichier — le format est alors
déduit de l'extension si `--format` est absent. Le rapport part sur **stdout**,
la progression sur **stderr** : `aphrody model pull … --json | jq` fonctionne
pendant qu'un téléchargement de plusieurs gigaoctets défile.

### Validation réelle (pas seulement `cargo check`)

- **105 tests** unitaires + 1 doctest, `clippy -D warnings` avec `pedantic`
  activé au niveau du crate.
- `cargo check` OK sur `wasm32-unknown-unknown`.
- **Bout en bout sur cette machine** : `model pull ppocr-v5-mobile` télécharge
  les 4 artefacts (20.51 MiB), les digests correspondent aux valeurs épinglées,
  `model verify` repasse en `ok` sur les 4, et `model info` décode l'en-tête
  ONNX réel (`graph_name = "PaddlePaddle Graph in PIR mode"`, `ir_version = 6`).

---

## 4. Matériel de référence (poste de développement, 2026-08-21)

Relevé par `aphrody model accel` :

| Champ | Valeur |
|---|---|
| GPU | NVIDIA GeForce RTX 4070 |
| VRAM | 11.99 GiB (10.83 GiB libres au relevé) |
| Compute capability | 8.9 (Ada Lovelace) |
| Driver | 610.88 |
| Toolkit CUDA | 13.3 |
| Accélérateurs | `cuda`, `directml`, `cpu` |
| Threads CPU | 24 |

### Conséquence sur le choix de modèle — transcription d'image en masse

`aphrody model recommend --task ocr --prefer fast` classe, sur cette machine :

1. **`ppocr-v5-mobile`** (20.5 MiB, CUDA) — **le choix pour le débit**. Détection
   DB + reconnaissance CRNN, aucune génération autorégressive : le débit est
   borné par la convolution, pas par la production de tokens. C'est un ordre de
   grandeur au-dessus de n'importe quel VLM sur un lot d'images.
2. `ppocr-v5-server` (164.8 MiB, CUDA) — même architecture, ~18× les paramètres :
   nettement meilleur sur les petites polices, les tableaux denses et les scans
   dégradés, toujours sans décodage token par token.
3. `trocr-base-printed` — encodeur-décodeur, ligne de texte par ligne de texte.
4. `dots-ocr` (4.12 GiB, CUDA) — plafond de qualité, tient dans 12 GiB, mais à
   réserver au document difficile, pas au lot.

Pour du **document → markdown structuré** (tableaux, ordre de lecture),
`--task visual-transcription` place `granite-docling-258m` (351 MiB) devant :
258M paramètres seulement, donc le GPU reste alimenté au lieu d'attendre le
chargeur.

### Outillage tiers présent / absent sur ce poste

| Outil | État | Note |
|---|---|---|
| `uv` | ✅ présent | gestionnaire Python retenu (§2 de `CLAUDE.md`) |
| CUDA toolkit 13.3 | ✅ présent | `CUDA_PATH` renseigné |
| `hf` (Hugging Face CLI) | ❌ absent | **non requis** : `aphrody model pull` couvre le besoin, avec vérification de digest et reprise, ce que `hf download` ne garantit pas de la même façon |
| `ollama` | ❌ absent | entrée `PATH` périmée (`…\Programs\Ollama` n'existe plus) |
| `llama.cpp` | ❌ absent | entrée `PATH` périmée (`C:\tools\llama` n'existe plus) — à installer pour l'étape 2, backend `llama-cpp` |

---

## 4bis. Étape 2 — `aphrody-infer` ✅ livré (2026-08-21)

Backend d'inférence locale : **découverte du runtime**, **sélection de
l'execution provider**, **chargement de session** piloté par le catalogue.

### Les deux backends, et pourquoi ces deux-là

| Backend | Pour quoi | Intégration |
|---|---|---|
| **ONNX Runtime + CUDA EP** | tout le tier `onnx-runtime` du catalogue, dont **PP-OCRv5** — le chemin OCR de masse | **lié** via `ort` en `load-dynamic` |
| **llama.cpp CUDA** | le tier `llama-cpp` (dots.ocr, granite-docling, SmolVLM) | **piloté** en spawn des binaires upstream |

**vLLM a été écarté** : pas de support Windows natif (Linux + Python), et son
modèle de déploiement — un serveur d'inférence à héberger — ne correspond pas à
un CLI qui doit démarrer, traiter un lot et rendre la main.

**llama.cpp est piloté, pas lié.** Lier `llama-cpp-2` figerait un build d'un
projet C++ à évolution rapide dans le build d'aphrody, tirerait une toolchain
CUDA dans chaque compilation et se heurterait au `+crt-static` MSVC (§7).
Spawner le binaire de release garde le build hermétique et fait d'une montée de
version un téléchargement, pas une recompilation. C'est le même choix que
`gemini-runtime` pour le CLI Gemini.

### Runtime installé sur ce poste

| Composant | Version | Emplacement |
|---|---|---|
| ONNX Runtime GPU | **1.29.0, build `gpu_cuda13`** | `~/.aphrody/runtimes/onnxruntime-win-x64-gpu_cuda13-1.29.0/` |
| llama.cpp | **b10549, build `cuda-13.3`** | `~/.aphrody/runtimes/llama-b10549/` |

Le build `gpu_cuda13` a été choisi après vérification des imports de
`onnxruntime_providers_cuda.dll` (`llvm-readobj --coff-imports`) : il ne
dépend que de **`cublas64_13.dll` / `cublasLt64_13.dll`**, tous deux fournis par
le toolkit CUDA 13.3 déjà installé, et **n'importe pas cuDNN**. Aucune
installation supplémentaire n'a donc été nécessaire — c'est ce qui rend ce build
préférable au build `gpu_cuda12`, qui aurait exigé le runtime CUDA 12 *et*
cuDNN 9.

### Décisions de conception

- **`load-dynamic` obligatoire, pas un choix de confort** : aphrody force
  `+crt-static` sur MSVC alors que tout ONNX Runtime prébuilt est `/MD`. Lier
  statiquement les deux est irréconciliable (§7). La bibliothèque est donc
  chargée au runtime via `libloading`, ce qui rend aussi le build CUDA
  échangeable sans recompiler.
- **`download-binaries` est désactivé** : la feature d'`ort` récupère un build
  CPU-only par-dessus `native-tls`, alors qu'aphrody est rustls-only et veut le
  build CUDA. Le runtime est installé une fois sous `~/.aphrody/runtimes/` et
  découvert par `aphrody_infer::runtime::discover()`.
- **La feature `api-24` est obligatoire** dès qu'on met `default-features =
  false` sur `ort` : sans elle `ort-sys` retombe à l'API 17 et les modules EP
  d'`ort` ne compilent plus contre un `OrtApi` plus étroit. C'est le seul
  réglage non évident de la dépendance.
- **Le fallback n'est jamais silencieux** : `LoadedModel::provider` porte
  l'accélérateur réellement obtenu et `LoadedModel::fallbacks` liste ce qui a
  été refusé, avec la raison. Un « pipeline GPU » qui tourne en fait sur CPU est
  le mode d'échec classique de cette couche ; ici il est dans la valeur de
  retour, pas dans une ligne de log. `aphrody infer probe --require cuda` sort
  en code non nul si la session n'a pas eu CUDA.
- **La chaîne de providers vient de la sonde**, pas d'une constante : sur ce
  poste `cuda -> directml -> cpu`, sur un serveur Linux sans GPU elle se réduit
  à `cpu`.

### CLI livrée

```
aphrody infer runtime                     # quelle DLL ORT, d'où, et charge-t-elle
aphrody infer llama                       # binaires llama.cpp disponibles
aphrody infer probe <spec> [--role R] [--require cuda] [--provider P]
```

Toutes héritent du contrat `--format text|json|markdown|html|csv` + `--out`.

### Validation réelle sur cette machine

```
$ aphrody infer runtime
library    ~/.aphrody/runtimes/onnxruntime-win-x64-gpu_cuda13-1.29.0/lib/onnxruntime.dll
gpu build  true
loads      yes

$ aphrody infer probe ppocr-v5-mobile --role detector --require cuda
provider     cuda
accelerated  true
input        x: Tensor { ty: Float32, shape: [-1, 3, -1, -1] }
output       fetch_name_0: Tensor { ty: Float32, shape: [-1, 1, -1, -1] }
(exit 0, aucun fallback)

$ aphrody infer probe ppocr-v5-mobile --role recognizer --require cuda
provider     cuda
output       fetch_name_0: Tensor { ty: Float32, shape: [-1, -1, 18385] }

$ aphrody infer llama
4 tool(s) available   (llama-cli, llama-server, llama-bench, llama-mtmd-cli)
```

Les 18385 classes de sortie du recogniser correspondent au dictionnaire
multilingue PP-OCRv5 — le graphe chargé est bien le bon.

### Preuve de bout en bout : une image réellement transcrite

Chaîne complète, sur une image générée pour l'occasion (900×300, trois lignes
de texte) :

```
$ aphrody model pull granite-docling-258m
2 artefact(s), 350.90 MiB on disk        # poids + mmproj, digests vérifiés

$ aphrody model info granite-docling-258m --json | grep general.architecture
"general.architecture": "clip"           # en-tête GGUF décodé sur le vrai fichier

$ llama-mtmd-cli -m granite-docling-258M-Q8_0.gguf \
    --mmproj mmproj-granite-docling-258M-f16.gguf \
    --image ocr-test.png -p "Convert this page to docling." -ngl 99 --temp 0
```

Sortie du modèle :

```
<doctag><section_header_level_1><loc_17><loc_68><loc_308><loc_126>APHRODY LOCAL INFERENCE</section_header_level_1>
<text><loc_18><loc_185><loc_209><loc_252>Invoice 2026-08-21</text>
<text><loc_18><loc_302><loc_213><loc_366>Total: 1337.42 EUR</text>
</doctag>
```

Les trois lignes sont lues sans erreur, **avec leur rôle structurel**
(`section_header_level_1` vs `text`) et leurs boîtes englobantes. L'encodage
visuel a pris ~250 ms pour 19 chunks sur la RTX 4070 (`-ngl 99`, tout sur GPU).

C'est la démonstration du cap de bout en bout : recommandation → téléchargement
vérifié → inspection → chargement GPU → transcription structurée, sans clé
d'API et sans réseau au moment de l'inférence.

22 tests unitaires, `clippy -D warnings` avec `pedantic`.

---

## 4ter. Étape 3 — `aphrody-ocr` ✅ livré (2026-08-21)

Pipeline image → texte : pilote un modèle de vision local sur une image ou un
dossier entier, et rend du markdown — ou un verdict « pas de texte ».

### La distinction qui structure tout le crate

`PageText::None` est un **résultat de plein droit**, pas une erreur. Un modèle
de vision à qui l'on montre une illustration pleine page la décrira volontiers ;
enregistrer cette description comme transcription corrompt silencieusement un
corpus. Une planche dont tous les blocs décodés sont des images ou du mobilier
de page rend donc `None`, qu'un appelant transmet comme un null explicite.

### Surface

- `doctags` — parse le `DocTags` qu'émettent les modèles Docling, avec repli
  texte brut pour les modèles qui répondent en markdown ou en HTML.
- `vlm` — `VlmRunner` : un process par planche, sortie JSONL au fil de l'eau,
  reprise via `read_dir_filtered`.
- CLI : `aphrody ocr page <image>` et `aphrody ocr batch <dir> --out x.jsonl
  --skip-done` (feature `ocr`).

33 tests, `clippy -D warnings` avec `pedantic`.

### Première application réelle : le corpus databooks

Branché de bout en bout avec **shenron** le même jour — 11 513 planches de
databooks Dragon Ball à transcrire, GPU local d'un côté, API de l'autre.
Chaîne, choix de modèle mesuré, défauts trouvés en production et coût réel :
[`../databooks-transcription-bridge.md`](../databooks-transcription-bridge.md).

Trois défauts du pipeline ont été trouvés **par des planches réelles**, pas par
raisonnement : sortie sans balises jetée, boucle dégénérée du modèle prise pour
du texte, et remplissage (`4# 4# 4#`) confondu avec une transcription. Les trois
sont corrigés et couverts par des tests qui citent le cas observé.

---

## 5. Dépendances déclarées

Ajoutées à `[workspace.dependencies]` (déclaration seule : rien n'est lié tant
qu'un crate ne les active pas) :

| Crate | Version | Pourquoi cette version |
|---|---|---|
| `ort` | `=2.0.0-rc.12` | pin **imposé par fastembed** ; prendre rc.13 dupliquerait ONNX Runtime dans l'arbre et casserait `cargo deny` |
| `tokenizers` | `0.22` | idem — 0.23 existe mais fastembed force 0.22 |
| `ndarray` | `0.16` | idem — 0.17 existe mais fastembed force 0.16 |
| `fast_image_resize` | `6.1` | redimensionnement SIMD ; l'OCR de masse passe une part majeure de son temps mural dans le resize |
| `imageproc` | `0.27` | seuillage / morphologie / contours : transformer la heatmap d'un détecteur en boîtes de texte |

**Piège MSVC à respecter côté consommateur** (`CLAUDE.md` §7) : aphrody force
`+crt-static` alors que l'ONNX Runtime prébuilt est `/MD`. Un crate qui active
`ort` doit donc sélectionner `load-dynamic` (chargement `libloading` au runtime,
pas de lien statique), exactement comme `aphrody-embed`. La feature `cuda` n'est
volontairement **pas** activée au niveau workspace : chaque crate l'expose
derrière sa propre feature, pour qu'un build par défaut n'exige jamais de
toolkit CUDA. La sélection du provider au runtime est pilotée par
`aphrody_models::accel::probe()`.

---

## 6. Étapes suivantes

| # | Livrable | Dépend de | Contenu |
|---|---|---|---|
| **2** | `aphrody-infer` | étape 1 | trait `LocalBackend` + sessions ONNX Runtime (`ort`, EP CUDA) et GGUF ; résolution des rôles du catalogue vers des sessions chargées ; cache de sessions |
| ~~3~~ | ~~`aphrody-ocr`~~ | — | ✅ **livré** (§4ter) — via VLM GGUF. Le chemin PP-OCRv5 ONNX (détection DB + CRNN, `fast_image_resize` + `imageproc`) reste à faire : il serait ~20x plus rapide que le VLM pour du texte latin |
| **4** | `aphrody-vision` | étape 2 | transcription visuelle Florence-2 / granite-docling : caption, dense-region, OCR pleine page, VQA ; entrée depuis `aphrody-capture` |
| **5** | STT sans stub | étapes 1-2 | `local_whisper` cesse de renvoyer `NotImplemented` : les poids viennent du store, la feature devient un choix de backend, pas un interrupteur on/off |
| **6** | `aphrody-jobs` | étapes 1-4 | file persistante SQLite au-dessus de `aphrody-cron` + `aphrody-task-runner` : soumission, statut, reprise après crash, logs NDJSON, daemon |
| **7** | Surface MCP | étapes 3-6 | outils `aphrody_ocr`, `aphrody_describe_image`, `aphrody_transcribe`, `aphrody_job_submit` sur `aphrody-mcp` |

### Invariants à tenir sur toute la suite

1. **Aucune clé d'API requise** pour un chemin d'exécution local.
2. **Sortie structurée** (`--format`) sur chaque commande, exit code significatif.
3. **Pas de TTY requis** : tout est scriptable, cf. §0.1 de `CLAUDE.md`.
4. **Linux #1** reste bloquant pour merge.
5. **Vérifier avant d'inférer** : un job charge des octets dont le digest a été
   contrôlé, sinon il échoue au lieu de produire un résultat silencieusement faux.

---

## 7. Pièges relevés pendant l'étape 1

- **Cross-compile Linux depuis cette machine Windows** : `ring` (build C) exige
  `x86_64-linux-gnu-gcc`, absent → `cargo check --target x86_64-unknown-linux-gnu`
  échoue pour **tout** crate liant rustls (vérifié aussi sur `a2a-client-lf`).
  Environnemental, pas un défaut de code ; la validation Linux se fait sur le VPS
  (cf. [`../../DEPLOY.md`](../../DEPLOY.md)).
- **`#![forbid(unsafe_code)]` + `std::env::set_var`** : depuis l'Edition 2024,
  `set_var` est `unsafe`, donc intestable dans un crate qui interdit `unsafe`.
  Solution retenue : isoler la logique pure (`is_hub_url`, `first_usable_token`)
  et ne laisser que la lecture d'environnement dans un wrapper non testé.
- **`serde::Deserialize` + `&'static str`** : un champ `&'static str` dans une
  struct dérivant `Deserialize` produit une erreur de durée de vie (`'de` doit
  survivre à `'static`). Passer en `String`.
- **`Path::canonicalize` sur Windows** renvoie un chemin verbatim `\\?\C:\…`,
  qui fuitait dans le registre et dans les références affichées. `store::strip_unc`
  retire le préfixe, sauf pour un vrai partage `\\?\UNC\…` où il porte du sens.
- **Parser protobuf sur préfixe** : voir §3, `graph_truncated`.
