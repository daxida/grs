use crate::diagnostic::{Diagnostic, Fix};
use crate::range::TextRange;
use crate::registry::Rule;

const OUTDATED_SPELLINGS_MULTIPLE: &[(&str, &str)] = &[
    ("κρεββάτι", "κρεβάτι"),
    ("Κρεββάτι", "Κρεβάτι"),
    ("εξ άλλου", "εξάλλου"),
    ("Εξ άλλου", "Εξάλλου"),
    ("εξ αιτίας", "εξαιτίας"),
    ("Εξ αιτίας", "Εξαιτίας"),
];

/// Outdated spelling of strings.
///
/// Two caveats:
/// - Without regex or some more logic, this is agnostic of word boundaries
///   and could replace chunks inside words. This is fine.
/// - The const table needs manual adding of uppercase variants since the
///   prize of casting .to_lowercase() is too harsh, and I have not figured out
///   how to build a const array with capitalized variants at compile time.
pub fn outdated_spelling(text: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Probably the other order is a better choice
    for (target, destination) in OUTDATED_SPELLINGS_MULTIPLE.iter() {
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
