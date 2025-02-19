use crate::range::TextRange;
use grac::is_greek_word;
use grac::split_word_punctuation;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Very simplified version of:
/// https://github.com/explosion/spaCy/blob/311f7cc9fbd44e3de14fa673fa9c5146ea223624/spacy/tokenizer.pyx#L25
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

impl Token<'_> {
    /// Start and end byte of the text part of the token.
    ///
    /// Compare it with Token::range, which includes whitespace.
    pub const fn range_text(&self) -> TextRange {
        if self.whitespace.is_empty() {
            self.range
        } else {
            let text_end = self.range.end().saturating_sub(self.whitespace.len());
            TextRange::new(self.range.start(), text_end)
        }
    }
}

pub type Doc<'a> = Vec<Token<'a>>;

// Note: numbers are treated as PUNCT (not ideal)
pub fn tokenize(text: &str) -> Doc {
    let mut end = 0;
    let mut index = 0;
    let mut tokens = Vec::new();

    for w in text.split_inclusive(|c: char| c.is_whitespace()) {
        let non_whitespace = w.trim_end_matches(|c: char| c.is_whitespace());
        let (lpunct, word, rpunct) = split_word_punctuation(non_whitespace);

        let start = end;
        end = start + w.len();

        // Empty non_whitespace quick exit case.
        // Treat it as NOT punct since it is only whitespace.
        if non_whitespace.is_empty() {
            let token = Token {
                text: "",
                whitespace: w,
                index,
                range: TextRange::new(start, end),
                punct: false,
                greek: false,
            };
            tokens.push(token);
            index += 1;
            continue;
        }

        if !lpunct.is_empty() {
            let token = Token {
                text: lpunct,
                whitespace: "",
                index,
                range: TextRange::new(start, start + lpunct.len()),
                punct: true,
                greek: false,
            };
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
            let token = Token {
                text: word,
                whitespace,
                index,
                range: TextRange::new(start_at, start_at + word.len() + whitespace.len()),
                punct: false,
                greek: is_greek_word(word),
            };
            tokens.push(token);
            index += 1;
        }

        if !rpunct.is_empty() {
            // May be empty
            let whitespace = &w[lpunct.len() + word.len() + rpunct.len()..];

            let start_at = start + lpunct.len() + word.len();
            let token = Token {
                text: rpunct,
                whitespace,
                index,
                range: TextRange::new(start_at, start_at + whitespace.len() + rpunct.len()),
                punct: true,
                greek: false,
            };
            tokens.push(token);
            index += 1;
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;

    fn splitting(text: &str, expected: &[&str]) {
        let received: Vec<_> = tokenize(text).iter().map(|token| token.text).collect();
        assert_eq!(received, expected)
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
    fn test_split_word_punctuation() {
        assert_eq!(split_word_punctuation("λέξη..."), ("", "λέξη", "..."));
        assert_eq!(split_word_punctuation(";?λέξη"), (";?", "λέξη", ""));
        assert_eq!(split_word_punctuation(";?λέξη..."), (";?", "λέξη", "..."));
        assert_eq!(split_word_punctuation(";?λέ-ξη..."), (";?", "λέ-ξη", "..."));
        assert_eq!(split_word_punctuation(";?..."), (";?...", "", ""));
        assert_eq!(split_word_punctuation("2ος"), ("2", "ος", ""));
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

        for pair in doc.iter().zip_longest(expected.iter()) {
            match pair {
                itertools::EitherOrBoth::Both(rec, exp) => assert_eq!(rec, exp),
                _ => assert!(false),
            }
        }

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

        for pair in doc.iter().zip_longest(expected.iter()) {
            match pair {
                itertools::EitherOrBoth::Both(rec, exp) => assert_eq!(rec, exp),
                _ => assert!(false),
            }
        }

        assert_eq!(doc, expected);
    }
}
