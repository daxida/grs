use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::constants::{APOSTROPHES, MONOSYLLABLE_ACCENTED_WITH_PRONOUNS};
use grac::{
    ends_with_diphthong, has_diacritic, has_diacritics, remove_diacritic_at, syllabify_el,
    syllabify_el_mode, Diacritic, Synizesis,
};

fn is_monosyllable_accented(word: &str) -> bool {
    syllabify_el_mode(word, Synizesis::Never).len() == 1
        && has_diacritic(word, Diacritic::ACUTE)
        // Do not treat "πλάι" as en error.
        && !ends_with_diphthong(word)
}

/// A word is considered an abbreviation if:
/// * It is followed by an apostrophe. Ex. όλ' αυτά
/// * It is followed by a dot that it not a period (the sentence does not end at it).
///   Ex. απεβ. το 330 OR μέλ... και άλλα.
fn is_abbreviation(token: &Token, doc: &Doc) -> bool {
    if let Some(ntoken) = doc.get(token.index + 1) {
        if token.whitespace.is_empty() && ntoken.punct {
            if let Some(npunct_first_char) = ntoken.text.chars().next() {
                if APOSTROPHES.contains(&npunct_first_char) {
                    return true;
                }
                // A final period requires checking that the next word is capitalized
                if npunct_first_char == '.' {
                    // Consider ellipsis as a black box
                    if ntoken.text.starts_with("...") {
                        return true;
                    }

                    let mut index = 2;
                    // If we reached EOF (the .get retuned None), it makes sense
                    // to claim that there was no abbreviation. It meant that the text
                    // ended with some period-starting ntoken: "... μέλ.[EOF]"
                    while let Some(nntoken) = doc.get(token.index + index) {
                        index += 1;
                        if !nntoken.punct {
                            if let Some(nnpunct_first_char) = nntoken.text.chars().next() {
                                if !nnpunct_first_char.is_uppercase() {
                                    return true;
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    false
}

fn monosyllable_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.greek
        || MONOSYLLABLE_ACCENTED_WITH_PRONOUNS.contains(&token.text)
        || is_abbreviation(token, doc)
    {
        return None;
    }

    if is_monosyllable_accented(token.text) {
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

fn is_multisyllable_not_accented(word: &str) -> bool {
    syllabify_el(word).len() > 1
        && !has_diacritics(
            word,
            &[Diacritic::ACUTE, Diacritic::GRAVE, Diacritic::CIRCUMFLEX],
        )
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
        || is_abbreviation(token, doc)
    {
        return None;
    }

    // Ignore if all caps. Titles do not have accents.
    // Ignore also some inside punctuation. Ex. ΒΟΥΤΥΡΑ-ΕΛΑΙΑ is correct.
    if token.text.chars().all(|c| c.is_uppercase() || c == '-') {
        return None;
    }

    // Ignore acronyms and some other compounds:
    // * Α.Υ.
    // * {{ετικ|λαϊκ|ιατρ}}
    if token.text.contains(['.', '|', ':']) {
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

    if is_multisyllable_not_accented(token.text) {
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
    test_mono!(base_mono_one, "μέλ", false);
    test_mono!(base_mono_two, "μέλ  ", false);
    test_mono!(mono_final_period, "μέλ. Και άλλα.", false);
    // * Has no error
    test_mono!(abbreviation_period, "μέλ. και άλλα.", true);
    test_mono!(ellipsis_one, "μέλ... Και άλλα.", true);
    test_mono!(ellipsis_two, "μέλ... και άλλα.", true);
    test_mono!(old_numbers, "είς των βοσκών", true);

    // ** Multisyllable
    // * Has error
    test_multi!(base_multi, "καλημερα", false);
    // * Has no error
    test_multi!(acronym, "Α.Υ.", true);
    test_multi!(capital_hyphen, "ΒΟΥΤΥΡΑ-ΕΛΑΙΑ", true);
    test_multi!(final_n, "μιαν ανήσυχη ματιά", true);
    test_multi!(gero_one, "γερο - Ευθύμιο", true);
    test_multi!(gero_two, "γερο-Ευθύμιο", true);
    test_multi!(papa, "παπα - Ευθύμιο", true);
    test_multi!(synizesis, "δια", true);
    test_multi!(multi_final_period, "απεβ. το 330 π.Χ.", true);

    #[test]
    fn apostrophe() {
        // Requires the given token to be on some position > 0
        let text = "να ’λεγε";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        multisyllable_not_accented(&doc[2], &doc, &mut diagnostics);
        assert_eq!(doc[2].text, "λεγε");
        assert_eq!(diagnostics.is_empty(), true);
    }
}
