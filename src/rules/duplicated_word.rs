use crate::diagnostic::Diagnostic;
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};

// Based on common expressions
//
// Should also add pronouns and not open this can of worms:
// https://www.babiniotis.gr/lexilogika/leksilogika/leitourgikos-tonismos-sto-monotoniko/
#[rustfmt::skip]
const DUPLICATED_WORD_EXCEPTIONS: &[&str] = &[
    "κάτω", "γύρω", "μπροστά", "πλάι",
    "λίγα", "πολύ",
    "καλά",
    "πρώτα", "πρώτη", "πρώτον",
    "ίσως",
    "πότε",
    "κάπου", "όπως",
    "πρωί",
    "γρήγορα", "σιγά", "αργά",
    "ίσα",
    "δυο", "τρία", "πενήντα",
    "κούτσα",
    "άκρη",
    // Can of worms
    "με", "μου", "του", "της",
];

fn duplicated_word_opt(token: &Token, doc: &Doc) -> Option<()> {
    // Ignore punct
    if token.punct || token.text.is_empty() {
        return None;
    }

    if DUPLICATED_WORD_EXCEPTIONS.contains(&token.text) || token.punct {
        return None;
    }

    let ntoken = doc.get(token.index + 1)?;
    if token.text == ntoken.text {
        Some(())
    } else {
        None
    }
}

/// Detect duplicated word
///
/// No fixes until I decide what to do with consecutive duplications:
/// - το το το
///
/// Don't exclude pronouns since it is recommended to accent one of them.
/// Ex.  η μητέρα του του 'μείνε
///
/// Unfixable: removing the duplicated word may not be the intended
/// approach, sometimes what is needed is extra punctuation:
/// '— Τζωρτζ Τζωρτζ.' > '— Τζωρτζ, Τζωρτζ!'
pub fn duplicated_word(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if duplicated_word_opt(token, doc).is_some() {
        // let fix = Fix {
        //     replacement: String::new(), // also remove the whitespace
        //     range: token.range,
        // };
        diagnostics.push(Diagnostic {
            kind: Rule::DuplicatedWord,
            range: token.range,
            fix: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    macro_rules! test {
        ($name:ident, $text:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let text = $text;
                let doc = tokenize(text);
                let mut diagnostics = Vec::new();
                duplicated_word(&doc[0], &doc, &mut diagnostics);
                assert_eq!(diagnostics.is_empty(), $expected);
            }
        };
    }

    test!(base, "λάθος λάθος", false);
    test!(numbers_one, "δυο δυο", true);
    test!(numbers_two, "τρία τρία", true);
}
