use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::rules::missing_double_accents::PRONOUNS_LOWERCASE;
use crate::tokenizer::{Doc, Token};
use grac::{
    Diacritic, Merge, conc, has_diacritic, is_greek_char, remove_diacritic_at, syllabify_with_merge,
};

// https://el.wiktionary.org/wiki/τίς
const TIS_VARIANTS: [&str; 13] = [
    "τις",
    "τινος",
    "τινι",
    "τινα",
    "τι",
    "τω",
    "τινες",
    "τινων",
    "τισι",
    "τισιν",
    "τινας",
    "τινε",
    "τινοιν",
];

// https://en.wiktionary.org/wiki/εἰμί#Ancient_Greek
#[rustfmt::skip]
const ANCIENT_EINAI: [&str; 14] = [
    "εἰμι", "ἐστι", "ἐστιν", "εἰσι", "εἰσιν", "ἐσμεν", "ἐστε",
    // And their monotonic counterparts
    "ειμι", "εστι", "εστιν", "εισι", "εισιν", "εσμεν", "εστε",
];

// https://en.wiktionary.org/wiki/φημί#Ancient_Greek
// * φατε may conflict with φάω
#[rustfmt::skip]
const ANCIENT_LEO: [&str; 7] = [
    "φημι", "φασι", "φασιν", "φησι", "φησιν", "φαμεν", "φατε",
];

const PRONOUN_EXPANDED: [&str; 3] = ["τηνε", "τονε", "τωνε"];
const OTHER_EXTENSIONS: [&str; 3] = ["ποτε", "που", "γε"];

// Also used in accents.rs as an exception list.
// We share this list even though in accents.rs we only look for
// multisyllables (so checking for τι, of the TIS_VARIANTS, is a waste)
pub const CORRECT_MULTISYLLABLE_NOT_ACCENTED: [&str; 40] = conc!(
    TIS_VARIANTS,
    ANCIENT_EINAI,
    ANCIENT_LEO,
    PRONOUN_EXPANDED,
    OTHER_EXTENSIONS
);

#[rustfmt::skip]
const PRONOUN_VARIANTS: [&str; 7] = [
    // Ancient pronouns
    "των", "τας", "τε", "μοι", "σοι",
    // Abbreviations: καπετάνισσά μ᾿
    "μ", "τ",
];

// Maybe this could be added in missing_double_accents.
// The extension is intended to cover old greek.
const ALLOWED_WORDS_AFTER_DOUBLE_ACCENT: [&str; 62] = conc!(
    PRONOUNS_LOWERCASE,
    CORRECT_MULTISYLLABLE_NOT_ACCENTED,
    PRONOUN_VARIANTS
);

// Check for two type of errors:
// 1. words with accents before the antepenult.
// 2. words with two accents but not followed by a pronoun.
//
// We do them at the same time (instead of two rules) because the cost
// of calling syllabify_el_mode is relatively high.
//
// Caveats (for 1.):
// * Words elongated for emphasis: τίιιποτα.
// * Foreign names: Μπάουχαους

// Rewrite of grac::diacritic_pos that forces synizesis to prevent
// many false positives.
fn diacritic_pos(s: &str, diacritic: char, merge: Merge) -> Vec<usize> {
    syllabify_with_merge(s, merge)
        .iter()
        .rev()
        .enumerate()
        .filter_map(|(index, syllable)| {
            if has_diacritic(*syllable, diacritic) {
                Some(index + 1)
            } else {
                None
            }
        })
        .collect()
}

// Fast discard if possible (10 bytes ~ 5 Greek chars)
//
// 12 bytes should be fine for accents over the antepenult, but
// it is too short for double accents in wrong position. Cf. ρούχά του
fn skip_heuristic(token: &Token) -> bool {
    token.text().len() < 10 || !token.text().chars().all(is_greek_char)
}

fn forbidden_accent_opt(token: &Token, doc: &Doc) -> Option<Rule> {
    debug_assert!(token.is_greek_word());

    if skip_heuristic(token) {
        return None;
    }

    let pos = diacritic_pos(token.text(), Diacritic::ACUTE, Merge::Every);

    // accent before antepenult
    if pos.last().is_some_and(|pos| *pos > 3) {
        return Some(Rule::ForbiddenAccent);
    }

    // double accents with no pronoun
    // TODO: These should be fixable > remove last acute accent
    if pos.len() > 1 {
        // Check that the double accents are in the correct position
        //
        // We recompute the pos to not get a false positive on synizesis.
        // It should be quite cheap since this branch is quite rare already.
        let pos = diacritic_pos(token.text(), Diacritic::ACUTE, Merge::Never);
        if pos != [1, 3] {
            return Some(Rule::ForbiddenDoubleAccent);
        }

        // Compare against the first greek token found
        if let Some(ntoken) = doc.next_token_greek_word(token) {
            return if ALLOWED_WORDS_AFTER_DOUBLE_ACCENT.contains(&ntoken.text()) {
                None
            } else {
                Some(Rule::ForbiddenDoubleAccent)
            };
        }
    }

    None
}

pub fn forbidden_accent(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if token.is_greek_word() && forbidden_accent_opt(token, doc).is_some() {
        diagnostics.push(Diagnostic {
            kind: Rule::ForbiddenAccent,
            range: token.range(),
            fix: None,
        });
    }
}

fn forbidden_double_accent_opt(token: &Token, doc: &Doc) -> Option<()> {
    // This is separate from forbidden accent because it is fixable
    debug_assert!(token.is_greek_word());

    if skip_heuristic(token) {
        return None;
    }

    let pos = diacritic_pos(token.text(), Diacritic::ACUTE, Merge::Every);

    // double accents with no pronoun
    if pos.len() > 1 {
        // Check that the double accents are in the correct position
        //
        // We recompute the pos to not get a false positive on synizesis.
        // It should be quite cheap since this branch is quite rare already.
        let pos = diacritic_pos(token.text(), Diacritic::ACUTE, Merge::Never);
        if pos != [1, 3] {
            return Some(());
        }

        // Compare against the first greek token found
        if let Some(ntoken) = doc.next_token_greek_word(token) {
            return if ALLOWED_WORDS_AFTER_DOUBLE_ACCENT.contains(&ntoken.text()) {
                None
            } else {
                Some(())
            };
        }
    }

    None
}

pub fn forbidden_double_accent(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if token.is_greek_word() && forbidden_double_accent_opt(token, doc).is_some() {
        let without_accent = remove_diacritic_at(token.text(), 1, Diacritic::ACUTE);
        diagnostics.push(Diagnostic {
            kind: Rule::ForbiddenDoubleAccent,
            range: token.range(),
            fix: Some(Fix {
                replacement: without_accent,
                range: token.range(),
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_fix, test_rule};

    macro_rules! test_fa {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, forbidden_accent, $text, $expected);
        };
    }

    macro_rules! test_fda_fix {
        ($name:ident, $text:expr, $expected:expr) => {
            test_fix!($name, &[Rule::ForbiddenDoubleAccent], $text, $expected);
        };
    }

    test_fa!(fa_basic_ok, "θάλασσα", true);
    test_fa!(fa_basic_nok1, "η θαλασσόταραχη", false);
    test_fa!(fa_basic_nok2, "η θαλάσσοταραχη", false);
    test_fa!(fa_basic_nok3, "η θάλασσοταραχη", false);
    test_fa!(fa_basic_nok4, "η θαλασσότάραχη", false);

    // Shortest possible words
    test_fa!(fa_shortest1, "όταραχη", false);
    test_fa!(fa_shortest2, "άαα", true);
    test_fa!(fa_shortest3, "άααα", true);
    test_fa!(fa_shortest4, "άαααα", false); // we start at 5
    test_fa!(fa_shortest5, "άααααα", false);

    // These get syllabized as a unit
    test_fa!(fa_nonalpha_strings1, "ανέγερση|ανέγερσης", true);
    test_fa!(fa_nonalpha_strings2, "[[εορτάζοντας]]/[[εορτάζων]]", true);

    // Double accent no pronoun
    test_fa!(fa_double_accent_ok1, "το πρόσωπό μου", true);
    test_fa!(fa_double_accent_ok2, "για την μετακίνησή τους.", true);
    test_fa!(fa_double_accent_ok3, "και τον στηθόδεσμό της.", true);
    test_fa!(fa_double_accent_ok4, "τὸ παρηγόρημά μου.", true);
    test_fa!(fa_double_accent_nok, "το πρόσωσωπό μου", false);

    test_fda_fix!(fda_fix_base_nok, "το ποκάμισό και", "το ποκάμισο και");
    test_fda_fix!(fda_fix_base_ok, "το ποκάμισό μου", "το ποκάμισό μου");

    // Double accent at wrong position
    test_fa!(fa_double_accent_pos_nok1, "στην καμάρά της", false);
    test_fa!(fa_double_accent_pos_nok2, "εμάζωξε τα ρούχά του", false);

    // Should correctly detect <which> next word must be a pronoun
    test_fa!(fa_double_accent_spaces1, "Ανάμεσά τους", true);
    test_fa!(fa_double_accent_spaces2, "Ανάμεσά  τους", true);

    // Some are not really pronouns
    test_fa!(fa_double_accent_pronouns1, "μετὰ τὸ πρόγευμά των", true);
    test_fa!(fa_double_accent_pronouns2, "ἔκαμε κίνησίν τινα", true);
    test_fa!(fa_double_accent_pronouns3, "βραδύτερόν τι", true);
    test_fa!(fa_double_accent_pronouns4, "καπετάνισσά μ᾿", true);

    test_fa!(fa_double_accent_einai_old1, "Κύριός ἐστιν", true);
    test_fa!(fa_double_accent_einai_old2, "Ὄμφακές εἰσι", true);
}
