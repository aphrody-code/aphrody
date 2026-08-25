<!-- SPDX-License-Identifier: Apache-2.0 -->
# Pont aphrody ↔ shenron — transcription des databooks

> Branché et vérifié de bout en bout le **2026-08-21**.
> Corriger le corpus déjà déposé : [`databooks-hallucinations.md`](databooks-hallucinations.md).
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

### Exactitude mesurée contre une transcription humaine

Le corpus contient quelques planches transcrites à la main avant ce pont. La
fiche 288 (*WSJ 50th Anniversary — Best Scenes Top 10*), page 277, sert donc de
référence. Passée au pipeline :

| | Texte |
|---|---|
| référence humaine | `宿敵ベジータが悟空を認める！` … `ふたりのライバル関係が決着したといえるエピソード！` |
| dots.ocr | **identique, caractère pour caractère** |

Le modèle capture même ce que la référence avait omis : `JC42巻 其之五百十`
(volume et numéro de chapitre) et le titre `「ベジータとカカロット」`. Il lit
aussi le filigrane `capsulecommentary.com` incrusté dans l'image — fidèle, mais
c'est du bruit qu'un nettoyage éditorial voudra retirer.

**Attention au format attendu.** Ces transcriptions de référence sont
**enrichies** : japonais, puis romaji, puis traduction française. Le pipeline ne
produit que le premier étage — ce que le prompt demande explicitement
(« ne traduis pas de toi-même : transcris »). Romaji et traduction restent une
étape éditoriale distincte.

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

### Vérifier avant de déposer

```bash
aphrody ocr audit lot-001.jsonl          # exit != 0 sur un défaut bloquant
```

Un dépôt est difficile à défaire. `audit` cherche ce qu'un lot entier peut
cacher : jeton de contrôle survivant, génération bloquée en boucle, balisage
résiduel — tous bloquants — et les filigranes, signalés sans bloquer parce que
c'est du bruit et non de la corruption. Chaque constat nomme sa page.

La boucle autonome l'appelle entre la lecture et le dépôt : un lot refusé
n'arrête pas les suivants.

### Rattraper des résultats produits avant une règle

```bash
aphrody ocr batch … --raw                # conserve la sortie brute
aphrody ocr clean lot-001.jsonl          # rejoue parsing et filtres
```

Quand une règle de nettoyage est ajoutée, relire les images coûterait des heures
de GPU pour un changement qui ne touche que le post-traitement. `clean` rejoue
le parsing sur la sortie brute conservée par `--raw`. Les lignes sans `raw`
traversent intactes : une commande de maintenance ne doit pas faire disparaître
des pages en silence.

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

## 7bis. Déploiement sur le VPS

Le VPS **n'a pas de GPU**. Ce qui s'y déploie utilement n'est donc pas la
lecture, mais tout le reste de la chaîne :

| Commande | Sur le VPS | Pourquoi |
|---|---|---|
| `aphrody model …` | ✅ | catalogue, téléchargement vérifié, store — aucun GPU |
| `aphrody infer runtime` / `llama` | ✅ | diagnostic de ce qui est installé |
| `aphrody ocr audit` / `clean` | ✅ | pur traitement de JSONL |
| `aphrody ocr batch` | ⚠️ | fonctionne, mais sur CPU : hors de question pour un lot |

La répartition qui en découle : **la lecture reste sur le poste à GPU**, le VPS
audite et dépose. C'est déjà la forme du pont, le déploiement ne fait que rendre
les deux moitiés disponibles là où elles servent.

```bash
# Sur le VPS
cd ~/aphrody && git pull
export RUSTC_WRAPPER= CARGO_CONFIG="$HOME/aphrody/.cargo/config.linux-vps.toml"
export CARGO_TARGET_DIR="$HOME/aphrody/target/x86_64-unknown-linux-gnu"
cargo build --release --target x86_64-unknown-linux-gnu -p aphrody --features ocr
install -m 0755 "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/aphrody" ~/.local/bin/aphrody
```

Le `.cargo/config.toml` par défaut vise Windows MSVC : sans `CARGO_CONFIG`, le
build échoue (cf. [`../DEPLOY.md`](../DEPLOY.md)).

**Déployé et vérifié le 2026-08-22** — build release en 28 min, binaire de
105 Mo installé dans `~/.local/bin/aphrody` (l'ancien conservé en `.prev`) :

```
$ aphrody model accel
accelerators  cpu          # 12 threads, pas de CUDA — conforme au VPS
$ aphrody ocr audit <lot avec un jeton de contrôle>   → exit 1, page nommée
$ aphrody ocr audit <lot propre>                      → exit 0
```

Le VPS définit `APHRODY_HOME=/home/ubuntu`, donc le store vit dans
`/home/ubuntu/models` et non `~/.aphrody/models` : c'est la racine d'état que
`ModelStore::open` documente, et celle qu'`aphrody-embed` utilise déjà.

---

## 8. État au 2026-08-21

**Lot 001 terminé** : 400/400 planches lues, **zéro échec**, en 50 minutes
(7,6 s/planche). **82 %** portaient du texte. **328 transcriptions déposées** et
publiquement visibles :

| Fiche | Transcrites |
|---|---|
| Daizenshuu 1 — Complete Illustrations | 5/233 |
| Daizenshuu 2 — Story Guide | 210/267 |
| Daizenshuu 4 — World Guide | 113/167 |

Le Daizenshuu 1 reste à 5 : c'est un artbook, ses planches sont des
illustrations pleine page. Le pipeline les a classées « sans texte » plutôt que
de les décrire — le comportement voulu.

**Audit de ce qui est en production** : 328 transcriptions, 0 jeton de contrôle,
0 boucle résiduelle, 0 filigrane. Le seul texte très court (`体術技`) est
légitime.

**Lot 002 terminé** : 400/400, zéro échec, **396 avec texte (99 %)** — un lot de
guides, bien plus dense en texte que l'artbook du lot 001. Audité sans défaut
puis déposé ; la fiche 4 (*Daizenshuu 3 — TV Animation Part 1*) passe de 0 à
**310/313**.

### Avancement du corpus

| | |
|---|---|
| Corpus | 318 fiches, 11 775 planches |
| Transcrites | **960 (8,2 %)** sur 18 fiches |
| dont déposées par ce pont | 724 |
| préexistantes | 236 |

Lots 3 à 29 en cours de traitement par la boucle autonome.

---

## 9. Révision du 2026-08-22 — qualité de lecture et débit

Quatre audits menés en parallèle sur le corpus déposé, sur le code, et sur la
documentation amont de dots.ocr. Ce qui suit distingue **ce qui est mesuré** de
**ce qui reste à mesurer sur GPU** : aucune inférence n'a été relancée.

### 9.1. Le vrai défaut du backend résident n'était pas celui qu'on croyait

Le pont notait que `llama-server` « lit moins » que `llama-mtmd-cli` : 1384
caractères contre 1624 sur la planche 18-0249, un folio manquant, et un budget
de jetons porté à 3072 sans effet. La conclusion inscrite dans le code était que
« les mêmes poids voient l'image différemment selon la façade ». **C'est faux.**

Les journaux que le serveur écrit lui-même sous `%TEMP%\aphrody-llama-server-*.log`
disent autre chose :

| | |
|---|---|
| `n_ctx_slot` | 131 072 |
| Jetons de prompt | 1 936 |
| `truncated` | **0**, sur les 61 requêtes journalisées |
| Jetons générés, lot de 12 planches | 437 · **1024** · **1024** · 292 · **1024** · 390 · 301 · 364 · 275 · **1024** · **1024** · **1024** |

Six planches sur douze s'arrêtent à **exactement** 1024, la valeur de
`max_tokens`. Le serveur ne s'arrêtait pas trop tôt : **il ne s'arrêtait pas du
tout**.

Cause : dots.ocr ferme son tour par `<|endofassistant|>`, jeton **151673**. Son
GGUF déclare `eos_token_id = 151643` et **aucun** `eot_token_id`, et llama.cpp
construit son ensemble de fin de génération à partir d'une liste de **noms** de
jetons en dur — où `<|endofassistant|>` ne figure pas. Le modèle disait « j'ai
fini » et personne n'écoutait. Les 2048 jetons gagnés en passant à 3072 étaient
de la boucle, que `truncate_loop` retaillait au même préfixe : d'où l'impression
que le budget n'y changeait rien.

Seconde cause, indépendante : **les deux façades ne prompt pas pareil.**
`--jinja` est activé par défaut sur `llama-server` et désactivé sur
`llama-mtmd-cli` (`common/arg.cpp`, cas `LLAMA_EXAMPLE_MTMD`). Le CLI tombait
donc sur le détecteur de gabarits de llama.cpp, qui le classait **GLMEDGE** — le
gabarit d'un autre modèle, qui omet `<|endofuser|>` et insère un saut de ligne.
Deux prompts différents, deux décodages gloutons différents.

**Corrigé** : `--override-kv tokenizer.ggml.eot_token_id=int:151673` sur les deux
backends, `--jinja` sur le CLI, `-c` explicite côté serveur (`--fit` choisissait
131 072 sur sept lancements et 8 192 sur quatorze autres),
`--no-context-shift`, `--reasoning-format none`, `--skip-chat-parsing`, et
l'échantillonnage épinglé dans le corps de la requête.

### 9.2. Débit : deux leviers, aucun surcoût de VRAM significatif

1. **Une page finie s'arrête.** Sur l'échantillon mesuré, la moitié des planches
   brûlaient 1024 jetons là où elles en avaient produit 300 à 400 d'utiles.
2. **`--parallel N` et N requêtes en vol.** La génération est bornée par la bande
   passante mémoire, pas par le calcul : à une séquence, le GPU relit quatre
   gigaoctets de poids pour produire **un** jeton. Deux séquences relisent les
   mêmes poids une fois et en produisent deux. Le coût est d'un cache KV par
   slot, pas d'une seconde copie du modèle. Défaut : `--slots 2`, réglable.

La boucle de lot lit désormais plusieurs pages de front, mais **écrit sur un
seul fil** : une ligne JSONL déchirée par deux écritures concurrentes
corromprait le fichier même qui rend une reprise possible. L'ordre de sortie
n'est plus l'ordre alphabétique — sans conséquence, `--skip-done` reconnaissant
les planches par leur chemin et non par un décalage.

### 9.3. Le prompt était une paraphrase

`Extract all text from this image.` était câblé ; la chaîne d'entraînement
(`dots_ocr/utils/prompts.py`, clé `prompt_ocr`) est
`Extract the text content from this image.` Trois mots d'écart, sur un modèle
dont le mainteneur écrit qu'il **n'a aucune capacité de suivi d'instructions** :
il ne lit pas la consigne, il reconnaît une des quatre chaînes vues à
l'entraînement. Aligné.

**Écarté après examen** : basculer le corpus sur `prompt_layout_all_en` pour
récupérer les boîtes englobantes et retrier les colonnes. C'est le mode où les
boucles infinies sont attestées en amont ; son JSON est assez peu fiable pour
qu'upstream livre un réparateur dédié (`output_cleaner.py`) ; et surtout **le
désordre qu'il corrigerait n'existe pas ici** — les 50 725 coupures de ligne
mesurées tombent au milieu de mots *contigus*, ce qui prouve que le modèle
descend correctement chaque colonne et se contente d'un saut de ligne au retour.

### 9.4. Qualité de lecture : ce qui est corrigé, et ce qui est seulement signalé

Audit du corpus déposé — **6 305 planches transcrites**, pas un échantillon.

**Corrigé, règle déterministe et contre-exemples bornés :**

| Défaut | Planches | Règle |
|---|---|---|
| Noms propres, sourde ↔ sonore | **479** | Table mesurée (`lexique.rs`), gardée contre `ベジタブル`, `フルマラソン`, `フリーサイズ` |
| `･` demi-chasse répété = ellipse | **638** | `･{2,}` → `…` ; `･` isolé → `・` |
| Boucle à motif long | **112** | Fenêtre portée de 6 à 40 caractères, seuil dégressif |
| `...` ASCII au contact du japonais | **108** | 3 à 5 points → `…` ; jamais 6 et plus, qui sont des points de conduite |
| Marqueur de page halluciné | **62** | Ligne dont le contenu entier est `Page N` |
| Sosies `口`/`二`/`力` → `ロ`/`ニ`/`カ` | **23** | Substitution confirmée par IPADIC |
| Débordement d'énumération en hangul | **15** | Chiffres cerclés repliés comme les chiffres ASCII |
| JSON brut en base | **4** | Défaut **bloquant** à l'audit |

**Signalé, jamais corrigé** — parce qu'une règle qui casse du texte légitime est
pire que le défaut :

| Défaut | Planches | Pourquoi |
|---|---|---|
| Furigana rendu en ligne propre | 585 | Une ligne tout en hiragana peut être du vrai texte |
| Confusions de kana | 124 | Environ la moitié de faux positifs (`ヤシ`, `ミート`, `キラー`) |
| Sosie `一` → `ー` hors contexte | 88 | 95 % de légitime : `一味`, `一家`, `一ツ橋`, `一同` |
| Charabia | mesuré au fil de l'eau | Nouveau : part de caractères hors dictionnaire ≥ 60 % |

Le charabia est le seul défaut qu'aucun filtre de forme ne voyait : une planche
de bulles manuscrites que le modèle n'a pas su lire **a la forme du japonais**.
Le dictionnaire, lui, le voit tout de suite. Mesuré en **caractères** et non en
morphèmes — une file de katakana illisible ne se découpe pas en trente morphèmes
inconnus, elle ressort en **un seul**, et comptée ainsi la planche la plus
illisible du lot passait pour trop courte pour être jugée.

**Relecture image obligatoire** : 783 planches dont la mise en page est aplatie
(aucun saut de ligne sur plus de 300 caractères) et 113 dont le texte vertical
est éclaté à un caractère par ligne. Ni l'une ni l'autre ne se répare depuis le
texte seul.

### 9.5. Correction par voisinage, et le piège qu'elle évite

Un lexique de **321 termes** — établi contre le wiki puis **vérifié terme à terme
contre le corpus**, ce qui a éliminé les graphies inventées (`気孔砲`,
`惑星ナメック` : zéro occurrence) — permet de corriger un nom propre à un kana
près, là où aucun dictionnaire japonais ne peut rien puisqu'il les ignore tous.

La règle naïve « à une substitution d'un terme, donc c'est ce terme » est
**fausse** : 54 paires du lexique collisionnent, en 18 groupes. `孫悟空`,
`孫悟飯` et `孫悟天` sont mutuellement à distance un, comme les six
`人造人間1X号` et les quatre `X の界王神`. Appliquée telle quelle, elle
changerait Gohan en Goku, en silence, sur un corpus public.

La règle retenue exige **exactement un** candidat, plus trois gardes : katakana
pur uniquement, suite complète de katakana, quatre caractères au minimum. Un
test de propriété vérifie qu'aucun terme du lexique n'est réécrit vers un autre.

Effet sur un cas réel : `トラククス` devient `トランクス`, tandis que `ワーロン`
— à une substitution de `ウーロン` **et** de `マーロン` — est laissé tel quel.

### 9.6. Mesuré sur GPU le 2026-08-23

Les trois points laissés ouverts ci-dessus ont été repris sur la carte. Deux
sont confirmés, le troisième a produit un résultat que personne n'attendait.

**1. Le jeton de fin est bien installé.** La preuve n'est pas la ligne
d'override espérée mais un avertissement de llama.cpp :
`load: special_eot_id is not in special_eog_ids - the tokenizer config may be
incorrect`. C'est exactement le chemin qui insère le jeton dans l'ensemble de
fin — sans l'override, il n'y a rien à signaler et le message n'apparaît pas.
Effet mesuré : six planches donnent 169, 254, 219, 196, 361 et 422 jetons. Plus
une seule valeur ronde, plus un seul arrêt sur le plafond.

**2. L'écart de fidélité entre les deux backends n'existe plus** — et il allait
dans l'autre sens que ce que le pont affirmait. Mêmes six planches, même carte :

| Backend | Par planche | Transcriptions identiques |
|---|---|---|
| `llama-mtmd-cli` (par processus) | 8,4 s | référence |
| `llama-server` (résident) | 2,6 s | **5 sur 6, au caractère** |

Sur la sixième, c'est le backend **par processus** qui dégénère : il répète
quatre fois `上段で選んだカードと流派、星の数、数字…合わせていく修行です。`
là où le serveur lit des furigana distincts. Les 175 caractères de plus que la
note d'origine comptait comme de la fidélité étaient de la boucle.

**3. Il n'y a pas de configuration reproductible.** C'est le résultat qui
compte.

En cherchant à savoir si un grand nombre de slots dégradait la lecture, deux
runs **strictement identiques** — mêmes douze planches, même `--slots 4`, même
échantillonnage épinglé, `temperature 0`, `seed 0` — ont divergé sur **sept
planches sur douze**, dont une à 0,213 de ressemblance. Le décodage glouton
n'est déterministe que pour une composition de lot donnée ; celle-ci dépend de
l'ordre d'arrivée des pages dans les slots, et les noyaux d'attention batchés de
llama.cpp ne sont pas numériquement invariants à la taille du lot.

Conséquence directe : **choisir un nombre de slots « pour la stabilité » n'a pas
de sens**, il n'y a rien de stable à préserver. Le choix redevient purement une
question de débit.

Conséquence gênante : une planche peut ressortir à 47 caractères là où un autre
passage en rend 775 — non pas parce qu'elle en contient moins, mais parce que la
génération s'est arrêtée tôt. Le remède ne coûte aucun code : lire le corpus une
seconde fois dans un JSONL parallèle, puis fusionner en gardant par planche la
lecture que `ocr audit` ne signale pas, et à défaut la plus longue. Le dépôt
étant en `mode: "merge"`, améliorer après coup ne casse rien.

### 9.7. Le débit dépend de la planche, pas du réglage

La première estimation — 3 s par planche — venait d'un échantillon non
représentatif. Les planches du lot 029 font 1600×1056 et rendent ~250 jetons ;
celles du corpus font **1340×2048 et en rendent 1 357**. Le profil réel d'une
requête :

| | |
|---|---|
| Prompt (image) | 3 141 jetons, 2,3 s à 1 350 j/s |
| Génération | 1 357 jetons, 13,5 s à 100 j/s |

Le décodage pèse donc 85 % du temps. Comme il est borné par la bande passante
mémoire, le nombre de slots redevient le levier — l'inverse de ce que disaient
les petites planches, où le prompt dominait :

| slots | 12 planches denses |
|---|---|
| 2 | 196 s |
| 3 | 194 s |
| 4 | 146 s |
| 6 | 127 s |
| 8 | **118 s** |

`--slots 8` est le réglage retenu pour ce corpus. Sur des planches légères la
courbe s'inverse et deux slots gagnent : le bon réglage se mesure sur les
planches qu'on va lire, pas sur les premières venues.

### 9.8. Une planche sur trois cents que le serveur refuse

`llama-server` rend un **500** — `The model produced output that does not match
the expected peg-native format` — quand dots.ocr produit du texte que la
grammaire PEG du gabarit n'accepte pas. Le journal nomme le fautif :
`common_chat_peg_parse: unparsed peg-native output: Welcome to the`.

`--skip-chat-parsing` est bien passé sur la ligne de commande et **ne
court-circuite pas** `common_chat_peg_parse` dans `llama-b10549`. Le refus est
**déterministe** : la même planche retombe dessus à chaque reprise, donc relire
par le serveur ne sert à rien.

`llama-mtmd-cli` n'a aucun parseur de chat. Le rattrapage est donc structurel et
ne demande aucun correctif : une planche perdue n'est pas écrite dans le JSONL,
donc `--skip-done` la représente, et une passe finale avec le backend par
processus la lit. C'est ce que fait `balayage.sh` dans la boucle de production.

Le correctif propre — que le backend résident bascule seul sur le processus
quand le serveur refuse une page — reste à écrire dans `server.rs`.

### 9.9. Une règle écrite, mesurée, et retirée : le voisement par dictionnaire

Le site rend `キャブテン翼` là où la planche dit `キャプテン翼`. La confusion
sourde ↔ sonore ↔ semi-sonore n'est donc **pas** limitée au vocabulaire Dragon
Ball, et le lexique de 321 termes ne peut rien pour Captain Tsubasa — mais
IPADIC connaît `キャプテン`. D'où une règle : dans une suite de katakana que le
dictionnaire ne reconnaît pas, essayer chaque variante de voisement, et ne
corriger que si **exactement une** rend un mot connu. Même discipline que la
correction par voisinage du lexique, six tests, contre-exemples compris.

Mesurée sur les 396 planches du lot 018 avant d'être déposée : **242
substitutions**, inspectées une par une.

| Justes | Fautes |
|---|---|
| `パンダイ → バンダイ` (21) | **`プレイ → フレイ` (36)** |
| `バワー → パワー` (13) | **`ゲット → ケット` (16)** |
| `ワンビース → ワンピース` (4) | **`フロスト → プロスト` (6)** — Frost, univers 6 |
| `キャブテン → キャプテン` (2) | **`グルド → クルド` (3)** — Guldo |
| `ジャンフ → ジャンプ`, `タメージ → ダメージ`, `トリフル → トリプル`… | **`コルド → コルト` (3)** — le roi Cold |

Environ quatre-vingt-dix corrections justes contre cent dix dégâts. **Retirée.**

La cause n'est pas la garde du candidat unique, elle est en amont : **IPADIC
n'arbitre pas les katakana.** Il juge `プレイ` inconnu — c'est pourtant
l'orthographe correcte — et connaît `フレイ`. Quand le verdict « inconnu » porté
sur l'original est faux, aucune garde sur le candidat ne rattrape quoi que ce
soit. Pire, le mécanisme détruisait exactement les noms que le lexique existe
pour protéger : absents du dictionnaire par construction, ils ont toujours une
variante qui, elle, y est.

Ce que le dictionnaire sait faire reste vrai et mesuré : valider un sosie de
kanji (`corrige_sosies`, 3 corrections sur ce lot, aucune fausse) et compter du
charabia. Ce qu'il ne sait pas faire, c'est trancher une graphie étrangère. Pour
les katakana, **le lexique fermé, mesuré et gardé est la seule forme sûre** —
et si `キャプテン翼` doit être corrigé un jour, c'est par une entrée de table
avec son comptage et ses contre-exemples, pas par une inférence.

---

## 10. Fin de campagne — 2026-08-23

**Les vingt-neuf lots exportés sont lus et déposés.** Le corpus passe de
**81,5 % à 91,0 %** — 10 481 planches transcrites sur 11 516, les 262 sans scan
hors compte. Vingt-deux dépôts, aucun en échec.

### Ce que la dernière session a lu

| Étage | Résultat |
|---|---|
| Lecture des lots 026-029 | 1 191 planches, serveur résident, 8 slots |
| Balayage des refus PEG sur les douze lots | **22 planches** rattrapées par le backend par processus |
| Rejeu du nettoyage sur les lots déjà déposés | 78 632 octets de boucle retirés |

Les douze lots locaux sont à **4 477 / 4 477**, aucune planche manquante. Le
balayage a fait exactement ce que §9 en attendait : les vingt-deux planches que
`llama-server` refusait par un 500 `peg-native` sont toutes passées par
`llama-mtmd-cli`, qui n'a pas de parseur de chat.

Débit : 3,1 à 7,4 s la planche selon la densité, environ 57 jetons/s par slot.
Les lots denses tournent à 7 s, ceux riches en planches illustrées à 3,1 s —
une planche que le modèle ne lit pas se rend vite.

### Ce qui reste n'est pas du travail en attente

Mille trente-cinq planches restent sans transcription, dont trois cents dans
ces douze lots. **Ce ne sont pas des planches vides.** Vérification faite sur
l'image : `312-0014.jpg` — *DBZ TV Special : Bardock*, planche 18 — porte
`カナッサ星`, une bulle `クッ!!`, les onomatopées `グォーッ` et `ドゥッ`, et le
folio 18. dots.ocr rend `none`.

C'est ce que §3 annonçait : du texte de manga, bulles verticales et onomatopées
dessinées, hors du domaine d'un modèle de document. Le reliquat se concentre
donc là où ce texte vit :

| Catégorie | Transcrit |
|---|---|
| Databook | 95,4 % |
| Pamphlet & Fair | 94,0 % |
| V-Jump | 92,2 % |
| Weekly Shōnen Jump | 90,4 % |
| **Art Book** | **56,6 %** |
| **Jump Anime Comics** | **54,3 %** |

Aucune passe supplémentaire de dots.ocr n'y changera quoi que ce soit. Les
récupérer demande une détection de bulles suivie d'un modèle spécialisé sur
bulles pré-découpées — un chantier distinct, pas une reprise de celui-ci.

**Corollaire opérationnel** : une vague de planches `textless` dans un ouvrage
n'est pas un symptôme de panne. Vérifier d'abord la catégorie de l'ouvrage. Le
mode d'échec du mauvais prompt (§3) produit le même symptôme, mais lui vide
*tout*, y compris les fiches techniques bien imprimées.

---

## 11. Les bulles — ce que §10 croyait impossible

§10 concluait que les 1 035 planches restantes demandaient « une détection de
bulles suivie d'un modèle spécialisé sur bulles pré-découpées ». La première
moitié est juste. **La seconde est fausse, et la mesure du 2026-08-23 le dit
sans ambiguïté.**

### L'expérience que personne n'avait faite

La note disait qu'il faudrait un modèle travaillant *sur bulles pré-découpées*
— sans avoir jamais donné une bulle pré-découpée au modèle qu'on a déjà. Quatre
zones de la planche 18 de *DBZ TV Special : Bardock* ont donc été recadrées à
la main et passées à dots.ocr, telles quelles puis agrandies trois fois :

| Zone | crop brut | crop ×3 |
|---|---|---|
| `カナッサ星`, titre vertical imprimé | lu | lu |
| `クッ!!`, **bulle manuscrite** | rien | **`クッ!!`** |
| `グォーッ`, `ドゥッ`, onomatopées dessinées | rien | rien |

dots.ocr lit donc les bulles. Ce qui lui manquait n'était pas le vocabulaire,
c'était le **cadrage** : sur une planche de 1128×1600, cette bulle occupe
130×100 pixels — une poignée de jetons visuels dans une image qu'il traite
comme un dessin.

### Le pavage ne suffit pas

Découper la planche en tuiles est la réponse évidente, et elle ne marche pas.
Six tuiles agrandies ×2 : seul le titre ressort. Douze tuiles ×3, pour 28 s au
lieu de 5 : le titre ressort *mieux* (`カナッサ星` correct au lieu de
`カナツサ星`), la bulle reste muette. Le modèle ne veut pas plus de pixels, il
veut une image dont le texte est le **sujet**.

### Ce que la détection récupère

`aphrody-ocr::bulles` cherche les régions claires connexes — ce qu'est une
bulle — et les remet en ordre de lecture japonais. Sur douze planches que le
pipeline de page rend entièrement muettes : **44 bulles récupérées sur 9
d'entre elles**, portant du dialogue vérifiable contre la scène :

```
フリーザ様 たった今 カナッサ星を占領したという報告が入りました
さすがはバーダックだな わずか数日でほとんど完治してしまうとは…!
戦闘力が1万近くになっているはずだ……
ベジータさん しっかり働いてきてくださいね
```

Coût : 114 s pour 254 régions, soit environ une lecture de page par planche.

### Deux résultats qui vont contre l'intuition

**Le filtre d'encre ne filtre rien.** Quatre régions détectées sur cinq ne
portent aucun texte, et l'idée évidente — « une bulle vide a peu de pixels
sombres » — est démentie : les régions avec texte ont 0,187 d'encre médiane,
celles sans texte 0,195. Les distributions se recouvrent entièrement, parce
que les faux positifs ne sont pas des bulles vides mais des zones claires
*dans les dessins* — visages, ciels, vêtements pâles — pleines de traits.

**La sortie n'est pas déposable telle quelle.** Une région qui tient un
*fragment* de glyphe fait inventer au modèle des caractères plausibles plutôt
que rien : un fragment d'onomatopée orange est revenu en `禁 幸`. Et une bulle
lisible peut sortir en romaji approximatif — `ありがとうございます` rendu
`ary ga thu / gōzoku`. D'où le passage obligatoire par
`aphrody ocr audit --japonais`, dont le détecteur de charabia est exactement
l'outil pour ça.

### Mode d'emploi

```bash
aphrody ocr bulles lot-028-resultats.jsonl   --images lot-028/images --out lot-028-bulles.jsonl   --server --slots 8 --skip-done
aphrody ocr audit lot-028-bulles.jsonl --japonais
```

Construire avec `--features ocr-bulles`. `--decoupes <dir>` conserve les
recadrages, ce qui est le seul moyen de confronter une lecture surprenante aux
pixels qui l'ont produite — et cette commande en a plus besoin que `batch`,
puisque son entrée est un fragment de page et qu'un fragment peut tromper.

---

## 12. Résultat de la passe par bulles — 2026-08-24

Les 300 planches que les douze lots rendaient muettes ont été relues bulle par
bulle. **235 ont retrouvé leur texte, soit 78 %.**

| Lot | Récupérées | | Lot | Récupérées |
|---|---|---|---|---|
| 018 | 3/3 | | 024 | 15/17 |
| 019 | 5/5 | | 025 | 2/14 |
| 020 | 8/8 | | 026 | 11/13 |
| 021 | 18/19 | | 027 | 19/20 |
| 022 | 15/19 | | 028 | **121/159** |
| 023 | 18/21 | | 029 | 0/2 |

Débit : 3,6 à 10,6 s la planche selon le nombre de régions, backend résident,
8 slots. Les deux lots à faible rendement (025, 029) sont des planches
d'illustration sans bulle — la chaîne y rend un `none` honnête plutôt que
d'inventer, ce qui est le comportement voulu.

### Ce qui a été déposé, et ce qui ne l'a pas été

**223 planches déposées** en mode `merge`, 0 en échec. Douze ont été écartées :
celles dont le texte tient en quatre caractères ou moins, où se concentrent les
fragments d'onomatopée mal lus — `力 力`, `二三`, `取 後`. Le prix de ce filtre
est mesuré : quatre vraies onomatopées de bulle (`ハッ！`, `クッ！`) partent
avec, et n'apportaient presque rien.

Défaut résiduel assumé : **30 % des planches récupérées portent une suite
latine**. Une partie est authentique — `CHINESE RESTAURANT` est une enseigne
dessinée dans la case — le reste est de l'artefact ponctuel au milieu de texte
correct (`sti`, `lest`, `lesslook`). Aucun audit `--japonais` n'a signalé de
défaut bloquant, mais son détecteur de charabia vise les textes longs et ne
voit pas ces intrusions : elles sont documentées ici plutôt que masquées.

### Effet sur le corpus

| | Avant | Après |
|---|---|---|
| Corpus complet | 91,0 % | **92,9 %** (10 704 / 11 516) |
| Jump Anime Comics | 54,3 % | **72,0 %** |
| V-Jump | 92,2 % | 94,7 % |

Le §10 de ce document déclarait la catégorie Jump Anime Comics structurellement
plafonnée. Elle ne l'était pas. Ce qui reste hors de portée, à toute échelle,
ce sont les **onomatopées dessinées** — du dessin, pas de la lettre.

---

## 13. Les lots 001-017 — le corpus à 95,9 %

La passe par bulles du §12 n'avait touché que les douze lots traités en local.
Les **dix-sept premiers lots n'y étaient jamais passés** : 595 planches muettes
de plus, soit les trois quarts du reliquat.

### Ne rapatrier que ce qu'on va lire

Les images de ces lots pèsent environ 3,4 Go, mais seules 9 % des planches sont
concernées. Extraire la liste des muettes côté VPS et n'archiver que
celles-là : **188 Mo, transférés en 17 secondes**. Le JSONL d'entrée, lui, n'a
pas été rapatrié du tout — `ocr bulles` n'y lit que le nom de l'image et le
verdict, donc il se fabrique localement en trois lignes.

### Résultat

| | |
|---|---|
| Planches relues | 595, aucun échec |
| Texte récupéré | **365 (61 %)** |
| Déposées après filtrage | **344** |
| Débit | 3,8 s/planche, backend résident, 8 slots |

Le taux est plus bas que les 78 % du §12, et c'est attendu : ces lots sont des
V-Jump et des databooks, dont les planches muettes sont souvent des
illustrations pleine page sans une seule bulle. La chaîne y rend un `none`
honnête — c'est le comportement voulu, pas un échec.

Écartées du dépôt : 21 planches. Les 19 dont le texte tient en quatre
caractères (`高`, `2004`, `z`, `D V` — des fragments), plus les 2 que
`audit --japonais` signalait en charabia.

### Le bug qui a coûté vingt-six minutes de GPU

La reprise après interruption a annoncé « 595 planches à relire, 448 déjà
faites », puis les a **toutes relues**. Le compte était juste, le filtre ne
l'était pas : il comparait des chemins bruts, et la reprise avait été lancée
avec un chemin relatif là où la première passe portait un chemin absolu.

`batch` peut comparer des chemins — il relit son propre répertoire, la forme
est la même des deux côtés. `ocr bulles` prend un JSONL et ré-enracine chaque
planche sous `--images` : la forme dépend de la ligne de commande, alors que
l'identité d'une planche est son **nom de fichier**. Corrigé.

Le symptôme est silencieux : rien n'échoue, le travail est simplement refait.

**Ce qui a été récupéré du gâchis** : ces 448 secondes lectures ont été
fusionnées avec les premières en gardant la plus longue de chaque planche —
le décodage n'étant pas reproductible (cf. mémoire projet), deux passages
divergent. **83 planches y ont gagné du texte**, dont une muette dans un
passage et lue dans l'autre.

### Le corpus au terme des trois campagnes

| Catégorie | Départ (21 h) | Fin |
|---|---|---|
| **Corpus complet** | **81,5 %** | **95,9 %** |
| Weekly Shōnen Jump | 78,6 % | 98,6 % |
| Databook | 95,4 % | 97,9 % |
| V-Jump | 78,4 % | 97,1 % |
| Jump Anime Comics | 42,9 % | **84,1 %** |
| Art Book | 48,7 % | 59,2 % |

Restent 469 planches. Art Book est le dernier bloc, et pour la raison déjà
donnée : des planches d'illustration où il n'y a pas de texte à lire.

---

## 14. La résolution n'explique pas le mutisme — mesure contre hypothèse

Une observation avait fait naître une hypothèse séduisante, et fausse. La voici
avec ce qui l'a tuée, parce que le raisonnement se reproduira.

### L'observation, réelle

Sur le livre 23 (*TV Anime Guide*), onze pages classées « confirmées muettes »
par les campagnes précédentes se sont transcrites sans peine dès lors qu'elles
étaient relues depuis les scans du site plutôt que depuis les lots exportés. La
raison paraissait évidente, et elle est mesurée :

| Planche 23-0100 | Largeur | Poids |
|---|---|---|
| dans `lot-007/images/` | **422 px** | 20 Ko |
| scan du site | **2048 px** | ~400 Ko |

Un facteur 25 en surface. Sur une image de 422 px, une bulle occupe 40×30
pixels — illisible, et cohérent avec ce que §11 avait établi : une bulle de
130×100 ne se lit pas, la même agrandie ×3 se lit.

D'où la généralisation : les 283 planches déclarées muettes l'auraient été sur
des vignettes, et seraient massivement récupérables en pleine résolution.

### La mesure, qui dit non

317 planches relues depuis les scans du site, taux de récupération croisé avec
la largeur du scan :

| Largeur du scan | Lues | Avec texte | Taux |
|---|---|---|---|
| < 600 px | 18 | 5 | **27 %** |
| 600 – 1000 | 25 | 6 | 24 % |
| 1000 – 1500 | 166 | 19 | 11 % |
| 1500 – 2500 | 5 | 2 | 40 % |
| **> 2500 px** | 66 | 2 | **3 %** |

**Le taux décroît quand la résolution croît.** Les plus grands scans donnent le
plus mauvais résultat, et le taux global tombe à 12 % là où le cas du livre 23
en laissait espérer 55.

L'explication tient en une phrase : la taille d'un scan renseigne sur le *type*
de page, pas sur sa lisibilité. Un scan de plus de 2500 px est une double page
ou un poster d'artbook — de l'illustration pure. La corrélation existait, elle
pointait dans l'autre sens que supposé.

Le livre 23 reste un vrai cas particulier : ses exports étaient des vignettes.
Ce n'est pas une règle, et un cas ne fait pas une distribution.

### Ce que §10 disait, et qui tient

« Ces planches ne sont pas vides, elles sont hors domaine » était vrai pour les
onomatopées dessinées. « Ce sont des pages sans texte à lire » est vrai pour le
reste. Les deux conclusions ont survécu à la vérification ; c'est l'hypothèse
de la résolution qui n'a pas tenu.

### Un bug trouvé en chemin

Les scans de 2048 px ont fait apparaître un défaut invisible jusque-là :
`REQUEST_TIMEOUT` valait trois minutes en dur, quel que soit le nombre de
slots. Avec huit requêtes en vol sur des images quatre fois plus lourdes en
jetons visuels, une requête attend derrière les sept autres et dépasse — et le
client rendait `os error 10060` sur des planches qui généraient parfaitement.
Le timeout lisait de la contention comme un blocage. Il suit désormais le
nombre de slots.

---

## 15. Clôture — 97,6 %

Les 317 planches ont toutes été lues, 41 déposées, aucun échec. Le corpus
termine à **11 240 planches sur 11 516, soit 97,6 %**.

| Catégorie | Avant-hier soir | Maintenant |
|---|---|---|
| **Corpus complet** | **81,5 %** | **97,6 %** |
| Weekly Shōnen Jump | 78,6 % | 99,6 % |
| V-Jump | 78,4 % | 99,5 % |
| Light Novel | 99,0 % | 99,5 % |
| Databook | 95,4 % | 98,8 % |
| Pamphlet & Fair | 94,0 % | 95,3 % |
| Jump Anime Comics | 42,9 % | 86,1 % |
| Art Book | 48,7 % | 61,1 % |

### Un faux positif de l'audit, tranché sur l'image

`ocr audit --japonais` signalait deux planches en charabia — 68 % et 100 % de
caractères hors dictionnaire. L'image dit autre chose. La planche 142-26 est
une carte *Super Dragon Ball Heroes* (SDVPJ-030, PR), et la transcription est
exacte caractère pour caractère :

```
ベジータ
HP 3500  パワー 5300  ガード 1000
ゴッドギャリック砲
サイヤ人の本能
```

Un seul écart, `ゴット` pour `ゴッド` — le défaut sourde/sonore déjà inventorié.
La seconde planche est un tableau de trophées de jeu :
`プラチナ ゴールド シルバー ブロンズ トロフィー`.

La cause est celle que la mémoire du projet documente pour le lexique et qui
vaut aussi pour l'audit : **IPADIC n'arbitre pas les katakana**. Un texte fait
de mots étrangers translittérés — platine, or, argent, bronze, trophée — est
intégralement « hors dictionnaire » tout en étant parfaitement juste.

Les deux planches ont donc été déposées, contre l'avis de l'audit et sur preuve
visuelle. C'est le seul motif acceptable pour passer outre : regarder l'image,
pas trouver le verdict gênant.

### Ce qui reste — 276 planches

Art Book à 61,1 % est le dernier bloc, et pour la raison mesurée au §14 : des
pages d'illustration où il n'y a pas de texte à lire. Les onomatopées dessinées
restent hors de portée à toute échelle.

Trois angles ont été essayés et mesurés — lecture de page, pavage, détection de
bulles — et un quatrième écarté sur mesure : la relecture en pleine résolution,
qui ne rapporte que 14 % sur des planches déjà classées muettes.

