use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

type TextSize = usize;

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TextRange {
    // Invariant: start <= end
    start: TextSize,
    end: TextSize,
}

impl fmt::Debug for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl TextRange {
    #[inline]
    pub const fn new(start: TextSize, end: TextSize) -> TextRange {
        assert!(start <= end);
        TextRange { start, end }
    }
}

/// Identity methods.
impl TextRange {
    /// The start point of this range.
    #[inline]
    pub const fn start(self) -> TextSize {
        self.start
    }

    /// The end point of this range.
    #[inline]
    pub const fn end(self) -> TextSize {
        self.end
    }
}
