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

use grs::rules::{
    add_final_n, duplicated_word, missing_accent_capital, missing_double_accents,
    monosyllable_accented, multisyllable_not_accented, outdated_spelling, remove_final_n,
};
use itertools::Itertools;
use std::collections::HashMap;
use strum::IntoEnumIterator;

use grs::diagnostic::*;
use grs::registry::*;
use grs::text_diff::*;
use grs::tokenizer::*;

use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

type Config<'a> = &'a [Rule];

fn check_token_with_context<'a>(
    token: &Token<'a>,
    doc: &Doc<'a>,
    config: Config,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    if config.contains(&Rule::MonosyllableAccented) {
        monosyllable_accented(token, doc, &mut diagnostics);
    }
    if config.contains(&Rule::MissingAccentCapital) {
        missing_accent_capital(token, doc, &mut diagnostics);
    }
    if config.contains(&Rule::MultisyllableNotAccented) {
        multisyllable_not_accented(token, doc, &mut diagnostics);
    }
    if config.contains(&Rule::MissingDoubleAccents) {
        missing_double_accents(token, doc, &mut diagnostics);
    }
    if config.contains(&Rule::AddFinalN) {
        add_final_n(token, doc, &mut diagnostics);
    }
    if config.contains(&Rule::RemoveFinalN) {
        remove_final_n(token, doc, &mut diagnostics);
    }
    if config.contains(&Rule::DuplicatedWord) {
        duplicated_word(token, doc, &mut diagnostics);
    }

    diagnostics
}

fn check_raw(text: &str, config: Config) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    if config.contains(&Rule::OutdatedSpelling) {
        outdated_spelling(text, &mut diagnostics);
    }
    diagnostics
}

fn check(text: &str, config: Config) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    // Raw replacements that need no tokenizing
    diagnostics.extend(check_raw(text, config));

    let doc = tokenize(text);

    for token in doc.iter().filter(|token| !token.punct && token.greek) {
        // TODO: A better tokenizer require locator > no but do it for printing lines

        // Run the token-based rules.
        // Atm. every rule requires some context and can not work with the token alone.
        // diagnostics.extend(check_token(token, config));

        // Run the token-context-based rules.
        diagnostics.extend(check_token_with_context(token, &doc, config));
    }

    diagnostics
}

/// Compare two fixes.
fn cmp_fix(rule1: Rule, rule2: Rule, fix1: &Fix, fix2: &Fix) -> std::cmp::Ordering {
    // Always apply `DuplicatedWords` at the start
    match (rule1, rule2) {
        (Rule::DuplicatedWord, _) => std::cmp::Ordering::Less,
        (_, Rule::DuplicatedWord) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
    // Apply fixes in Ascending order of their start position.
    .then_with(|| fix1.range.start().cmp(&fix2.range.start()))
}

/// Get sentence context to print for visualization.
///
/// Highlights the (start, end) range in red.
///
/// TODO: continue printing if we face a period that turns out to be an ellipsis
fn get_context_message(text: &str, fix: &Fix) -> String {
    let start = fix.range.start();
    let end = fix.range.end();

    let max_spaces = 5;
    let ellipsis = "[…] ";

    let (ctx_start, ellipsis_start) = {
        let mut spaces_count = 0;

        let pos = text[..start].rfind(|c| {
            if matches!(c, '.' | '\n') {
                true
            } else if c == ' ' {
                spaces_count += 1;
                spaces_count > max_spaces
            } else {
                false
            }
        });

        let ctx_start = pos.map_or(0, |pos| std::cmp::max(0, pos + 1));
        let ellipsis_start = if spaces_count > max_spaces {
            ellipsis
        } else {
            ""
        };

        (ctx_start, ellipsis_start)
    };

    let (ctx_end, ellipsis_end) = {
        let mut spaces_count = 0;

        let position = text[end..].find(|c| {
            if matches!(c, '.' | '\n') {
                true
            } else if c == ' ' {
                spaces_count += 1;
                spaces_count > max_spaces
            } else {
                false
            }
        });

        let ctx_end = position.map_or(text.len(), |pos| std::cmp::min(text.len(), end + pos + 1));
        let ellipsis_end = if spaces_count > max_spaces {
            ellipsis
        } else {
            ""
        };

        (ctx_end, ellipsis_end)
    };

    let prefix = &text[ctx_start..start];
    let highlighted = &text[start..end];
    let suffix = &text[end..ctx_end];

    // The trim is probably a bad idea
    format!(
        "{}{}{}{}{}",
        ellipsis_start,
        prefix,
        highlighted.red(),
        suffix,
        ellipsis_end,
    )
    .trim()
    .to_string()
}

const MAX_ITERATIONS: usize = 100;

type FixTable = HashMap<Rule, usize>;

/// Should return result
/// cf
/// ruff_linter/src/linter.rs::lint_fix
/// ruff_linter/src/fix/mod.rs
///
/// NOTE:
/// Should do statistics always, for safety // and it's cheap
///
fn fix(text: &str, config: Config, statistics: bool) -> (String, Vec<String>, FixTable) {
    let mut transformed = text.to_string();
    let mut messages = Vec::new();
    let mut fixed = HashMap::new();
    let mut iterations = 0;

    // This is potentially a bad idea iif a fix could affect previous tokens,
    // which is possible but rare since there is not much dependency across tokens.
    //
    // The whole idea is to store the unchanged prefix, where we found no errors,
    // so that we do not have to re-tokenize it on the next pass.
    //
    // Note that the behaviour can be controlled simply by setting first_fix to false:
    // that will re-tokenize the transformed string at each pass.
    let mut final_transformed = String::with_capacity(text.len());

    loop {
        let mut last_pos: Option<usize> = None;

        let diagnostics = check(&transformed, config);

        // Select diagnostics that can be fixed
        let mut with_fixes = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.fix.is_some())
            .peekable();
        // And exit if there are none
        if with_fixes.peek().is_none() {
            break;
        }
        // println!(
        //     "{}",
        //     format!(
        //         "At iteration {}. {} diagnostics.",
        //         iterations,
        //         diagnostics.len().to_string().red().bold()
        //     )
        //     .italic()
        // );

        let rfixes = with_fixes
            .filter_map(|diagnostic| diagnostic.fix.as_ref().map(|fix| (diagnostic.kind, fix)))
            .sorted_by(|(rule1, fix1), (rule2, fix2)| cmp_fix(*rule1, *rule2, fix1, fix2));

        // dbg!("{:?}", rfixes.clone().take(10).collect::<Vec<_>>());

        let mut first_fix = true;
        let mut transformed_this_iter = String::with_capacity(transformed.len());

        for (rule, fix) in rfixes {
            // May happen if we push some rules due to priority
            // We skip this fix.
            if let Some(last_pos) = last_pos {
                if last_pos > fix.range.start() {
                    dbg!("Break due to disordered fixes");
                    break;
                }
            }

            // Debug colored print
            //
            // This should go somewhere else, but it is fine to keep it here for now
            // since it also gives feedback on the behaviour of this fix looping.
            let ctx = get_context_message(&transformed, fix);
            if !statistics && !ctx.is_empty() {
                let message = format!("{:<3}: {}", format!("{}", rule).cyan(), ctx);
                // println!("{}", message);
                messages.push(message);
            }

            *fixed.entry(rule).or_insert(0) += 1;

            if first_fix {
                final_transformed.push_str(&transformed[last_pos.unwrap_or(0)..fix.range.start()]);
                final_transformed.push_str(&fix.replacement);
                first_fix = false;
            } else {
                transformed_this_iter
                    .push_str(&transformed[last_pos.unwrap_or(0)..fix.range.start()]);
                transformed_this_iter.push_str(&fix.replacement);
            }

            last_pos = Some(fix.range.end());
        }

        if let Some(last_pos) = last_pos {
            assert!(last_pos < transformed.len());
            transformed_this_iter.push_str(&transformed[last_pos..]);
        }

        transformed = transformed_this_iter;

        iterations += 1;
        if iterations > MAX_ITERATIONS {
            panic!()
        }
    }

    final_transformed.push_str(&transformed);

    (final_transformed, messages, fixed)
}

#[derive(Parser, Debug)]
#[command(name = "grs", about = "Grs: a rule-based speel checker for Greek.")]
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
    #[arg(long)]
    select: Option<String>,

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
        eprintln!("Failed to read directory {:?}: {}", dir_path, err);
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

    if args.to_monotonic {
        for file in text_files.iter() {
            let text = std::fs::read_to_string(file)
                .unwrap_or_else(|err| panic!("Failed to read file {:?}: {}", file, err));
            let monotonic = grac::to_mono(&text);
            if let Err(err) = std::fs::write(file, &monotonic) {
                eprintln!("Failed to write to file {:?}: {}", file, err);
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

    // let mut config_str: Vec<String> = Vec::new();
    let mut config_str: Vec<String> = ["MDA", "OS"].iter().map(|s| s.to_string()).collect();
    // Add all rules
    // config_str = Rule::iter().map(|rule| rule.code().to_string()).collect();
    if let Some(selection) = args.select {
        if selection == "ALL" {
            config_str = Rule::iter().map(|rule| rule.to_string()).collect();
        } else {
            config_str = selection.split(',').map(|c| c.to_string()).collect();
        }
    }

    // Does not crash if rules to ignore were not in config.
    if let Some(selection) = args.ignore {
        let ignore_rules: Vec<String> = selection.split(',').map(|c| c.to_string()).collect();
        config_str.retain(|rule| !ignore_rules.contains(rule));
    }

    println!("Config: {:?}", config_str);
    // Convert to a Vec<Rules>
    let config: Vec<Rule> = config_str.iter().map(|code| rule_from_code(code)).collect();

    let mut global_statistics_counter = HashMap::new();

    for file in text_files.iter() {
        let text = std::fs::read_to_string(file)
            .unwrap_or_else(|err| panic!("Failed to read file {:?}: {}", file, err));
        let (_fixed, messages, statistics_counter) = fix(&text, &config, args.statistics);

        let mut had_error = false;
        for (key, value) in statistics_counter {
            had_error = true;
            *global_statistics_counter.entry(key).or_insert(0) += value;
        }

        if had_error {
            // println!("{}", file.to_str().unwrap().purple());
            // header
            if args.diff {
                // I dont know how to remove colors
                let text_diff = CodeDiff::new(&text, &_fixed);
                println!("{}", text_diff);
            } else if !args.statistics {
                println!("{}", messages.join("\n"));
            }
        }

        if args.fix {
            // Overwrite the file with the modified content
            if let Err(err) = std::fs::write(file, &_fixed) {
                eprintln!("Failed to write to file {:?}: {}", file, err);
            }
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
                    "{:padding$}    {:<4}    {}",
                    v,
                    format!("{}", k).red().bold(),
                    format!("{:?}", k).cyan(),
                )
            });
    }

    let n_errors = global_statistics_counter.values().sum::<usize>();

    // Should probably count those with fixes...
    if n_errors == 0 {
        println!("No errors!");
    } else if args.fix {
        println!("Fixed {} errors.", format!("{}", n_errors).red().bold());
    } else {
        println!("Detected {} errors.", format!("{}", n_errors).red().bold());
    }

    Ok(ExitStatus::Success)
}

fn main() {
    let fr = std::time::Instant::now();
    let _ = run();
    let to = std::time::Instant::now();
    println!("Execution time: {:.2?}", to.duration_since(fr));
}

// For ad-hoc tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ad_hoc() {
        let text = r#"
        φήμη στην Χάρλεϋ Στρήτ. Θα 
        "#
        .trim()
        .split_inclusive("\n")
        .map(|w| w.trim_start())
        .collect::<String>();

        let config_str = vec!["MA"];
        let config: Vec<Rule> = config_str
            .iter()
            .map(|code| rule_from_code(code))
            .collect::<Vec<_>>();

        println!("Text: '{}'", &text);
        for token in tokenize(&text) {
            println!("{:?}", token)
        }
        let (fixed, messages, _) = fix(&text, &config, false);
        println!("{}", messages.join("\n"));
        println!("{}", fixed);

        // assert!(false);
    }
}
