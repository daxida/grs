use crate::diagnostic::*;
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::add_acute_at;
use grac::constants::APOSTROPHES;
use grac::diacritic_pos;
use grac::Diacritic;

// TODO: γένεια

/// Does not count double accented proparoxytones
fn is_proparoxytone_strict(word: &str) -> bool {
    diacritic_pos(word, Diacritic::ACUTE) == [3]
}

/// Pronouns
///
/// It is fine to only consider lowercase since they are never
/// expected to be capitalized in our logic.
pub const PRON: &[&str] = &[
    "με", "σε", "τον", "την", "τη", "το", // Accusative Singular
    "μας", "σας", "τους", "τις", "τα", // Accusative Plural
    "μου", "σου", "του", "της", // Genitive Singular
];

/// Punctuation that prevents a positive diagnostic of an error on the
/// second token.
///
/// From \" onward they come from testing against the wikidump,
/// and, even if rare, they make sense to keep.
#[rustfmt::skip]
const STOKEN_AMBIGUOUS_INITIAL_PUNCT: &[&str] = &[
    "...", "…", "«", "\"", "“",
    // Testing
    "[", "{", "*", "<", "#", "}"
];

/// Words that signify some separations that allows us to detect an error.
#[rustfmt::skip]
const STOKEN_SEPARATOR_WORDS: &[&str] = &[
    "και", "κι", "όταν", 
    // Testing
    "του", "με",
];

// https://el.wiktionary.org/wiki/το
const SE_TO_COMPOUNDS: &[&str] = &[
    "στου",
    "στης",
    "στον",
    "στη",
    "στην",
    "στο",
    "στων",
    "στους",
    "στις",
    "στα",
];

/// Return true iif we need to fix the missing double accent
///
/// Uses an option so we can gracefully exit when there is not a next token
///
/// The proparoxytone test is the most expensive part, so we try to compute it last.
fn missing_double_accents_opt(token: &Token, doc: &Doc) -> Option<()> {
    // We do not deal with diminutives at the moment.
    if token.text.ends_with("άκια") || token.text.ends_with("ούλια") {
        return None;
    }

    // For an error to exist, the next token must be a pronoun
    let ntoken = doc.get(token.index + 1)?;
    if ntoken.punct || !PRON.contains(&ntoken.text) {
        return None;
    }

    if !is_proparoxytone_strict(token.text) {
        return None;
    }

    let nntoken = doc.get(token.index + 2)?;
    if nntoken.punct {
        let fst_char = nntoken.text.chars().next()?;

        // The token must not start with ellipsis, quotation marks etc.
        // But a period, a comma, a question mark etc. should indicate an error.
        if !STOKEN_AMBIGUOUS_INITIAL_PUNCT
            .iter()
            .any(|punct| nntoken.text.starts_with(punct))
            && !APOSTROPHES.contains(&fst_char)
            // Numbers too should be ignored:
            // Ex. "ανακαλύφθηκε το 1966" is correct.
            && !fst_char.is_numeric()
        {
            return Some(());
        }
    // If it is not punctuation...
    } else if STOKEN_SEPARATOR_WORDS.contains(&nntoken.text) {
        return Some(());
    } else if ntoken.text == nntoken.text {
        // Testing
        // επιφυλακτικότητα της της στερούσε
        return Some(());
    } else if SE_TO_COMPOUNDS.contains(&nntoken.text) {
        // Testing
        return Some(());
    }

    None
}

pub fn missing_double_accents(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if missing_double_accents_opt(token, doc).is_some() {
        diagnostics.push(Diagnostic {
            kind: Rule::MissingDoubleAccents,
            fix: Some(Fix {
                replacement: format!("{}{}", add_acute_at(token.text, 1), token.whitespace),
                range: token.range,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    #[test]
    fn test_missing_double_accents_no_proparoxytone() {
        let token = Token {
            text: "καλός",
            ..Token::default()
        };
        let doc: Vec<Token> = Vec::new();
        let mut diagnostics = Vec::new();
        missing_double_accents(&token, &doc, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_missing_double_accents_proparoxytone_with_punct() {
        let doc = vec![
            Token {
                text: "άνθρωπος",
                ..Token::default()
            },
            Token {
                text: ".",
                punct: true,
                ..Token::default()
            },
        ];
        let mut diagnostics = Vec::new();
        missing_double_accents(&doc[0], &doc, &mut diagnostics);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_missing_double_accents_proparoxytone() {
        let doc = vec![
            Token {
                text: "άνθρωπος",
                ..Token::default()
            },
            Token {
                text: "του",
                ..Token::default()
            },
            Token {
                text: ".",
                punct: true,
                ..Token::default()
            },
        ];
        let mut diagnostics = Vec::new();
        missing_double_accents(&doc[0], &doc, &mut diagnostics);
        assert!(!diagnostics.is_empty());
    }

    macro_rules! test_no_errors {
        ($name:ident, $text:expr) => {
            #[test]
            fn $name() {
                let text = $text;
                let doc = tokenize(text);
                let mut diagnostics = Vec::new();
                missing_double_accents(&doc[0], &doc, &mut diagnostics);
                assert!(diagnostics.is_empty());
            }
        };
    }

    test_no_errors!(test_numbers, "ανακαλύφθηκε το 1966");
    test_no_errors!(test_newline_asterisk, "διακρίνονται σε\n*");
    test_no_errors!(test_before_quote_marks, "διάρκεια του “πειράματος”");
}
