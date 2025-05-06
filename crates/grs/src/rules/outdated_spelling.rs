use crate::diagnostic::{Diagnostic, Fix};
use crate::range::TextRange;
use crate::registry::Rule;
use aho_corasick::AhoCorasick;
use std::sync::OnceLock;

static AC: OnceLock<AhoCorasick> = OnceLock::new();

const OUTDATED_SPELLINGS_MULTIPLE: [(&str, &str); 20] = [
    // Superfluous diaereses
    ("άϊ", "άι"),
    ("άϋ", "άυ"),
    ("έϊ", "έι"),
    ("έϋ", "έυ"),
    ("όϊ", "όι"),
    ("όϋ", "όυ"),
    ("ούϊ", "ούι"),
    // Capitalized
    ("Άϊ", "Άι"),
    ("Άϋ", "Άυ"),
    ("Έϊ", "Έι"),
    ("Έϋ", "Έυ"),
    ("Όϊ", "Όι"),
    ("Όϋ", "Όυ"),
    ("Ούϊ", "Ούι"),
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
/// # Panics
///
/// Panics on the first call if any pattern in `OUTDATED_SPELLINGS_MULTIPLE`
/// is invalid and cannot be compiled into an Aho-Corasick automaton.
//
// Some caveats:
// - Without regex or some more logic, this is agnostic of word boundaries
//   and could replace chunks inside words. This is fine (for now).
// - The const table needs manual adding of uppercase variants since the
//   prize of casting .to_lowercase() is too harsh, and I have not figured out
//   how to build a const array with capitalized variants at compile time.
pub fn outdated_spelling(text: &str, diagnostics: &mut Vec<Diagnostic>) {
    let ac = AC.get_or_init(|| {
        AhoCorasick::new(OUTDATED_SPELLINGS_MULTIPLE.iter().map(|(s, _)| *s)).unwrap()
    });

    for mat in ac.find_iter(text) {
        let target = &text[mat.start()..mat.end()];
        if let Some(&(_, destination)) = OUTDATED_SPELLINGS_MULTIPLE
            .iter()
            .find(|&&(k, _)| k == target)
        {
            let range = TextRange::new(mat.start(), mat.end());
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
    use crate::test_rule_no_token;

    macro_rules! test_os {
        ($name:ident, $text:expr, $expected:expr) => {
            test_rule_no_token!($name, outdated_spelling, $text, $expected);
        };
    }

    test_os!(os_ok, "(θηλυκό γαϊδάρα ή γαϊδούρα)", true);
    test_os!(os_nok1, "κακόϋπνος", false);
    test_os!(os_nok2, "Έϊμι Γουάντζ", false);
    test_os!(os_nok3, "Πάσχα Ρωμέϊκο", false);
}
