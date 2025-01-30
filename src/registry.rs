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
    }
}

fn stringify_code(rule: &Rule) -> String {
    stringify(rule)
        .chars()
        .filter(|c| c.is_uppercase())
        .collect()
}

pub fn rule_from_code(code: &str) -> Rule {
    match code {
        "MDA" => Rule::MissingDoubleAccents,
        "MAC" => Rule::MissingAccentCapital,
        "DW" => Rule::DuplicatedWord,
        "AFN" => Rule::AddFinalN,
        "RFN" => Rule::RemoveFinalN,
        "OS" => Rule::OutdatedSpelling,
        "MA" => Rule::MonosyllableAccented,
        "MNA" => Rule::MultisyllableNotAccented,
        _ => panic!("Unknown rule code: {}", code),
    }
}
