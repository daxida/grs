use crate::tokenizer::Token;
use grac::constants::APOSTROPHES;

// This should probably be a struct containing the functions below as methods.
//
// Could also include some token methods like token_ctx
pub type Doc<'a> = Vec<Token<'a>>;

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
pub fn is_abbreviation_or_ends_with_dot(token: &Token, doc: &Doc) -> bool {
    if let Some(ntoken) = doc.get(token.index + 1) {
        if token.whitespace.is_empty() && ntoken.punct {
            if let Some(npunct_first_char) = ntoken.text.chars().next() {
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

#[inline(always)]
const fn is_adjacent_punct(token: &Token) -> bool {
    token.punct && token.whitespace.is_empty()
}

#[inline(always)]
fn is_num(token: &Token) -> bool {
    is_adjacent_punct(token) && token.text.chars().all(|c| c.is_ascii_digit())
}

pub fn previous_token_is_num(token: &Token, doc: &Doc) -> bool {
    doc.get(token.index.saturating_sub(1)).is_some_and(is_num)
}

fn is_apostrophe(token: &Token) -> bool {
    is_adjacent_punct(token)
        && match token.text.chars().next() {
            Some(first_char) => APOSTROPHES.contains(&first_char),
            None => false,
        }
}

pub fn previous_token_is_apostrophe(token: &Token, doc: &Doc) -> bool {
    doc.get(token.index.saturating_sub(1))
        .is_some_and(is_apostrophe)
}

/// Returns `true` if this `token` (or some combination of tokens starting
/// at this token) conforms an abbreviation which fulfills the role of
/// an ellipsis. Ex. κ.τ.λ., κτλ, κτλ.
///
/// Includes common typos like κ.λ.π. instead of κ.λπ.
#[allow(unused_variables)]
pub fn followed_by_elliptic_abbreviation(token: &Token, doc: &Doc) -> bool {
    // The last dot must be removed because of our tokenizing logic
    if [
        "κ.τ.λ", "κτλ", "κ.λπ", "κ.λ.π", "κ.τ.ό", "κ.τ.ο", "κ.τ.ρ", "κ.τ.τ", "κ.ά", "κ.α",
    ]
    .contains(&token.text)
    {
        return true;
    }
    // Here some more logic could be added to deal with compounds
    // after the current token.
    false
}
