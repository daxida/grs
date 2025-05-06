// Check for the following common error patterns:
// * μ'αυτό, ν'αγαπάς (with no space after the apostrophe)
//
// ! It may false positive if the apostrophe is used to omit a vowel
//   inside a word (but this should be relatively rare)
//
// ! It depends highly on our tokenization logic.

use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::constants::APOSTROPHES;

const PUNCT_STARTING_STRINGS: [&str; 6] = ["ν", "μ", "σ", "τ", "ουδ", "κ"];

fn punctuation_opt(token: &Token, _doc: &Doc) -> Option<String> {
    for apostrophe in APOSTROPHES {
        // This should discard most tokens so it should be fine to call the
        // ~expensive "to_lowercase" just right after.
        if let Some((fst, snd)) = token.text().split_once(apostrophe) {
            if !PUNCT_STARTING_STRINGS.contains(&fst.to_lowercase().as_str()) {
                return None;
            }
            let replacement = format!("{fst}{apostrophe} {snd}");
            return Some(replacement);
        }
    }

    None
}

pub fn punctuation(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(replacement) = punctuation_opt(token, doc) {
        let range = token.range();
        diagnostics.push(Diagnostic {
            kind: Rule::Punctuation,
            range,
            fix: Some(Fix { replacement, range }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_fix, test_rule};

    macro_rules! test_fix_p {
        ($name:ident, $text:expr, $expected:expr) => {
            test_fix!($name, &[Rule::Punctuation], $text, $expected);
        };
    }

    macro_rules! test_p {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, punctuation, $text, $expected);
        };
    }

    test_fix_p!(
        punct_basic_fix,
        "αναφέρεται σ'αυτόν ως",
        "αναφέρεται σ' αυτόν ως"
    );

    test_p!(punct_basic_ok, "παρόμοιο μ' αυτό.", true);
    test_p!(punct_basic_nok1, "παρόμοιο μ'αυτό.", false);
    test_p!(punct_basic_nok2, "για ν'αναπτυχθεί", false);
    test_p!(punct_basic_nok3, "έχει λάβει γνώση σ'αυτό.", false);
    test_p!(punct_basic_nok4, "Ουδ'η γης", false);

    test_p!(punct_alt_apostrophe, "κ᾿ἐκρύπτετο", false);
}
