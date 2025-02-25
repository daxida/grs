/*
* TODO:
*
* - Better deal with multiple errors in one line
*
* Links
*
* Stop words
* - https://www.translatum.gr/forum/index.php?topic=3550.0?topic=3550.0
* Final n
* - https://www.translatum.gr/converter/teliko-n-diorthosi.php
*
* Spacy
* - https://github.com/explosion/spaCy/tree/master/spacy/lang/el
* Ruff
* - https://github.com/astral-sh/ruff
* clippy?
*/

use itertools::Itertools;
use std::collections::HashMap;
use strum::IntoEnumIterator;

use grs::linter::{fix, lint_only};
use grs::registry::{Rule, RULES};
use grs::text_diff::CodeDiff;

use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "grs", about = "Grs: a rule-based spell checker for Greek.")]
pub struct Args {
    /// Files to process. Anything other than .txt files will be ignored.
    #[arg(value_parser, required = true)]
    files: Vec<PathBuf>,

    /// Replace the input file.
    #[arg(long)]
    fix: bool,

    /// Show differences between original and corrected text.
    #[arg(long)]
    diff: bool,

    /// Specify which types of mistakes to check.
    #[arg(long, value_delimiter = ',')]
    select: Option<Vec<String>>,

    /// Specify which types of mistakes to ignore.
    #[arg(long)]
    ignore: Option<String>,

    /// Show statistics after processing.
    #[arg(long)]
    statistics: bool,

    /// Convert text to monotonic Greek.
    // Does this belong to this project?
    #[arg(long = "to-monotonic")]
    to_monotonic: bool,
}

#[derive(Copy, Clone)]
pub enum ExitStatus {
    Success,
    Failure,
    Error,
}

#[allow(dead_code)]
fn find_text_files_in_tests() -> Result<Vec<PathBuf>, ExitStatus> {
    let dir_path = PathBuf::from(".");
    let entries = std::fs::read_dir(&dir_path).map_err(|err| {
        eprintln!("Failed to read directory {dir_path:?}: {err}");
        ExitStatus::Failure
    })?;
    let mut text_files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| ExitStatus::Failure)?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
            text_files.push(path);
        }
    }
    Ok(text_files)
}

// TODO: run_for_file
fn run() -> Result<ExitStatus, ExitStatus> {
    let args = Args::parse();

    let text_files = args
        .files
        .iter()
        .filter(|file| file.extension().and_then(|ext| ext.to_str()) == Some("txt"))
        .collect::<Vec<_>>();
    // let text_files = find_text_files_in_tests()?;

    if text_files.is_empty() {
        eprintln!("No valid text files found.");
        return Ok(ExitStatus::Success);
    }

    if args.to_monotonic {
        for file in &text_files {
            let text = match std::fs::read_to_string(file) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Failed to read file {file:?}: {err}");
                    return Err(ExitStatus::Failure);
                }
            };
            let monotonic = grac::to_monotonic(&text);
            if let Err(err) = std::fs::write(file, &monotonic) {
                eprintln!("Failed to write to file {file:?}: {err}");
                return Err(ExitStatus::Failure);
            }
        }
        println!("Successfully converted to monotonic.");
        return Ok(ExitStatus::Success);
    }

    if args.fix {
        println!("Fix flag is enabled.");
    }
    if args.statistics {
        println!("Statistics is enabled.");
    }
    if args.diff {
        println!("Diff is enabled.");
    }

    let mut config_str: Vec<String> = match args.select {
        None => ["MDA", "OS", "MA", "MNA"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        Some(selection) => {
            if selection.contains(&"ALL".to_string()) {
                Rule::iter().map(|rule| rule.to_string()).collect()
            } else {
                selection
            }
        }
    };

    // Does not crash if rules to ignore were not in config.
    if let Some(selection) = args.ignore {
        let ignore_rules: Vec<String> = selection.split(',').map(|c| c.to_string()).collect();
        config_str.retain(|rule| !ignore_rules.contains(rule));
    }

    // Convert to a Vec<Rules>
    println!("Config: {config_str:?}");
    // TODO: This should be done at CLI parsing
    let config_res: Result<Vec<Rule>, ExitStatus> = config_str
        .iter()
        .map(|code| {
            code.parse::<Rule>().map_err(|err| {
                // Print all rules and exit
                eprintln!(
                    "{}\n  [possible values: {}]",
                    err,
                    RULES
                        .iter()
                        .map(|(code, _)| code.to_string().green())
                        .join(", ")
                );
                ExitStatus::Error
            })
        })
        .collect();
    let config = config_res?;

    let mut global_statistics_counter = HashMap::new();

    for file in &text_files {
        let text = match std::fs::read_to_string(file) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Failed to read file {file:?}: {err}");
                return Err(ExitStatus::Failure);
            }
        };

        // Header
        // println!("{}", file.to_str().unwrap().purple());

        let statistics_counter = if args.diff {
            let (fixed, _messages, statistics_counter) = fix(&text, &config);
            // I dont know how to remove colors
            let text_diff = CodeDiff::new(&text, &fixed);
            println!("{text_diff}");
            statistics_counter
        } else if args.fix {
            let (fixed, _messages, statistics_counter) = fix(&text, &config);
            // Overwrite the file with the modified content
            if let Err(err) = std::fs::write(file, &fixed) {
                eprintln!("Failed to write to file {file:?}: {err}");
            }
            // if !args.statistics {
            //     println!("{}", messages.join("\n"));
            // }
            statistics_counter
        } else {
            let (messages, statistics_counter) = lint_only(&text, &config);
            if !args.statistics && !messages.is_empty() {
                println!("{}", messages.join("\n"));
            }
            statistics_counter
        };

        for (key, value) in statistics_counter {
            *global_statistics_counter.entry(key).or_insert(0) += value;
        }
    }

    if args.statistics {
        let padding = global_statistics_counter
            .values()
            .map(|k| k.to_string().len())
            .max()
            .unwrap_or(0);

        global_statistics_counter
            .iter()
            .sorted_by(|a, b| b.1.cmp(a.1))
            .for_each(|(k, v)| {
                println!(
                    "{:padding$}    {:<4}   [{}] {:?}",
                    v,
                    format!("{k}").red().bold(),
                    (if k.has_fix() { "*" } else { " " }).to_string().cyan(),
                    k
                );
            });
    }

    let n_errors = global_statistics_counter.values().sum::<usize>();
    let n_fixable_errors = global_statistics_counter
        .iter()
        .filter_map(|(rule, cnt)| if rule.has_fix() { Some(cnt) } else { None })
        .sum::<usize>();

    // Should probably count those with fixes...
    if n_errors == 0 {
        println!("No errors!");
    } else if args.fix {
        println!("Fixed {n_errors} errors.");
    } else {
        println!(
            "Found {} errors.\n[{}] {} fixable with the `--fix` option.",
            n_errors,
            "*".to_string().cyan(),
            n_fixable_errors,
        );
    }

    Ok(ExitStatus::Success)
}

fn main() {
    let fr = std::time::Instant::now();
    let _ = run();
    println!("Execution time: {:.2?}", fr.elapsed());
}

// For ad-hoc tests
#[cfg(test)]
mod tests {
    use super::*;
    use grs::tokenizer::tokenize;

    #[test]
    fn test_ad_hoc() {
        let text = r#"
        φήμη στην Χάρλεϋ Στρήτ. Θα 
        "#
        .trim()
        .split_inclusive("\n")
        .map(|w| w.trim_start())
        .collect::<String>();

        let config_str = ["MA"];
        let config: Vec<Rule> = config_str
            .iter()
            .map(|code| code.parse::<Rule>().unwrap())
            .collect::<Vec<_>>();

        println!("Text: '{}'", &text);
        for token in tokenize(&text) {
            println!("{token:?}");
        }
        let (fixed, messages, _) = fix(&text, &config);
        println!("{}", messages.join("\n"));
        println!("{fixed}");

        // assert!(false);
    }
}
