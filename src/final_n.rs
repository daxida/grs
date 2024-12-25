use crate::diagnostic::*;
use crate::registry::Rule;
use crate::tokenizer::*;
use grac::is_vowel_el;
use std::fmt::Write;

fn remove_last_char(s: &str) -> String {
    let sz = s.chars().collect::<Vec<_>>().len();
    s.chars().take(sz - 1).collect()
}

const PLOSIVE_CLUSTERS: [&str; 8] = ["κ", "π", "τ", "μπ", "ντ", "γκ", "ξ", "ψ"];

// For debugging
#[allow(dead_code)]
fn print_context_token(token: &Token, doc: &Doc) {
    let mut out = Vec::new();
    for idx in 1..3 {
        if let Some(ntoken) = doc.get(token.index + idx) {
            out.push(ntoken.clone());
        }
    }
    println!(
        "'{}'",
        out.iter().fold(String::new(), |mut output, token| {
            let _ = write!(output, "{}{}", token.text, token.whitespace);
            output
        })
    );
    println!("> '{:?}'\n> '{:?}'", token, out)
}

// Care uppercase
// Passing the token && doc is for dedbug
#[allow(unused_variables)]
fn starts_with_vowel_or_plosive(s: &str, token: &Token, doc: &Doc) -> bool {
    // Manually remove starting punct because our tokenize logic is faulty
    let ts = s.trim_start_matches(|c: char| !c.is_alphabetic());

    // This only happens if we call this function from a Token that was punct
    // or if the text is empty, which is an acceptable case
    if let Some(ch) = ts.chars().next() {
        PLOSIVE_CLUSTERS
            .iter()
            .any(|&prefix| ts.to_lowercase().starts_with(prefix))
            || is_vowel_el(ch)
    } else {
        // print_context_token(token, doc);
        // panic!("'{}' was reduced to '{}' at starts_with_vowel", s, ts)
        false
    }
}

// αυτή αυτήν requires some extra work
//
// μη and δε are also probably not safe.
const FINAL_N_CANDIDATES_WITH: &[&str] = &["την", "στην", "Την", "Στην"]; // , "μην", "δεν"];
const FINAL_N_CANDIDATES_WITHOUT: &[&str] = &["τη", "στη", "Τη", "Στη"]; // , "μη", "δε"];

fn remove_final_n_opt(token: &Token, doc: &Doc) -> Option<()> {
    if FINAL_N_CANDIDATES_WITH.contains(&token.text) {
        let mut index = 1;
        loop {
            let ntoken = doc.get(token.index + index)?;
            if !ntoken.punct {
                // First non punct token
                if !starts_with_vowel_or_plosive(ntoken.text, ntoken, doc) {
                    return Some(());
                } else {
                    return None;
                }
            }
            index += 1
        }
    }

    None
}

pub fn remove_final_n(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if remove_final_n_opt(token, doc).is_some() {
        let replacement = format!("{}{}", remove_last_char(token.text), token.whitespace);

        diagnostics.push(Diagnostic {
            kind: Rule::RemoveFinalN,
            fix: Some(Fix {
                replacement,
                range: token.range,
            }),
        });
    }
}

fn add_final_n_opt(token: &Token, doc: &Doc) -> Option<()> {
    if FINAL_N_CANDIDATES_WITHOUT.contains(&token.text) {
        let mut index = 1;
        loop {
            let ntoken = doc.get(token.index + index)?;
            if !ntoken.punct {
                // First non punct token
                if starts_with_vowel_or_plosive(ntoken.text, token, doc) {
                    return Some(());
                } else {
                    return None;
                }
            }
            index += 1
        }
    }

    None
}

/// Add or remove final ν
pub fn add_final_n(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if add_final_n_opt(token, doc).is_some() {
        let replacement = format!("{}ν{}", token.text, token.whitespace);
        // let start = token.range.start();
        // let end = start + replacement.len();

        diagnostics.push(Diagnostic {
            kind: Rule::AddFinalN,
            fix: Some(Fix {
                replacement,
                range: token.range,
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_basic() {
        let text = "θα είναι στην διάθεσή σας";
        let tokens = tokenize(text);
        let mut diagnostics = Vec::new();
        remove_final_n(&tokens[2], &tokens, &mut diagnostics);
        assert!(!diagnostics.is_empty());

        let mut diagnostics = Vec::new();
        remove_final_n(&tokens[0], &tokens, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }
}
