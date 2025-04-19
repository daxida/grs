// Notes
//
// While rare enough, the current logic contains false positives. Ex:
// * και το κτήριο του, παλαιού πλέον, Μουσείου Ακρόπολης

use crate::diagnostic::{Diagnostic, Fix};
use crate::doc::Doc;
use crate::doc::followed_by_elliptic_abbreviation;
use crate::registry::Rule;
use crate::tokenizer::Token;
use grac::Diacritic;
use grac::add_acute_at;
use grac::constants::{APOSTROPHES, MULTIPLE_PRONUNCIATION};
use grac::diacritic_pos;

/// Returns `true` if this `word` has only an accent on the antepenultimate.
fn is_proparoxytone_strict(word: &str) -> bool {
    diacritic_pos(word, Diacritic::ACUTE) == [3]
}

/// Does not include των or archaic versions like τας.
pub const PRONOUNS_LOWERCASE: [&str; 15] = [
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
const STOKEN_AMBIGUOUS_INITIAL_PUNCT: [&str; 17] = [
    "...", "…", "«", "\"", "“",
    "[", "]", "{", "}", "(", ")", "*", "<", "#", ":",
    "-", "~",
];

/// Words that signify some separations that allows us to detect an error.
#[rustfmt::skip]
const STOKEN_SEPARATOR_WORDS: [&str; 15] = [
    // Conjunctions (groups SCONJ and CCONJ from similar spacy concepts.)
    "και", "κι", "ή", "αλλά", "είτε", "ενώ", "όμως", "ωστόσο", "αφού",
    // Others
    "με", "όταν", "θα", "μήπως", "λοιπόν", "για",
];

// https://el.wiktionary.org/wiki/το
const SE_TO_COMPOUNDS: [&str; 10] = [
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

/// Extract the lemma of a word.
///
/// This allows us to fully use the syllabification logic of grac by
/// being able to use the synizesis table against the lemma instead of
/// the word (since the word itself may very well not be in the table).
///
/// Example: lemmatize("παλιοκατσάριαν") == "κατσάρια"
#[inline(always)]
fn lemmatize(s: &str) -> &str {
    s.trim_end_matches('ν').trim_start_matches("παλιο")
}

/// Return true iif we need to fix the missing double accent
///
/// Uses an option so we can gracefully exit when there is not a next token
///
/// The proparoxytone test is the most expensive part, so we compute it last,
/// outside of this function.
#[allow(clippy::similar_names)]
fn missing_double_accents_opt(token: &Token, doc: &Doc) -> Option<()> {
    // Discarded ideas:
    //
    // * σε + τον (or other acc. pronouns)
    // Ex.  σπρώχνοντας τον σε μια καρέγλα κοντά του.
    // CEx. χτύπησε τον σε σύγχυση εχθρό...

    // For an error to exist, the next token must be a pronoun
    let ntoken = doc.get(token.index + 1)?;
    if ntoken.punct || !PRONOUNS_LOWERCASE.contains(&ntoken.text) {
        return None;
    }

    if MULTIPLE_PRONUNCIATION.contains(&token.text)
        // We do not deal with diminutives at the moment.
        || token.text.ends_with("άκια")
        || token.text.ends_with("ούλια")
        // See also `crate::rules::accents::multisyllable_not_accented_opt`
        || token.text.contains(['[', ']'])
    {
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
    } else if STOKEN_SEPARATOR_WORDS.contains(&nntoken.text)
        // > επιφυλακτικότητα της της στερούσε
        || ntoken.text == nntoken.text
        || SE_TO_COMPOUNDS.contains(&nntoken.text)
        || followed_by_elliptic_abbreviation(nntoken, doc)
    {
        return Some(());
    // Case να. Ex. Άφησε τον να βρει μόνος του...
    // The only two pronouns that introduce ambiguity are το & του
    } else if nntoken.text == "να" && ntoken.text != "το" && ntoken.text != "του" {
        return Some(());
    }

    // Testing
    // > δίνοντας μου μια μπατσιά στη ράχη
    // if ["μου", "σου", "του", "της", "μας", "σας"].contains(&ntoken.text)
    //     && ["μια", "ένα", "έναν", "δυο", "δύο", "τρία", "τρια"].contains(&nntoken.text)
    // {
    //     // eprintln!("* '{}'", token.token_ctx(doc));
    //     return Some(());
    // }

    None
}

pub fn missing_double_accents(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if missing_double_accents_opt(token, doc).is_some()
        && is_proparoxytone_strict(lemmatize(token.text))
    {
        diagnostics.push(Diagnostic {
            kind: Rule::MissingDoubleAccents,
            range: token.range_text(),
            fix: Some(Fix {
                replacement: format!("{}{}", add_acute_at(token.text, 1), token.whitespace),
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

    test_mda!(basic1, "ανακαλύφθηκε το.", false);
    test_mda!(basic2, "Όταν ανακαλύφθηκε το.", false);
    test_mda!(stoken1, "αντίκτυπο του και", false);
    test_mda!(stoken2, "αντίκτυπο του κ.λ.π.", false);
    test_mda!(stoken3, "αντίκτυπο του κ.α.", false);
    test_mda!(stoken4, "Η ύπαρξη μου μήπως;", false);
    test_mda!(tha1, "Το κιτρινιάρικο μούτσουνο σου θα", false);
    test_mda!(tha2, "Και τ' όνομα του θα το μετάλεγαν οι άνθρωποι", false);

    // STOKEN_SEPARATOR
    // * Conjunctions
    test_mda!(conj1, "την πρόσβαση σας ή την", false);
    test_mda!(conj2, "το τηλέφωνο σας ενώ οδηγείτε,", false);
    test_mda!(conj3, "χτυπά τα θύματα της είτε αργά και", false);
    test_mda!(conj4, "Μετά την ανάσταση μου όμως θα σας", false);
    test_mda!(conj5, "θέση στο πολίτευμα μας αφού είναι το", false);
    test_mda!(conj6, "Στα ποιήματα του ωστόσο διαβάζουμε ότι", false);
    // * Others
    test_mda!(stok1, "αποβίβασε το στράτευμα του για να βοηθήσει", false);
    test_mda!(stok2, "Το ένστικτο του λοιπόν του λέγει να σφάζει", false);

    test_mda!(already_correct, "ανακαλύφθηκέ το.", true);
    test_mda!(no_proparoxytone, "καλός.", true);
    test_mda!(numbers, "ανακαλύφθηκε το 1966", true);
    test_mda!(newline_asterisk, "διακρίνονται σε\n*", true);
    test_mda!(me_tou, "περισσότερο με του αλόγου", true);

    // Before na
    test_mda!(before_na1, "Άφησε τον να βρει μόνος του", false);
    test_mda!(before_na2, "τάζοντας της να τη στεφανωθή,", false);
    test_mda!(before_na3, "τερπνήν ενασχόλησιν το να ρίπτωσι λίθους", true);
    test_mda!(before_na4, "πρόθεση του να παραιτηθεί", true);

    // Punctuation
    test_mda!(before_quote_marks, "διάρκεια του “πειράματος”", true);
    test_mda!(colon, "ανακαλύφθηκε το: 'Φέγγαρι'", true);
    test_mda!(inner_punct, "χτυπώ τα [[πόδι]]α μου στο", true);
    test_mda!(punct1, "τα υπόλοιπα ρήματα σε -άω", true);
    // Happens in dictionaries
    test_mda!(punct2, "ένας μόνο ξέφυγε ολότελα την ~ ως", true);

    // Regression
    test_mda!(reg1, "Κάθε κίνηση που κάνετε μου κοστίζει ένα...", true);

    // Experimental
    test_mda!(synizesis, "Στάσου, έννοια σου!", true);
}
