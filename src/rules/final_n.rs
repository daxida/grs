use crate::diagnostic::{Diagnostic, Fix};
use crate::doc::Doc;
use crate::registry::Rule;
use crate::tokenizer::Token;
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

fn starts_with_vowel_or_plosive(token: &Token) -> bool {
    if let Some(ch) = token.text.chars().next() {
        PLOSIVE_CLUSTERS
            .iter()
            .any(|&prefix| token.text.starts_with(prefix))
            || is_vowel_el(ch)
    } else {
        false
    }
}

// Used at some point to get the next token in both rules in this module.
// Introduces quite some false positives in exchange for more coverage if
// the text has html formatting (i.e. wikipedia)
#[allow(unused)]
fn get_next_non_punct_token<'a>(token: &'a Token, doc: &'a Doc) -> Option<&'a Token<'a>> {
    let mut index = token.index + 1;
    loop {
        let ntoken = doc.get(index)?;
        if !ntoken.punct || ntoken.text.chars().all(|c| c.is_ascii_digit()) {
            return Some(ntoken);
        }
        index += 1;
    }
}

fn remove_final_n_opt(token: &Token, doc: &Doc) -> Option<()> {
    if CANDIDATES_REM_N.contains(&token.text) {
        // Treat archaic construction "εις την" as valid
        if token.text == "την" {
            if let Some(ptoken) = doc.get(token.index.saturating_sub(1)) {
                if ptoken.text == "εις" {
                    return None;
                }
            }
        }

        let ntoken = doc.get(token.index + 1)?;
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
        // Treat archaic construction "εν τη" as valid
        if token.text == "τη" {
            if let Some(ptoken) = doc.get(token.index.saturating_sub(1)) {
                if ptoken.text == "εν" {
                    return None;
                }
            }
        }

        let ntoken = doc.get(token.index + 1)?;
        if starts_with_vowel_or_plosive(ntoken) {
            // To avoid false positives in case of formal expressions
            // with dative (Ex. επί τη εμφανίσει OR πρώτος τη τάξει),
            // we return None in case ntoken ends with ει.
            // This may cause false negatives, which are preferable anyway.
            return if ntoken.text.ends_with("ει") {
                None
            } else {
                Some(())
            };
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
    use crate::test_rule;
    use crate::tokenizer::tokenize;

    macro_rules! test_add {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, add_final_n, $text, $expected);
        };
    }

    macro_rules! test_remove {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, remove_final_n, $text, $expected);
        };
    }

    test_add!(add_base, "στη πόλη σας", false);
    test_add!(add_ignore_nums, "τη 2η θέση", true);
    test_add!(add_dative1, "τη τάξει", true);
    test_add!(add_dative2, "φωνή βοώντος εν τη ερήμω", true);

    test_remove!(remove_base1, "στην διάθεσή σας", false);
    test_remove!(remove_base2, "Είμαι στην διάθεσή σας", false);
    test_remove!(remove_ignore_nums, "την 5η θέση", true);
    test_remove!(remove_mixed_langs, "την Creative Commons", true);
    test_remove!(remove_punct1, "Πιάστε την! Για τον θεό", true);
    test_remove!(remove_punct2, "Πιάστε την, για τον θεό", true);
    test_remove!(remove_ignore_eis, "εις την θάλασσαν", true);
}
