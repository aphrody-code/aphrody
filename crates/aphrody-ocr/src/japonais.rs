//! Remise en forme du japonais, guidée par l'analyse morphologique.
//!
//! Le japonais s'écrit sans espaces, ce qui prive le nettoyage générique de son
//! repère principal : sans savoir où finissent les mots, on ne peut ni recoller
//! une ligne coupée en plein milieu de l'un d'eux, ni dire si une lettre latine
//! posée entre deux kanji est un mot légitime ou une hallucination du modèle.
//! D'où lindera, portage Rust de Kuromoji, avec IPADIC compilé dans le binaire.
//!
//! Ce qui ne demande aucun dictionnaire — reconnaître une écriture, ramener la
//! demi-chasse en pleine chasse, retirer une espace entre deux kanji — vit dans
//! [`crate::kana`], qui compile sans la feature `japanese` et pour wasm.
//!
//! CE QUE ÇA CORRIGE, MESURÉ sur 5762 planches de databooks déjà lues :
//!
//! - **50 725 coupures de ligne tombent entre deux caractères japonais**, soit
//!   15,6 % de toutes les coupures. Elles viennent du texte en colonnes : le
//!   modèle rend un saut de ligne là où la page revenait à la ligne, au milieu
//!   d'un mot. `よろしか\nたら` doit se lire d'un seul tenant.
//! - **4 538 intrusions de caractères étrangers** dans un mot japonais, sur
//!   1814 planches — 31,5 % du corpus. `ギновー` pour `ギニュー`,
//!   `実力は特戦隊一Butが`, `Vジャ^nプ` où `^n` remplace `ン`.
//!
//! L'inventaire des écritures vient d'un audit de sept planches faites à
//! l'image : au cyrillique s'ajoutent le hangul (`원`), le devanagari (`ज` au
//! milieu de `ミッション`) et d'autres. Aucune n'a sa place dans un databook
//! japonais, alors que le latin, lui, y figure légitimement — d'où deux
//! critères distincts plutôt qu'un.
//!
//! Les intrusions sont **signalées, pas corrigées** : deviner le caractère juste
//! demanderait de relire l'image, pas le texte. Le signalement suffit à orienter
//! une relecture ciblée, et n'invente rien.
//!
//! # Ce que le dictionnaire permet en plus du signalement
//!
//! Une famille d'erreurs, elle, se corrige sans relire l'image : les **sosies
//! typographiques**. `力` (chikara, la force) et `カ` (le katakana ka) sont deux
//! caractères distincts au dessin quasi identique ; un modèle de vision confond
//! l'un pour l'autre. Au milieu d'un mot en katakana, le kanji est toujours
//! l'erreur — mais seulement si la substitution donne un mot que le
//! dictionnaire connaît, et que la forme d'origine, elle, était inconnue. Cette
//! double condition est ce qui sépare une correction d'une supposition.

use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

use crate::kana::{japonais, katakana};

pub use crate::kana::{espaces_parasites, normalise_demi_chasse};

/// Ce qui peut échouer au chargement du dictionnaire.
#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// IPADIC est compilé dans le binaire ; un échec ici signale un binaire
    /// construit sans la feature, pas un fichier manquant sur le disque.
    #[error("dictionnaire japonais indisponible : {0}")]
    Dictionnaire(String),
}

/// Analyseur morphologique prêt à l'emploi.
///
/// Le chargement du dictionnaire coûte assez cher pour qu'on le fasse une fois
/// et qu'on réutilise l'objet sur tout un lot, pas une fois par planche.
pub struct Analyseur {
    segmenter: Segmenter,
}

/// Un fragment d'écriture étrangère posé au milieu d'un mot japonais.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intrusion {
    /// Position du fragment, en octets depuis le début du texte.
    pub debut: usize,
    /// Le fragment lui-même (`But`, `^n`, `нов`, `원`…).
    pub fragment: String,
    /// Le morceau de texte autour, pour qu'un rapport soit lisible sans le
    /// fichier sous les yeux.
    pub contexte: String,
    /// Vrai quand le fragment relève d'une écriture qu'un databook japonais ne
    /// contient jamais — cyrillique, hangul, devanagari, grec, hébreu, arabe,
    /// thaï. C'est alors une hallucination certaine et non un doute. Le latin,
    /// lui, y figure légitimement : il lui faut un second critère.
    pub certaine: bool,
}

/// Un sosie typographique remplacé par le caractère que le mot appelait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    /// Position du mot corrigé, en octets dans le texte d'origine.
    pub debut: usize,
    /// Ce que le modèle avait rendu.
    pub avant: String,
    /// Ce qui a été écrit à la place.
    pub apres: String,
}

/// Ce que l'analyse morphologique dit de la vraisemblance d'un texte.
///
/// Une planche de bulles manuscrites que le modèle n'a pas su lire ne ressort
/// ni vide ni en boucle : elle ressort en **charabia**, une suite de kana
/// plausibles qui ne forme aucun mot. Aucun filtre de forme ne l'attrape,
/// puisque la forme est correcte. Le dictionnaire, lui, le voit tout de suite :
/// du japonais réel se segmente en mots connus, du charabia en une file de
/// morphèmes inconnus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct Confiance {
    /// Caractères japonais examinés. Le latin, les chiffres et la ponctuation
    /// ne comptent pas : ils ne sont pas dans IPADIC et ne disent rien.
    pub caracteres: usize,
    /// Ceux qui tombent dans un morphème qu'IPADIC ne connaît pas.
    pub inconnus: usize,
    /// Morphèmes japonais découpés, pour le rapport. Ne sert pas au verdict :
    /// voir pourquoi sur [`Confiance::part_inconnue`].
    pub morphemes: usize,
}

impl Confiance {
    /// Part de caractères japonais tombant dans un morphème inconnu, entre 0 et
    /// 1. Vaut 0 sans caractère japonais : une planche en anglais n'est pas du
    /// charabia japonais.
    ///
    /// **En caractères, et non en morphèmes** — mesuré : une file de katakana
    /// que le dictionnaire ne reconnaît pas ne se découpe pas en trente
    /// morphèmes inconnus, elle ressort en **un seul**. Compter les morphèmes
    /// donnait donc `1 inconnu sur 1` à une planche entièrement illisible,
    /// c'est-à-dire un texte trop court pour être jugé — exactement le cas que
    /// cette mesure existe pour attraper.
    #[must_use]
    pub fn part_inconnue(&self) -> f64 {
        if self.caracteres == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        {
            self.inconnus as f64 / self.caracteres as f64
        }
    }

    /// Le texte ressemble-t-il à du charabia plutôt qu'à du japonais ?
    ///
    /// Deux conditions, parce qu'aucune ne suffit seule. Le seuil de 60 % est
    /// haut à dessein : un databook est plein de noms propres inventés par
    /// Toriyama, absents d'IPADIC — `フリーザ`, `ナメック星`, `かめはめ波` —
    /// et une planche de fiches techniques dépasse couramment 30 % d'inconnus
    /// sans être fausse pour autant. Et le plancher de 24 caractères évite de
    /// condamner un titre de deux mots, où un seul nom propre suffirait à
    /// franchir n'importe quelle part.
    #[must_use]
    pub fn charabia(&self) -> bool {
        self.caracteres >= CHARABIA_MINIMUM && self.part_inconnue() >= CHARABIA_PART
    }
}

/// En dessous, un texte est trop court pour que la part d'inconnus veuille dire
/// quoi que ce soit.
const CHARABIA_MINIMUM: usize = 24;

/// Part d'inconnus à partir de laquelle un texte cesse d'être du japonais.
const CHARABIA_PART: f64 = 0.6;

impl Analyseur {
    /// Charge IPADIC. À faire une fois par lot.
    ///
    /// # Errors
    ///
    /// Échoue si le binaire a été construit sans la feature `japanese` : le
    /// dictionnaire est compilé dedans, il n'y a rien à chercher sur le disque.
    pub fn nouveau() -> Result<Self, Erreur> {
        let dictionnaire =
            load_dictionary("embedded://ipadic").map_err(|e| Erreur::Dictionnaire(e.to_string()))?;
        Ok(Self {
            segmenter: Segmenter::new(Mode::Normal, dictionnaire, None),
        })
    }

    /// Recolle les lignes coupées au milieu d'un mot.
    ///
    /// Rend le texte remis en forme et le nombre de coupures recollées.
    ///
    /// La règle n'est pas « coller tout ce qui est japonais » : une liste, un
    /// tableau ou un titre légitiment un retour à la ligne entre deux kanji. On
    /// ne recolle que si la jointure reconstitue un morphème que le
    /// dictionnaire connaît et qui enjambe la coupure — c'est-à-dire seulement
    /// quand la coupure tombe *à l'intérieur* d'un mot.
    #[must_use]
    pub fn recolle_lignes(&self, texte: &str) -> (String, usize) {
        let lignes: Vec<&str> = texte.split('\n').collect();
        let mut out = String::with_capacity(texte.len());
        let mut recollees = 0_usize;

        for (i, ligne) in lignes.iter().enumerate() {
            out.push_str(ligne);
            let Some(suivante) = lignes.get(i + 1) else {
                continue;
            };
            if self.recollable(ligne, suivante) {
                recollees += 1; // pas de séparateur : le mot reprend directement
            } else {
                out.push('\n');
            }
        }

        (out, recollees)
    }

    /// Décide si deux lignes consécutives n'en formaient qu'une.
    fn recollable(&self, avant: &str, apres: &str) -> bool {
        // Une ligne vide sépare deux paragraphes : elle est du sens, pas un
        // accident de mise en page.
        if avant.trim().is_empty() || apres.trim().is_empty() {
            return false;
        }
        // Un titre markdown ou une puce ouvrent un bloc ; les recoller au
        // paragraphe précédent détruirait la structure que le modèle a vue.
        if apres.trim_start().starts_with(['#', '-', '*', '|', '>']) {
            return false;
        }
        let (Some(fin), Some(debut)) = (avant.chars().last(), apres.chars().next()) else {
            return false;
        };
        if !japonais(fin) || !japonais(debut) {
            return false;
        }
        // Une ponctuation finale ferme la phrase : la suite est un nouveau bloc.
        if ferme_une_phrase(fin) || ouvre_un_bloc(debut) {
            return false;
        }

        // Fenêtre courte de part et d'autre : segmenter la planche entière pour
        // trancher une coupure coûterait le prix d'une analyse complète par
        // ligne, et le contexte lointain ne change pas le verdict.
        let queue = queue(avant, FENETRE);
        let tete = tete(apres, FENETRE);
        let frontiere = queue.len();
        let sonde = format!("{queue}{tete}");

        let Ok(morphemes) = self.segmenter.segment(Cow::Owned(sonde)) else {
            return false;
        };
        morphemes
            .iter()
            .any(|m| m.byte_start < frontiere && m.byte_end > frontiere)
    }

    /// Remplace les sosies typographiques par le caractère que le mot appelait.
    ///
    /// Rend le texte corrigé et la liste de ce qui a été changé, pour qu'une
    /// relecture puisse vérifier chaque décision plutôt que de faire confiance
    /// à un compteur.
    ///
    /// Deux règles, de sûreté décroissante :
    ///
    /// 1. Un `一` (le kanji « un ») **encadré de katakana des deux côtés** est
    ///    forcément un `ー`, la marque d'allongement. Aucun mot japonais ne
    ///    place le chiffre un entre deux katakana ; `ス一パ一` ne peut être que
    ///    `スーパー`. Celle-là ne demande pas le dictionnaire.
    /// 2. Pour les autres sosies (`力`/`カ`, `口`/`ロ`, `二`/`ニ`…), la
    ///    substitution n'est retenue que si elle transforme une suite que le
    ///    dictionnaire ignore en un mot qu'il connaît. Sans cette confirmation
    ///    on ne corrige pas : `一力` peut parfaitement être « une force ».
    #[must_use]
    pub fn corrige_sosies(&self, texte: &str) -> (String, Vec<Correction>) {
        let chars: Vec<(usize, char)> = texte.char_indices().collect();
        let mut out = String::with_capacity(texte.len());
        let mut corrections = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            let Some(fin) = fin_de_suite(&chars, i) else {
                out.push(chars[i].1);
                i += 1;
                continue;
            };

            let avant: String = chars[i..fin].iter().map(|(_, c)| *c).collect();
            let apres = self.corrige_suite(&chars, i, fin);
            if apres == avant {
                out.push_str(&avant);
            } else {
                corrections.push(Correction {
                    debut: chars[i].0,
                    avant,
                    apres: apres.clone(),
                });
                out.push_str(&apres);
            }
            i = fin;
        }

        (out, corrections)
    }

    /// Corrige une suite de katakana et de sosies, `chars[debut..fin]`.
    fn corrige_suite(&self, chars: &[(usize, char)], debut: usize, fin: usize) -> String {
        let mut suite: Vec<char> = chars[debut..fin].iter().map(|(_, c)| *c).collect();
        let mut certain = false;

        // Règle 1 : le `一` encadré de katakana, sans consulter le dictionnaire.
        for k in 1..suite.len().saturating_sub(1) {
            if suite[k] == '一' && katakana(suite[k - 1]) && katakana(suite[k + 1]) {
                suite[k] = 'ー';
                certain = true;
            }
        }

        // Règle 2 : les autres sosies, sous condition du dictionnaire.
        let intermediaire: String = suite.iter().collect();
        let candidate: String = suite.iter().map(|&c| sosie(c).unwrap_or(c)).collect();
        if candidate != intermediaire && self.mot_connu(&candidate) && !self.mot_connu(&intermediaire)
        {
            return candidate;
        }

        if certain { intermediaire } else { chars[debut..fin].iter().map(|(_, c)| *c).collect() }
    }

    /// Le dictionnaire reconnaît-il cette suite comme un seul mot ?
    ///
    /// Un seul, pas plusieurs : `カメハメ` découpé en `カメ` + `ハメ` serait
    /// « connu » au sens large tout en n'étant pas un mot, et cette indulgence
    /// suffirait à valider n'importe quelle substitution.
    fn mot_connu(&self, mot: &str) -> bool {
        let Ok(morphemes) = self.segmenter.segment(Cow::Owned(mot.to_owned())) else {
            return false;
        };
        morphemes.len() == 1 && !morphemes[0].word_id.is_unknown()
    }

    /// Mesure la part de morphèmes que le dictionnaire ne connaît pas.
    ///
    /// C'est le seul signal de ce pipeline qui repère une planche **lue de
    /// travers** plutôt que mal formatée : le charabia a la forme du japonais,
    /// donc aucun filtre de forme ne le voit passer.
    #[must_use]
    pub fn confiance(&self, texte: &str) -> Confiance {
        let Ok(morphemes) = self.segmenter.segment(Cow::Owned(texte.to_owned())) else {
            return Confiance::default();
        };
        let mut mesure = Confiance::default();
        for morpheme in &morphemes {
            // Seul le japonais est jugé : IPADIC ignore `EUR`, `2026` et `—`,
            // et les compter en inconnus ferait passer une facture en anglais
            // pour du charabia.
            let japonais_dedans = morpheme.surface.chars().filter(|&c| japonais(c)).count();
            if japonais_dedans == 0 {
                continue;
            }
            mesure.morphemes += 1;
            mesure.caracteres += japonais_dedans;
            if morpheme.word_id.is_unknown() {
                mesure.inconnus += japonais_dedans;
            }
        }
        mesure
    }
}

/// Fin de la suite de katakana et de sosies commençant en `debut`, si elle
/// mérite d'être examinée.
///
/// Une suite n'est retenue que si elle est assez longue pour être un mot et
/// contient au moins un vrai katakana et un sosie : sans katakana, `二力` est
/// du japonais ordinaire ; sans sosie, il n'y a rien à corriger.
fn fin_de_suite(chars: &[(usize, char)], debut: usize) -> Option<usize> {
    let mut fin = debut;
    let mut katakanas = 0_usize;
    let mut sosies = 0_usize;
    while fin < chars.len() {
        let c = chars[fin].1;
        if katakana(c) {
            katakanas += 1;
        } else if sosie(c).is_some() || c == '一' {
            sosies += 1;
        } else {
            break;
        }
        fin += 1;
    }
    (fin - debut >= SUITE_MINIMUM && katakanas > 0 && sosies > 0).then_some(fin)
}

/// Longueur minimale d'une suite pour qu'on ose y toucher.
///
/// Trois : en deçà, un katakana isolé collé à un kanji est de la langue
/// courante (`力ップ` est douteux, `力技` ne l'est pas), et la substitution
/// n'aurait pas assez de contexte pour être autre chose qu'un pari.
const SUITE_MINIMUM: usize = 3;

/// Le katakana qu'un kanji sosie remplace.
///
/// Les paires viennent de la ressemblance de dessin, pas du son : c'est un
/// modèle de vision qui les confond. `ー` est traité à part — sa règle est
/// certaine et n'a pas besoin du dictionnaire.
const fn sosie(c: char) -> Option<char> {
    let katakana = match c {
        '力' => 'カ', // chikara / ka
        '口' => 'ロ', // kuchi / ro
        '二' => 'ニ', // ni (kanji) / ni (katakana)
        '卜' => 'ト', // boku / to
        '夕' => 'タ', // yuu / ta
        '工' => 'エ', // kou / e
        '八' => 'ハ', // hachi / ha
        '匕' => 'ヒ', // hi (kanji rare) / hi
        'へ' => 'ヘ', // hiragana he / katakana he, au dessin identique
        'べ' => 'ベ',
        'ぺ' => 'ペ',
        _ => return None,
    };
    Some(katakana)
}

/// Repère les fragments d'écriture étrangère coincés dans un mot japonais.
///
/// Un mot latin entouré d'espaces (`DRAGON BALL`, `TV`) est du texte réel :
/// les databooks en sont pleins. Ce qui trahit l'hallucination, c'est le
/// fragment latin *collé* des deux côtés à du japonais — et, pour les écritures
/// qui n'ont rien à faire là, leur seule présence.
///
/// Ne consulte pas le dictionnaire : reconnaître une écriture est affaire de
/// plages Unicode, pas de morphologie.
#[must_use]
pub fn intrusions(texte: &str) -> Vec<Intrusion> {
    let mut out = Vec::new();
    let chars: Vec<(usize, char)> = texte.char_indices().collect();
    let mut i = 0;

    while i < chars.len() {
        if !etranger(chars[i].1) {
            i += 1;
            continue;
        }
        let debut = i;
        while i < chars.len() && etranger(chars[i].1) {
            i += 1;
        }
        let fin = i;

        // Encadré de japonais des deux côtés, sans espace : le fragment est
        // à l'intérieur d'un mot, là où rien de latin n'a sa place.
        let colle = debut > 0
            && fin < chars.len()
            && japonais(chars[debut - 1].1)
            && japonais(chars[fin].1);
        let fragment: String = chars[debut..fin].iter().map(|(_, c)| *c).collect();
        let certaine = fragment.chars().any(autre_ecriture);
        // Un long mot latin collé est presque toujours un vrai mot que la
        // typographie japonaise accole (`ドラゴンボールZ`) ; c'est le
        // fragment court qui remplace un caractère mal lu.
        if !certaine && (!colle || fragment.chars().count() > INTRUSION_MAX) {
            continue;
        }
        out.push(Intrusion {
            debut: chars[debut].0,
            contexte: contexte(&chars, debut, fin),
            fragment,
            certaine,
        });
    }

    out
}

/// Nombre de caractères pris de part et d'autre d'une coupure pour la juger.
///
/// Huit suffisent : le plus long morphème d'IPADIC en tient largement moins, et
/// élargir la fenêtre ne ferait qu'alourdir chaque test.
const FENETRE: usize = 8;

/// Au-delà, un fragment latin collé est un vrai mot, pas un caractère mal lu.
const INTRUSION_MAX: usize = 4;

/// Toute écriture qui n'a rien à faire dans un databook japonais.
///
/// Le latin y figure légitimement (DRAGON BALL, TV, une référence produit) ;
/// les autres, jamais. Relevé sur sept planches auditées à l'image : cyrillique
/// `ю`, hangul `원`, devanagari `ज` — chacun posé au milieu d'un mot japonais,
/// à la place d'un caractère que le modèle n'a pas su lire.
///
/// Les **lettres hangul cerclées** (U+3260–U+327F) y ont été ajoutées après
/// coup : quinze planches en portent 1 118 occurrences, non pas comme intrusion
/// d'écriture mais comme débordement d'énumération — les chiffres cerclés
/// s'arrêtent à `㉟` et la table continue en `㉠`. Le mécanisme diffère, le
/// verdict est le même : un `㉠` dans un databook japonais est certain d'être
/// une erreur.
fn autre_ecriture(c: char) -> bool {
    matches!(c,
        '\u{0370}'..='\u{03FF}'   // grec
        | '\u{0400}'..='\u{04FF}' // cyrillique
        | '\u{0590}'..='\u{05FF}' // hébreu
        | '\u{0600}'..='\u{06FF}' // arabe
        | '\u{0900}'..='\u{097F}' // devanagari
        | '\u{0E00}'..='\u{0E7F}' // thaï
        | '\u{1100}'..='\u{11FF}' // jamo hangul
        | '\u{3260}'..='\u{327F}' // lettres hangul cerclées : ㉠ ㉡ ㉢…
        | '\u{AC00}'..='\u{D7A3}' // syllabes hangul
    )
}

fn etranger(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '^' || autre_ecriture(c)
}

/// Ponctuation qui clôt une phrase : ce qui suit commence ailleurs.
fn ferme_une_phrase(c: char) -> bool {
    matches!(
        c,
        '。' | '、' | '！' | '？' | '」' | '』' | '）' | '】' | '〉' | '…' | '・'
    )
}

/// Ponctuation ouvrante : elle ne se recolle pas à ce qui précède.
fn ouvre_un_bloc(c: char) -> bool {
    matches!(c, '「' | '『' | '（' | '【' | '〈' | '・')
}

fn queue(s: &str, n: usize) -> String {
    let compte = s.chars().count();
    s.chars().skip(compte.saturating_sub(n)).collect()
}

fn tete(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn contexte(chars: &[(usize, char)], debut: usize, fin: usize) -> String {
    let a = debut.saturating_sub(6);
    let b = (fin + 6).min(chars.len());
    chars[a..b].iter().map(|(_, c)| *c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyseur() -> Analyseur {
        Analyseur::nouveau().expect("IPADIC est compilé dans le binaire")
    }

    #[test]
    fn une_coupure_au_milieu_dun_mot_est_recollee() {
        // Le cas mesuré 50 725 fois : la page revenait à la ligne au milieu du
        // mot, le modèle a rendu le retour à la ligne.
        let (out, n) = analyseur().recolle_lignes("よろしか\nたら大きな心で");
        assert_eq!(n, 1, "la coupure doit être vue : {out}");
        assert_eq!(out, "よろしかたら大きな心で");
    }

    #[test]
    fn une_ligne_vide_reste_une_separation_de_paragraphe() {
        let texte = "第一段落です\n\n第二段落です";
        let (out, n) = analyseur().recolle_lignes(texte);
        assert_eq!(n, 0);
        assert_eq!(out, texte);
    }

    #[test]
    fn un_titre_markdown_nest_jamais_recolle_au_paragraphe_precedent() {
        // Recoller détruirait la structure que le modèle a vue sur la page.
        let texte = "本文の続き\n# 見出し";
        let (out, n) = analyseur().recolle_lignes(texte);
        assert_eq!(n, 0);
        assert_eq!(out, texte);
    }

    #[test]
    fn une_phrase_terminee_ne_se_recolle_pas_a_la_suivante() {
        let texte = "これで終わりです。\n次の話をしよう";
        let (out, n) = analyseur().recolle_lignes(texte);
        assert_eq!(n, 0);
        assert_eq!(out, texte);
    }

    #[test]
    fn du_cyrillique_dans_un_mot_japonais_est_signale() {
        // `ギновー` vu dans le corpus : le modèle a rendu `ニュ` en cyrillique.
        let trouve = intrusions("ギновー特戦隊");
        assert_eq!(trouve.len(), 1, "{trouve:?}");
        assert!(trouve[0].certaine);
        assert_eq!(trouve[0].fragment, "нов");
    }

    #[test]
    fn du_hangul_ou_du_devanagari_dans_un_mot_japonais_est_signale() {
        // Relevés à l'image sur deux planches : `ミンजション` avec un
        // devanagari au lieu de `ミッション`, et un `원` hangul en pleine phrase.
        let devanagari = intrusions("ウルトラゴッドミンजション");
        assert_eq!(devanagari.len(), 1, "{devanagari:?}");
        assert!(devanagari[0].certaine);

        let hangul = intrusions("遭遇を원かた");
        assert_eq!(hangul.len(), 1, "{hangul:?}");
        assert!(hangul[0].certaine);
    }

    #[test]
    fn un_mot_latin_entoure_despaces_est_du_texte_reel() {
        // Les databooks écrivent DRAGON BALL en toutes lettres : le signaler
        // noierait les vraies hallucinations.
        assert!(intrusions("これは DRAGON BALL の本です").is_empty());
    }

    #[test]
    fn un_fragment_latin_court_colle_entre_deux_kanji_est_signale() {
        let trouve = intrusions("実力は特戦隊一Butが強い");
        assert_eq!(trouve.len(), 1, "{trouve:?}");
        assert_eq!(trouve[0].fragment, "But");
        assert!(!trouve[0].certaine);
        assert!(trouve[0].contexte.contains("But"));
    }

    #[test]
    fn un_suffixe_latin_long_colle_reste_du_texte_reel() {
        // `ドラゴンボールZ` est correct ; un seuil trop bas le casserait.
        assert!(intrusions("ドラゴンボールSUPERの世界").is_empty());
    }

    #[test]
    fn les_espaces_poses_entre_deux_caracteres_japonais_disparaissent() {
        let (out, n) = espaces_parasites("全集を出して いただい た");
        assert_eq!(n, 2);
        assert_eq!(out, "全集を出していただいた");
    }

    #[test]
    fn un_kanji_un_entre_deux_katakana_est_une_marque_dallongement() {
        // `ス一パ一` : le modèle a lu le trait d'allongement comme le kanji
        // « un ». Aucun mot japonais ne place le chiffre un entre deux
        // katakana, donc la correction ne demande pas le dictionnaire.
        let (out, corrections) = analyseur().corrige_sosies("ス一パ一マン");
        assert_eq!(out, "スーパーマン", "{corrections:?}");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].avant, "ス一パ一マン");
    }

    #[test]
    fn un_kanji_un_qui_compte_vraiment_nest_pas_touche() {
        // `コーヒー一杯` : le `一` suit un katakana mais précède un kanji —
        // il compte bel et bien une tasse.
        let texte = "コーヒー一杯";
        let (out, corrections) = analyseur().corrige_sosies(texte);
        assert_eq!(out, texte);
        assert!(corrections.is_empty(), "{corrections:?}");
    }

    #[test]
    fn un_sosie_kanji_nest_corrige_que_si_le_mot_obtenu_existe() {
        // `カメラ` existe dans IPADIC ; `力メラ`, non.
        let (out, corrections) = analyseur().corrige_sosies("力メラで撮る");
        assert_eq!(out, "カメラで撮る", "{corrections:?}");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].avant, "力メラ");
        assert_eq!(corrections[0].apres, "カメラ");
    }

    #[test]
    fn une_substitution_qui_ne_donne_aucun_mot_connu_est_refusee() {
        // Sans confirmation du dictionnaire, corriger serait parier. La suite
        // reste telle quelle et l'anomalie ressort par ailleurs, en intrusion
        // ou en confiance basse.
        let texte = "力ヌピ";
        let (out, corrections) = analyseur().corrige_sosies(texte);
        assert_eq!(out, texte);
        assert!(corrections.is_empty(), "{corrections:?}");
    }

    #[test]
    fn du_japonais_ordinaire_traverse_la_correction_intact() {
        // Le piège de cette passe : `二`, `力`, `口` sont des kanji courants.
        // Une règle qui les remplace hors contexte katakana casserait le
        // corpus entier.
        for texte in ["二人の力", "入り口は北", "第一巻", "工場で働く", "八時に"] {
            let (out, corrections) = analyseur().corrige_sosies(texte);
            assert_eq!(out, texte, "{corrections:?}");
            assert!(corrections.is_empty(), "{texte} : {corrections:?}");
        }
    }

    #[test]
    fn du_japonais_reel_a_une_confiance_haute() {
        let mesure = analyseur().confiance(
            "今日は天気がいいので、公園に行って写真を撮ることにしました。友達も一緒に来ます。",
        );
        assert!(mesure.caracteres >= CHARABIA_MINIMUM, "{mesure:?}");
        assert!(!mesure.charabia(), "{mesure:?} part={}", mesure.part_inconnue());
    }

    #[test]
    fn du_charabia_en_kana_est_repere() {
        // Ce que rend le modèle sur une bulle manuscrite qu'il ne sait pas
        // lire : la forme est du japonais, le contenu n'est rien.
        let mesure = analyseur()
            .confiance("ヌポギヅェザムクィヘゾラヴォヌポギヅェザムクィヘゾラヴォヌポギヅェザム");
        assert!(mesure.charabia(), "{mesure:?} part={}", mesure.part_inconnue());
    }

    #[test]
    fn un_texte_sans_japonais_nest_jamais_du_charabia_japonais() {
        // Une planche de crédits en anglais n'a rien à se reprocher.
        let mesure = analyseur().confiance("BIRD STUDIO / SHUEISHA, TOEI ANIMATION 1989");
        assert_eq!(mesure.caracteres, 0);
        assert_eq!(mesure.part_inconnue(), 0.0);
        assert!(!mesure.charabia());
    }

    #[test]
    fn une_file_de_kana_illisible_compte_pour_sa_longueur_et_non_pour_un() {
        // Le piège qui a fait changer l'unité de mesure : lindera rend UN seul
        // morphème inconnu pour toute une file de katakana. Compté en
        // morphèmes, `1 sur 1` passait sous le plancher et la planche la plus
        // illisible du lot ressortait innocentée.
        let mesure = analyseur().confiance("ヌポギヅェザムクィヘゾラヴォヌポギヅェザムクィヘゾラヴォ");
        assert_eq!(mesure.morphemes, 1, "{mesure:?}");
        assert!(mesure.caracteres >= 24, "{mesure:?}");
    }

    #[test]
    fn un_titre_court_bourre_de_noms_propres_nest_pas_condamne() {
        // `フリーザ` et `ナメック星` sont absents d'IPADIC. Sans le plancher de
        // morphèmes, un titre pareil sortirait en charabia à chaque planche.
        let mesure = analyseur().confiance("フリーザとナメック星");
        assert!(!mesure.charabia(), "{mesure:?} part={}", mesure.part_inconnue());
    }
}
