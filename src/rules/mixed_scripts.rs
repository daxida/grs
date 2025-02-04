use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::is_greek_char;
use crate::tokenizer::{Doc, Token};

const LATIN_TO_GREEK: [(char, char); 21] = [
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
];

/// Checks if a token, which is expected to be in Greek, contains any Latin characters.
/// Ex. νέo (the o is the latin letter o)
pub fn mixed_scripts(token: &Token, _doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if token.greek || token.punct {
        return;
    }

    let mut has_latin = false;
    let mut has_greek = false;
    for ch in token.text.chars() {
        if is_greek_char(ch) {
            has_greek = true;
        } else if LATIN_TO_GREEK.iter().any(|(latin, _)| ch == *latin) {
            has_latin = true;
        }
    }

    if has_latin && has_greek {
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

    #[test]
    fn test_mixed_scripts() {
        let text = "νέo";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        mixed_scripts(&doc[0], &doc, &mut diagnostics);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_mixed_scripts_two() {
        let text = "Áλλα";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        mixed_scripts(&doc[0], &doc, &mut diagnostics);
        assert!(!diagnostics.is_empty());
    }
}
