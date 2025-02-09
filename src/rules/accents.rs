use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::constants::APOSTROPHES;
use grac::{
    ends_with_diphthong, has_diacritic, has_diacritics, remove_diacritic_at, syllabify_el,
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
    if let Some(ntoken) = doc.get(token.index + 1) {
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
            range: token.range,
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

// They should never appear on capitalized position, so no uppercase.
#[rustfmt::skip]
const CORRECT_MULTISYLLABLE_NOT_ACCENTED: &[&str] = &[
    "ποτε",
    // https://el.wiktionary.org/wiki/τινά
    "τινες", "τινα", "τινε", "τινος", "τινων", "τινοιν", "τινι", "τισι", "τινας",
    "τονε", "τηνε",
];

fn multisyllable_not_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.greek {
        return None;
    }

    if CORRECT_MULTISYLLABLE_NOT_ACCENTED.contains(&token.text) {
        return None;
    }

    // Ignore if all caps. Titles do not have accents.
    if token.text.chars().all(|c| c.is_uppercase()) {
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
            range: token.range,
            fix: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

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

    test!(base_mono_one, monosyllable_accented, "μέλ", false);
    test!(base_mono_two, monosyllable_accented, "μέλ  ", false);
    test!(final_period, monosyllable_accented, "μέλ. Και άλλα.", false);
    test!(
        abbreviation_period,
        monosyllable_accented,
        "μέλ. και άλλα.",
        true
    );
    test!(ellipsis, monosyllable_accented, "μέλ... Και άλλα.", true);

    test!(base_multi, multisyllable_not_accented, "καλημερα", false);
    test!(acronym, multisyllable_not_accented, "Α.Υ.", true);

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
