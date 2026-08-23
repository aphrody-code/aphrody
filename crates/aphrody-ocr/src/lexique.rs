// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Correction par lexique fermé : remplacer une forme fautive mesurée par la
//! forme juste, et seulement celle-là.
//!
//! # Pourquoi un lexique plutôt qu'une règle
//!
//! Les autres passes de ce crate raisonnent : elles reconnaissent une écriture,
//! interrogent un dictionnaire, comptent des répétitions. Celle-ci ne raisonne
//! pas — elle applique une table établie en comptant le corpus réel. C'est le
//! bon outil pour une classe précise de fautes : celles qui portent sur des
//! **noms propres**, que par construction aucun dictionnaire ne connaît.
//!
//! Un modèle de vision confond régulièrement une consonne sourde et sa sonore,
//! parce que le dakuten est deux petits traits en haut à droite d'un kana :
//! `プ`/`ブ`, `ビ`/`ピ`, `コ`/`ゴ`. Sur un mot courant, le dictionnaire tranche.
//! Sur `ブロリー` il n'a rien à dire, et la faute passe.
//!
//! # Ce que ça vaut, mesuré
//!
//! Audit du 2026-08-22 sur les **6 305 planches transcrites** de
//! dragonballfr.com : **479 planches** portent au moins une de ces neuf fautes,
//! pour **969 occurrences**. C'est la famille de défauts la plus volumineuse
//! parmi celles qu'on peut corriger sans relire l'image.
//!
//! # La garde, et pourquoi elle n'est pas décorative
//!
//! Deux planches du corpus — celles qui expliquent l'étymologie du nom de
//! Végéta — écrivent `ベジタブル`, « vegetable », d'où le personnage tire son
//! nom. Une règle `ベジタ → ベジータ` sans garde détruirait exactement le
//! passage qui justifie la graphie. D'où [`Entree::interdits`] : la
//! substitution est refusée si l'un de ces suffixes suit. Chaque garde
//! correspond à une collision **relevée dans le corpus ou vérifiée absente**,
//! pas à une précaution imaginée.

/// Une forme fautive et sa correction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entree {
    /// Ce que le modèle rend.
    pub fautif: &'static str,
    /// Ce que la planche porte réellement.
    pub juste: &'static str,
    /// Suffixes qui font de `fautif` un mot légitime, et interdisent alors la
    /// substitution.
    pub interdits: &'static [&'static str],
}

/// Une correction effectuée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remplacement {
    /// Position dans le texte d'origine, en octets.
    pub debut: usize,
    /// La forme fautive remplacée.
    pub avant: String,
    /// Ce qui a été écrit à la place.
    pub apres: String,
}

/// Un terme du vocabulaire de la série, dans sa graphie attestée.
///
/// Sert de forme de référence : ce n'est pas une faute à corriger mais une
/// graphie **juste**, contre laquelle on peut mesurer une graphie douteuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terme {
    /// La graphie japonaise attestée.
    pub japonais: String,
    /// Sa transcription en rōmaji Hepburn.
    pub romaji: String,
    /// La forme française officielle.
    pub francais: String,
    /// Personnage, technique, planète, race…
    pub categorie: String,
    /// Autres graphies attestées du même terme. Une variante est juste, elle
    /// aussi : la corriger vers la forme principale serait une faute.
    pub variantes: Vec<String>,
    /// Faux quand une substitution d'un seul caractère mènerait à un mot
    /// japonais courant. Ces termes-là restent dans la table — c'est ce qui
    /// rend l'ambiguïté visible — mais ne sont jamais une cible de correction.
    pub corrigeable: bool,
}

/// Une table de corrections applicable à un texte.
#[derive(Debug, Clone)]
pub struct Lexique {
    entrees: Vec<Entree>,
    termes: Vec<Terme>,
}

impl Lexique {
    /// Le lexique mesuré sur les databooks Dragon Ball.
    ///
    /// Chaque entrée vient d'un comptage sur le corpus déposé, avec le nombre
    /// de planches touchées en commentaire. Les collisions ont été cherchées :
    /// une entrée sans garde est une entrée dont la forme fautive n'apparaît
    /// **jamais** légitimement dans le corpus.
    #[must_use]
    pub fn databooks_dragon_ball() -> Self {
        Self {
            termes: termes_tsv(DRAGON_BALL_TSV),
            entrees: vec![
                // ---------------------------------------------------------
                // Le vocabulaire ordinaire que le scan abime.
                //
                // Ces mots ne sont pas de la serie — ce sont ceux des pages de
                // jeux, d'editeurs et de cartes qui remplissent les databooks.
                // Aucun lexique Dragon Ball ne pouvait les atteindre, et une
                // regle par dictionnaire non plus : la §9.9 du pont raconte
                // comment IPADIC, essaye pour ca, a detruit Frost, Guldo et le
                // roi Cold. Reste la table fermee, mesuree une paire a la fois.
                //
                // La colonne qui tranche est le nombre de fois ou la BONNE
                // graphie apparait dans le meme corpus : elle prouve que la
                // fautive est un defaut de lecture et non un mot.
                // ---------------------------------------------------------
                // 446 planches, 1198 occurrences ; `バトル` vu 1658 fois.
                Entree { fautif: "パトル", juste: "バトル", interdits: &[] },
                // 271 planches, 958 occurrences. Le modele se trompe plus
                // souvent qu'il ne reussit : `アビリティ` n'est vu que 74 fois.
                Entree { fautif: "アピリティ", juste: "アビリティ", interdits: &[] },
                // 334 planches, 657 occurrences ; `バンダイ` vu 182 fois.
                Entree { fautif: "パンダイ", juste: "バンダイ", interdits: &[] },
                // 58 planches, 201 occurrences ; `パワー` vu 829 fois.
                Entree { fautif: "バワー", juste: "パワー", interdits: &[] },
                // 69 planches, 100 occurrences. La garde est obligatoire : le
                // corpus ecrit `スーパードコンポールヒローズ` et
                // `ドラコンポール`, ou `コンポ` est un debris de
                // `ドラゴンボール` et non un `コンボ`. Corriger la sourde y
                // aggraverait la faute au lieu de la reparer.
                Entree { fautif: "コンポ", juste: "コンボ", interdits: &["ール"] },
                // 10 planches, 68 occurrences ; `ワンピース` vu 32 fois.
                Entree { fautif: "ワンビース", juste: "ワンピース", interdits: &[] },
                // 37 planches, 53 occurrences ; `ビクトリー` vu 180 fois.
                Entree { fautif: "ピクトリー", juste: "ビクトリー", interdits: &[] },
                // 14 planches, 44 occurrences. Garde contre `ハワード`, que le
                // corpus ne porte pas mais qu'une page d'interview porterait.
                // Le suffixe est `ド` et non `ード` : la voyelle longue fait
                // deja partie de la forme fautive, et l'ecrire `ード` rendait
                // la garde inoperante — un test l'a montre.
                Entree { fautif: "ハワー", juste: "パワー", interdits: &["ド"] },
                // 19 planches, 37 occurrences ; `ジャンプ` vu 2008 fois.
                Entree { fautif: "ジャンフ", juste: "ジャンプ", interdits: &[] },
                // 29 planches, 33 occurrences.
                Entree { fautif: "ポリューム", juste: "ボリューム", interdits: &[] },
                // 22 planches, 29 occurrences ; `ダメージ` vu 909 fois.
                Entree { fautif: "タメージ", juste: "ダメージ", interdits: &[] },
                // 10 planches, 11 occurrences. A ne pas confondre avec
                // `ブロリー` : ce sont deux mots differents.
                Entree { fautif: "プロッコリー", juste: "ブロッコリー", interdits: &[] },
                // 6 planches, 8 occurrences.
                Entree { fautif: "バッケージ", juste: "パッケージ", interdits: &[] },
                // 8 planches, 8 occurrences. Garde contre `インテックス大阪`,
                // le centre d'exposition, qui est bien ecrit ainsi.
                Entree { fautif: "インテックス", juste: "インデックス", interdits: &["大阪"] },
                // 5 et 2 planches. Captain Tsubasa, hors univers Dragon Ball —
                // le cas qui avait motive la regle par dictionnaire.
                Entree { fautif: "キャブテン", juste: "キャプテン", interdits: &[] },
                Entree { fautif: "キャフテン", juste: "キャプテン", interdits: &[] },
                // 5 planches, 6 occurrences.
                Entree { fautif: "ピキニ", juste: "ビキニ", interdits: &[] },
                // 4 planches, 5 occurrences.
                Entree { fautif: "トリフル", juste: "トリプル", interdits: &[] },
                // 2 planches, 3 occurrences.
                Entree { fautif: "アッフ", juste: "アップ", interdits: &[] },
                // 2 planches, 2 occurrences.
                Entree { fautif: "ヒクトリー", juste: "ビクトリー", interdits: &[] },
                // 1 planche. Faible, mais sans ambiguite.
                Entree { fautif: "カギシャポン", juste: "ガシャポン", interdits: &[] },
                //
                // ECARTES, et pourquoi :
                //
                // `テッキ -> デッキ` (11 occurrences) — juste partout dans le
                // corpus (`使用テッキ`, `スターターテッキ`), mais `ステッキ`
                // est un mot et le risque est en PREFIXE, que `interdits` ne
                // sait pas garder. Onze occurrences ne valent pas d'elargir le
                // modele de donnees.
                //
                // `パッチリ -> バッチリ` (6 occurrences) — `ぱっちり` existe et
                // se rencontre en katakana pour insister. Trop peu pour prendre
                // le risque.
                //
                // ---------------------------------------------------------
                // 111 planches, 280 occurrences. `プロリー` n'est pas un mot.
                Entree { fautif: "プロリー", juste: "ブロリー", interdits: &[] },
                // 111 planches, 204 occurrences.
                Entree { fautif: "ビッコロ", juste: "ピッコロ", interdits: &[] },
                // 93 planches, 135 occurrences.
                Entree { fautif: "ドラコンボール", juste: "ドラゴンボール", interdits: &[] },
                // 68 planches, 113 occurrences. `フルマラソン` (le marathon) et
                // `フルマカモメ` (le fulmar) existent en japonais ; ni l'un ni
                // l'autre n'apparaît dans le corpus, mais la garde ne coûte
                // rien et un ouvrage sportif en contiendrait.
                Entree {
                    fautif: "フルマ",
                    juste: "ブルマ",
                    interdits: &["ラソン", "カモメ"],
                },
                // 57 planches, 102 occurrences. La garde est ici obligatoire :
                // deux planches du corpus écrivent `ベジタブル` en expliquant
                // d'où vient le nom du personnage.
                Entree {
                    fautif: "ベジタ",
                    juste: "ベジータ",
                    interdits: &["ブル", "リアン"],
                },
                // 36 planches, 59 occurrences.
                Entree { fautif: "ペジータ", juste: "ベジータ", interdits: &[] },
                // 21 planches, 50 occurrences.
                Entree { fautif: "ベージータ", juste: "ベジータ", interdits: &[] },
                // 8 planches, 14 occurrences. `フリーサイズ` (taille unique)
                // est un mot japonais courant, absent du corpus.
                Entree { fautif: "フリーサ", juste: "フリーザ", interdits: &["イズ"] },
                // 8 planches, 12 occurrences.
                Entree { fautif: "ヘジータ", juste: "ベジータ", interdits: &[] },
            ],
        }
    }

    /// Un lexique vide.
    #[must_use]
    pub const fn vide() -> Self {
        Self { entrees: Vec::new(), termes: Vec::new() }
    }

    /// Les entrées de ce lexique.
    #[must_use]
    pub fn entrees(&self) -> &[Entree] {
        &self.entrees
    }

    /// Les formes de référence de ce lexique.
    #[must_use]
    pub fn termes(&self) -> &[Terme] {
        &self.termes
    }

    /// Corrige les mots en katakana qui ne diffèrent d'un terme du lexique que
    /// par **un seul caractère**.
    ///
    /// C'est la deuxième moitié de ce module, et la plus délicate. Un modèle de
    /// vision qui bute sur un kana en rend un autre : `トラククス` pour
    /// `トランクス`, `ワーロン` pour `ウーロン`. Le dictionnaire japonais ne
    /// peut rien y faire — ce sont des noms propres, il les ignore tous les
    /// deux. Un lexique fermé, lui, sait.
    ///
    /// # La règle qui rend ça sûr, et pourquoi elle n'est pas évidente
    ///
    /// « À une substitution d'un terme du lexique, donc c'est ce terme » est
    /// **faux**, et le corpus le prouve : `孫悟空`, `孫悟飯` et `孫悟天` sont
    /// mutuellement à distance un, comme le sont les six `人造人間1X号`, les
    /// quatre `X の界王神`, et `界王拳`/`界王星`/`界王神`. Cinquante-quatre
    /// paires du lexique collisionnent ainsi. Appliquée sans garde, la règle
    /// changerait Gohan en Goku, en silence, sur un corpus public.
    ///
    /// D'où : la correction n'a lieu que si **exactement un** terme est à
    /// distance un. Deux candidats, c'est un doute, et un doute ne se corrige
    /// pas tout seul.
    ///
    /// Trois autres gardes, chacune fermant une porte différente :
    ///
    /// - **Katakana pur uniquement.** Les entrées à kanji sont écartées : ce
    ///   sont elles qui portent l'essentiel des collisions, et un mot en kanji
    ///   n'a pas de frontière franche dans une phrase japonaise.
    /// - **Le mot doit être une suite complète de katakana.** Le japonais
    ///   délimite ses mots en katakana par le changement d'écriture ; corriger
    ///   un fragment reviendrait à parier sur l'endroit où le mot commence.
    /// - **Quatre caractères au minimum.** En deçà, une substitution tombe trop
    ///   facilement sur un mot japonais courant — `パン` est le pain autant que
    ///   le personnage.
    #[must_use]
    pub fn corrige_par_distance(&self, texte: &str) -> (String, Vec<Remplacement>) {
        let references = self.references_katakana();
        let mut out = String::with_capacity(texte.len());
        let mut faits = Vec::new();

        for (debut, mot, katakana) in suites(texte) {
            if !katakana {
                out.push_str(mot);
                continue;
            }
            match self.voisin_unique(mot, &references) {
                Some(juste) => {
                    faits.push(Remplacement {
                        debut,
                        avant: mot.to_owned(),
                        apres: juste.clone(),
                    });
                    out.push_str(&juste);
                }
                None => out.push_str(mot),
            }
        }

        (out, faits)
    }

    /// Toutes les graphies katakana justes, variantes comprises.
    fn references_katakana(&self) -> Vec<&str> {
        self.termes
            .iter()
            .filter(|t| t.corrigeable)
            .flat_map(|t| {
                core::iter::once(t.japonais.as_str())
                    .chain(t.variantes.iter().map(String::as_str))
            })
            .filter(|forme| katakana_pur(forme) && forme.chars().count() >= DISTANCE_MINIMUM)
            .collect()
    }

    /// Le terme juste, s'il y en a **un seul** à une substitution près.
    fn voisin_unique(&self, mot: &str, references: &[&str]) -> Option<String> {
        let longueur = mot.chars().count();
        if longueur < DISTANCE_MINIMUM {
            return None;
        }
        let mut trouve: Option<&str> = None;
        for reference in references {
            if *reference == mot {
                // La graphie est déjà juste : rien à corriger, et surtout rien
                // à chercher plus loin.
                return None;
            }
            if !distance_un(mot, reference) {
                continue;
            }
            match trouve {
                // Deux candidats, donc un doute. Le corpus a mesuré 54 paires
                // dans ce cas ; y choisir au hasard vaudrait pire que de ne
                // rien faire.
                Some(deja) if deja != *reference => return None,
                Some(_) => {}
                None => trouve = Some(reference),
            }
        }
        trouve.map(ToOwned::to_owned)
    }

    /// Applique la table à un texte.
    ///
    /// Rend le texte corrigé et la liste de ce qui a changé — la liste, pas un
    /// compteur : une correction automatique sur un corpus public doit pouvoir
    /// être relue entrée par entrée.
    ///
    /// Les formes les plus longues sont essayées d'abord, pour qu'une entrée
    /// courte n'ampute pas une entrée longue qui la contiendrait.
    #[must_use]
    pub fn applique<'a>(&'a self, texte: &str) -> (String, Vec<Remplacement>) {
        let mut ordre: Vec<&'a Entree> = self.entrees.iter().collect();
        ordre.sort_by_key(|e| core::cmp::Reverse(e.fautif.len()));

        let mut out = String::with_capacity(texte.len());
        let mut faits = Vec::new();
        let mut i = 0;

        while i < texte.len() {
            let reste = &texte[i..];
            let trouve = ordre.iter().find(|entree| {
                reste.starts_with(entree.fautif)
                    && !entree
                        .interdits
                        .iter()
                        .any(|suffixe| reste[entree.fautif.len()..].starts_with(suffixe))
            });

            if let Some(entree) = trouve {
                faits.push(Remplacement {
                    debut: i,
                    avant: entree.fautif.to_owned(),
                    apres: entree.juste.to_owned(),
                });
                out.push_str(entree.juste);
                i += entree.fautif.len();
                continue;
            }

            // Avancer d'un caractère entier : un index au milieu d'un point de
            // code ferait paniquer le `starts_with` du tour suivant.
            let large = reste.chars().next().map_or(1, char::len_utf8);
            out.push_str(&reste[..large]);
            i += large;
        }

        (out, faits)
    }
}

impl Default for Lexique {
    fn default() -> Self {
        Self::databooks_dragon_ball()
    }
}

/// Le lexique Dragon Ball, compilé dans le binaire.
///
/// Établi contre le wiki de dragonballfr.com puis **vérifié terme à terme
/// contre le corpus des databooks** : une graphie que le corpus n'atteste pas
/// a été retirée, même quand elle semblait correcte de mémoire — `気孔砲`,
/// `惑星ナメック`, `亀ハウス` sortent tous à zéro occurrence.
const DRAGON_BALL_TSV: &str = include_str!("../data/dragon-ball.tsv");

/// Longueur minimale d'un mot pour qu'une correction à distance un soit sûre.
const DISTANCE_MINIMUM: usize = 4;

/// Découpe un texte en suites de katakana et en tout le reste.
///
/// Rend `(offset en octets, tranche, est_une_suite_de_katakana)`. Les suites
/// sont maximales : c'est ce qui donne au mot ses frontières, puisque le
/// japonais délimite un mot en katakana par le changement d'écriture et non
/// par une espace.
fn suites(texte: &str) -> Vec<(usize, &str, bool)> {
    let mut out = Vec::new();
    let mut debut = 0;
    let mut dans_du_katakana = None;

    for (offset, c) in texte.char_indices() {
        let katakana = crate::kana::katakana(c);
        match dans_du_katakana {
            Some(precedent) if precedent == katakana => {}
            Some(precedent) => {
                out.push((debut, &texte[debut..offset], precedent));
                debut = offset;
            }
            None => debut = offset,
        }
        dans_du_katakana = Some(katakana);
    }
    if let Some(precedent) = dans_du_katakana {
        out.push((debut, &texte[debut..], precedent));
    }
    out
}

/// Une graphie n'est-elle faite que de katakana ?
fn katakana_pur(forme: &str) -> bool {
    !forme.is_empty() && forme.chars().all(crate::kana::katakana)
}

/// Deux mots diffèrent-ils par exactement un caractère, à longueur égale ?
///
/// Pas d'insertion ni de suppression : la faute visée est un caractère **mal
/// lu**, pas un caractère perdu. Élargir à la distance d'édition complète
/// ferait entrer `ベジタ` et `ベジータ` dans le même voisinage, or l'un est un
/// mot japonais courant.
fn distance_un(a: &str, b: &str) -> bool {
    let mut differences = 0;
    let mut ca = a.chars();
    let mut cb = b.chars();
    loop {
        match (ca.next(), cb.next()) {
            (None, None) => return differences == 1,
            (Some(x), Some(y)) => {
                if x != y {
                    differences += 1;
                    if differences > 1 {
                        return false;
                    }
                }
            }
            // Longueurs différentes : hors du champ de cette mesure.
            _ => return false,
        }
    }
}

/// Lit le lexique au format TSV.
///
/// Tolérant : une ligne mal formée est ignorée plutôt que de faire échouer le
/// chargement. Le fichier est compilé dans le binaire, donc une ligne cassée
/// est un bug de données à corriger, pas une raison de refuser de démarrer.
fn termes_tsv(tsv: &str) -> Vec<Terme> {
    tsv.lines()
        .skip(1) // l'en-tête
        .filter_map(|ligne| {
            let champs: Vec<&str> = ligne.split('\t').collect();
            let [japonais, romaji, francais, categorie, variantes, corrigeable] = champs[..] else {
                return None;
            };
            if japonais.is_empty() {
                return None;
            }
            Some(Terme {
                japonais: japonais.to_owned(),
                romaji: romaji.to_owned(),
                francais: francais.to_owned(),
                categorie: categorie.to_owned(),
                variantes: variantes
                    .split('|')
                    .filter(|v| !v.is_empty())
                    .map(ToOwned::to_owned)
                    .collect(),
                corrigeable: corrigeable.trim() == "oui",
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_neuf_fautes_mesurees_sont_corrigees() {
        let lexique = Lexique::databooks_dragon_ball();
        for (fautif, juste) in [
            ("プロリー", "ブロリー"),
            ("ビッコロ", "ピッコロ"),
            ("ドラコンボール", "ドラゴンボール"),
            ("フルマ", "ブルマ"),
            ("ベジタ", "ベジータ"),
            ("ペジータ", "ベジータ"),
            ("ベージータ", "ベジータ"),
            ("フリーサ", "フリーザ"),
            ("ヘジータ", "ベジータ"),
        ] {
            let (out, faits) = lexique.applique(fautif);
            assert_eq!(out, juste, "{fautif}");
            assert_eq!(faits.len(), 1, "{fautif} : {faits:?}");
            assert_eq!(faits[0].debut, 0);
        }
    }

    #[test]
    fn le_passage_qui_explique_letymologie_de_vegeta_survit() {
        // Le contre-exemple qui justifie toute la mécanique de gardes : deux
        // planches du corpus écrivent `ベジタブル` pour dire d'où vient le nom.
        // Les corriger détruirait précisément l'explication.
        let lexique = Lexique::databooks_dragon_ball();
        for texte in ["「ベジタブル」から", "ベジタブル = ベジ", "ベジタリアンの食事"] {
            let (out, faits) = lexique.applique(texte);
            assert_eq!(out, texte, "{texte}");
            assert!(faits.is_empty(), "{texte} : {faits:?}");
        }
    }

    #[test]
    fn les_mots_japonais_courants_sous_garde_survivent() {
        let lexique = Lexique::databooks_dragon_ball();
        for texte in ["フルマラソンに挑戦", "フルマカモメ", "フリーサイズのTシャツ"] {
            let (out, faits) = lexique.applique(texte);
            assert_eq!(out, texte, "{texte}");
            assert!(faits.is_empty(), "{texte} : {faits:?}");
        }
    }

    #[test]
    fn une_planche_qui_porte_les_deux_graphies_est_unifiee() {
        // Cas réel, planche 140/p14 : la faute et la forme juste sur la même
        // ligne — la preuve que c'est une lecture et non une convention.
        let (out, faits) = Lexique::databooks_dragon_ball()
            .applique("デザインも超クールだ!! プロリー SDV8- DPUR5 ブロリー");
        assert_eq!(out, "デザインも超クールだ!! ブロリー SDV8- DPUR5 ブロリー");
        assert_eq!(faits.len(), 1);
    }

    #[test]
    fn plusieurs_fautes_dans_une_meme_ligne_partent_ensemble() {
        // Cas réel, planche 140/p6.
        let (out, faits) =
            Lexique::databooks_dragon_ball().applique("孫悟空／ビッコロ／プロリー／フルマ");
        assert_eq!(out, "孫悟空／ピッコロ／ブロリー／ブルマ");
        assert_eq!(faits.len(), 3, "{faits:?}");
    }

    #[test]
    fn la_forme_longue_gagne_sur_la_forme_courte() {
        // `ベージータ` et `ベジタ` visent la même correction ; si la courte
        // était essayée d'abord elle ne matcherait pas, mais l'ordre doit être
        // garanti plutôt que fortuit.
        let (out, faits) = Lexique::databooks_dragon_ball().applique("ベージータの誇り");
        assert_eq!(out, "ベジータの誇り");
        assert_eq!(faits.len(), 1);
        assert_eq!(faits[0].avant, "ベージータ");
    }

    #[test]
    fn un_texte_sans_faute_traverse_intact_et_sans_realloc_inutile() {
        let texte = "孫悟空とベジータはナメック星で戦った。DRAGON BALL 1989";
        let (out, faits) = Lexique::databooks_dragon_ball().applique(texte);
        assert_eq!(out, texte);
        assert!(faits.is_empty());
    }

    #[test]
    fn le_tsv_embarque_se_lit_entierement() {
        let lexique = Lexique::databooks_dragon_ball();
        let termes = lexique.termes();
        assert!(termes.len() > 300, "{} termes lus", termes.len());
        // Un échantillon dont on sait ce qu'il doit contenir.
        let goku = termes.iter().find(|t| t.japonais == "孫悟空").expect("孫悟空");
        assert_eq!(goku.francais, "Son Goku");
        assert_eq!(goku.categorie, "personnage");
        assert_eq!(goku.variantes, vec!["悟空".to_owned()]);
        assert!(goku.corrigeable);
        // Et un piège, exclu de la correction automatique.
        let pan = termes.iter().find(|t| t.japonais == "パン").expect("パン");
        assert!(!pan.corrigeable, "パン est aussi le pain");
    }

    #[test]
    fn un_nom_propre_a_un_kana_pres_est_corrige() {
        // Cas réels du corpus : le modèle a lu un kana pour un autre.
        let lexique = Lexique::databooks_dragon_ball();
        let (out, faits) = lexique.corrige_par_distance("404 超サイヤ人トラククス(Y)");
        assert_eq!(out, "404 超サイヤ人トランクス(Y)", "{faits:?}");
        assert_eq!(faits.len(), 1);
        assert_eq!(faits[0].avant, "トラククス");
        assert_eq!(faits[0].apres, "トランクス");
    }

    #[test]
    fn un_nom_deja_juste_nest_pas_touche() {
        let lexique = Lexique::databooks_dragon_ball();
        for texte in ["トランクスとゴテンクス", "ベジータの誇り", "フリーザ第一形態"] {
            let (out, faits) = lexique.corrige_par_distance(texte);
            assert_eq!(out, texte, "{faits:?}");
            assert!(faits.is_empty(), "{texte} : {faits:?}");
        }
    }

    #[test]
    fn deux_candidats_a_distance_un_font_renoncer() {
        // La garde que le corpus a rendue obligatoire : `ワーロン` est à une
        // substitution de `ウーロン` ET de `マーロン`. Choisir serait parier.
        let lexique = Lexique::databooks_dragon_ball();
        let (out, faits) = lexique.corrige_par_distance("ワーロンのようなタイプ");
        assert_eq!(out, "ワーロンのようなタイプ");
        assert!(faits.is_empty(), "{faits:?}");
    }

    #[test]
    fn un_mot_trop_court_nest_jamais_corrige_a_distance() {
        // Trois caractères suffisent à tomber sur un vrai mot japonais.
        let lexique = Lexique::databooks_dragon_ball();
        for texte in ["カビト", "ヤシの木", "ミート", "パン"] {
            let (out, faits) = lexique.corrige_par_distance(texte);
            assert_eq!(out, texte, "{texte}");
            assert!(faits.is_empty(), "{texte} : {faits:?}");
        }
    }

    #[test]
    fn un_fragment_de_mot_plus_long_nest_pas_corrige() {
        // La correction ne porte que sur une suite COMPLÈTE de katakana. Sans
        // cette borne, un préfixe de `ドラゴンボール` pourrait être réécrit
        // vers un autre terme du lexique.
        let lexique = Lexique::databooks_dragon_ball();
        let texte = "スーパードラゴンボールヒーローズ";
        let (out, faits) = lexique.corrige_par_distance(texte);
        assert_eq!(out, texte, "{faits:?}");
    }

    #[test]
    fn du_japonais_ordinaire_traverse_la_distance_intact() {
        let lexique = Lexique::databooks_dragon_ball();
        for texte in [
            "今日はカメラで写真を撮りました。",
            "テレビアニメの放送が始まる",
            "DRAGON BALL 1989 BIRD STUDIO",
            "",
        ] {
            let (out, faits) = lexique.corrige_par_distance(texte);
            assert_eq!(out, texte, "{texte} : {faits:?}");
        }
    }

    #[test]
    fn les_collisions_du_lexique_ne_produisent_aucune_correction() {
        // Le résultat qui a dicté la règle : 54 paires du lexique sont à
        // distance un les unes des autres. Aucune ne doit être « corrigée »
        // vers une autre — Gohan ne devient pas Goku.
        let lexique = Lexique::databooks_dragon_ball();
        for terme in lexique.termes().iter().filter(|t| t.corrigeable) {
            let (out, faits) = lexique.corrige_par_distance(&terme.japonais);
            assert_eq!(out, terme.japonais, "{} réécrit en {out} : {faits:?}", terme.japonais);
        }
    }

    #[test]
    fn les_variantes_attestees_sont_des_graphies_justes() {
        // `フリーザー` est attesté 82 fois dans le corpus : le ramener à
        // `フリーザ` détruirait une graphie que l'éditeur a réellement imprimée.
        let lexique = Lexique::databooks_dragon_ball();
        for terme in lexique.termes() {
            for variante in &terme.variantes {
                let (out, _) = lexique.corrige_par_distance(variante);
                assert_eq!(&out, variante, "variante {variante} de {}", terme.japonais);
            }
        }
    }

    #[test]
    fn la_distance_un_ne_compte_que_les_substitutions() {
        assert!(distance_un("トラククス", "トランクス"));
        assert!(!distance_un("トランクス", "トランクス"), "identique n'est pas à distance un");
        assert!(!distance_un("ベジタ", "ベジータ"), "longueurs différentes");
        assert!(!distance_un("アイウエオ", "カキクケコ"), "cinq différences");
    }

    #[test]
    fn le_decoupage_en_suites_reconstitue_le_texte() {
        for texte in ["孫悟空とベジータ", "ドラゴンボール", "abc", "", "ー", "あアa亜"] {
            let recolle: String = suites(texte).iter().map(|(_, s, _)| *s).collect();
            assert_eq!(recolle, texte, "{texte}");
            for (debut, tranche, _) in suites(texte) {
                assert_eq!(&texte[debut..debut + tranche.len()], tranche);
            }
        }
    }

    #[test]
    fn un_lexique_vide_ne_touche_a_rien() {
        let texte = "プロリーとビッコロ";
        let vide = Lexique::vide();
        let (out, faits) = vide.applique(texte);
        assert_eq!(out, texte);
        assert!(faits.is_empty());
        let (out, faits) = vide.corrige_par_distance("トラククス");
        assert_eq!(out, "トラククス");
        assert!(faits.is_empty());
    }

    #[test]
    fn le_parcours_ne_coupe_jamais_un_caractere_en_deux() {
        // Le texte avance en octets ; un pas d'un octet sur du japonais
        // ferait paniquer le `starts_with` suivant, ou pire, découperait un
        // point de code. Un émoji hors du plan de base est le cas extrême.
        let texte = "🐉ドラコンボール🐉あ";
        let (out, _) = Lexique::databooks_dragon_ball().applique(texte);
        assert_eq!(out, "🐉ドラゴンボール🐉あ");
    }

    #[test]
    fn les_positions_rendues_pointent_bien_sur_la_faute() {
        let texte = "これはプロリーです";
        let (_, faits) = Lexique::databooks_dragon_ball().applique(texte);
        assert_eq!(faits.len(), 1);
        assert!(texte[faits[0].debut..].starts_with("プロリー"), "{faits:?}");
    }
}
#[cfg(test)]
mod tests_vocabulaire_ordinaire {
    use super::*;

    #[test]
    fn les_mots_de_jeu_les_plus_abimes_sont_retablis() {
        let lex = Lexique::databooks_dragon_ball();
        for (source, attendu) in [
            ("パトル開始時", "バトル開始時"),
            ("カードアクションアピリティ", "カードアクションアビリティ"),
            ("メーカー: パンダイナムコ", "メーカー: バンダイナムコ"),
            ("バワースピード", "パワースピード"),
            ("キャブテン翼 FCG", "キャプテン翼 FCG"),
            ("ワンビース", "ワンピース"),
        ] {
            let (texte, _) = lex.applique(source);
            assert_eq!(texte, attendu, "depuis {source}");
        }
    }

    #[test]
    fn un_debris_de_dragon_ball_nest_pas_pris_pour_un_combo() {
        // La garde qui compte. Le corpus ecrit `スーパードコンポールヒローズ`
        // et `ドラコンポール` : la ou `コンポ` est suivi de `ール`, c'est un
        // `ドラゴンボール` massacre, pas un `コンボ`. Le corriger aggraverait.
        let lex = Lexique::databooks_dragon_ball();
        for source in ["スーパードコンポールヒローズ", "ドラコンポールがついている"] {
            let (texte, remplacements) = lex.applique(source);
            assert_eq!(texte, source, "{source} doit traverser intact");
            assert!(remplacements.is_empty());
        }
        // Sans le suffixe, la correction se fait.
        let (texte, _) = lex.applique("スキルやコンポを決めて");
        assert_eq!(texte, "スキルやコンボを決めて");
    }

    #[test]
    fn les_homonymes_gardes_traversent() {
        let lex = Lexique::databooks_dragon_ball();
        for source in ["インテックス大阪", "ハワード氏の証言"] {
            let (texte, remplacements) = lex.applique(source);
            assert_eq!(texte, source, "{source} est legitime");
            assert!(remplacements.is_empty(), "{source}");
        }
    }

    #[test]
    fn les_formes_ecartees_ne_sont_pas_dans_la_table() {
        // `テッキ` et `パッチリ` ont ete mesures puis ecartes faute de garde
        // possible. Ce test fige la decision : les rajouter sans resoudre le
        // probleme de prefixe le fera echouer.
        let lex = Lexique::databooks_dragon_ball();
        for source in ["ステッキを持つ", "パッチリした目"] {
            let (texte, _) = lex.applique(source);
            assert_eq!(texte, source, "{source} doit traverser intact");
        }
    }
}
