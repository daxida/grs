// Notes
//
// While rare enough, the current logic contains false positives. Ex:
// * και το κτήριο του, παλαιού πλέον, Μουσείου Ακρόπολης

use crate::diagnostic::*;
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
use grac::add_acute_at;
use grac::constants::APOSTROPHES;
use grac::diacritic_pos;
use grac::Diacritic;

/// Returns `true` if this `word` has only an accent on the antepenultimate.
fn is_proparoxytone_strict(word: &str) -> bool {
    diacritic_pos(word, Diacritic::ACUTE) == [3]
}

/// Returns `true` if this `token` (or some combination of tokens starting
/// at this token) conforms an abbreviation which fulfills the role of
/// an ellipsis. Ex. κ.τ.λ., κτλ, κτλ.
///
/// Includes common typos like κ.λ.π. instead of κ.λπ.
#[allow(unused_variables)]
fn followed_by_elliptic_abbreviation(token: &Token, doc: &Doc) -> bool {
    // The last dot must be removed because of our tokenizing logic
    if [
        "κ.τ.λ", "κτλ", "κ.λπ", "κ.λ.π", "κ.τ.ό", "κ.τ.ο", "κ.τ.ρ", "κ.τ.τ", "κ.ά", "κ.α",
    ]
    .contains(&token.text)
    {
        return true;
    }
    // Here some more logic could be added to deal with compounds
    // after the current token.
    false
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
    "[", "{", "*", "<", "#", "}", ":",
];

/// Words that signify some separations that allows us to detect an error.
#[rustfmt::skip]
const STOKEN_SEPARATOR_WORDS: &[&str] = &[
    "και", "κι", "όταν",
    // Testing
    "με",
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
    } else if followed_by_elliptic_abbreviation(nntoken, doc) {
        return Some(());
    }

    None
}

pub fn missing_double_accents(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if missing_double_accents_opt(token, doc).is_some() {
        diagnostics.push(Diagnostic {
            kind: Rule::MissingDoubleAccents,
            range: token.range_text(),
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
    use crate::test_rule;
    use crate::tokenizer::tokenize;

    #[test]
    fn test_tokens_without_error() {
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
    fn test_tokens_with_error() {
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

    #[test]
    fn test_range() {
        let text = "άνθρωπος του.";
        let doc = tokenize(text);
        let mut diagnostics = Vec::new();
        missing_double_accents(&doc[0], &doc, &mut diagnostics);
        assert!(!diagnostics.is_empty());

        let diagnostic = &diagnostics[0];
        let range = diagnostic.range;
        assert_eq!(range.start(), 0);
        assert_eq!(range.end(), "άνθρωπος".len());
    }

    macro_rules! test_mda {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule!($name, missing_double_accents, $text, $expected);
        };
    }

    test_mda!(basic_onw, "ανακαλύφθηκε το.", false);
    test_mda!(basic_two, "Όταν ανακαλύφθηκε το.", false);
    test_mda!(stoken_separator_one, "αντίκτυπο του και", false);
    test_mda!(stoken_separator_two, "αντίκτυπο του κ.λ.π.", false);
    test_mda!(stoken_separator_three, "αντίκτυπο του κ.α.", false);

    test_mda!(no_proparoxytone, "καλός.", true);
    test_mda!(numbers, "ανακαλύφθηκε το 1966", true);
    test_mda!(colon, "ανακαλύφθηκε το: 'Φέγγαρι'", true);
    test_mda!(newline_asterisk, "διακρίνονται σε\n*", true);
    test_mda!(before_quote_marks, "διάρκεια του “πειράματος”", true);
    test_mda!(me_tou, "περισσότερο με του αλόγου", true);
}
