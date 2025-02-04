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
];

impl std::str::FromStr for Rule {
    type Err = String;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        for (rule_code, rule) in RULES.iter() {
            if code == *rule_code {
                return Ok(*rule);
            }
        }
        Err(format!("Unknown rule code: {}", code))
    }
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", stringify_code(self))
    }
}

impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", stringify(self))
    }
}

fn stringify(rule: &Rule) -> &str {
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
    }
}

fn stringify_code(rule: &Rule) -> String {
    stringify(rule)
        .chars()
        .filter(|c| c.is_uppercase())
        .collect()
}
