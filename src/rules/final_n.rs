use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::is_vowel_el;

#[rustfmt::skip]
const PLOSIVE_CLUSTERS: [&str; 16] = [
    "κ", "π", "τ", "μπ", "ντ", "γκ", "ξ", "ψ",
    "Κ", "Π", "Τ", "Μπ", "Ντ", "Γκ", "Ξ", "Ψ"
];

// Notes:
// - αυτή αυτήν requires some extra work
// - μη and δε are also probably not safe.
const CANDIDATES_REM_N: &[&str] = &["την", "στην", "Την", "Στην"]; // , "μην", "δεν"];
const CANDIDATES_ADD_N: &[&str] = &["τη", "στη", "Τη", "Στη"]; // , "μη", "δε"];

fn remove_last_char(s: &str) -> &str {
    let mut chars = s.chars();
    chars.next_back();
    chars.as_str()
}

// Care uppercase
// Passing the token && doc is for dedbug
fn starts_with_vowel_or_plosive(token: &Token) -> bool {
    debug_assert!(!token.punct);
    // This only fails if the text is empty, which is an acceptable case
    if let Some(ch) = token.text.chars().next() {
        PLOSIVE_CLUSTERS
            .iter()
            .any(|&prefix| token.text.starts_with(prefix))
            || is_vowel_el(ch)
    } else {
        false
    }
}

fn get_next_non_punct_token<'a>(token: &'a Token, doc: &'a Doc) -> Option<&'a Token<'a>> {
    let mut index = token.index + 1;
    loop {
        let ntoken = doc.get(index)?;
        if !ntoken.punct {
            return Some(ntoken);
        }
        index += 1;
    }
}

fn remove_final_n_opt(token: &Token, doc: &Doc) -> Option<()> {
    if CANDIDATES_REM_N.contains(&token.text) {
        let ntoken = get_next_non_punct_token(token, doc)?;
        if ntoken.greek && !starts_with_vowel_or_plosive(ntoken) {
            return Some(());
        } else {
            return None;
        }
    }
    None
}

pub fn remove_final_n(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if remove_final_n_opt(token, doc).is_some() {
        diagnostics.push(Diagnostic {
            kind: Rule::RemoveFinalN,
            range: token.range_text(),
            fix: Some(Fix {
                replacement: format!("{}{}", remove_last_char(token.text), token.whitespace),
                range: token.range,
            }),
        });
    }
}

fn add_final_n_opt(token: &Token, doc: &Doc) -> Option<()> {
    if CANDIDATES_ADD_N.contains(&token.text) {
        let ntoken = get_next_non_punct_token(token, doc)?;
        if starts_with_vowel_or_plosive(ntoken) {
            return Some(());
        } else {
            return None;
        }
    }

    None
}

pub fn add_final_n(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if add_final_n_opt(token, doc).is_some() {
        diagnostics.push(Diagnostic {
            kind: Rule::AddFinalN,
            range: token.range_text(),
            fix: Some(Fix {
                replacement: format!("{}ν{}", token.text, token.whitespace),
                range: token.range,
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    macro_rules! test {
        ($name:ident, $fn:expr, $text:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let text = $text;
                let doc = tokenize(text);
                let mut diagnostics = Vec::new();
                $fn(&doc[0], &doc, &mut diagnostics);
                assert_eq!(diagnostics.is_empty(), $expected);
            }
        };
    }

    macro_rules! test_add {
        ($name:ident, $text:expr, $expected:expr) => {
            test!($name, add_final_n, $text, $expected);
        };
    }

    macro_rules! test_remove {
        ($name:ident, $text:expr, $expected:expr) => {
            test!($name, remove_final_n, $text, $expected);
        };
    }

    test_add!(add_base, "στη πόλη σας", false);

    test_remove!(remove_base, "στην διάθεσή σας", false);
    test_remove!(non_punct, "στην, ?διάθεσή σας", false);
    test_remove!(mixed_langs, "την Creative Commons", true);
}
