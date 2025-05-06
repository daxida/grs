// Notes
//
// While rare enough, the current logic contains false positives. Ex:
// * και το κτήριο του, παλαιού πλέον, Μουσείου Ακρόπολης

use crate::diagnostic::{Diagnostic, Fix};
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token};
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

/// Punctuation that prevents a positive diagnostic of an error on the second token.
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
#[inline]
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
    //
    // * GEN_PRON + ACC_PRON
    // Ex.  άφησε μου τον καιρό
    // CEx. Όποιος μου τον φέρει...

    // For an error to exist, the next token must be a pronoun
    // let ntoken = doc.get(token.index + 1)?;
    let ntoken = doc.next_token_not_whitespace(token)?;
    if ntoken.is_punctuation() || !PRONOUNS_LOWERCASE.contains(&ntoken.text()) {
        return None;
    }

    if MULTIPLE_PRONUNCIATION.contains(&token.text())
        // We do not deal with diminutives at the moment.
        || token.text().ends_with("άκια")
        || token.text().ends_with("ούλια")
        // See also `crate::rules::accents::multisyllable_not_accented_opt`
        || token.text().contains(['[', ']'])
    {
        return None;
    }

    let nntoken = doc.next_token_not_whitespace(ntoken)?;
    if nntoken.is_punctuation() {
        let fst_char = nntoken.text().chars().next()?;

        // The token must not start with ellipsis, quotation marks etc.
        // But a period, a comma, a question mark etc. should indicate an error.
        if !STOKEN_AMBIGUOUS_INITIAL_PUNCT
            .iter()
            .any(|punct| nntoken.text().starts_with(punct))
            && !APOSTROPHES.contains(&fst_char)
            // Numbers too should be ignored:
            // Ex. "ανακαλύφθηκε το 1966" is correct.
            && !fst_char.is_numeric()
        {
            return Some(());
        }
    // If it is not punctuation...
    } else if STOKEN_SEPARATOR_WORDS.contains(&nntoken.text())
        // > επιφυλακτικότητα της της στερούσε
        || ntoken.text() == nntoken.text()
        || SE_TO_COMPOUNDS.contains(&nntoken.text())
        || nntoken.is_elliptic_abbreviation()
    {
        return Some(());
    // Case να.
    // Ex. Άφησε τον να βρει μόνος του...
    //
    // The only two pronouns that introduce ambiguity are το & του
    } else if nntoken.text() == "να" && ntoken.text() != "το" && ntoken.text() != "του" {
        return Some(());
    // Case πως (but not πώς!).
    // Ex.  βεβαίωσε τον πως μόνος ο δρόμος...
    // Ex.  εξήγησε του πως είναι ανάγκη να...
    //
    // We exclude το & του to avoid false positive when πώς is mispelled as πως.
    // CEx. ...να δει και στην πραγματικότητα το πως δουλεύει.
    } else if nntoken.text() == "πως" && !["το", "του"].contains(&ntoken.text()) {
        return Some(());
    }

    // Testing
    // > δίνοντας μου μια μπατσιά στη ράχη
    // if ["μου", "σου", "του", "της", "μας", "σας"].contains(&ntoken.text())
    //     && ["μια", "ένα", "έναν", "δυο", "δύο", "τρία", "τρια"].contains(&nntoken.text())
    // {
    //     return Some(());
    // }

    None
}

pub fn missing_double_accents(token: &Token, doc: &Doc, diagnostics: &mut Vec<Diagnostic>) {
    if token.is_greek_word()
        && missing_double_accents_opt(token, doc).is_some()
        && is_proparoxytone_strict(lemmatize(token.text()))
    {
        diagnostics.push(Diagnostic {
            kind: Rule::MissingDoubleAccents,
            range: token.range(),
            fix: Some(Fix {
                replacement: add_acute_at(token.text(), 1),
                range: token.range(),
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rule;

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

    // Before pos
    test_mda!(before_pos1, "βεβαίωσε τον πως μόνος ο δρόμος", false);
    // Exclude του even if there is an error.
    test_mda!(before_pos2, "εξήγησε του πως είναι ανάγκη να", true);

    // Punctuation
    test_mda!(before_quote_marks, "διάρκεια του “πειράματος”", true);
    test_mda!(colon1, "ανακαλύφθηκε το: 'Φέγγαρι'", true);
    test_mda!(colon2, "αναγνωρίζοντας τον:", true); // Could be an error
    test_mda!(inner_punct, "χτυπώ τα [[πόδι]]α μου στο", true);
    test_mda!(punct1, "τα υπόλοιπα ρήματα σε -άω", true);
    // Happens in dictionaries
    test_mda!(punct2, "ένας μόνο ξέφυγε ολότελα την ~ ως", true);

    // Whitespace
    test_mda!(whitespace1, "σύμφωνα με\n", true); // no next token
    test_mda!(whitespace2, "σύμφωνα με\n.", false);
    test_mda!(whitespace3, "σύμφωνα με\n\n.", false);
    test_mda!(whitespace4, "σύμφωνα με ", true);
    test_mda!(whitespace5, "σύμφωνα με .", false);
    test_mda!(whitespace6, "σύμφωνα με  .", false);

    // Regression
    test_mda!(reg1, "Κάθε κίνηση που κάνετε μου κοστίζει ένα...", true);

    // Experimental
    test_mda!(synizesis, "Στάσου, έννοια σου!", true);
}
