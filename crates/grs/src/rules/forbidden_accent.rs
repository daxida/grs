use crate::diagnostic::Diagnostic;
use crate::doc::Doc;
use crate::range::TextRange;
use crate::registry::Rule;
use crate::rules::missing_double_accents::PRONOUNS_LOWERCASE;
use crate::tokenizer::Token;
use grac::{has_diacritic, is_greek_char, syllabify_el_mode, Diacritic, Merge};

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
fn diacritic_pos(s: &str, diacritic: char) -> Vec<usize> {
    syllabify_el_mode(s, Merge::Every)
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

fn forbidden_accent_opt(token: &Token, doc: &Doc) -> Option<()> {
    // Fast discard if possible (12 bytes ~ 6 Greek chars)
    if token.text.len() < 12 || !token.text.chars().all(is_greek_char) {
        return None;
    }

    let pos = diacritic_pos(token.text, Diacritic::ACUTE);

    // accent before antepenult
    if pos.last().is_some_and(|pos| *pos > 3) {
        return Some(());
    }

    // double accents with no pronoun
    if pos.len() > 1 {
        // TODO: What about punct...?
        let ntoken = doc.get(token.index + 1)?;
        let res = !PRONOUNS_LOWERCASE.contains(&ntoken.text);
        // if res {
        //     eprintln!(
        //         "{:?} {} || {} || {}",
        //         pos,
        //         token.token_ctx(doc),
        //         token.text,
        //         ntoken.text
        //     );
        // }
        return if res { Some(()) } else { None };
    }

    None
}

pub fn forbidden_accent(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if forbidden_accent_opt(token, doc).is_some() {
        let range = TextRange::new(token.range.start(), token.range_text().end());
        diagnostics.push(Diagnostic {
            kind: Rule::ForbiddenAccent,
            range,
            fix: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rule;
    use crate::tokenizer::tokenize;

    macro_rules! test_fa {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, forbidden_accent, $text, $expected);
        };
    }

    test_fa!(fa_basic_ok, "θάλασσα", true);
    test_fa!(fa_basic_nok1, "η θαλασσόταραχη", false);
    test_fa!(fa_basic_nok2, "η θαλάσσοταραχη", false);
    test_fa!(fa_basic_nok3, "η θάλασσοταραχη", false);
    test_fa!(fa_basic_nok4, "η θαλασσότάραχη", false);

    // Shortest possible words
    test_fa!(fa_shortest1, "όταραχη", false);
    test_fa!(fa_shortest2, "άααα", true);
    test_fa!(fa_shortest3, "άαααα", true);
    test_fa!(fa_shortest4, "άααααα", false); // we start at 6

    // These get syllabized as a unit
    test_fa!(fa_nonalpha_strings1, "ανέγερση|ανέγερσης", true);
    test_fa!(fa_nonalpha_strings2, "[[εορτάζοντας]]/[[εορτάζων]]", true);

    // Double accent no pronoun
    test_fa!(fa_double_accent_ok, "το πρόσωπό μου", true);
    test_fa!(fa_double_accent_nok, "το πρόσωσωπό μου", false);
}
