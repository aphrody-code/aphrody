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

### 9.6. Ce qui reste à mesurer sur GPU

Rien de ce qui précède n'a été revalidé par une inférence. À faire, dans cet
ordre :

1. **Vérifier que l'override du jeton de fin prend.** Chercher dans le journal
   `Using metadata override (int) 'tokenizer.ggml.eot_token_id' = 151673`, puis
   vérifier qu'aucune génération ne bute plus sur un multiple exact de
   `max_tokens`.
2. **Re-mesurer les deux backends sur les mêmes planches**, maintenant qu'ils
   voient le même gabarit et le même jeton d'arrêt. Si l'écart de fidélité a
   disparu, le backend résident devient le défaut du corpus — et le lot passe de
   13 s à moins de 3 s par planche.
3. **Ne pas mélanger les deux moitiés du corpus.** Ce qui a été déposé avant
   cette révision a été lu avec l'ancien prompt et sans jeton d'arrêt ; un corpus
   lu à moitié d'une façon et à moitié de l'autre est pire que l'un ou l'autre.
