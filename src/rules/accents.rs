use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
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

/// A word is considered an abbreviation if it is followed by an apostrophe.
/// Ex. όλ' αυτά
///
/// A dot must be treated like a black box since there is no way to distinguish
/// if it is a period, an ellipsis or an abbreviation dot. Checking if the next word
/// is capitalized is not a solution, since an abbreviation might be followed by
/// a proper noun, invalidating the logic. Ex. Λεωφ. Κηφισού.
fn is_abbreviation_or_ends_with_dot(token: &Token, doc: &Doc) -> bool {
    if let Some(ntoken) = doc.get(token.index + 1) {
        if token.whitespace.is_empty() && ntoken.punct {
            if let Some(npunct_first_char) = ntoken.text.chars().next() {
                if ['.', '…'].contains(&npunct_first_char)
                    || APOSTROPHES.contains(&npunct_first_char)
                {
                    return true;
                }
            }
        }
    }

    false
}

fn previous_token_is_num(token: &Token, doc: &Doc) -> bool {
    match doc.get(token.index.saturating_sub(1)) {
        Some(ptoken) => {
            ptoken.punct
                && ptoken.whitespace.is_empty()
                && ptoken.text.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
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
    // Uppercase
    "Αγια", "Αγιο", "Αϊ", "Γερο", "Γρια", "Θεια",
    "Κυρα", "Μαστρο", "Μπαρμπα", "Παπα", "Χατζη"
];

fn multisyllable_not_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.greek
        || CORRECT_MULTISYLLABLE_NOT_ACCENTED.contains(&token.text)
        || is_abbreviation_or_ends_with_dot(token, doc)
        || previous_token_is_num(token, doc)
    {
        return None;
    }

    // Ignore acronyms and some other compounds:
    // * Α.Υ.
    // * {{ετικ|λαϊκ|ιατρ}}
    // * Ο,ΤΙ ΝΑ 'ΝΑΙ
    if token.text.contains(['.', '|', ':', ',', '/', '-']) {
        return None;
    }

    // Ignore if all caps. Ex. ΒΟΥΤΥΡΑ is correct.
    if token.text.chars().all(|c| c.is_uppercase()) {
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

    macro_rules! test {
        ($name:ident, $fn:expr, $text:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let text = $text;
                let doc = tokenize(text);
                let mut diagnostics = Vec::new();
                $fn(&doc[0], &doc, &mut diagnostics);
                assert_eq!(diagnostics.is_empty(), $expected);
            }
        };
    }

    macro_rules! test_mono {
        ($name:ident, $text:expr, $expected:expr) => {
            test!($name, monosyllable_accented, $text, $expected);
        };
    }

    macro_rules! test_multi {
        ($name:ident, $text:expr, $expected:expr) => {
            test!($name, multisyllable_not_accented, $text, $expected);
        };
    }

    // ** Monosyllable
    // * Has error
    test_mono!(mono_base_one, "μέλ", false);
    test_mono!(mono_base_two, "μέλ  ", false);
    // * Has no error
    test_mono!(mono_period_one, "μέλ. Και άλλα.", true);
    test_mono!(mono_period_two, "μέλ. και άλλα.", true);
    test_mono!(mono_ellipsis_one, "μέλ... Και άλλα.", true);
    test_mono!(mono_ellipsis_two, "μέλ... και άλλα.", true);
    test_mono!(mono_ellipsis_three, "μέλ… και άλλα.", true);
    test_mono!(mono_old_numbers, "είς των βοσκών", true);
    test_mono!(mono_abbreviation, "ἄρ᾽ Ἀθήνας", true);

    // ** Multisyllable
    // * Has error
    test_multi!(multi_base, "καλημερα", false);
    // * Has no error
    test_multi!(multi_period_one, "επεξ. επιλεγμένο", true);
    test_multi!(multi_period_two, "επεξ. Επιλεγμένο", true);
    test_multi!(multi_acronym, "Α.Υ.", true);
    test_multi!(multi_punct, "του/της", true);
    test_multi!(multi_hyphen, "Μπαρτ-Χιρστ", true);
    test_multi!(multi_hyphen_capital, "ΒΟΥΤΥΡΑ-ΕΛΑΙΑ", true);
    test_multi!(capital_comma, "Ο,ΤΙ ΝΑ 'ΝΑΙ", true);
    test_multi!(final_n, "μιαν ανήσυχη ματιά", true);
    test_multi!(gero_one, "γερο - Ευθύμιο", true);
    test_multi!(gero_two, "γερο-Ευθύμιο", true);
    test_multi!(papa, "παπα - Ευθύμιο", true);
    test_multi!(synizesis, "δια", true);
    test_multi!(multi_final_period, "απεβ. το 330 π.Χ.", true);
    test_multi!(multi_ellipsis, "αλλω… τι;", true);

    // Requires the given token to be on some position > 0
    #[test]
    fn multi_apostrophe() {
        let text = "να ’λεγε";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        multisyllable_not_accented(&doc[2], &doc, &mut diagnostics);
        assert_eq!(doc[2].text, "λεγε");
        assert_eq!(diagnostics.is_empty(), true);
    }

    // After numbers, with and without accent should be accepted
    #[test]
    fn mono_after_number() {
        let text = "του 20ού αιώνα";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        monosyllable_accented(&doc[2], &doc, &mut diagnostics);
        assert_eq!(doc[2].text, "ού");
        assert_eq!(diagnostics.is_empty(), true);
    }

    #[test]
    fn multi_after_number() {
        let text = "ο 39χρονος αγνοούμενος";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        multisyllable_not_accented(&doc[2], &doc, &mut diagnostics);
        assert_eq!(doc[2].text, "χρονος");
        assert_eq!(diagnostics.is_empty(), true);
    }
}
