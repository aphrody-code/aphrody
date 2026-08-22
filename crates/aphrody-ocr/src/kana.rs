// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Normalisation des écritures japonaises, sans dictionnaire.
//!
//! Ce module ne connaît que l'Unicode : il range les caractères en catégories
//! et remet en forme ce qu'on peut corriger sans savoir ce que le texte veut
//! dire. Tout ce qui demande de reconnaître un mot vit dans
//! [`crate::japonais`], derrière la feature `japanese` et son dictionnaire.
//!
//! La séparation n'est pas cosmétique : ces règles-ci sont vraies de tout
//! japonais imprimé, elles n'ont donc aucune raison de coûter un dictionnaire
//! de plusieurs mégaoctets à un binaire qui ne le veut pas — et elles
//! compilent pour wasm, ce que lindera ne fait pas.
//!
//! # Pourquoi la demi-chasse est toujours une erreur ici
//!
//! Les katakana demi-chasse (`ｶ`, `ｷ`, `ﾞ`) datent des terminaux à 8 bits.
//! **Aucun databook imprimé n'en contient** : la typographie japonaise en plomb
//! puis en PAO n'a jamais eu de demi-chasse pour les kana. Quand le modèle en
//! rend, c'est donc à coup sûr sa propre erreur — il a vu un kana étroit dans
//! une colonne serrée et a choisi la variante étroite du jeu de caractères.
//! Contrairement à la plupart des défauts de lecture, celui-là se corrige avec
//! certitude et sans deviner : `ｶﾞ` ne peut vouloir dire que `ガ`.

/// Un caractère appartient-il à l'écriture japonaise ?
///
/// Large à dessein : le recollage de lignes et la détection d'intrusions
/// s'appuient dessus, et un caractère japonais oublié ici devient un trou par
/// lequel un défaut passe. Les marques d'itération (`々`), les kanji des
/// extensions et les formes de compatibilité en font partie — un databook des
/// années 1990 en contient, ne serait-ce que dans les noms d'auteurs.
#[must_use]
pub const fn japonais(c: char) -> bool {
    matches!(c,
        '\u{3005}'                // 々, marque d'itération de kanji
        | '\u{3006}'              // 〆
        | '\u{3007}'              // 〇, le zéro idéographique
        | '\u{303B}'              // 〻
        | '\u{3040}'..='\u{309F}' // hiragana (avec ゛ ゜ et ゝ ゞ)
        | '\u{30A0}'..='\u{30FF}' // katakana (avec ー ヽ ヾ)
        | '\u{3400}'..='\u{4DBF}' // kanji, extension A
        | '\u{4E00}'..='\u{9FFF}' // kanji, bloc principal
        | '\u{F900}'..='\u{FAFF}' // idéogrammes de compatibilité
        | '\u{FF66}'..='\u{FF9F}' // katakana demi-chasse (avec ﾞ ﾟ)
    )
}

/// Un caractère est-il un kana, hiragana ou katakana ?
///
/// Distingué des kanji parce que plusieurs règles ne valent que pour les kana :
/// une longue voyelle ne suit qu'un kana, et c'est au milieu d'un mot en
/// katakana qu'un kanji sosie trahit une erreur de lecture.
#[must_use]
pub const fn kana(c: char) -> bool {
    matches!(c,
        '\u{3041}'..='\u{309F}'   // hiragana
        | '\u{30A0}'..='\u{30FF}' // katakana
        | '\u{FF66}'..='\u{FF9F}' // katakana demi-chasse
    )
}

/// Un caractère est-il un katakana pleine chasse ?
#[must_use]
pub const fn katakana(c: char) -> bool {
    matches!(c, '\u{30A1}'..='\u{30FA}' | '\u{30FC}')
}

/// Une espace, sous n'importe laquelle de ses formes.
///
/// Le modèle ne rend pas seulement `U+0020` : il rend aussi l'espace
/// idéographique `U+3000` — celle que la typographie japonaise emploie pour
/// l'alinéa — et, plus rarement, une insécable. Ne filtrer que `U+0020`
/// laissait donc passer la majorité des espaces parasites d'une page mise en
/// colonnes.
#[must_use]
pub const fn espace(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\u{00A0}' | '\u{2002}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

/// Remet en pleine chasse les kana et la ponctuation japonaise demi-chasse.
///
/// Rend le texte corrigé et le nombre de caractères remplacés. Recompose les
/// marques de sonorisation : `ｶ` suivi de `ﾞ` donne `ガ`, un seul caractère, et
/// non `カ゛` en deux.
///
/// N'y touche à rien d'autre. Les chiffres et les lettres latines pleine
/// chasse (`１`, `Ａ`), eux, existent bel et bien dans les databooks — les
/// normaliser détruirait de la mise en page réelle.
#[must_use]
pub fn normalise_demi_chasse(texte: &str) -> (String, usize) {
    let mut out = String::with_capacity(texte.len());
    let mut remplaces = 0_usize;
    let mut chars = texte.chars().peekable();

    while let Some(c) = chars.next() {
        let Some(pleine) = pleine_chasse(c) else {
            out.push(c);
            continue;
        };
        remplaces += 1;

        // Une marque de sonorisation suit son kana : la recomposer donne le
        // caractère unique que la typographie imprime réellement.
        match chars.peek() {
            Some('\u{FF9E}') => {
                if let Some(sonore) = voise(pleine) {
                    chars.next();
                    remplaces += 1;
                    out.push(sonore);
                    continue;
                }
            }
            Some('\u{FF9F}') => {
                if let Some(sourd) = semi_voise(pleine) {
                    chars.next();
                    remplaces += 1;
                    out.push(sourd);
                    continue;
                }
            }
            _ => {}
        }
        out.push(pleine);
    }

    (out, remplaces)
}

/// Remet en forme la ponctuation que le modèle rend en caractères de secours.
///
/// Rend le texte corrigé et le nombre de caractères remplacés.
///
/// Deux règles, toutes deux mesurées sur les 6 305 planches transcrites des
/// databooks Dragon Ball :
///
/// 1. **Une suite de points médians demi-chasse est une ellipse.** 638 planches
///    portent `･･` ou `･･･` ; **quatre** portent un `･` isolé. La distribution
///    ne laisse pas de doute : ce n'est pas un séparateur de largeur fautive,
///    c'est `…` que le modèle a rendu en points de secours. Le `･` isolé, lui,
///    reste un séparateur et devient `・`, ce dont
///    [`normalise_demi_chasse`] se charge.
/// 2. **Trois à cinq points ASCII contre du japonais sont une ellipse.**
///    108 planches. Le voisinage est la condition : les 123 autres planches à
///    `...` sont en contexte latin, où l'ASCII est la bonne graphie. Et une
///    suite de six points ou plus n'est jamais touchée — ce sont les points de
///    conduite d'un tableau, du mobilier légitime.
///
/// L'ordre compte : cette passe doit tourner **après** la coupure de boucles.
/// Une planche du corpus porte 2 034 `･` d'affilée, ce qui relève de la
/// génération bloquée et non de la typographie ; la convertir en 678 ellipses
/// remplacerait un défaut par un autre.
#[must_use]
pub fn normalise_ponctuation(texte: &str) -> (String, usize) {
    let chars: Vec<char> = texte.chars().collect();
    let mut out = String::with_capacity(texte.len());
    let mut remplaces = 0_usize;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c != '\u{FF65}' && c != '.' {
            out.push(c);
            i += 1;
            continue;
        }
        let debut = i;
        while i < chars.len() && chars[i] == c {
            i += 1;
        }
        let longueur = i - debut;

        let ellipse = if c == '\u{FF65}' {
            longueur >= 2
        } else {
            // Les points ASCII n'ont ce sens qu'au contact du japonais, et
            // seulement en petit nombre.
            (3..=5).contains(&longueur) && voisin_japonais(&chars, debut, i)
        };

        if ellipse {
            // Trois points font une ellipse ; six en font deux, comme la
            // typographie japonaise les compose par paires.
            for _ in 0..(longueur / 3).max(1) {
                out.push('…');
            }
            remplaces += longueur;
        } else {
            out.extend(std::iter::repeat_n(c, longueur));
        }
    }

    (out, remplaces)
}

/// Y a-t-il du japonais juste avant ou juste après `chars[debut..fin]` ?
fn voisin_japonais(chars: &[char], debut: usize, fin: usize) -> bool {
    let avant = debut.checked_sub(1).map(|i| chars[i]).is_some_and(japonais);
    let apres = chars.get(fin).copied().is_some_and(japonais);
    avant || apres
}

/// Supprime les espaces posées entre deux caractères japonais.
///
/// Le japonais ne sépare pas ses mots ; une espace au milieu d'une suite de
/// kana ou de kanji vient du modèle, qui calque l'espacement typographique de
/// la page. Rend le texte et le nombre d'espaces retirées.
///
/// Une suite d'espaces compte pour une : `全集 　を` perd les deux d'un coup,
/// sinon une espace ordinaire suivie d'une idéographique survivait à la passe.
#[must_use]
pub fn espaces_parasites(texte: &str) -> (String, usize) {
    let chars: Vec<char> = texte.chars().collect();
    let mut out = String::with_capacity(texte.len());
    let mut retires = 0_usize;
    let mut i = 0;

    while i < chars.len() {
        if !espace(chars[i]) {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let debut = i;
        while i < chars.len() && espace(chars[i]) {
            i += 1;
        }
        // Encadrée de japonais des deux côtés : la page revenait à la ligne ou
        // aérait une colonne, la langue n'a rien demandé.
        let parasite = debut > 0
            && i < chars.len()
            && japonais(chars[debut - 1])
            && japonais(chars[i]);
        if parasite {
            retires += i - debut;
        } else {
            out.extend(&chars[debut..i]);
        }
    }

    (out, retires)
}

/// Le kana ou la ponctuation pleine chasse correspondant à une demi-chasse.
fn pleine_chasse(c: char) -> Option<char> {
    let pleine = match c {
        '\u{FF61}' => '。',
        '\u{FF62}' => '「',
        '\u{FF63}' => '」',
        '\u{FF64}' => '、',
        '\u{FF65}' => '・',
        '\u{FF66}' => 'ヲ',
        '\u{FF67}' => 'ァ',
        '\u{FF68}' => 'ィ',
        '\u{FF69}' => 'ゥ',
        '\u{FF6A}' => 'ェ',
        '\u{FF6B}' => 'ォ',
        '\u{FF6C}' => 'ャ',
        '\u{FF6D}' => 'ュ',
        '\u{FF6E}' => 'ョ',
        '\u{FF6F}' => 'ッ',
        '\u{FF70}' => 'ー',
        '\u{FF71}'..='\u{FF9D}' => KATAKANA_PLEINE[c as usize - 0xFF71],
        _ => return None,
    };
    Some(pleine)
}

/// `ｱ` à `ﾝ` dans l'ordre du bloc demi-chasse, en pleine chasse.
const KATAKANA_PLEINE: [char; 45] = [
    'ア', 'イ', 'ウ', 'エ', 'オ', // FF71..FF75
    'カ', 'キ', 'ク', 'ケ', 'コ', // FF76..FF7A
    'サ', 'シ', 'ス', 'セ', 'ソ', // FF7B..FF7F
    'タ', 'チ', 'ツ', 'テ', 'ト', // FF80..FF84
    'ナ', 'ニ', 'ヌ', 'ネ', 'ノ', // FF85..FF89
    'ハ', 'ヒ', 'フ', 'ヘ', 'ホ', // FF8A..FF8E
    'マ', 'ミ', 'ム', 'メ', 'モ', // FF8F..FF93
    'ヤ', 'ユ', 'ヨ', // FF94..FF96
    'ラ', 'リ', 'ル', 'レ', 'ロ', // FF97..FF9B
    'ワ', 'ン', // FF9C..FF9D
];

/// La forme sonore d'un katakana, quand elle existe (`カ` → `ガ`).
const fn voise(c: char) -> Option<char> {
    let sonore = match c {
        'カ' => 'ガ',
        'キ' => 'ギ',
        'ク' => 'グ',
        'ケ' => 'ゲ',
        'コ' => 'ゴ',
        'サ' => 'ザ',
        'シ' => 'ジ',
        'ス' => 'ズ',
        'セ' => 'ゼ',
        'ソ' => 'ゾ',
        'タ' => 'ダ',
        'チ' => 'ヂ',
        'ツ' => 'ヅ',
        'テ' => 'デ',
        'ト' => 'ド',
        'ハ' => 'バ',
        'ヒ' => 'ビ',
        'フ' => 'ブ',
        'ヘ' => 'ベ',
        'ホ' => 'ボ',
        // `ヴ` : rare, mais c'est ainsi que s'écrivent les noms occidentaux,
        // dont Dragon Ball est plein.
        'ウ' => 'ヴ',
        'ワ' => 'ヷ',
        'ヲ' => 'ヺ',
        _ => return None,
    };
    Some(sonore)
}

/// La forme semi-sonore d'un katakana, quand elle existe (`ハ` → `パ`).
const fn semi_voise(c: char) -> Option<char> {
    let sourd = match c {
        'ハ' => 'パ',
        'ヒ' => 'ピ',
        'フ' => 'プ',
        'ヘ' => 'ペ',
        'ホ' => 'ポ',
        _ => return None,
    };
    Some(sourd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_demi_chasse_redevient_de_la_pleine_chasse() {
        let (out, n) = normalise_demi_chasse("ｻｲﾔ人");
        assert_eq!(out, "サイヤ人");
        assert_eq!(n, 3);
    }

    #[test]
    fn une_marque_de_sonorisation_se_recompose_en_un_seul_caractere() {
        // `ｶﾞ` est deux caractères ; `ガ` en est un. Laisser `カ゛` derrière
        // serait remplacer un défaut par un autre.
        let (out, n) = normalise_demi_chasse("ﾄﾞﾗｺﾞﾝﾎﾞｰﾙ");
        assert_eq!(out, "ドラゴンボール");
        assert_eq!(out.chars().count(), 7);
        assert_eq!(n, 10, "dix caractères demi-chasse consommés");
    }

    #[test]
    fn une_marque_semi_sonore_se_recompose_aussi() {
        let (out, _) = normalise_demi_chasse("ﾊﾟﾝ");
        assert_eq!(out, "パン");
    }

    #[test]
    fn une_marque_de_sonorisation_orpheline_ne_casse_rien() {
        // `ﾝﾞ` n'existe pas : la marque n'a pas de forme sonore à composer.
        // Elle doit ressortir telle quelle plutôt que d'avaler le kana.
        let (out, _) = normalise_demi_chasse("ﾝﾞ");
        assert_eq!(out, "ン\u{FF9E}");
    }

    #[test]
    fn la_ponctuation_demi_chasse_redevient_pleine() {
        let (out, n) = normalise_demi_chasse("｢ｺﾞｸｳ｣｡");
        assert_eq!(out, "「ゴクウ」。");
        assert_eq!(n, 7);
    }

    #[test]
    fn du_texte_deja_en_pleine_chasse_nest_pas_touche() {
        let texte = "孫悟空とベジータ、そして「ドラゴンボール」１２３ＡＢＣ";
        let (out, n) = normalise_demi_chasse(texte);
        assert_eq!(out, texte);
        assert_eq!(n, 0, "les chiffres et lettres pleine chasse sont du texte réel");
    }

    #[test]
    fn lespace_ideographique_entre_deux_kanji_est_parasite() {
        // `U+3000` est l'espace que la typographie japonaise emploie pour
        // l'alinéa ; au milieu d'un mot, c'est le modèle qui l'a posée.
        let (out, n) = espaces_parasites("全集　を出して");
        assert_eq!(out, "全集を出して");
        assert_eq!(n, 1);
    }

    #[test]
    fn une_suite_despaces_disparait_dun_coup() {
        let (out, n) = espaces_parasites("全集 　を");
        assert_eq!(out, "全集を");
        assert_eq!(n, 2, "l'ordinaire et l'idéographique partent ensemble");
    }

    #[test]
    fn une_espace_qui_separe_du_latin_du_japonais_est_gardee() {
        let texte = "これは DRAGON BALL です";
        let (out, n) = espaces_parasites(texte);
        assert_eq!(out, texte);
        assert_eq!(n, 0);
    }

    #[test]
    fn une_espace_en_debut_ou_en_fin_de_ligne_est_gardee() {
        // L'indentation est de la mise en page ; elle n'est encadrée de rien.
        for texte in ["　孫悟空", "孫悟空　"] {
            let (out, n) = espaces_parasites(texte);
            assert_eq!(out, texte);
            assert_eq!(n, 0);
        }
    }

    #[test]
    fn une_suite_de_points_medians_demi_chasse_devient_une_ellipse() {
        // Cas réels : 638 planches du corpus, contre quatre au `･` isolé.
        for (avant, apres) in [
            ("ここから未知の世界の冒険がはじまるのか･･･?!", "ここから未知の世界の冒険がはじまるのか…?!"),
            ("見覚えのあるような人物が･･･?!", "見覚えのあるような人物が…?!"),
            ("身を寄せていたフリーザ一味も壊滅･･･。", "身を寄せていたフリーザ一味も壊滅…。"),
            ("見知らぬサーヤ人だった･･!!", "見知らぬサーヤ人だった…!!"),
        ] {
            let (out, n) = normalise_ponctuation(avant);
            assert_eq!(out, apres, "{avant}");
            assert!(n > 0);
        }
    }

    #[test]
    fn un_point_median_isole_reste_un_separateur() {
        // Quatre planches seulement, et là c'est bien un séparateur : c'est à
        // `normalise_demi_chasse` de le ramener en pleine chasse, pas ici.
        let (out, n) = normalise_ponctuation("孫悟空･ベジータ");
        assert_eq!(out, "孫悟空･ベジータ");
        assert_eq!(n, 0);
        let (pleine, _) = normalise_demi_chasse(&out);
        assert_eq!(pleine, "孫悟空・ベジータ");
    }

    #[test]
    fn six_points_medians_font_deux_ellipses() {
        let (out, _) = normalise_ponctuation("そうか･･････");
        assert_eq!(out, "そうか……");
    }

    #[test]
    fn trois_points_ascii_contre_du_japonais_deviennent_une_ellipse() {
        for (avant, apres) in [
            ("そうか...", "そうか…"),
            ("...そうか", "…そうか"),
            ("待って....ください", "待って…ください"),
        ] {
            let (out, n) = normalise_ponctuation(avant);
            assert_eq!(out, apres, "{avant}");
            assert!(n > 0);
        }
    }

    #[test]
    fn trois_points_ascii_en_contexte_latin_restent_en_ascii() {
        // 123 planches du corpus sont dans ce cas : le `...` y sépare des
        // entrées d'une liste en caractères latins, où l'ASCII est correct.
        for texte in ["S H ... VJB", "wait... what", "..."] {
            let (out, n) = normalise_ponctuation(texte);
            assert_eq!(out, texte, "{texte}");
            assert_eq!(n, 0);
        }
    }

    #[test]
    fn les_points_de_conduite_dun_tableau_survivent() {
        // Cas réel, planche 88/p103 : `アタック ....... P` est du mobilier de
        // tableau. Six points ou plus ne sont jamais une ellipse, même au
        // contact du japonais.
        let texte = "アタック .................................... P";
        let (out, n) = normalise_ponctuation(texte);
        assert_eq!(out, texte);
        assert_eq!(n, 0);
    }

    #[test]
    fn un_point_final_ordinaire_nest_pas_touche() {
        let texte = "これは文です. そして次.";
        let (out, n) = normalise_ponctuation(texte);
        assert_eq!(out, texte);
        assert_eq!(n, 0);
    }

    #[test]
    fn les_marques_diteration_comptent_comme_du_japonais() {
        // `人々`, `時々` : sans `々` dans la classe, une coupure juste avant
        // passait pour une frontière entre deux écritures.
        assert!(japonais('々'));
        assert!(japonais('〇'));
        assert!(japonais('〆'));
    }

    #[test]
    fn les_kanji_des_extensions_comptent_aussi() {
        assert!(japonais('\u{3400}'), "extension A");
        assert!(japonais('\u{F900}'), "compatibilité");
        assert!(japonais('漢'));
    }

    #[test]
    fn le_latin_et_la_ponctuation_ascii_ne_sont_pas_du_japonais() {
        for c in ['A', 'z', '1', '.', ' ', '-'] {
            assert!(!japonais(c), "{c}");
        }
    }

    #[test]
    fn les_classes_de_kana_ne_debordent_pas() {
        assert!(kana('あ') && kana('ア') && kana('ｱ'));
        assert!(!kana('漢') && !kana('々') && !kana('A'));
        assert!(katakana('ア') && katakana('ー'));
        assert!(!katakana('あ') && !katakana('漢'));
    }

    #[test]
    fn toute_demi_chasse_du_bloc_a_une_forme_pleine() {
        // Le tableau est indexé par position : un décalage d'un cran rendrait
        // `ｷ` en `カ` sur tout un corpus, en silence.
        for code in 0xFF61_u32..=0xFF9D {
            let c = char::from_u32(code).unwrap();
            let pleine = pleine_chasse(c).unwrap_or_else(|| panic!("{c} sans forme pleine"));
            assert!(!pleine.is_ascii(), "{c} -> {pleine}");
        }
        assert_eq!(pleine_chasse('ｱ'), Some('ア'));
        assert_eq!(pleine_chasse('ﾝ'), Some('ン'));
        assert_eq!(pleine_chasse('ﾂ'), Some('ツ'));
        assert_eq!(pleine_chasse('ﾎ'), Some('ホ'));
    }
}
