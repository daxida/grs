use crate::{range::TextRange, registry::Rule};

// We do not use Edit since our replacement logic is much simpler,
// usually consisting of modifying solely substrings.
#[derive(Debug, Clone)]
pub struct Fix {
    pub replacement: String,
    pub range: TextRange,
}

// Simplified version of:
// https://github.com/astral-sh/ruff/blob/main/crates/ruff_diagnostics/src/diagnostic.rs
//
// * kind is simply a Rule for our purposes.
#[derive(Debug)]
pub struct Diagnostic {
    pub kind: Rule,
    pub range: TextRange,
    pub fix: Option<Fix>,
}
