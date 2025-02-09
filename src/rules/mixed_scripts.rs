use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::is_greek_char;

const LATIN_TO_GREEK: [(char, char); 22] = [
    ('A', 'Α'),
    ('Á', 'Ά'),
    ('B', 'Β'),
    ('E', 'Ε'),
    ('É', 'Έ'),
    ('H', 'Η'),
    ('I', 'Ι'),
    ('Í', 'Ί'),
    ('K', 'Κ'),
    ('M', 'Μ'),
    ('N', 'Ν'),
    ('O', 'Ο'),
    ('Ó', 'Ό'),
    ('P', 'Ρ'),
    ('T', 'Τ'),
    ('X', 'Χ'),
    ('Y', 'Υ'),
    ('o', 'ο'),
    ('ó', 'ό'),
    ('u', 'υ'),
    ('v', 'ν'),
    // Not so ambiguous but can happen
    ('í', 'ί'),
];

fn mixed_scripts_opt(token: &Token) -> Option<()> {
    let mut has_latin = false;
    let mut has_greek = false;
    for ch in token.text.chars() {
        if is_greek_char(ch) {
            has_greek = true;
        } else if LATIN_TO_GREEK.iter().any(|(latin, _)| ch == *latin) {
            has_latin = true;
        } else if !ch.is_alphabetic() {
            // If ch is not alphabetic, avoid diagnosing an error, since
            // it can be anything:
            // - μτφδ|en|el|text=1|radio
            // - Αρχείο:Gravestone
            // - B-λεμφοκύτταρο,
            // - σελ.629@books.google]
            // etc.
            return None;
        }
    }

    if has_latin && has_greek {
        Some(())
    } else {
        None
    }
}

/// Checks if a token, which is expected to be in Greek, contains any Latin characters.
/// Ex. νέo (the o is the latin letter o)
pub fn mixed_scripts(token: &Token, _doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    debug_assert!(!token.greek && !token.punct);

    if mixed_scripts_opt(token).is_some() {
        let mut fixed = String::new();
        for ch in token.text.chars() {
            let fixed_ch = LATIN_TO_GREEK
                .iter()
                .find_map(|(latin, greek)| if ch == *latin { Some(greek) } else { None })
                .unwrap_or(&ch);
            fixed.push(*fixed_ch);
        }

        diagnostics.push(Diagnostic {
            kind: Rule::MixedScripts,
            range: token.range,
            fix: Some(Fix {
                replacement: format!("{}{}", fixed, token.whitespace),
                range: token.range,
            }),
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tokenizer::tokenize;

    macro_rules! test_empty_diagnostics {
        ($name:ident, $text:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let text = $text;
                let doc = tokenize(text);
                let mut diagnostics = Vec::new();
                mixed_scripts(&doc[0], &doc, &mut diagnostics);
                assert_eq!(diagnostics.is_empty(), $expected);
            }
        };
    }

    test_empty_diagnostics!(lowercase_o, "νέo", false);
    test_empty_diagnostics!(uppercase_a, "Áλλα", false);
    test_empty_diagnostics!(lowercase_i, "Χωρíς", false);
}
