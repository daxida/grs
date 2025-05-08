// It is frequent to see (newspapers, etc.) the French rule where the first word of a
// sentence does not take accent if it happened to be on it's first letter (a vowel).
// Ex. Ηταν μόλις 31…

use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::{add_acute_at, has_any_diacritic, is_vowel};

/// The first character is uppercase and the rest are lowercase.
fn is_capitalized(s: &str) -> bool {
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        if !first.is_uppercase() {
            return false;
        }
        return chars.all(char::is_lowercase);
    }
    false
}

fn missing_accent_capital_opt(token: &Token, doc: &Doc) -> Option<()> {
    if is_capitalized(token.text())
        // This is not some "has_acute" method to avoid false positives in polytonic
        && !has_any_diacritic(token.text())
        // We know there is at least one char based on is_capitalized
        && is_vowel(token.text().chars().next().unwrap())
        && !doc.is_abbreviation_or_ends_with_dot(token)
    {
        Some(())
    } else {
        None
    }
}

pub fn missing_accent_capital(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if missing_accent_capital_opt(token, doc).is_some() {
        let n_syllables = token.num_syllables();
        if n_syllables > 1 {
            diagnostics.push(Diagnostic {
                kind: Rule::MissingAccentCapital,
                range: token.range(),
                fix: Some(Fix {
                    replacement: add_acute_at(token.text(), n_syllables),
                    range: token.range(),
                }),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rule;

    macro_rules! test_mac {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, missing_accent_capital, $text, $expected);
        };
    }

    test_mac!(base_correct, "Άλλο", true);
    test_mac!(base_wrong, "Αλλο", false);
    test_mac!(starts_with_consonant, "Χγεννα", true);
    test_mac!(abbreviation, "(Κύρ. Αναβ. Ι 7,3)", true);
}
