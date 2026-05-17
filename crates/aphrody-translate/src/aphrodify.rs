//! Réécriture stylistique « Aphrody » : sobre, impersonnel, centrée sur le code.
//!
//! Le style vise une prose courte, sans fioriture ni adresse au lecteur. Aucune
//! ornementation poétique injectée — les références classiques restent dans la
//! tête de l'auteur. Le rôle ici est de retirer les tournures conversationnelles
//! héritées d'un dialogue assistant (« je vais », « nous allons », « voyons »,
//! « note que… ») et de remettre le texte au présent technique.

use regex::Regex;
use std::sync::OnceLock;

static PREFIX_NOTE: OnceLock<Regex> = OnceLock::new();
static PREFIX_TODO_VAGUE: OnceLock<Regex> = OnceLock::new();
static FILLER_OPENERS: OnceLock<Vec<Regex>> = OnceLock::new();
static FIRST_PERSON_FR: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
static FIRST_PERSON_EN: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

fn prefix_note() -> &'static Regex {
    PREFIX_NOTE.get_or_init(|| {
        Regex::new(r"(?i)^\s*(note|nota bene|nb|remarque|attention)\s*[:\-—]\s*").unwrap()
    })
}

fn prefix_todo_vague() -> &'static Regex {
    PREFIX_TODO_VAGUE
        .get_or_init(|| Regex::new(r"(?i)^\s*todo\s*[:\-]?\s*(implement|finish|complete|do)\s*$").unwrap())
}

fn filler_openers() -> &'static Vec<Regex> {
    FILLER_OPENERS.get_or_init(|| {
        [
            r"(?i)^\s*(voyons|regardons|allons-y|d'abord|tout d'abord|en effet|bien sûr|évidemment)\s*[,.:]?\s*",
            r"(?i)^\s*(let's|here we|here's how|let me|i'll|i will|we'll|we will|so,|now,|first,|alright,?)\s*",
            r"(?i)^\s*(this function|this method|this code|cette fonction|cette méthode|ce code)\s+",
        ]
        .iter()
        .map(|p| Regex::new(p).unwrap())
        .collect()
    })
}

fn first_person_fr() -> &'static Vec<(Regex, &'static str)> {
    FIRST_PERSON_FR.get_or_init(|| {
        let raw: &[(&str, &str)] = &[
            (r"\bj['e]\s*ai\b", ""),
            (r"\bj'ai\b", ""),
            (r"\bje vais\b", ""),
            (r"\bje\b", ""),
            (r"\bnous allons\b", ""),
            (r"\bnous\b", ""),
            (r"\bnotre\b", "le"),
            (r"\bnos\b", "les"),
            (r"\bmon\b", "le"),
            (r"\bma\b", "la"),
            (r"\bmes\b", "les"),
        ];
        raw.iter()
            .map(|(p, r)| (Regex::new(&format!("(?i){p}")).unwrap(), *r))
            .collect()
    })
}

fn first_person_en() -> &'static Vec<(Regex, &'static str)> {
    FIRST_PERSON_EN.get_or_init(|| {
        let raw: &[(&str, &str)] = &[
            (r"\bi['']?ll\b", ""),
            (r"\bi['']?ve\b", ""),
            (r"\bi['']?m\b", ""),
            (r"\bi\b", ""),
            (r"\bwe['']?ll\b", ""),
            (r"\bwe['']?ve\b", ""),
            (r"\bwe\b", ""),
            (r"\bour\b", "the"),
            (r"\bmy\b", "the"),
        ];
        raw.iter()
            .map(|(p, r)| (Regex::new(&format!("(?i){p}")).unwrap(), *r))
            .collect()
    })
}

pub fn aphrodify(text: &str) -> String {
    let mut s = text.to_string();

    // 1. Préfixes vides
    s = prefix_note().replace(&s, "").to_string();
    if prefix_todo_vague().is_match(&s) {
        return String::new();
    }

    // 2. Ouvertures de filler conversationnel
    for re in filler_openers() {
        s = re.replace(&s, "").to_string();
    }

    // 3. Première personne
    for (re, repl) in first_person_fr() {
        s = re.replace_all(&s, *repl).to_string();
    }
    for (re, repl) in first_person_en() {
        s = re.replace_all(&s, *repl).to_string();
    }

    // 4. Espaces multiples + ponctuation orpheline
    let re_multi_space = Regex::new(r"\s{2,}").unwrap();
    s = re_multi_space.replace_all(&s, " ").to_string();
    let re_orphan_punct = Regex::new(r"\s+([,.;:!?])").unwrap();
    s = re_orphan_punct.replace_all(&s, "$1").to_string();
    s = s.trim().to_string();

    // 5. Capitalisation + point final
    if let Some(first) = s.chars().next() {
        if first.is_alphabetic() && first.is_lowercase() {
            let mut chars = s.chars();
            let head = chars.next().unwrap();
            let tail: String = chars.collect();
            s = format!("{}{tail}", head.to_uppercase());
        }
    }
    if !s.is_empty()
        && !s.ends_with('.')
        && !s.ends_with('!')
        && !s.ends_with('?')
        && !s.ends_with(':')
    {
        s.push('.');
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_note_prefix() {
        assert_eq!(aphrodify("Note: parse the buffer"), "Parse the buffer.");
    }

    #[test]
    fn drop_vague_todo() {
        assert_eq!(aphrodify("TODO: implement"), "");
    }

    #[test]
    fn strip_first_person_fr() {
        let out = aphrodify("je vais ouvrir le fichier puis je le lis");
        assert!(!out.contains("je "));
        assert!(out.ends_with('.'));
    }

    #[test]
    fn strip_first_person_en() {
        let out = aphrodify("I'll open the file, then I parse it");
        assert!(!out.contains("I "));
    }

    #[test]
    fn capitalize_and_period() {
        assert_eq!(aphrodify("ouvre le buffer"), "Ouvre le buffer.");
    }

    #[test]
    fn keep_question_mark() {
        // L'espace orphelin avant `?` est collé, et la phrase étant déjà
        // ponctuée, aucun point supplémentaire n'est ajouté.
        assert_eq!(aphrodify("pourquoi ce check ?"), "Pourquoi ce check?");
    }
}
