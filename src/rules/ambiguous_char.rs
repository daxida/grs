use crate::diagnostic::{Diagnostic, Fix};
use crate::range::TextRange;
use crate::registry::Rule;

// It is easier to work with strings, but these should really be chars
const AMBIGUOUS_PAIRS: &[(&str, &str)] = &[("µ", "μ")];

pub fn ambiguous_char(text: &str, diagnostics: &mut Vec<Diagnostic>) {
    for (target, destination) in AMBIGUOUS_PAIRS.iter() {
        if let Some((start, _)) = text.match_indices(target).next() {
            let range = TextRange::new(start, start + target.len());
            diagnostics.push(Diagnostic {
                kind: Rule::AmbiguousChar,
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
        // The first one is the mathematical one, then the Greek one.
        let text = "µμ";
        let mut chars = text.chars();
        let fst = chars.next();
        let snd = chars.next();
        assert_ne!(fst, snd);

        let mut diagnostics = Vec::new();
        ambiguous_char(text, &mut diagnostics);
        assert!(!diagnostics.is_empty());
    }
}
