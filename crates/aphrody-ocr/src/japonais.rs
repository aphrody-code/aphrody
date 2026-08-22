//! Remise en forme du japonais, guidée par l'analyse morphologique.
//!
//! Le japonais s'écrit sans espaces, ce qui prive le nettoyage générique de son
//! repère principal : sans savoir où finissent les mots, on ne peut ni recoller
//! une ligne coupée en plein milieu de l'un d'eux, ni dire si une lettre latine
//! posée entre deux kanji est un mot légitime ou une hallucination du modèle.
//! D'où lindera, portage Rust de Kuromoji, avec IPADIC compilé dans le binaire.
//!
//! CE QUE ÇA CORRIGE, MESURÉ sur 5762 planches de databooks déjà lues :
//!
//! - **50 725 coupures de ligne tombent entre deux caractères japonais**, soit
//!   15,6 % de toutes les coupures. Elles viennent du texte en colonnes : le
//!   modèle rend un saut de ligne là où la page revenait à la ligne, au milieu
//!   d'un mot. `よろしか\nたら` doit se lire `よろしかったら`.
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

use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

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

/// Supprime les espaces posés entre deux caractères japonais.
///
/// Le japonais ne sépare pas ses mots ; un espace au milieu d'une suite de
/// kana ou de kanji vient du modèle, qui calque l'espacement typographique de
/// la page. Rend le texte et le nombre d'espaces retirés. Ne demande pas le
/// dictionnaire : la règle est vraie de toute la langue.
#[must_use]
pub fn espaces_parasites(texte: &str) -> (String, usize) {
    let chars: Vec<char> = texte.chars().collect();
    let mut out = String::with_capacity(texte.len());
    let mut retires = 0_usize;

    for (i, &c) in chars.iter().enumerate() {
        if c == ' '
            && i > 0
            && i + 1 < chars.len()
            && japonais(chars[i - 1])
            && japonais(chars[i + 1])
        {
            retires += 1;
            continue;
        }
        out.push(c);
    }

    (out, retires)
}

/// Nombre de caractères pris de part et d'autre d'une coupure pour la juger.
///
/// Huit suffisent : le plus long morphème d'IPADIC en tient largement moins, et
/// élargir la fenêtre ne ferait qu'alourdir chaque test.
const FENETRE: usize = 8;

/// Au-delà, un fragment latin collé est un vrai mot, pas un caractère mal lu.
const INTRUSION_MAX: usize = 4;

fn japonais(c: char) -> bool {
    matches!(c,
        '\u{3040}'..='\u{309F}'   // hiragana
        | '\u{30A0}'..='\u{30FF}' // katakana
        | '\u{4E00}'..='\u{9FFF}' // kanji
        | '\u{FF66}'..='\u{FF9D}' // katakana demi-chasse
    )
}

/// Toute écriture qui n'a rien à faire dans un databook japonais.
///
/// Le latin y figure légitimement (DRAGON BALL, TV, une référence produit) ;
/// les autres, jamais. Relevé sur sept planches auditées à l'image : cyrillique
/// `ю`, hangul `원`, devanagari `ज` — chacun posé au milieu d'un mot japonais,
/// à la place d'un caractère que le modèle n'a pas su lire.
fn autre_ecriture(c: char) -> bool {
    matches!(c,
        '\u{0370}'..='\u{03FF}'   // grec
        | '\u{0400}'..='\u{04FF}' // cyrillique
        | '\u{0590}'..='\u{05FF}' // hébreu
        | '\u{0600}'..='\u{06FF}' // arabe
        | '\u{0900}'..='\u{097F}' // devanagari
        | '\u{0E00}'..='\u{0E7F}' // thaï
        | '\u{1100}'..='\u{11FF}' // jamo hangul
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
        assert!(
            intrusions("これは DRAGON BALL の本です")
                .is_empty()
        );
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
        assert!(
            intrusions("ドラゴンボールSUPERの世界")
                .is_empty()
        );
    }

    #[test]
    fn les_espaces_poses_entre_deux_caracteres_japonais_disparaissent() {
        let (out, n) = espaces_parasites("全集を出して いただい た");
        assert_eq!(n, 2);
        assert_eq!(out, "全集を出していただいた");
    }

    #[test]
    fn un_espace_qui_separe_du_latin_du_japonais_est_garde() {
        let texte = "これは DRAGON BALL です";
        let (out, n) = espaces_parasites(texte);
        assert_eq!(n, 0);
        assert_eq!(out, texte);
    }
}
