use crate::diagnostic::Diagnostic;
use crate::doc::Doc;
use crate::range::TextRange;
use crate::registry::Rule;
use crate::tokenizer::Token;

// Based on common expressions
//
// Should also add pronouns and not open this can of worms:
// https://www.babiniotis.gr/lexilogika/leksilogika/leitourgikos-tonismos-sto-monotoniko/
#[rustfmt::skip]
const DUPLICATED_WORD_EXCEPTIONS: &[&str] = &[
    "κάτω", "γύρω", "μπροστά", "πλάι", "πέρα",
    "λίγο", "λίγα", "πολύ",
    "καλά",
    "πρώτα", "πρώτη", "πρώτον",
    "ίσως",
    "πότε",
    "κάπου", "όπως",
    "πρωί", "νωρίς",
    "γρήγορα", "σιγά", "αργά", "χονδρά",
    "ίσα",
    "ένα", "έναν", "μια", "δυο", "τρία", "πενήντα",
    "κούτσα",
    "άκρη",
    "λογής",
    // Can of worms
    "με", "μας", "μου", "του", "της", "τους",
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
        // Guaranteed to exist at this point.
        let ntoken = doc.get(token.index + 1).unwrap();
        let range = TextRange::new(token.range.start(), ntoken.range_text().end());
        diagnostics.push(Diagnostic {
            kind: Rule::DuplicatedWord,
            range,
            fix: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rule;
    use crate::tokenizer::tokenize;

    macro_rules! test_dw {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, duplicated_word, $text, $expected);
        };
    }

    test_dw!(base, "λάθος λάθος", false);
    test_dw!(numbers1, "δυο δυο", true);
    test_dw!(numbers2, "τρία τρία", true);
    test_dw!(other1, "θα διαφθαρούν όλα πέρα πέρα", true);
}
