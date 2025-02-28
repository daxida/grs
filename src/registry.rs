use strum_macros::EnumIter;

#[derive(EnumIter, Clone, Copy, PartialEq, Eq, Hash)]
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
        )
    }

    pub const fn requires_tokenizing(&self) -> bool {
        !matches!(self, Rule::OutdatedSpelling | Rule::AmbiguousChar)
    }
}

// This is probably slower than using a match statement in Rule::FromStr
// but it has the advantage of being reusable in other parts of the code,
// like when returning a list of codes if an non existent code is passed
// to --select via the CLI.
pub const RULES: &[(&str, Rule)] = &[
    ("MDA", Rule::MissingDoubleAccents),
    ("MAC", Rule::MissingAccentCapital),
    ("DW", Rule::DuplicatedWord),
    ("AFN", Rule::AddFinalN),
    ("RFN", Rule::RemoveFinalN),
    ("OS", Rule::OutdatedSpelling),
    ("MA", Rule::MonosyllableAccented),
    ("MNA", Rule::MultisyllableNotAccented),
    ("MS", Rule::MixedScripts),
    ("AC", Rule::AmbiguousChar),
];

impl std::str::FromStr for Rule {
    type Err = String;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        for (rule_code, rule) in RULES {
            if code == *rule_code {
                return Ok(*rule);
            }
        }
        Err(format!("Unknown rule code: {code}"))
    }
}

// Return the acronym of the rule: MDA
impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", stringify_code(*self))
    }
}

// Return the Pascal case name of the rule: MissingDoubleAccents
impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", stringify(*self))
    }
}

const fn stringify(rule: Rule) -> &'static str {
    match rule {
        Rule::MissingDoubleAccents => "MissingDoubleAccents",
        Rule::MissingAccentCapital => "MissingAccentCapital",
        Rule::DuplicatedWord => "DuplicatedWord",
        Rule::AddFinalN => "AddFinalN",
        Rule::RemoveFinalN => "RemoveFinalN",
        Rule::OutdatedSpelling => "OutdatedSpelling",
        Rule::MonosyllableAccented => "MonosyllableAccented",
        Rule::MultisyllableNotAccented => "MultisyllableNotAccented",
        Rule::MixedScripts => "MixedScripts",
        Rule::AmbiguousChar => "AmbiguousChar",
    }
}

fn stringify_code(rule: Rule) -> String {
    stringify(rule)
        .chars()
        .filter(|c| c.is_uppercase())
        .collect()
}
