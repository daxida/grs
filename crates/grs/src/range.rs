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
    /// # Panics
    ///
    /// Panics if `end < start`.
    #[inline]
    pub const fn new(start: TextSize, end: TextSize) -> Self {
        assert!(start <= end);
        Self { start, end }
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
