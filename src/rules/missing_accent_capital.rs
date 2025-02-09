use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::add_acute_at;
use grac::has_any_diacritic;
use grac::syllabify_el;

/// The first character is uppercase and the rest are lowercase.
fn is_capitalized(s: &str) -> bool {
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        if !first.is_uppercase() {
            return false;
        }
        return chars.all(|c| c.is_lowercase());
    }
    false
}

// It is frequent to see in newspaper the french rule where
// the first word of a sentence does not take accent if it should
// have gone to its first letter.
// Ex. Ηταν μόλις 31…
fn missing_accent_capital_opt(token: &Token) -> Option<()> {
    if is_capitalized(token.text) && !has_any_diacritic(token.text) {
        Some(())
    } else {
        None
    }
}

pub fn missing_accent_capital(token: &Token, _doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    let n_syllables = syllabify_el(token.text).len();
    if n_syllables > 1 && missing_accent_capital_opt(token).is_some() {
        diagnostics.push(Diagnostic {
            kind: Rule::MissingAccentCapital,
            range: token.range,
            fix: Some(Fix {
                replacement: format!(
                    "{}{}",
                    // The accent should go to the first syllable
                    add_acute_at(token.text, n_syllables),
                    token.whitespace
                ),
                range: token.range,
            }),
        })
    }
}
