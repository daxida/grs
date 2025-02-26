use crate::registry::Rule;
use clap::builder::{PossibleValue, TypedValueParser};
use clap::Parser;
use std::path::PathBuf;
use strum::IntoEnumIterator;

#[derive(Parser, Debug)]
#[command(name = "grs", about = "Grs: a rule-based spell checker for Greek.")]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
    /// Files to process. Anything other than .txt files will be ignored.
    #[arg(value_parser, required = true)]
    pub files: Vec<PathBuf>,

    /// Replace the input file.
    #[arg(long)]
    pub fix: bool,

    /// Show differences between original and corrected text.
    #[arg(long)]
    pub diff: bool,

    /// Specify which types of mistakes to check.
    #[arg(
        long,
        value_delimiter = ',',
        value_name = "RULE_CODE",
        value_parser = RuleSelectorParser,
    )]
    pub select: Option<Vec<RuleSelector>>,

    /// Specify which types of mistakes to ignore.
    #[arg(
        long,
        value_delimiter = ',',
        value_name = "RULE_CODE",
        value_parser = RuleSelectorParser,
    )]
    pub ignore: Option<Vec<RuleSelector>>,

    /// Show statistics after processing.
    #[arg(long)]
    pub statistics: bool,

    /// Convert text to monotonic Greek.
    // Does this belong to this project?
    #[arg(long = "to-monotonic")]
    pub to_monotonic: bool,
}

// The whole point of selector is to deal with the --select ALL
// option in the CLI. While they do it like this in ruff to expand linter
// groups, that is most likely out of our reach for this project.
//
// Though it has the advantage of customizing the possible values, printed
// when one types --select with no extra arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleSelector {
    All,
    Selection(Rule),
}

impl RuleSelector {
    pub fn rules(&self) -> Vec<Rule> {
        match self {
            Self::All => Rule::iter().collect(),
            Self::Selection(selection) => vec![*selection],
        }
    }
}

impl std::str::FromStr for RuleSelector {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ALL" => Ok(Self::All),
            _ => Ok(Self::Selection(s.parse()?)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleSelectorParser;

impl TypedValueParser for RuleSelectorParser {
    type Value = RuleSelector;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value
            .to_str()
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))?;

        value.parse().map_err(|_| {
            let mut error = clap::Error::new(clap::error::ErrorKind::ValueValidation).with_cmd(cmd);
            if let Some(arg) = arg {
                error.insert(
                    clap::error::ContextKind::InvalidArg,
                    clap::error::ContextValue::String(arg.to_string()),
                );
            }
            error.insert(
                clap::error::ContextKind::InvalidValue,
                clap::error::ContextValue::String(value.to_string()),
            );
            error
        })
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(
            std::iter::once(PossibleValue::new("ALL").help("all rules")).chain(Rule::iter().map(
                |rule| {
                    let code: String = rule.to_string();
                    PossibleValue::new(code)
                },
            )),
        ))
    }
}
