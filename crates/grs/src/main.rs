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

use clap::Parser;
use colored::Colorize;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;

use grs::cli::Args;
use grs::linter::{fix, lint_only};
use grs::registry::Rule;
use grs::text_diff::CodeDiff;

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
#[allow(clippy::too_many_lines)]
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

    let default_rules: Vec<_> = ["MDA", "OS", "MA", "MNA"]
        .iter()
        .map(|code| code.parse::<Rule>().unwrap())
        .collect();

    let mut config: Vec<Rule> = args.select.map_or(default_rules, |selection| {
        selection
            .iter()
            .flat_map(grs::cli::RuleSelector::rules)
            .unique()
            .collect()
    });

    // Does not crash if rules to ignore were not in config.
    if let Some(selection) = args.ignore {
        config.retain(|rule| {
            !selection
                .iter()
                .any(|selector| selector.rules().contains(rule))
        });
    }

    println!(
        "Config: [{}]",
        config
            .iter()
            .map(|rule| rule.to_string().green().to_string())
            .collect::<Vec<String>>()
            .join(", ")
    );

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
        let text = r"
        φήμη στην Χάρλεϋ Στρήτ. Θα 
        "
        .trim()
        .split_inclusive('\n')
        .map(str::trim_start)
        .collect::<String>();

        let config_str = ["MA"];
        let config: Vec<_> = config_str
            .iter()
            .map(|code| code.parse::<Rule>().unwrap())
            .collect();

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
