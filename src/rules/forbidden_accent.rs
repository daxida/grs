use crate::diagnostic::Diagnostic;
use crate::doc::Doc;
use crate::range::TextRange;
use crate::registry::Rule;
use crate::tokenizer::Token;
use grac::{has_diacritic, is_greek_char, syllabify_el_mode, Diacritic, Merge};

// Try to identify words with accents before the antepenult.
//
// Caveats:
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

fn is_forbidden_accent(token: &Token) -> bool {
    // Fast discard if possible
    token.text.len() > 12
        && diacritic_pos(token.text, Diacritic::ACUTE)
            .last()
            .map_or(false, |pos| *pos > 3) // 3 is tricky
}

fn forbidden_accent_opt(token: &Token, _doc: &Doc) -> Option<()> {
    if is_forbidden_accent(token) && token.text.chars().all(|c| is_greek_char(c)) {
        Some(())
    } else {
        None
    }
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
}
