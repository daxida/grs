use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::{doc::Doc, tokenizer::Token};

const PUNCT_STARTING_STRINGS: [&str; 5] = ["ν", "μ", "σ", "τ", "ουδ"];

// Check for the following common patterns:
// * μ'αυτό, ν'αγαπάς (with no space after the apostrophe)
// ! It may false positive if the apostrophe is used to omit a vowel
//   inside a word (but this should be relatively rare)
//
// Note: it depends highly on our tokenization logic. Since at the moment,
// we would have only one token, we can't fix it via the whitespace string.
fn punctuation_opt(token: &Token, _doc: &Doc) -> Option<String> {
    // Need an apostrophe somewhere inside the word.
    //
    // Note that this should discard most tokens so it should be fine
    // to call the ~expensive "to_lowercase" later on.
    let apostrophe_idx = token.text.find('\'')?;

    let fst_substr = &token.text[..apostrophe_idx];
    if !PUNCT_STARTING_STRINGS.contains(&fst_substr.to_lowercase().as_str()) {
        return None;
    }

    let rest = &token.text[apostrophe_idx + 1..];
    let replacement = format!("{fst_substr}' {rest}");
    Some(replacement)
}

pub fn punctuation(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(replacement) = punctuation_opt(token, doc) {
        let range = token.range_text();
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
    use crate::linter::fix;
    use crate::test_rule;
    use crate::tokenizer::tokenize;

    // TODO: Export it?
    macro_rules! test_fix {
        ($name:ident, $text:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let text = $text;
                let res = fix(text, &[Rule::Punctuation]);
                let received = res.0;
                assert_eq!(received, $expected, "(text: {text})");
            }
        };
    }

    macro_rules! test_p {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, punctuation, $text, $expected);
        };
    }

    test_fix!(
        punct_basic_fix,
        "αναφέρεται σ'αυτόν ως",
        "αναφέρεται σ' αυτόν ως"
    );

    test_p!(punct_basic_ok, "παρόμοιο μ' αυτό.", true);
    test_p!(punct_basic_nok1, "παρόμοιο μ'αυτό.", false);
    test_p!(punct_basic_nok2, "για ν'αναπτυχθεί", false);
    test_p!(punct_basic_nok3, "έχει λάβει γνώση σ'αυτό.", false);
    test_p!(punct_basic_nok4, "Ουδ'η γης", false);
}
