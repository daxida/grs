use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::rules::forbidden_accent::CORRECT_MULTISYLLABLE_NOT_ACCENTED;
use crate::tokenizer::{Doc, Token};
use grac::constants::{APOSTROPHES, MONOSYLLABLE_ACCENTED_WITH_PRONOUNS};
use grac::{
    Diacritic, ends_with_diphthong, has_diacritic, has_diacritics, remove_diacritic_at,
    with_capitalized,
};

fn is_monosyllable_accented(token: &Token) -> bool {
    // Fast discard if possible
    token.text().len() < 12
        && has_diacritic(token.text(), Diacritic::ACUTE)
        // Do not treat "πλάι" as en error.
        && !ends_with_diphthong(token.text())
        // Expensive check
        && token.num_syllables() == 1
}

// This extra list is intended to deal with τί (not included) and ποιός variants.
//
// While τι is already detected, ποιός escapes our logic by not being a monosyllable
// once it has the accent.
//
// It does not include ποιόν, ποιού since they can also come from the noun ποιόν.
const EXTRA_MONOSYLLABLES: [&str; 16] = with_capitalized!([
    "ποιός",
    "ποιό",
    "ποιοί",
    "ποιών",
    "ποιούς",
    "ποιά",
    "ποιάς",
    "ποιές",
]);

fn monosyllable_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.is_greek_word()
        || MONOSYLLABLE_ACCENTED_WITH_PRONOUNS.contains(&token.text())
        || doc.is_abbreviation_or_ends_with_dot(token)
        || doc.previous_token_is_num(token)
        || doc.previous_token_is_apostrophe(token)
    {
        return None;
    }

    if EXTRA_MONOSYLLABLES.contains(&token.text()) || is_monosyllable_accented(token) {
        return Some(());
    }

    None
}

/// Detect wrongly accented monosyllables
pub fn monosyllable_accented(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if monosyllable_accented_opt(token, doc).is_some() {
        let without_accent = remove_diacritic_at(token.text(), 1, Diacritic::ACUTE);
        diagnostics.push(Diagnostic {
            kind: Rule::MonosyllableAccented,
            range: token.range(),
            fix: Some(Fix {
                replacement: without_accent,
                range: token.range(),
            }),
        });
    }
}

fn is_multisyllable_not_accented(token: &Token) -> bool {
    !has_diacritics(
        token.text(),
        &[Diacritic::ACUTE, Diacritic::GRAVE, Diacritic::CIRCUMFLEX],
    ) && token.num_syllables() > 1
}

// ** Can appear on capitalized position.
// https://el.wiktionary.org/wiki/προτακτικό
#[rustfmt::skip]
const PROSTAKTIKOI: [&str; 26] = with_capitalized!([
    "αγια", "αγιο", "αϊ", "γερο", "γρια", "θεια",
    "κυρα", "κερα", "μαστρο", "μπαρμπα", "παπα",
    "χατζη", "ψευτο",
]);

const fn is_dash(ch: char) -> bool {
    matches!(ch, '–' | '-')
}

fn multisyllable_not_accented_opt(token: &Token, doc: &Doc) -> Option<()> {
    if !token.is_greek_word()
        || CORRECT_MULTISYLLABLE_NOT_ACCENTED.contains(&token.text())
        || doc.is_abbreviation_or_ends_with_dot(token)
        || doc.previous_token_is_num(token)
        // Ignore if all caps. Ex. ΒΟΥΤΥΡΑ is correct.
        || token.text().chars().all(char::is_uppercase)
        // Ignore acronyms and some other compounds. Ex. Α.Υ., Ο,ΤΙ ΝΑ 'ΝΑΙ
        || token.text().contains(['.', '|', ':', ',', '/', '-', '('])
    {
        return None;
    }

    if let Some(ptoken) = doc.prev_token_not_whitespace(token)
        && ptoken.is_punctuation()
        && let Some(ppunct_first_char) = ptoken.text().chars().next()
        && APOSTROPHES.contains(&ppunct_first_char)
    {
        return None;
    }
    if let Some(ntoken) = doc.next_token_not_whitespace(token)
        && ntoken.is_punctuation()
        && let Some(npunct_first_char) = ntoken.text().chars().next()
        && (APOSTROPHES.contains(&npunct_first_char)
            || (PROSTAKTIKOI.contains(&token.text()) && is_dash(npunct_first_char)))
    // Maybe just ignoring all dashes makes more sense
    // || is_dash(npunct_first_char)
    {
        return None;
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
            range: token.range(),
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

    // After numbers, with and without accent should be accepted
    test_mono!(mono_number, "του 20ού αιώνα", true);

    // After apostrophe, it can be anything
    test_mono!(mono_apostrophe_start, "Ξέρω ’γώ;", true);
    // test_mono!(mono_apostrophe_inside, "Tι βλέπ'ς;", true); // Unsupported

    // Ποιος
    test_mono!(mono_poios1, "Μα ποιός ή ποιά έγραψε το λήμμα;", false);
    test_mono!(mono_poios2, "Ποιάς φλόγας;", false);
    test_mono!(mono_poios3, "ηθοποιός", true);
    test_mono!(mono_poios4, "το ποιόν της κοινωνικής περίθαλψης", true);

    // ** Multisyllable
    // * Has error
    test_multi!(multi_base, "καλημερα", false);
    // * Has no error
    test_multi!(multi_period1, "επεξ. επιλεγμένο", true);
    test_multi!(multi_period2, "επεξ. Επιλεγμένο", true);
    test_multi!(multi_acronym, "Α.Υ.", true);
    test_multi!(multi_punct1, "του/της", true);
    test_multi!(multi_punct2, "ΒΙΒΛΙΟΝ Θ(Ο τύπος)", true);
    test_multi!(multi_hyphen1, "Μπαρτ-Χιρστ", true);
    test_multi!(multi_hyphen2, "ΒΟΥΤΥΡΑ-ΕΛΑΙΑ", true);
    test_multi!(multi_hyphen3, "5ος–6ος αιώνας π.Χ.", true);
    test_multi!(multi_apostrophe1, "μου 'ρχεται να", true);
    test_multi!(multi_apostrophe2, "μου ' ρχεται να", true);
    test_multi!(multi_apostrophe3, "μου' ρχεται να", true);
    test_multi!(multi_apostrophe4, "να ’λεγε", true);
    test_multi!(multi_capital_comma, "Ο,ΤΙ ΝΑ 'ΝΑΙ", true);
    test_multi!(multi_final_n, "μιαν ανήσυχη ματιά", true);
    test_multi!(multi_synizesis1, "δια", true);
    test_multi!(multi_synizesis2, "αυτή μες στο βιο της", true);
    test_multi!(multi_final_period, "απεβ. το 330 π.Χ.", true);
    test_multi!(multi_ellipsis, "αλλω… τι;", true);
    test_multi!(multi_number, "ο 39χρονος αγνοούμενος", true);
    // This includes wrong APOSTROPHE variations
    test_multi!(multi_number_greek1, "τομ. ΙΑ΄, σελ.", true);
    test_multi!(multi_number_greek2, "ΙΑ'. Θεσσαλονίκη", true);
    test_multi!(multi_number_greek3, "(Κατά Λουκάν ιη´)", true);
    test_multi!(multi_number_greek4, " οη΄.", true);
    test_multi!(multi_pio1, "και έκυψε να πιη ύδωρ", true);
    test_multi!(multi_pio2, "Άμα πιης τσάι", true);

    // Ancient non accented words
    test_multi!(multi_einai, "Κύριός εστιν", true);
    test_multi!(multi_tis, "ἐπὶ δυοῖν τινοιν διαφοραῖν", true);
    test_multi!(multi_leo1, "ξύλου μόλιβδόν φασιν", true);
    test_multi!(multi_leo2, "σώματά φασι", true);

    // Prostaktikoi
    test_multi!(multi_prostatiko1, "γερο - Ευθύμιο", true);
    test_multi!(multi_prostatiko2, "γερο-Ευθύμιο", true);
    test_multi!(multi_prostatiko3, "παπα - Ευθύμιο", true);
    test_multi!(multi_prostatiko4, "διέκοπτε ο σιορ- Αμπρουζής", true);
    test_multi!(multi_prostatiko5, "τούτος ο ψευτο - Εγγλέζος.", true);
    test_multi!(multi_prostatiko6, "τον μπαρμπα – Δημητρό", true);
    test_multi!(multi_prostatiko7, "Η κερα - Κατερίνα", true);
}
