use crate::diagnostic::{Diagnostic, Fix};
use crate::doc::Doc;
use crate::doc::{is_abbreviation_or_ends_with_dot, previous_token_is_num};
use crate::registry::Rule;
use crate::tokenizer::Token;
use grac::constants::{APOSTROPHES, MONOSYLLABLE_ACCENTED_WITH_PRONOUNS};
use grac::{ends_with_diphthong, has_diacritic, has_diacritics, remove_diacritic_at, Diacritic};

fn is_monosyllable_accented(token: &Token) -> bool {
    // Fast discard if possible
    token.text.len() < 12
        && has_diacritic(token.text, Diacritic::ACUTE)
        // Do not treat "πλάι" as en error.
        && !ends_with_diphthong(token.text)
        // Expensive check
        && token.syllables().len() == 1
}

fn monosyllable_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.greek
        || MONOSYLLABLE_ACCENTED_WITH_PRONOUNS.contains(&token.text)
        || is_abbreviation_or_ends_with_dot(token, doc)
        || previous_token_is_num(token, doc)
    {
        return None;
    }

    if is_monosyllable_accented(token) {
        return Some(());
    }

    None
}

/// Detect wrongly accented monosyllables
pub fn monosyllable_accented(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if monosyllable_accented_opt(token, doc).is_some() {
        let without_accent = remove_diacritic_at(token.text, 1, Diacritic::ACUTE);
        diagnostics.push(Diagnostic {
            kind: Rule::MonosyllableAccented,
            range: token.range_text(),
            fix: Some(Fix {
                replacement: format!("{}{}", without_accent, token.whitespace),
                range: token.range,
            }),
        });
    }
}

fn is_multisyllable_not_accented(token: &Token) -> bool {
    !has_diacritics(
        token.text,
        &[Diacritic::ACUTE, Diacritic::GRAVE, Diacritic::CIRCUMFLEX],
    ) && token.syllables().len() > 1
}

// ** Can NOT appear on capitalized position, so no uppercase.
#[rustfmt::skip]
const CORRECT_MULTISYLLABLE_NOT_ACCENTED: &[&str] = &[
    "ποτε",
    // https://el.wiktionary.org/wiki/τινά
    "τινες", "τινα", "τινε", "τινος", "τινων", "τινοιν", "τινι", "τισι", "τινας",
    "τονε", "τηνε",
    // ** These can appear on capitalized position
];

// ** Can appear on capitalized position.
// https://el.wiktionary.org/wiki/προτακτικό
#[rustfmt::skip]
const PROSTAKTIKOI: &[&str] = &[
    // Lowercase
    "αγια", "αγιο", "αϊ", "γερο", "γρια", "θεια",
    "κυρα", "μαστρο", "μπαρμπα", "παπα", "χατζη",
    "σιορ",
    // Uppercase
    "Αγια", "Αγιο", "Αϊ", "Γερο", "Γρια", "Θεια",
    "Κυρα", "Μαστρο", "Μπαρμπα", "Παπα", "Χατζη",
    "Σιορ",
];

fn multisyllable_not_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.greek
        || CORRECT_MULTISYLLABLE_NOT_ACCENTED.contains(&token.text)
        || is_abbreviation_or_ends_with_dot(token, doc)
        || previous_token_is_num(token, doc)
        // Ignore if all caps. Ex. ΒΟΥΤΥΡΑ is correct.
        || token.text.chars().all(char::is_uppercase)
        // Ignore acronyms and some other compounds. Ex. Α.Υ., Ο,ΤΙ ΝΑ 'ΝΑΙ
        || token.text.contains(['.', '|', ':', ',', '/', '-', '('])
    {
        return None;
    }

    if let Some(ptoken) = doc.get(token.index.saturating_sub(1)) {
        if ptoken.punct {
            if let Some(ppunct_first_char) = ptoken.text.chars().next() {
                if APOSTROPHES.contains(&ppunct_first_char) {
                    return None;
                }
            }
        }
    }
    if let Some(ntoken) = doc.get(token.index + 1) {
        if ntoken.punct {
            if let Some(npunct_first_char) = ntoken.text.chars().next() {
                if APOSTROPHES.contains(&npunct_first_char) {
                    return None;
                }
                if PROSTAKTIKOI.contains(&token.text) && npunct_first_char == '-' {
                    return None;
                }
            }
        }
    }

    if is_multisyllable_not_accented(token) {
        return Some(());
    }

    None
}

pub fn multisyllable_not_accented(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if multisyllable_not_accented_opt(token, doc).is_some() {
        // No Fix: we can not know where the accent was supposed to be.
        diagnostics.push(Diagnostic {
            kind: Rule::MultisyllableNotAccented,
            range: token.range_text(),
            fix: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rule;
    use crate::tokenizer::tokenize;

    #[test]
    fn test_range_mono() {
        let text = "Ώς κι ο μπαρμπα-Στάθης";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        monosyllable_accented(&doc[0], &doc, &mut diagnostics);
        assert!(!diagnostics.is_empty());

        let diagnostic = &diagnostics[0];
        let range = diagnostic.range;
        assert_eq!(range.start(), 0);
        assert_eq!(range.end(), "Ώς".len());
    }

    #[test]
    fn test_range_multi() {
        let text = "Αλλο ";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        multisyllable_not_accented(&doc[0], &doc, &mut diagnostics);
        assert!(!diagnostics.is_empty());

        let diagnostic = &diagnostics[0];
        let range = diagnostic.range;
        assert_eq!(range.start(), 0);
        assert_eq!(range.end(), "Αλλο".len());
    }

    macro_rules! test_mono {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, monosyllable_accented, $text, $expected);
        };
    }

    macro_rules! test_multi {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, multisyllable_not_accented, $text, $expected);
        };
    }

    // ** Monosyllable
    // * Has error
    test_mono!(mono_base1, "μέλ", false);
    test_mono!(mono_base2, "μέλ  ", false);
    // * Has no error
    test_mono!(mono_period1, "μέλ. Και άλλα.", true);
    test_mono!(mono_period2, "μέλ. και άλλα.", true);
    test_mono!(mono_ellipsis1, "μέλ... Και άλλα.", true);
    test_mono!(mono_ellipsis2, "μέλ... και άλλα.", true);
    test_mono!(mono_ellipsis3, "μέλ… και άλλα.", true);
    test_mono!(mono_old_numbers, "είς των βοσκών", true);
    test_mono!(mono_abbreviation, "ἄρ᾽ Ἀθήνας", true);

    // ** Multisyllable
    // * Has error
    test_multi!(multi_base, "καλημερα", false);
    // * Has no error
    test_multi!(multi_period_one, "επεξ. επιλεγμένο", true);
    test_multi!(multi_period_two, "επεξ. Επιλεγμένο", true);
    test_multi!(multi_acronym, "Α.Υ.", true);
    test_multi!(multi_punct1, "του/της", true);
    test_multi!(multi_punct2, "ΒΙΒΛΙΟΝ Θ(Ο τύπος)", true);
    test_multi!(multi_hyphen, "Μπαρτ-Χιρστ", true);
    test_multi!(multi_hyphen_capital, "ΒΟΥΤΥΡΑ-ΕΛΑΙΑ", true);
    test_multi!(multi_hyphen_variation, "5ος–6ος αιώνας π.Χ.", true);
    test_multi!(capital_comma, "Ο,ΤΙ ΝΑ 'ΝΑΙ", true);
    test_multi!(final_n, "μιαν ανήσυχη ματιά", true);
    test_multi!(synizesis, "δια", true);
    test_multi!(multi_final_period, "απεβ. το 330 π.Χ.", true);
    test_multi!(multi_ellipsis, "αλλω… τι;", true);

    // Prostaktikoi
    test_multi!(prostatiko1, "γερο - Ευθύμιο", true);
    test_multi!(prostatiko2, "γερο-Ευθύμιο", true);
    test_multi!(prostatiko3, "παπα - Ευθύμιο", true);
    test_multi!(prostatiko4, "διέκοπτε ο σιορ- Αμπρουζής", true);

    // Requires the given token to be on some position > 0
    #[test]
    fn multi_apostrophe() {
        let text = "να ’λεγε";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        multisyllable_not_accented(&doc[2], &doc, &mut diagnostics);
        assert_eq!(doc[2].text, "λεγε");
        assert!(diagnostics.is_empty());
    }

    // After numbers, with and without accent should be accepted
    #[test]
    fn mono_after_number() {
        let text = "του 20ού αιώνα";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        monosyllable_accented(&doc[2], &doc, &mut diagnostics);
        assert_eq!(doc[2].text, "ού");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn multi_after_number() {
        let text = "ο 39χρονος αγνοούμενος";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        multisyllable_not_accented(&doc[2], &doc, &mut diagnostics);
        assert_eq!(doc[2].text, "χρονος");
        assert!(diagnostics.is_empty());
    }
}
