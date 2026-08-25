<!-- SPDX-License-Identifier: Apache-2.0 -->
# Corriger les hallucinations du corpus databooks

> Passe complète menée le **2026-08-25** sur les 11 255 planches transcrites de
> `dragonballfr.com`. Amont : [`databooks-transcription-bridge.md`](databooks-transcription-bridge.md).
> Détecteur : `shenron:apps/site/scripts/detecte-hallucinations.ts`.

Ce document ne raconte pas une passe de nettoyage. Il consigne **ce qui se
corrige sur le texte seul, ce qui ne se corrige pas, et comment on distingue les
deux** — parce que la distinction n'est pas évidente et que se tromper coûte
plus cher que le défaut.

---

## 1. La règle de fond

Le corpus est public. Une règle de nettoyage à 50 % de faux positifs est **pire
que le défaut qu'elle corrige** : le défaut se voit, la fausse correction non.

Toute famille de défauts qu'on veut corriger doit venir avec trois choses :

1. son **comptage** de planches touchées, mesuré, pas estimé ;
2. le **contre-exemple** qu'on a cherché — la chose qui ressemble au défaut et
   n'en est pas ;
3. un **test de non-régression** sur ce contre-exemple.

Le cas d'école est `ベジタブル` : deux planches expliquent que le nom de Vegeta
vient de *vegetable*, et une règle `ベジタ → ベジータ` sans garde de frontière
détruirait exactement le passage qui la justifie.

---

## 2. Le mode d'échec le plus coûteux : le filtre silencieux

C'est le piège central du domaine, et il ne lève aucune erreur.

Une table de correction des noms propres avait été construite par un script
d'analyse qui comptait le point médian `・` comme un katakana **bloquant**. Le
module de correction, écrit ensuite, le tenait au contraire pour une **frontière
de mot**. L'analyse n'a jamais été relancée avec la règle du module.

Résultat : `プロリー` (faute de `ブロリー`) a **54 occurrences** dans le corpus,
dont toutes les propres sont bordées d'un `・`. Le script comptait donc zéro, et
un `if (occurrences === 0) continue;` l'écartait **en silence**. Vingt-cinq
planches sont restées fautives, invisibles dans tous les rapports.

**Deux règles en découlent, et elles valent au-delà de ce corpus :**

- Le filtre de **découverte** ne doit jamais être plus strict que le filtre de
  **correction**. Quand on écarte des candidats, on compte et on journalise ce
  qu'on écarte.
- Un détecteur **annote, il ne filtre pas**. Une occurrence agglutinée est
  souvent une sous-chaîne accidentelle (`ゴニック` dans `ドラゴニック`) mais
  parfois un vrai nom collé à son voisin. On la marque, un humain tranche.

---

## 3. Les quatre arbitres d'une lecture

Du plus fort au plus faible.

### 3.1. Le corpus

Une graphie attestée des centaines de fois est la bonne ; une graphie unique
est suspecte. C'est l'arbitre le plus simple et le plus fiable.

### 3.2. Le rapport de fréquence

**Une faute de lecture est toujours moins attestée que la forme dont elle
dérive.** Seuil retenu : la forme juste doit être au moins **2× plus fréquente**
que la fautive.

Sans ce critère, la simple ressemblance fait remonter du vocabulaire courant :
`アビリティ` (798 occurrences) sort comme faute de `レアリティ` (178), `ナルト`
comme faute de `ボルト`, `チョコボ` comme faute de `チョコ`.

### 3.3. La chronologie

L'arbitre le plus tranchant, et celui auquel on ne pense pas.

| Paire | Verdict | Raison |
|---|---|---|
| `ガンバー` → `カンバー` (Cumber) | refusé | titre de jeu de **1992**, treize ans avant le personnage |
| `トキドキ` → `トキトキ` | refusé | V-Jump **1997**, c'est l'adverbe *tokidoki* ; Tokitoki date de 2015 |
| `シレン` → `ジレン` | refusé | 風来のシレン, jeu Chunsoft cité de 1996 à 2000, vingt ans avant Jiren |

### 3.4. Le dictionnaire, et ses deux angles morts

JMdict écarte les pièges où une règle par distance réécrirait un mot japonais
réel en nom de personnage — `ジャンパ` (blouson) visait `シャンパ` (Champa),
`ドルビー` (Dolby) visait `トルビー`. **Douze cas** bloqués ainsi.

Mais le dictionnaire ignore deux choses :

- **Il n'arbitre pas les katakana.** Un texte de mots étrangers translittérés
  est intégralement « hors dictionnaire » tout en étant juste. Deux planches
  signalées charabia à 68 % et 100 % étaient exactes : une carte Heroes
  (`ベジータ / HP 3500 パワー 5300`) et un tableau de trophées
  (`プラチナ ゴールド シルバー ブロンズ`).
- **Il ne connaît pas les noms propres hors Dragon Ball.** `ゲール` → `ケール`
  refusé : c'est **Gale**, garde du corps de DBGT, et la planche porte sa propre
  traduction « シーラ&ゲール / Sheera & Gale ». Ni le dictionnaire ni la
  fréquence ne pouvaient l'attraper.

### 3.5. Le piège de la couverture du lexique

La couverture `name_ja` du wiki est **très inégale** : 95 % sur les databooks,
59 % sur les personnages, **2 % sur les techniques** (17 sur 825). `ギャリック`
(de ギャリック砲) est absent du lexique alors que `ガーリック` (Garlic, le
personnage) y figure — d'où des « corrections » de l'un vers l'autre.

**Une absence du lexique n'est pas une preuve de faute.**

---

## 4. Ce qui a été corrigé

| Famille | Planches | Le garde qui l'encadre |
|---|---|---|
| Artefacts du modèle (`�` tronqué, `･･`, `...`, marqueurs de page, phrases méta) | 878 | le folio authentique est un chiffre **nu** |
| Ellipses `・・・` pleine chasse | 333 | `・・` est une **puce de liste**, seuil à 3 |
| Noms propres — dakuten rendu | 391 + 212 | frontière de mot, fréquence, chronologie |
| Générations déraillées — boucles coupées, préfixe gardé | 142 | segment uniforme exclu ; seuil à 10 tours |
| Sosies `口`/`力`, compteurs hangul | 50 | sosie en bord de suite collé à un kanji : jamais |

Corpus : **6 167 250 → 5 976 468 signes**, soit 190 782 signes de bruit retirés,
**à nombre de planches transcrites inchangé** (11 255). Rien n'a été perdu.

### Les gardes qui ont payé, mesurés

- **Frontière de mot** : `ベジタブル` intact, et surtout **7 régressions évitées
  sur `スーパーボンバーマン`**, où `パーボン` est une sous-chaîne du titre de
  Hudson — que personne n'avait anticipé.
- **`・` isolé** : 15 317 occurrences sur 4 109 planches, **strictement
  inchangées** de part et d'autre de la passe.
- **Segment uniforme** : `ーーーー`, `……`, un cri étiré `おおおお` sont
  périodiques pour *toute* période et ressortiraient à tort de n'importe quel
  détecteur de répétition.
- **Le compteur cerclé est de l'arithmétique, pas de l'écriture** : Unicode met
  1-20 en U+2460, 21-35 en U+3251, 36-50 en U+32B1, et intercale le hangul
  cerclé en U+3260. Le modèle a suivi les codets. Formule `35 + (cp - U+325F)`,
  vérifiée 26 fois sur 26.

---

## 5. Ce qu'on ne corrige jamais

Mesuré, verdict stable. Ne pas re-tenter sans mesure nouvelle contradictoire.

| Famille | Volume | Pourquoi |
|---|---|---|
| Furigana en ligne propre | 3 737 | une ligne tout en hiragana peut être du vrai texte |
| Intrusions d'alphabet | 818 | `げмар`, `容питしない` — caractère isolé substitué, aucun motif ; exige l'image |
| Textes courts | 287 | sur les 84 planches de ≤ 4 signes, **44 sont purement numériques** — des folios légitimes |
| Romaji seul | 259 | latin authentique (logos, ISBN) + trois ouvrages réellement anglophones |
| Confusions ソ/ン, シ/ツ | 124 | ~50 % de faux positifs : `ヤシ`, `ミート`, `キラー` |
| Sosie `一` → `ー` | 88 | **95 % de légitime** : `一味`, `一家`, `一ツ橋` |
| `�` entre deux kana | 70 | le remplacer = deviner ; le **retirer** souderait `使える�けではない` en `使えるけではない` — faute silencieuse, pire que le signal |
| Compteur cerclé > 50 | 3 | Unicode n'a aucun nombre cerclé au-delà de 50 |

### Hypothèses infirmées par le comptage

Elles figuraient dans la mémoire du projet et étaient **fausses** :

- `力 力` et `二三` annoncés comme fragments d'onomatopée à vider : **0
  occurrence** de `力 力` dans tout le corpus.
- « Bulles rendues en romaji approximatif » : population **introuvable**.
- Jetons de contrôle du modèle : **0** sur 11 255 planches, déjà nettoyés.
- Hallucinations par répétition inter-planches : les 8 phrases récurrentes sont
  du **boilerplate authentique de magazine**.

---

## 6. Le détecteur

```bash
bun apps/site/scripts/detecte-hallucinations.ts               # rapport lisible
bun apps/site/scripts/detecte-hallucinations.ts --json out.json
bun apps/site/scripts/detecte-hallucinations.ts --famille boucle-motif-long
bun apps/site/scripts/detecte-hallucinations.ts --sans-lexique
```

Trois niveaux, qui ne se traitent pas pareil :

| Niveau | Sens | Action |
|---|---|---|
| **bloquant** | une règle existe, ceci ne devrait plus exister | régression : lot neuf non nettoyé, ou runner qui en a écrasé un autre |
| **signalé** | défaut réel, aucune règle fiable sur texte seul | file de relecture humaine |
| **témoin** | population légitime | une **BAISSE** est une régression : une règle a mangé du vrai texte |

Sortie en **code 1** si une famille bloquante est non vide ou si un témoin
s'écarte de plus de 2 % de sa référence.

Chaque famille du fichier porte son seuil **et la mesure qui l'a fixé** — lire
ces commentaires avant d'en ajouter une.

Le **balayage du lexique** est le détecteur le plus important : il régénère les
variantes sourde/sonore de chaque nom propre du wiki et les cherche dans le
corpus. C'est lui qui rattrape ce qu'une table figée laisse passer. Une table se
périme ; ce balayage, non.

---

## 7. Mécanique de dépôt

- Mode `merge`, **par planche** : seules les planches citées sont touchées.
- Une chaîne vide est **ignorée**, pas traitée comme un effacement. Pour retirer
  un texte, il faut `"text": null`.
- Chaque dépôt écrit une révision dans `public.wiki_revisions` : réversible
  depuis `/admin/wiki/history`.
- Deux runners en parallèle peuvent s'écraser — le second redépose depuis un
  texte lu avant le passage du premier. Tous étant idempotents, **une passe
  finale de `--simulation` sur chacun** dit s'il reste à redéposer.
- Le garde-fou « texte corrigé < 50 % de l'original » écarte la planche au lieu
  de l'envoyer. Une boucle dégénérée, où perdre 90 % du texte EST la correction,
  se traite par un prédicat **nommé et borné** à ces règles-là.
- **Avant tout `--appliquer` de masse** :
  `pg_dump "$DATABASE_URL" -t bot.db_databooks | gzip > ~/backups/…`

## 8. Organisation du code

Chaque famille vit dans son module pur sous `src/lib/databooks-ocr/`, avec son
runner `scripts/corrige-*.ts`. **Ne jamais mettre deux familles dans un même
fichier** : plusieurs agents y travaillent en parallèle et s'écraseraient.

---

## 9. Un modèle de langue japonais n'y arrive pas — mesuré le 2026-08-25

L'idée était naturelle : après les passes déterministes, confier le reliquat à
un modèle japonais rapide. Elle a été essayée et **elle échoue**. Ce qui suit
est consigné pour éviter qu'on la reprenne de zéro.

**Modèle** : `LFM2.5-1.2B-JP` Q8_0 (Liquid AI), choisi sur mesures et non sur
réputation — il bat Qwen3-1.7B sur les évaluations japonaises, là où
Sarashina2-7B, six fois plus gros et japonais par conception, plafonne à 0,400
JMMLU. Architecture hybride convolution + attention, **0,07 s par requête** sur
RTX 4070 : la vitesse n'a jamais été le facteur limitant.

Le modèle n'est pas en cause dans son fonctionnement : il répond `悟空` à « qui
est le héros de Dragon Ball » et `東京` à la capitale du Japon. Il génère du
japonais correct. Il ne sait pas **évaluer**.

| Usage | Résultat |
|---|---|
| **Réparer** une intrusion d'alphabet | **0 proposition attestée sur 40.** Il translittère au son (`م` → `ま`) au lieu de lire le contexte |
| **Juger** si un texte est du japonais valide | **50 % sur un jeu équilibré** — le score d'une réponse constante. Interrogé en question ouverte, il déclare la boucle `着衣に着いた衣を被衣に…` « grammaticalement correcte » |
| **Choisir** entre deux graphies candidates | 67 %, mais voir ci-dessous |

### Le détail qui tranche : *où* il se trompe

Sur le choix fermé, ses quatre erreurs sont **exactement les quatre pièges** que
le travail déterministe avait identifiés et protégés :

| Contexte | Il répond | Vérité | Coût de l'erreur |
|---|---|---|---|
| `不況の底値スラック` | `スラッグ` | `スラック` | détruit #82 p.65, l'horoscope financier |
| `風来のシレン` | `ジレン` | `シレン` | le jeu Chunsoft réécrit en personnage de 2018 |
| `名前の由来は野菜ベジタブル` | `ベジータブル` | `ベジタブル` | **casse le témoin** : la planche d'étymologie de Vegeta |
| `天才ピート` | `ビート` | `ピート` | le mécanicien de *Dub & Peter 1*, manga de Toriyama |

Il réussit là où le lexique donnait déjà la réponse, et échoue là où la
difficulté est réelle. Le brancher sur le corpus serait une régression.

### Ce que la mesure ne dit pas

Le même jeu n'a **pas** été passé à un modèle plus gros : le téléchargement de
Qwen3-8B (JMMLU 0,714) a été interrompu avant d'être complet, SHA-256 non
vérifié. La conclusion ci-dessus vaut donc **pour un 1,2 B**, et rien n'est
établi au-delà. Le refaire demande une heure : douze cas, dont les quatre
pièges, sont dans `scratchpad/choix.ts` du jour.

**Ce qui est établi, en revanche** : sur ce corpus, l'ancrage bat le modèle.
Une graphie attestée par 11 255 planches du même domaine est une preuve ; une
proposition plausible n'en est pas une. Le principe à conserver si l'on reprend
un modèle un jour — **il propose, le dump tranche** — n'est pas une précaution
de style, c'est ce qui a bloqué quatre destructions sur douze cas.

---

## 10. L'alternative optique : un second moteur OCR — mesuré le 2026-08-25

Après l'échec du modèle de langue (§ 9), la voie suivante ne raisonne plus sur
le texte mais relit le pixel. Elle repose sur un argument **structurel**, pas
statistique.

| | dots.ocr | PP-OCRv5/v6 |
|---|---|---|
| Famille | VLM autorégressif | détection + CRNN |
| Vocabulaire | **ouvert** | **fermé** |
| Peut émettre de l'arabe ? | **oui** — il l'a fait 2 412 fois | **non** |

Le dictionnaire de PP-OCRv5, vérifié dans le fichier livré : **86 hiragana,
94 katakana, 15 565 kanji, 11 cyrilliques, zéro arabe.** Il lui est
*impossible* de produire l'artefact qu'on cherche à retirer. Et comme rien
n'est généré jeton par jeton, les boucles dégénérées, les arrêts sur
`max_tokens` et le jeton de fin manquant disparaissent aussi.

### Ce qui est démontré

Sur 12 planches à intrusion relues (échantillon du VPS, scans dans
`shenron:apps/site/public/wiki/databooks/`) :

- **11 planches sur 12 relues sans AUCUNE intrusion.** La propriété structurelle
  tient en pratique.
- La relecture est **meilleure** sur des passages entiers. Planche #2 p.119 :

  | Base (dots.ocr) | PP-OCR |
  |---|---|
  | `而の未来のブルマ` | `別の未来のブルマ` |
  | `電船を喰されて` | `宇宙船を壊されて` |
  | `洞窟や崖の間に駆走していた` | `洞窟や岩の間に隠れていた` |

- **7,5 s par planche sur CPU**, soit ~1 h 35 pour les 763 planches concernées.
  Sans GPU, et sans le moindre jeton généré.

### Ce qui ne marche PAS, et pourquoi

**L'appariement automatique mot-à-mot.** Sur 34 mots abîmés, il n'en récupère
que **1 à 3** avec certitude. Cause mesurée : les deux moteurs **découpent leurs
régions différemment**, donc les bords d'un mot coïncident rarement. Assouplir
le critère par une distance d'édition a *dégradé* le résultat (3 → 1), les mots
courts tombant sous le seuil.

Deux essais d'ancrage, deux enseignements :

- Se limiter à `graphies-corpus.tsv` rendait tout mot en kanji ou hiragana
  structurellement « non attesté », `英雄` compris — **le même filtre trop
  strict** que celui qui avait fait disparaître les 54 occurrences de `プロリー`.
  L'ancrage correct est le corpus brut entier, 5 987 722 signes.
- Même corrigé, le taux reste trop bas pour un dépôt automatique.

**PP-OCR ne restitue pas la mise en page** — ni l'ordre de lecture, ni les
titres markdown. Remplacer une transcription entière par sa relecture
détruirait la structure que le corpus a acquise. Ce n'est donc pas un
remplaçant de dots.ocr, c'est un **second avis**.

### La voie viable

Non pas un correcteur automatique, mais une **seconde source affichée au
relecteur**. `/admin/databooks/<id>` montre déjà le scan à côté du texte ;
y ajouter la lecture PP-OCR change un déchiffrage en un choix d'une seconde,
sur les 763 planches à intrusion comme sur les 30 renvoyées en relecture.

Le principe de la maison est intact — **le moteur propose, le dump tranche** —
mais la proposition vaut désormais quelque chose : c'est une lecture optique
indépendante, pas une plausibilité linguistique.

---

## 11. Le service de seconde lecture — livré le 2026-08-25

La recommandation du § 10 est en place : le relecteur affiche désormais deux
lectures du même scan, pas une seule plus un scan à déchiffrer.

### Ce qui tourne

| Pièce | Où |
|---|---|
| Service résident | `shenron-relecture-ocr.service` (systemd), `127.0.0.1:8791` |
| Code | `shenron:services/relecture-ocr/serveur.py` |
| Endpoint | `GET /api/databooks/:id/relecture-ocr?planche=N` (admin ou jeton) |
| Interface | panneau « Seconde lecture » dans `/admin/databooks/<id>` |

**Mesuré sur le VPS** : 3,8 s pour une planche jamais lue, **6 ms depuis le
cache** — soit deux fois plus rapide que sur le poste local, sans GPU. Le cache
est indexé par **empreinte SHA-256 du scan** : remplacer une image la fait
relire, la renommer ne la fait pas relire deux fois.

### Trois choix qui méritent d'être expliqués

**À la demande, pas automatique.** Un bouton déclenche la lecture. Charger
chaque planche visitée coûterait quelques secondes de processeur sur la machine
qui sert *aussi* le site — pour un second avis dont le relecteur n'a pas besoin
à chaque page.

**Aucun dépôt automatique.** Le panneau propose d'ajouter une région au texte,
région par région, jamais de remplacer la transcription. PP-OCR ne restitue pas
la mise en page : ses régions sortent dans l'ordre du détecteur, pas dans
l'ordre de lecture japonais. Remplacer d'un bloc détruirait la structure
markdown que le corpus a acquise.

**Le service est optionnel.** S'il est arrêté, l'endpoint répond `200` avec
`indisponible: true` plutôt qu'une erreur : la relecture continue sans le
second avis. Un outil d'aide qui casse l'outil principal ne vaut rien.

### Confinement

Le service n'écoute que la boucle locale, refuse tout chemin sortant de
`public/` (vérifié : `/etc/passwd` → 403), et tourne sous systemd avec
`ProtectSystem=strict`, `ProtectHome=read-only` et un seul `ReadWritePaths`
vers son cache.

### Un piège rencontré, pour mémoire

La première version tenait **un seul verrou** pour le chargement des modèles et
pour l'inférence. `lire()` le prenait, puis appelait `moteur()` qui le reprenait
— et un `threading.Lock` n'est pas réentrant. Interblocage franc : la requête
ne rendait jamais la main, `/sante` continuait d'annoncer `charge: false`, et
rien dans les journaux ne parlait de verrou. Deux verrous distincts, et le
chargement fait **avant** de prendre celui de l'inférence.
