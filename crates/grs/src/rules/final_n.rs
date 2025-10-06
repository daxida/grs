use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::is_vowel;

#[rustfmt::skip]
const PLOSIVE_CLUSTERS: [&str; 16] = [
    "κ", "π", "τ", "μπ", "ντ", "γκ", "ξ", "ψ",
    "Κ", "Π", "Τ", "Μπ", "Ντ", "Γκ", "Ξ", "Ψ"
];

// Notes:
// - αυτή αυτήν requires some extra work
// - μη and δε are also probably not safe (to add).
const CANDIDATES_REM: [&str; 4] = ["την", "στην", "Την", "Στην"]; // "μην", "δεν"];
const CANDIDATES_ADD: [&str; 4] = ["τη", "στη", "Τη", "Στη"]; // , "μη", "δε"];

fn remove_last_char(s: &str) -> &str {
    let mut chars = s.chars();
    chars.next_back();
    chars.as_str()
}

fn starts_with_vowel_or_plosive(token: &Token) -> bool {
    token.text().chars().next().is_some_and(|ch| {
        PLOSIVE_CLUSTERS
            .iter()
            .any(|&prefix| token.text().starts_with(prefix))
            || is_vowel(ch)
    })
}

fn remove_final_n_opt(token: &Token, doc: &Doc) -> Option<()> {
    if CANDIDATES_REM.contains(&token.text()) {
        // Treat archaic construction "εις την" as valid
        if token.text() == "την"
            && let Some(ptoken) = doc.prev_token_not_whitespace(token)
                && ptoken.text() == "εις" {
                    return None;
                }

        if let Some(ntoken) = doc.next_token_not_whitespace(token)
            && ntoken.is_greek_word() && !starts_with_vowel_or_plosive(ntoken) {
                return Some(());
            }
    }
    None
}

pub fn remove_final_n(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if remove_final_n_opt(token, doc).is_some() {
        let replacement = remove_last_char(token.text()).to_string();
        diagnostics.push(Diagnostic {
            kind: Rule::RemoveFinalN,
            range: token.range(),
            fix: Some(Fix {
                replacement,
                range: token.range(),
            }),
        });
    }
}

fn add_final_n_opt(token: &Token, doc: &Doc) -> Option<()> {
    if CANDIDATES_ADD.contains(&token.text()) {
        // Treat archaic construction "εν τη" as valid
        if token.text() == "τη"
            && let Some(ptoken) = doc.prev_token_not_whitespace(token)
                && ptoken.text() == "εν" {
                    return None;
                }

        let ntoken = doc.next_token_not_whitespace(token)?;
        if starts_with_vowel_or_plosive(ntoken) {
            // To avoid false positives in case of formal expressions
            // with dative (Ex. επί τη εμφανίσει OR πρώτος τη τάξει),
            // we return None in case ntoken ends with ει.
            // This may cause false negatives, which are preferable anyway.
            return if ntoken.text().ends_with("ει") {
                None
            } else {
                Some(())
            };
        }
    }

    None
}

pub fn add_final_n(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if add_final_n_opt(token, doc).is_some() {
        diagnostics.push(Diagnostic {
            kind: Rule::AddFinalN,
            range: token.range(),
            fix: Some(Fix {
                replacement: format!("{}ν", token.text()),
                range: token.range(),
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rule;

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
