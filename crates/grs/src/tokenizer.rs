use crate::range::TextRange;
use colored::Colorize;
use grac::constants::APOSTROPHES;
use grac::{is_greek_word, syllabify};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// This should probably be a struct containing the functions below as methods.
// pub type Doc<'a> = Vec<LoadedToken<'a>>;

// TODO:
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Doc<'a> {
    src: &'a str,
    tokens: Vec<Token<'a>>,
}

impl<'a> Deref for Doc<'a> {
    type Target = Vec<Token<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.tokens
    }
}

impl DerefMut for Doc<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tokens
    }
}

// IntoIterator for LoadedDoc (move)
impl<'a> IntoIterator for Doc<'a> {
    type Item = Token<'a>;
    type IntoIter = std::vec::IntoIter<Token<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

// IntoIterator for &LoadedDoc (borrow)
impl<'a> IntoIterator for &'a Doc<'a> {
    type Item = &'a Token<'a>;
    type IntoIter = std::slice::Iter<'a, Token<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.iter()
    }
}

impl Doc<'_> {
    /// Debug function. Stringify the context of the token.
    pub fn context(&self, token: &Token) -> String {
        let n_tokens_before = 7;
        let n_tokens_after = 7;

        let start = token.index.saturating_sub(n_tokens_before);
        let end = token.index + n_tokens_after;

        let ctx = (start..=end)
            .filter_map(|idx| self.get(idx))
            .enumerate()
            .map(|(idx, t)| {
                // May fail at the very beginning of the text
                if idx == n_tokens_before {
                    t.text().bold().to_string()
                } else {
                    t.text().to_string()
                }
            })
            .collect::<String>();

        ctx.replace('\n', "⏎")
    }
}

// Iteration methods
impl<'a> Doc<'a> {
    #[inline]
    pub fn next_token_not_whitespace(&'a self, token: &Token) -> Option<&'a Token<'a>> {
        let mut idx = token.index;
        loop {
            idx += 1;
            let ntoken = self.get(idx)?;
            if ntoken.kind != TokenKind::Whitespace {
                return Some(ntoken);
            }
        }
    }

    #[inline]
    pub fn next_token_greek_word(&'a self, token: &Token) -> Option<&'a Token<'a>> {
        let mut idx = token.index;
        loop {
            idx += 1;
            let ntoken = self.get(idx)?;
            if ntoken.kind == TokenKind::GreekWord {
                return Some(ntoken);
            }
        }
    }

    #[inline]
    pub fn prev_token_not_whitespace(&'a self, token: &Token) -> Option<&'a Token<'a>> {
        let mut idx = token.index;
        while idx > 0 {
            idx -= 1;
            let ntoken = self.get(idx)?;
            if ntoken.kind != TokenKind::Whitespace {
                return Some(ntoken);
            }
        }
        None
    }
}

// Property methods
impl Doc<'_> {
    pub fn previous_token_is_num(&self, token: &Token) -> bool {
        self.prev_token_not_whitespace(token)
            .is_some_and(Token::is_num)
    }

    pub fn previous_token_is_apostrophe(&self, token: &Token) -> bool {
        self.prev_token_not_whitespace(token)
            .is_some_and(Token::is_apostrophe)
    }

    /// A word is considered an abbreviation if it is followed by an apostrophe.
    /// Ex. όλ' αυτά
    ///
    /// Note that όλ ' αυτά (with an space before the apostrophe) is not considered an
    /// abbreviation.
    ///
    /// A dot must be treated like a black box since there is no way to distinguish
    /// if it is a period, an ellipsis or an abbreviation dot. Checking if the next word
    /// is capitalized is not a solution, since an abbreviation might be followed by
    /// a proper noun, invalidating the logic. Ex. Λεωφ. Κηφισού.
    pub fn is_abbreviation_or_ends_with_dot(&self, token: &Token) -> bool {
        if let Some(ntoken) = self.next_token_not_whitespace(token) {
            if ntoken.is_punctuation() {
                if let Some(npunct_first_char) = ntoken.text().chars().next() {
                    if ['.', '…'].contains(&npunct_first_char)
                        || APOSTROPHES.contains(&npunct_first_char)
                    {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TokenKind {
    /// Any whitespace character sequence.
    Whitespace,

    /// A word.
    #[default]
    Word,

    /// A Greek word.
    GreekWord,

    /// Punctuation token.
    Punctuation,
}

/// Token type.
///
// Should probably not store the slice (text) part, and delegate that to Doc.
//
// Initially, very simplified version of:
// https://github.com/explosion/spaCy/blob/311f7cc9fbd44e3de14fa673fa9c5146ea223624/spacy/tokenizer.pyx#L25
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Token<'a> {
    /// Index in the Doc
    index: usize,
    /// Text value of the token
    text: &'a str,
    /// Start byte of the text in source
    offset: u32,
    kind: TokenKind,
}

impl<'a> Token<'a> {
    /// # Panics
    ///
    /// Panics if `offset` is larger than `u32::MAX`.
    pub fn new(text: &'a str, index: usize, offset: usize, kind: TokenKind) -> Self {
        Self {
            text,
            index,
            offset: u32::try_from(offset).unwrap(),
            kind,
        }
    }

    #[inline]
    pub const fn text(&self) -> &str {
        self.text
    }

    #[inline]
    pub const fn kind(&self) -> TokenKind {
        self.kind
    }

    /// Start and end byte of the token.
    #[inline]
    pub const fn range(&self) -> TextRange {
        let offset = self.offset as usize;
        TextRange::new(offset, offset + self.text.len())
    }
}

// Property methods
impl Token<'_> {
    #[inline]
    pub fn is_word(&self) -> bool {
        self.kind == TokenKind::Word
    }

    #[inline]
    pub fn is_whitespace(&self) -> bool {
        self.kind == TokenKind::Whitespace
    }

    #[inline]
    pub fn is_punctuation(&self) -> bool {
        self.kind == TokenKind::Punctuation
    }

    #[inline]
    pub fn is_greek_word(&self) -> bool {
        self.kind == TokenKind::GreekWord
    }

    #[inline]
    fn is_num(&self) -> bool {
        self.is_punctuation() && self.text().chars().all(|c| c.is_ascii_digit())
    }

    #[inline]
    fn is_apostrophe(&self) -> bool {
        self.is_punctuation()
            && self
                .text()
                .chars()
                .next()
                .is_some_and(|c| APOSTROPHES.contains(&c))
    }

    // Note that this function is very expensive.
    #[inline]
    pub fn num_syllables(&self) -> usize {
        syllabify(self.text()).len()
    }

    /// Returns `true` if this `token` conforms an abbreviation which fulfills the role of
    /// an ellipsis. Ex. κ.τ.λ., κτλ, κτλ.
    pub fn is_elliptic_abbreviation(&self) -> bool {
        // The last dot must be removed because of our tokenizing logic.
        // Includes common typos like κ.λ.π. instead of κ.λπ.
        const ELLIPTIC_ABBREVIATION: [&str; 10] = [
            "κ.τ.λ", "κτλ", "κ.λπ", "κ.λ.π", "κ.τ.ό", "κ.τ.ο", "κ.τ.ρ", "κ.τ.τ", "κ.ά", "κ.α",
        ];

        ELLIPTIC_ABBREVIATION.contains(&self.text())
    }
}

enum SplitKind {
    Word,
    Whitespace,
}

// Custom split logic.
// * "Hello   world" > ["Hello", "   ", "world"]
fn split_whitespace(text: &str) -> impl Iterator<Item = (SplitKind, &str)> {
    let mut offset = 0;
    let len = text.len();

    std::iter::from_fn(move || {
        if offset >= len {
            return None;
        }

        let s = &text[offset..];
        let mut iter = s.char_indices();
        let (_, first_ch) = iter.next()?;
        let is_ws = first_ch.is_whitespace();

        let mut split_at = s.len();
        for (i, ch) in iter {
            if ch.is_whitespace() != is_ws {
                split_at = i;
                break;
            }
        }

        let chunk = &s[..split_at];
        offset += split_at;

        let kind = if is_ws {
            SplitKind::Whitespace
        } else {
            SplitKind::Word
        };

        Some((kind, chunk))
    })
}

/// Build a `Doc` from a source `text`.
///
// Notes:
// * numbers are treated as punctuation (not ideal?)
// * marginal (~10%) performance gains can be obtained by using the Logos library
//   with unsafe allowed to replace the whitespace splitting logic. Without unsafe
//   the performance seems identical. It is probably not worth the dependency.
pub fn tokenize(text: &str) -> Doc {
    let mut end = 0;
    let mut index = 0;
    let mut tokens = Vec::new();

    for (kind, slice) in split_whitespace(text) {
        let start = end;
        end = start + slice.len();

        match kind {
            SplitKind::Whitespace => {
                let token = Token::new(slice, index, start, TokenKind::Whitespace);
                tokens.push(token);
                index += 1;
            }
            SplitKind::Word => {
                let (lpunct, word, rpunct) = grac::split_punctuation(slice);

                if !lpunct.is_empty() {
                    let range = TextRange::new(start, start + lpunct.len());
                    let kind = TokenKind::Punctuation;
                    let chunk = &text[range.start()..range.end()];
                    let token = Token::new(chunk, index, start, kind);
                    tokens.push(token);
                    index += 1;
                }

                if !word.is_empty() {
                    debug_assert!(!word.contains(|c: char| c.is_whitespace()));
                    let start_at = start + lpunct.len();
                    let len = word.len();
                    let range = TextRange::new(start_at, start_at + len);
                    let chunk = &text[range.start()..range.end()];
                    let kind = if is_greek_word(chunk) {
                        TokenKind::GreekWord
                    } else {
                        TokenKind::Word
                    };
                    let token = Token::new(chunk, index, start_at, kind);
                    tokens.push(token);
                    index += 1;
                }

                if !rpunct.is_empty() {
                    let start_at = start + lpunct.len() + word.len();
                    let range = TextRange::new(start_at, start_at + rpunct.len());
                    let kind = TokenKind::Punctuation;
                    let chunk = &text[range.start()..range.end()];
                    let token = Token::new(chunk, index, start_at, kind);
                    tokens.push(token);
                    index += 1;
                }
            }
        }
    }

    Doc { src: text, tokens }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(text: &str, expected: &[&str]) {
        let doc = tokenize(text);
        let received: Vec<_> = doc.iter().map(super::Token::text).collect();
        assert_eq!(received, expected);
    }

    #[test]
    fn test_splitting_ascii() {
        split("Hello world!  ", &["Hello", " ", "world", "!", "  "]);
    }

    #[test]
    fn test_splitting_non_ascii_ws() {
        split("Hello\u{3000}world!", &["Hello", "\u{3000}", "world", "!"]);
    }

    #[test]
    fn test_splitting_greek1() {
        split("Καλημέρα, κόσμε", &["Καλημέρα", ",", " ", "κόσμε"]);
        split("το: Φέγγαρι", &["το", ":", " ", "Φέγγαρι"]);
    }

    #[test]
    fn test_splitting_greek2() {
        split("την «ξεκρέμασε", &["την", " ", "«", "ξεκρέμασε"]);
        split(
            " την  «   ξεκρέμασε ",
            &[" ", "την", "  ", "«", "   ", "ξεκρέμασε", " "],
        );
    }

    #[test]
    fn test_splitting_newlone() {
        split("α\nβ", &["α", "\n", "β"]);
        split("α \nβ", &["α", " \n", "β"]);
        split("α\n β", &["α", "\n ", "β"]);
        split("α \n β", &["α", " \n ", "β"]);
    }

    #[test]
    fn test_splitting_apostrophe() {
        split("όλ' αυτά", &["όλ", "'", " ", "αυτά"]);
        split("ἄρ᾽ Ἀθήνας", &["ἄρ", "᾽", " ", "Ἀθήνας"]);
    }

    #[test]
    fn test_splitting_single_inner_punct() {
        split("σ'αυτόν", &["σ'αυτόν"]);
    }

    #[test]
    fn test_splitting_double_inner_punct() {
        split("του|-πουλος", &["του|-πουλος"]);
    }

    #[test]
    fn test_tokenization_text() {
        let variations = ["α 'β", "α ' β", "α' β"];
        for text in variations {
            let doc = tokenize(text);
            let filtered = doc.iter().filter(|t| t.kind != TokenKind::Whitespace);
            let expected = ["α", "'", "β"];
            for (rec, exp) in filtered.zip(expected.iter()) {
                eprintln!("'{}' '{:?}'\n- {:?}", rec.text(), rec.range(), rec,);
                assert_eq!(rec.text(), *exp);
            }
        }
    }
}
