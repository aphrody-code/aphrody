<!-- SPDX-License-Identifier: Apache-2.0 -->
# Pont aphrody ↔ shenron — transcription des databooks

> Branché et vérifié de bout en bout le **2026-08-21**.
> Côté shenron : [`~/shenron/docs/prompt-transcription-databooks.md`](https://dragonballfr.com)
> et `~/shenron/docs/databooks-transcription.md` (VPS `dbfr`).
> Cap projet : [`plans/local-inference-toolbox.md`](plans/local-inference-toolbox.md).

---

## 1. Le problème

Le corpus databooks de `dragonballfr.com` compte **318 ouvrages et 11 513
planches** scannées, dont **11 277 sans transcription**. Les lire demande un
modèle de vision, et les planches sont en **japonais** : titres imprimés,
fiches techniques, postfaces, et beaucoup de bulles de manga.

Les deux moitiés du problème vivent sur deux machines :

| Où | Quoi |
|---|---|
| **VPS `dbfr`** (alias SSH ; hôte de shenron) | l'API, la base, et 5 Go de planches déjà exportées en 29 lots sous `~/databooks-ocr/` |
| **Poste local** | le GPU (RTX 4070, 12 Gio) et la toolbox aphrody |

Le pont fait circuler : planches → poste local → texte → API.

---

## 2. La chaîne

```
  dbfr:~/databooks-ocr/lot-NNN/          (400 planches + manifeste)
        │  scp
        ▼
  aphrody ocr batch <images> --model dots-ocr --out lot-NNN.jsonl --skip-done
        │  une ligne JSON par planche, écrite et flushée au fil de l'eau
        ▼
  scp lot-NNN.jsonl → dbfr
        │
        ▼
  bun scripts/depose-transcriptions.ts lot-NNN.jsonl
        │  POST mode:"merge", par paquets de 50, par ouvrage
        ▼
  https://dragonballfr.com/api/databooks/<id>/transcription
```

### Ce qui a été ajouté de chaque côté

| Côté | Livrable |
|---|---|
| aphrody | crate `aphrody-ocr` + commande `aphrody ocr page/batch` (`--features ocr`) |
| shenron | `apps/site/scripts/depose-transcriptions.ts` |

---

## 3. Choix du modèle — mesuré, pas supposé

Deux modèles du catalogue ont été essayés sur les mêmes planches réelles.

| Modèle | Vitesse | Sur du japonais imprimé | Sur des bulles de manga |
|---|---|---|---|
| `granite-docling-258m` | ~5 s | **non** — classe les bulles en `<picture>` | non |
| `dots-ocr` | ~6–12 s | **oui** — titres, crédits, ISBN, fiches, postfaces | non |

`granite-docling` reconnaît parfaitement la **mise en page** (il a lu
`ORIGINAL COLOR WORKS part1` et le folio `103`) mais ne lit pas les glyphes
japonais : il rend six `<picture>` et un pied de page. `dots-ocr` lit le
japonais imprimé de façon exploitable :

```
1-0001  鳥山朗ワールド DRAGON BALL 大全集 ① COMPLETE ILLUSTRATIONS
1-0004  構成 キャラメル・ママ  アートディレクター 田熊樹美  デザイン フライハイト …
2-0001  DRAGON BALL 大全集 2 STORY GUIDE  集英社 定価1800円 (税込) ISBN4-08-782752-6
2-0007  むかしむかし のこと 都から数千公里も 彼方のある山奥…
```

**Ce que ni l'un ni l'autre ne fait** : lire le texte manuscrit vertical des
bulles de manga. C'est hors domaine pour les deux — ce sont des modèles de
document, pas de manga. Les planches de manga pur ressortent donc « sans
texte », ce qui est la réponse honnête et non destructrice (cf. §5).

**Le prompt n'est pas interchangeable.** Un prompt Docling donné à dots.ocr
produit un tour vide — c'est-à-dire, vu d'en haut, une planche sans texte.
`aphrody_ocr::default_prompt` associe donc le prompt au modèle, et se trompe de
prompt est le mode d'échec le plus coûteux du pipeline.

---

## 4. Trois défauts trouvés en production, et leur correctif

Chacun a été observé sur des planches réelles, pas imaginé.

1. **Sortie sans balises jetée.** `dots-ocr` répond en markdown ou en HTML, pas
   en DocTags. Le parseur ne trouvait aucun bloc et concluait « pas de texte »,
   ce qui aurait effacé toutes les transcriptions. → repli texte brut, avec
   aplatissement du HTML (`<td>A</td><td>B</td>` → `A B`, pas `AB`).

2. **Boucle dégénérée.** Sur une fiche technique de motos, dots.ocr a lu la
   fiche correctement puis répété `ふるさ` quarante fois. Jeter la réponse
   entière aurait perdu une bonne transcription à cause d'une mauvaise queue.
   → `truncate_loop` coupe à l'entrée de la boucle et **garde le bon préfixe**.

3. **Remplissage pris pour du texte.** granite-docling rend `4# 4# 4# 4# 4#`
   quand on lui montre des glyphes qu'il ne lit pas. → `looks_like_filler`
   rejette un contenu alphanumérique qui se réduit à un ou deux caractères
   distincts, sans toucher au texte court légitime (`103`, `うんこたれ`).

---

## 5. La règle qui protège le corpus

Côté API, `"text": null` **efface** une transcription ; une chaîne vide est
ignorée. Une planche que le modèle n'a pas su lire ne doit donc **jamais** être
déposée comme `null` par défaut, sinon un mauvais passage détruit le travail du
passage précédent.

Le pont applique cette règle à deux niveaux :

- `aphrody-ocr` distingue `PageText::None` (rien de lisible) de `PageText::Text`
  — ce n'est pas une chaîne vide, c'est un verdict.
- `depose-transcriptions.ts` **saute** les `None` sauf `--avec-vides` explicite.

Corollaire : une planche sans texte n'est simplement pas déposée. Comme elle est
déjà `null` en base, le résultat est identique et rien n'est risqué.

---

## 6. Mode d'emploi

```bash
# 1. Rapatrier un lot depuis le VPS
scp -r dbfr:'~/databooks-ocr/lot-001' ./databooks/

# 2. Lire (reprenable : relancer la même commande continue où elle en était)
aphrody ocr batch ./databooks/lot-001/images \
  --model dots-ocr --out lot-001.jsonl --skip-done

# 3. Renvoyer et déposer
scp lot-001.jsonl dbfr:'~/databooks-ocr/'
ssh dbfr 'cd ~/shenron/apps/site && \
  export SHENRON_ADMIN_TOKEN=$(grep -m1 "^SHENRON_ADMIN_TOKEN=" .env | cut -d= -f2-) && \
  bun scripts/depose-transcriptions.ts ~/databooks-ocr/lot-001.jsonl'
```

Ajouter `--simulation` au dépôt pour voir ce qui partirait sans rien envoyer.

**Reprise.** `--skip-done` relit le JSONL et saute les planches déjà traitées.
Chaque ligne est écrite ET flushée dès que la planche est finie, donc un
processus tué ne coûte que la planche en cours. `--limit` compte les planches
**nouvelles**, ce qui rend `--limit 50` utile sur une reprise.

**Idempotence.** Le dépôt est en `mode: "merge"` : il ne touche que le champ
`text` des pages citées. Rejouer le même lot ne change rien.

---

## 7. Coût réel

Mesuré sur ce poste (RTX 4070, `-ngl 99`) : **~9,3 s par planche** avec
`dots-ocr`, modèle rechargé à chaque planche.

| Portée | Planches | Durée |
|---|---|---|
| un lot | 400 | ~1 h |
| le corpus restant | 11 277 | **~29 h** |

### Deux backends, deux compromis

| Backend | Option | Compromis |
|---|---|---|
| processus par planche | *(défaut)* | recharge les poids à chaque page ; une planche qui plante ne coûte qu'elle-même |
| modèle résident | `--server` | charge une fois ; un serveur qui meurt emporte le run |

Le défaut reste le processus par planche : sur un lot de quatre cents, l'isolation
vaut son prix. `--server` démarre un `llama-server` sur loopback, vérifie sa
santé avant la première page, et le tue à la sortie — un batch interrompu ne
laisse jamais plusieurs gigaoctets de VRAM occupés.

Les deux backends partagent la même boucle et la même sélection de fichiers
(`list_images_sorted`) : un ordre différent ferait sauter, à une reprise, des
planches autres que celles enregistrées.

C'est précisément la charge que le cap du projet vise : une tâche répétée,
longue, sans humain dans la boucle, exécutée en tâche de fond.

---

## 8. État au 2026-08-21

- Pont **vérifié de bout en bout** : 11 planches lues, déposées, et visibles
  publiquement (`GET /api/databooks/1` renvoie `pagesTranscrites: 5`).
- Lot 001 (400 planches) en cours de traitement.
- 28 lots restants.
