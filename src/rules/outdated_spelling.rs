use crate::diagnostic::{Diagnostic, Fix};
use crate::range::TextRange;
use crate::registry::Rule;

const OUTDATED_SPELLINGS_MULTIPLE: &[(&str, &str)] = &[
    // Superfluous diaereses
    ("άϊ", "άι"),
    ("άϋ", "άυ"),
    ("έϊ", "έι"),
    ("έϋ", "έυ"),
    ("όϊ", "όι"),
    ("όϋ", "όυ"),
    ("ούϊ", "ούι"),
    // Capitalized
    ("Άι", "Άι"),
    ("Άυ", "Άυ"),
    ("Έι", "Έι"),
    ("Έυ", "Έυ"),
    ("Όι", "Όι"),
    ("Όυ", "Όυ"),
    ("Ούι", "Ούι"),
    // Others
    ("κρεββάτι", "κρεβάτι"),
    ("Κρεββάτι", "Κρεβάτι"),
    ("εξ άλλου", "εξάλλου"),
    ("Εξ άλλου", "Εξάλλου"),
    ("εξ αιτίας", "εξαιτίας"),
    ("Εξ αιτίας", "Εξαιτίας"),
];

/// Outdated spelling of strings.
///
/// Some caveats:
/// - If this becomes too slow, consider using aho-corasick
/// - Without regex or some more logic, this is agnostic of word boundaries
///   and could replace chunks inside words. This is fine.
/// - The const table needs manual adding of uppercase variants since the
///   prize of casting .to_lowercase() is too harsh, and I have not figured out
///   how to build a const array with capitalized variants at compile time.
pub fn outdated_spelling(text: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Probably the other order is a better choice
    for (target, destination) in OUTDATED_SPELLINGS_MULTIPLE {
        // There must be sth better without break
        if let Some((start, _)) = text.match_indices(target).next() {
            let range = TextRange::new(start, start + target.len());
            diagnostics.push(Diagnostic {
                kind: Rule::OutdatedSpelling,
                range,
                fix: Some(Fix {
                    replacement: destination.to_string(),
                    range,
                }),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let text = "γάιδαρος αρσενικό (θηλυκό γαϊδάρα ή γαϊδούρα)";
        let mut diagnostics = Vec::new();
        outdated_spelling(text, &mut diagnostics);
        assert!(diagnostics.is_empty());

        outdated_spelling("κακόϋπνος", &mut diagnostics);
        assert!(!diagnostics.is_empty());
    }
}
