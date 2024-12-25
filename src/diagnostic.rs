use crate::{range::TextRange, registry::Rule};

#[derive(Debug, Clone)]
pub struct Fix {
    pub replacement: String,
    pub range: TextRange,
}

#[derive(Debug)]
pub struct Diagnostic {
    // ruff_diagnostics/src/diagnostic
    // Kind: It is more complicated in ruff, here we just use a rule
    pub kind: Rule,
    pub fix: Option<Fix>,
}
