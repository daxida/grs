use strum::IntoEnumIterator;
use strum_macros::{EnumIter, IntoStaticStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(EnumIter, IntoStaticStr, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Rule {
    MissingDoubleAccents,
    MissingAccentCapital,
    DuplicatedWord,
    AddFinalN,
    RemoveFinalN,
    OutdatedSpelling,
    MonosyllableAccented,
    MultisyllableNotAccented,
    MixedScripts,
    AmbiguousChar,
    ForbiddenAccent,
    ForbiddenChar,
    Punctuation,
}

impl Rule {
    // Having hardcoded this here over extracting it from the rule
    // implementations is not ideal.
    pub const fn has_fix(&self) -> bool {
        #[allow(clippy::enum_glob_use)]
        use Rule::*;
        matches!(
            self,
            MissingDoubleAccents
                | MissingAccentCapital
                | AddFinalN
                | RemoveFinalN
                | OutdatedSpelling
                | MonosyllableAccented
                | MixedScripts
                | AmbiguousChar
                | Punctuation
        )
    }

    pub const fn requires_tokenizing(&self) -> bool {
        !matches!(
            self,
            Self::OutdatedSpelling | Self::AmbiguousChar | Self::ForbiddenChar
        )
    }
}

/// Get the code from the name:
/// "MissingDoubleAccents" => "MDA"
fn name_to_code(name: &str) -> String {
    name.chars().filter(|c| c.is_uppercase()).collect()
}

/// Get the rule from the code:
/// "MDA" => `Rule::MissingDoubleAccents`
pub fn code_to_rule(code: &str) -> Option<Rule> {
    Rule::iter().find(|rule| {
        let name: &'static str = rule.into();
        name_to_code(name) == code
    })
}

/// Get the name from the rule:
/// `Rule::MissingDoubleAccents` => "MissingDoubleAccents"
pub fn rule_to_name(rule: Rule) -> &'static str {
    rule.into()
}

/// Get the code from the rule:
/// `Rule::MissingDoubleAccents` => "MDA"
pub fn rule_to_code(rule: Rule) -> String {
    name_to_code(rule_to_name(rule))
}

impl std::str::FromStr for Rule {
    type Err = String;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        code_to_rule(code).ok_or_else(|| format!("Unknown rule code: {code}"))
    }
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", rule_to_code(*self))
    }
}

impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", rule_to_name(*self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converters() {
        let name = "MissingDoubleAccents";
        let code = "MDA";
        let rule = Rule::MissingDoubleAccents;

        assert_eq!(name_to_code(name), code);
        assert_eq!(code_to_rule(code), Some(rule));
        assert_eq!(rule_to_name(rule), name);
        assert_eq!(rule_to_code(rule), code);
    }
}
