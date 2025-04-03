use crate::diagnostic::Diagnostic;
use crate::doc::Doc;
use crate::range::TextRange;
use crate::registry::Rule;
use crate::rules::missing_double_accents::PRONOUNS_LOWERCASE;
use crate::tokenizer::Token;

// NOTE: Will not detect duplication if they are of different casing.
// Ex. Πρώτα πρώτα

// Based on common expressions
#[rustfmt::skip]
const DUPLICATED_WORD_EXCEPTIONS: [&str; 41] = [
    "κάτω", "γύρω", "μπροστά", "πλάι", "πέρα", "πάνω", "κάτω",
    "λίγο", "λίγα", "πολύ", "πάρα",
    "καλά",
    "πρώτα", "πρώτη", "πρώτης", "πρώτον", "πρώτοι",
    "ίσως",
    "πότε",
    "κάπου", "όπως",
    "πρωί", "βράδυ", "νωρίς",
    "γρήγορα", "σιγά", "αργά", "χονδρά",
    "ίσα",
    "ένα", "έναν", "ένας", "μια", "δυο", "τρία", "πενήντα",
    "κούτσα",
    "άκρη",
    "λογής",
    "αγάλι",
    "τσίμα",
];

fn duplicated_word_opt<'a>(token: &Token, doc: &'a Doc) -> Option<&'a Token<'a>> {
    debug_assert!(!token.punct && token.greek);

    // Ignore:
    // * empty text
    // * one-letter duplications (cf. s p a c i n g) (2 bytes)
    if token.text.len() < 3
        || DUPLICATED_WORD_EXCEPTIONS.contains(&token.text)
        // Should also add pronouns and not open this can of worms:
        // https://www.babiniotis.gr/lexilogika/leksilogika/leitourgikos-tonismos-sto-monotoniko/
        || PRONOUNS_LOWERCASE.contains(&token.text)
    {
        return None;
    }

    if let Some(ntoken) = doc.get(token.index + 1) {
        if token.text == ntoken.text {
            return Some(ntoken);
        }
    }

    None
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
    if let Some(ntoken) = duplicated_word_opt(token, doc) {
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
    test_dw!(other2, "είναι πάρα πάρα πολλά", true);
    test_dw!(other3, "κατέβαινε το βράδυ βράδυ", true);
    test_dw!(pron1, "Λοιπόν το ένστικτό σου σου φώναξε", true);
    test_dw!(expr1, "ο που αγάλι αγάλι περπατεί", true);
    test_dw!(expr2, "στο κάτω κάτω της γραφής", true);

    // Ignore spacing emphasis, i.e. s p a c i n g
    test_dw!(spacing, "Ω σ α ν ν ά", true);
}
