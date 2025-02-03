use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::constants::APOSTROPHES;
use grac::{
    add_acute_at, ends_with_diphthong, has_diacritic, remove_diacritic_at, syllabify_el,
    syllabify_el_mode, Diacritic, Synizesis,
};

// TODO: is this in grac?
const CORRECT_MONOSYLLABLE_ACCENTED: &[&str] = &[
    // Original
    "μού", "μάς", "τού", "τής", "τούς", "τών", "σού", "σάς", "πώς", "πού", "ή", "νά", "έν", "έξ",
    // Capitalized
    "Μού", "Μάς", "Τού", "Τής", "Τούς", "Τών", "Σού", "Σάς", "Πώς", "Πού", "Ή", "Νά", "Έν", "Έξ",
];

fn is_monosyllable_accented(word: &str) -> bool {
    syllabify_el_mode(word, Synizesis::Never).len() == 1
        && has_diacritic(word, Diacritic::ACUTE)
        // Do not treat "πλάι" as en error.
        && !ends_with_diphthong(word)
}

fn monosyllable_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.greek {
        return None;
    }

    if CORRECT_MONOSYLLABLE_ACCENTED.contains(&token.text) {
        return None;
    }

    // Do not remove accents from abbreviations: όλ' αυτά
    // Nor final periods thay may indicate abbreviation: Μέσ., μέλ.
    let ntoken = doc.get(token.index + 1)?;
    if token.whitespace.is_empty() && ntoken.punct {
        if let Some(npunct_first_char) = ntoken.text.chars().next() {
            if APOSTROPHES.contains(&npunct_first_char) {
                return None;
            }
            // A final period requires checking that the next word is capitalized
            if npunct_first_char == '.' {
                // Consider ellipsis as a black box
                if ntoken.text.starts_with("...") {
                    return None;
                }

                let mut index = 2;
                loop {
                    // Should actually return Some(()) if there is no token
                    let nntoken = doc.get(token.index + index)?;
                    index += 1;
                    if !nntoken.punct {
                        if let Some(nnpunct_first_char) = nntoken.text.chars().next() {
                            if !nnpunct_first_char.is_uppercase() {
                                return None;
                            }
                        }
                        break;
                    }
                }
            }
        }
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
            fix: Some(Fix {
                replacement: format!("{}{}", without_accent, token.whitespace),
                range: token.range,
            }),
        });
    }
}

fn is_multisyllable_not_accented(word: &str) -> bool {
    syllabify_el(word).len() > 1 && !has_diacritic(word, Diacritic::ACUTE)
}

fn multisyllable_not_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.greek {
        return None;
    }

    // Ignore if all caps. Titles do not have accents.
    if token.text.chars().all(|c| c.is_uppercase()) {
        return None;
    }

    // Do not remove accents from abbreviations: όλ' αυτά
    if let Some(ptoken) = doc.get(token.index.saturating_sub(1)) {
        if ptoken.punct {
            if let Some(npunct_first_char) = ptoken.text.chars().next() {
                if APOSTROPHES.contains(&npunct_first_char) {
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
        // This should have no Fix
        // but print the error anyway!
        // tmp solution to print the error: replace it with itself + add random acute
        let _fix = Fix {
            replacement: format!("{}{}", add_acute_at(token.text, 1), token.whitespace),
            range: token.range,
        };
        let diagnostic = Diagnostic {
            kind: Rule::MultisyllableNotAccented,
            fix: None,
        };
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    macro_rules! test_empty_diagnostics {
        ($name:ident, $text:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let text = $text;
                let doc = tokenize(text);
                let mut diagnostics = Vec::new();
                monosyllable_accented(&doc[0], &doc, &mut diagnostics);
                assert_eq!(diagnostics.is_empty(), $expected);
            }
        };
    }

    test_empty_diagnostics!(test_final_period, "μέλ. Και άλλα.", false);
    test_empty_diagnostics!(test_abbreviation_period, "μέλ. και άλλα.", true);
    test_empty_diagnostics!(test_ellipsis, "μέλ... Και άλλα.", true);
}
