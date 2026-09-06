<!-- SPDX-License-Identifier: Apache-2.0 -->
# Plan — monorepo OCR local de bout en bout

> État de départ vérifié le 2026-09-06 : `aphrody-models` possède les artefacts
> PP-OCRv5, `aphrody-infer` charge des sessions ONNX Runtime avec fallback
> observable, et `aphrody-ocr` ne relie encore au CLI que les backends VLM
> llama.cpp. Ce plan rend le chemin OCR ONNX réellement disponible sans casser
> le pipeline VLM ni le contrat JSONL déjà consommé par les corpus.

## 1. Résultat cible

`aphrody ocr` devient une unique surface locale, offline et reproductible :

```text
input image/PDF
  -> inspection + normalisation
  -> routage de modèle explicite ou automatique
  -> PP-OCRv5 (rapide : polygones, texte, scores)
     ou VLM (qualité : structure Markdown)
  -> évaluation de confiance et règles de sûreté corpus
  -> résultat OCR versionné, JSONL atomique et reprenable
  -> audit, reprise ciblée et export Markdown/plain text
```

Les deux voies doivent rester distinctes : PP-OCR optimise le débit et la
géométrie, un VLM optimise la structure documentaire. Une sortie VLM ne doit
jamais inventer de score OCR, et une sortie PP-OCR ne doit jamais prétendre
reconstruire une structure qu'elle n'a pas détectée.

## 2. Topologie cible

```text
aphrody-models       artefacts épinglés et digests
aphrody-infer        sessions ONNX et providers, sans logique OCR
aphrody-ocr-core     contrat portable, JSONL, audit, nettoyage et policy
aphrody-ocr-vlm      llama-mtmd-cli / llama-server et DocTags
aphrody-ocr-onnx     PP-OCRv5 : image, DB, géométrie, CTC et scores
aphrody-ocr          façade et routeur, réexports de compatibilité
cli / google_mcp     adaptateurs d'interface minces
```

L'extraction en crates intervient après le contrat et ses golden tests : une
extraction avant ce point ne ferait que déplacer le comportement ambigu. Les
types de `aphrody-ocr-core` restent compilables sans runtime natif, afin de
former la surface WASM/WASI. Les backends sont strictement host-only.

## 3. Contrat stable

La première migration introduit `aphrody.ocr.result/v2`, sans changer le sens
des lignes JSONL historiques. Le lecteur accepte les lignes historiques ; les
nouveaux batchs n'émettent v2 qu'après le passage complet de la migration CLI.

Chaque résultat porte : identifiant immuable de l'entrée, chemin de diagnostic,
digest SHA-256 optionnel, backend, modèle, provider effectivement employé,
durée, statut, blocs et avertissements. Un bloc textuel porte son texte,
confiance optionnelle, polygone optionnel et rôle optionnel.

Les statuts sont exclusifs : `text`, `no-text`, `unreadable`, `needs-review`,
`processing-error`.
`no-text` signifie que le backend a terminé et n'a trouvé aucune zone de texte ;
`unreadable` signifie que l'image n'a pas pu être exploitée ; `failed` porte
l'erreur d'exécution. Aucun de ces trois états ne peut être déposé comme une
suppression de transcription sans option explicite du consommateur.

Le schéma porte aussi l'empreinte du modèle/configuration et l'identité de
l'image. Le chemin demeure la clef de compatibilité de reprise ; le hash est un
champ additionnel qui détecte une image remplacée. Toute ligne invalide hors
dernière ligne tronquée est une anomalie d'audit, pas une page sans texte.

## 4. Découpage d'implémentation

| Phase | Propriétaire de fichiers | Livrable vérifiable |
|---|---|---|
| A. Contrats | nouveau `aphrody-ocr-core` | types versionnés, conversions depuis `PageResult`, fixtures JSON |
| B. Images | nouveau `aphrody-ocr-onnx` | décodage par magic bytes, EXIF/orientation, resize/normalisation, polygones |
| C. PP-OCR | nouveau `aphrody-ocr-onnx` | PP-OCRv5 mobile sur image fixture, texte + scores + polygones |
| D. Routage | `aphrody-ocr` | sélection explicite et auto, escalade VLM seulement sur résultat faible |
| E. VLM | nouveau `aphrody-ocr-vlm` | adaptation au contrat V2 sans perte de markdown ni de sortie brute |
| F. Persistance | `aphrody-ocr-core`, CLI | JSONL append durable, manifeste de run, reprise par digest et pas seulement chemin |
| G. Qualité | `aphrody-ocr-core` | audits de confiance, de couverture, de boucles et de contradictions ; aucun filtre silencieux |
| H. Interfaces | `crates/cli/src/ocr_cmd.rs`, `crates/google_mcp` | CLI stable, JSON schema, outil MCP local en lecture seule |
| I. Livraison | workflows + packaging | release native avec feature OCR, `aphrody-mcp`, SBOM/digests et smoke d'archive extraite |
| J. Validation | fixtures + CI/docs | tests unitaires, intégration ONNX, smoke de binaire et matrice multi-plateforme |

## 5. Ordre de livraison

1. Ajouter les dépendances image/resize nécessaires et le contrat V1, sans
   modifier la syntaxe CLI existante.
2. Exécuter PP-OCRv5 mobile localement avec des fixtures synthétiques et les
   quatre artefacts catalogués ; tester CPU et exiger CUDA lorsque demandé.
3. Adapter `page` et `batch` pour écrire le contrat V1, tout en acceptant les
   JSONL historiques dans `audit` et `clean`.
4. Ajouter le routeur `--engine auto|ppocr|vlm` et `--model`, en conservant le
   comportement VLM actuel comme chemin explicite.
5. Ajouter les seuils, la révision sélective VLM et le manifeste de batch.
6. Exposer le schéma/outil MCP uniquement après le smoke CLI réel.
7. Mettre à jour documentation et exemples à partir des sorties mesurées,
   jamais à partir de chiffres supposés.

## 6. Invariants non négociables

- Aucun téléchargement implicite durant une commande OCR : les poids passent
  toujours par `aphrody model pull` et leurs digests épinglés.
- Aucun fallback accélérateur silencieux : le provider effectif et les refus
  restent dans le résultat.
- Aucun écrasement du JSONL d'entrée par défaut ; les réécritures passent par
  un fichier temporaire puis renommage atomique.
- Aucun filtre de découverte ne peut faire disparaître un candidat sans compteur
  et avertissement. Les règles de correction sont distinctes de l'observation.
- Aucune publication ou dépôt externe fait partie de cette refonte.
- Linux est validé avant Windows, et WASM garde une surface de types/schema sans
  prétendre exécuter ONNX Runtime natif.

## 7. Matrice de validation

| Niveau | Preuve requise |
|---|---|
| Types/parseurs | `cargo nextest run -p aphrody-ocr` |
| Feature OCR | `cargo check -p aphrody --features "ocr,infer,index,forensics,firefly"` |
| Qualité | fixtures latin, japonais, rotation, faible contraste, page sans texte, sortie VLM en boucle |
| Runtime ONNX | session detector + recognizer, métadonnées, CPU ; CUDA obligatoire quand `--require cuda` |
| CLI | binaire compilé avec `ocr`, `aphrody ocr --help`, `page`, `batch --skip-done`, `audit` |
| Cibles | Linux x86_64, Windows MSVC ; check wasm des types sans backend natif |
| Supply chain | `cargo ci-offline`, `cargo deny check` après les dépendances nouvelles |

## 8. Décisions différées, avec critères

- PDF : accepter seulement après un renderer local déterministe et une preuve
  que l'origine/dpi de chaque page est conservée.
- OCR manuscrit/manga : piste séparée ; il ne doit pas être présenté comme une
  capacité PP-OCR ou VLM document sans benchmark corpus.
- Tables/formules : conserver le VLM structurel tant qu'un extractor dédié ne
  fournit pas une sortie plus fiable mesurée.
- Service long-vivant : hors premier lot ; le CLI batch possédé par son parent
  reste le mode de référence.
