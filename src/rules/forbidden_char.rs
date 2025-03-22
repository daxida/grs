use crate::diagnostic::Diagnostic;
use crate::range::TextRange;
use crate::registry::Rule;
use grac::is_greek_letter;

// Check if the char after ς is allowed.
fn wrong_after_sigma(c: char) -> bool {
    // For testing against the wiki dumps, uncomment this line.
    // c != 'Ο' && c != 'Γ' && c != 'ς' &&
    is_greek_letter(c)
}

/// Forbidden chars
///
/// Identify:
/// * "ς" not in final position.
/// > There is no fix since it could be caused by either:
///   * A simple confusion of ς and σ: πιςτεύοντας
///   * A missing space: πιστεύονταςτην
/// * Accents on non vowels.
/// > TODO:
pub fn forbidden_char(text: &str, diagnostics: &mut Vec<Diagnostic>) {
    let mut chars = text.char_indices();
    if let Some((mut idx1, mut c1)) = chars.next() {
        for (idx2, c2) in chars {
            if c1 == 'ς' && wrong_after_sigma(c2) {
                let range = TextRange::new(idx1, idx2);
                diagnostics.push(Diagnostic {
                    kind: Rule::ForbiddenChar,
                    range,
                    fix: None,
                });
            }
            idx1 = idx2;
            c1 = c2;
        }
    }
}
