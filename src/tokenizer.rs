use crate::doc::Doc;
use crate::range::TextRange;
use colored::Colorize;
use grac::is_greek_word;
use grac::split_punctuation;
use grac::syllabify_el;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Token type.
///
/// Also stores if the token is punctuation or a Greek word since these are
/// widely used through most rules. In case of huge corpora analysis this
/// may be an issue.
///
/// Following spaCy, whitespace is attached to the previous token.
//
// Very simplified version of:
// https://github.com/explosion/spaCy/blob/311f7cc9fbd44e3de14fa673fa9c5146ea223624/spacy/tokenizer.pyx#L25
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Token<'a> {
    pub text: &'a str,
    /// Trailing whistespace
    pub whitespace: &'a str,
    /// Index in the Doc
    pub index: usize,
    /// Start and end byte of the token
    pub range: TextRange,
    /// Is punctuation?
    pub punct: bool,
    /// Is greek word?
    pub greek: bool,
}

impl<'a> Token<'a> {
    pub const fn new(
        text: &'a str,
        whitespace: &'a str,
        index: usize,
        range: TextRange,
        punct: bool,
        greek: bool,
    ) -> Self {
        Self {
            text,
            whitespace,
            index,
            range,
            punct,
            greek,
        }
    }

    // Note that this function is very expensive.
    pub fn syllables(&self) -> Vec<&str> {
        syllabify_el(self.text)
    }

    /// Start and end byte of the text part of the token.
    ///
    /// Compare it with [`Token::range`], which includes whitespace.
    pub const fn range_text(&self) -> TextRange {
        if self.whitespace.is_empty() {
            self.range
        } else {
            let text_end = self.range.end().saturating_sub(self.whitespace.len());
            TextRange::new(self.range.start(), text_end)
        }
    }

    /// Debug function. Stringify the context of the token.
    pub fn token_ctx(&self, doc: &Doc) -> String {
        let start_from = 5;
        let start = self.index.saturating_sub(start_from);
        let end = self.index + 5;
        let ctx = (start..=end)
            .filter_map(|idx| doc.get(idx))
            .enumerate()
            .map(|(idx, t)| {
                if idx == start_from {
                    let chunk = format!("{}{}", t.text, t.whitespace);
                    chunk.bold().to_string()
                } else {
                    format!("{}{}", t.text, t.whitespace)
                }
            })
            .collect::<String>();

        ctx.replace('\n', "⏎")
    }
}

// Note: numbers are treated as PUNCT (not ideal)
pub fn tokenize(text: &str) -> Doc {
    let mut end = 0;
    let mut index = 0;
    let mut tokens = Vec::new();

    for w in text.split_inclusive(|c: char| c.is_whitespace()) {
        let non_whitespace = w.trim_end_matches(|c: char| c.is_whitespace());
        let (lpunct, word, rpunct) = split_punctuation(non_whitespace);

        let start = end;
        end = start + w.len();

        // Empty non_whitespace quick exit case.
        // Treat it as NOT punct since it is only whitespace.
        if non_whitespace.is_empty() {
            let range = TextRange::new(start, end);
            let token = Token::new("", w, index, range, false, false);
            tokens.push(token);
            index += 1;
            continue;
        }

        if !lpunct.is_empty() {
            let range = TextRange::new(start, start + lpunct.len());
            let token = Token::new(lpunct, "", index, range, true, false);
            tokens.push(token);
            index += 1;
        }

        if !word.is_empty() {
            // May be empty
            let whitespace = if rpunct.is_empty() {
                &w[lpunct.len() + word.len() + rpunct.len()..]
            } else {
                ""
            };

            let start_at = start + lpunct.len();
            let greek = is_greek_word(word);
            let range = TextRange::new(start_at, start_at + word.len() + whitespace.len());
            let token = Token::new(word, whitespace, index, range, false, greek);
            tokens.push(token);
            index += 1;
        }

        if !rpunct.is_empty() {
            // May be empty
            let whitespace = &w[lpunct.len() + word.len() + rpunct.len()..];

            let start_at = start + lpunct.len() + word.len();
            let range = TextRange::new(start_at, start_at + whitespace.len() + rpunct.len());
            let token = Token::new(rpunct, whitespace, index, range, true, false);
            tokens.push(token);
            index += 1;
        }
    }

    tokens
}

/// A simple macro for testing a rule.
#[macro_export]
macro_rules! test_rule {
    ($name:ident, $rule_fn:expr, $text:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let text = $text;
            let doc = tokenize(text);
            let mut diagnostics = Vec::new();
            for token in &doc {
                $rule_fn(&token, &doc, &mut diagnostics);
            }
            assert_eq!(diagnostics.is_empty(), $expected, "(text: {text})");
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn splitting(text: &str, expected: &[&str]) {
        let received: Vec<_> = tokenize(text).iter().map(|token| token.text).collect();
        assert_eq!(received, expected);
    }

    #[test]
    fn test_splitting() {
        splitting("Καλημέρα, κόσμε", &["Καλημέρα", ",", "κόσμε"]);
        splitting("την «ξεκρέμασε", &["την", "«", "ξεκρέμασε"]);
        splitting(
            " την  «   ξεκρέμασε ",
            &["", "την", "", "«", "", "", "ξεκρέμασε"],
        );
        splitting("το: Φέγγαρι", &["το", ":", "Φέγγαρι"]);
    }

    #[test]
    fn test_splitting_apostrophe() {
        splitting("όλ' αυτά", &["όλ", "'", "αυτά"]);
        splitting("ἄρ᾽ Ἀθήνας", &["ἄρ", "᾽", "Ἀθήνας"]);
    }

    #[test]
    fn test_tokenization_ascii() {
        let text = "Hello world!  ";
        //          01234567890123
        let doc = tokenize(text);

        let expected = vec![
            Token {
                text: "Hello",
                whitespace: " ",
                index: 0,
                range: TextRange::new(0, 6),
                punct: false,
                greek: false,
            },
            Token {
                text: "world",
                whitespace: "",
                index: 1,
                range: TextRange::new(6, 11),
                punct: false,
                greek: false,
            },
            Token {
                text: "!",
                whitespace: " ",
                index: 2,
                range: TextRange::new(11, 13),
                punct: true,
                greek: false,
            },
            Token {
                text: "",
                whitespace: " ",
                index: 3,
                range: TextRange::new(13, 14),
                punct: false,
                greek: false,
            },
        ];

        assert_eq!(doc, expected);
    }

    #[test]
    fn test_tokenization_non_ascii() {
        let text = "Καλημέρα, κόσμε";
        //          0123456789012345
        //          024681356792468
        let doc = tokenize(text);

        let expected = vec![
            Token {
                text: "Καλημέρα",
                whitespace: "",
                index: 0,
                range: TextRange::new(0, 16),
                punct: false,
                greek: true,
            },
            Token {
                text: ",",
                whitespace: " ",
                index: 1,
                range: TextRange::new(16, 18),
                punct: true,
                greek: false,
            },
            Token {
                text: "κόσμε",
                whitespace: "",
                index: 2,
                range: TextRange::new(18, 28),
                punct: false,
                greek: true,
            },
        ];

        assert_eq!(doc, expected);
    }
}
