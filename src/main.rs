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

use grs::missing_accent_capital::missing_accent_capital;
use grs::range::TextRange;
use itertools::Itertools;
use std::collections::HashMap;
use strum::IntoEnumIterator;

use grs::accents::*;
use grs::diagnostic::*;
use grs::duplicated_word::*;
use grs::final_n::*;
use grs::missing_double_accents::*;
use grs::registry::*;
use grs::text_diff::*;
use grs::tokenizer::*;

use clap::{Arg, Command};
use colored::Colorize;
use std::path::PathBuf;

const OUTDATED_SPELLINGS_MULTIPLE: &[(&str, &str)] = &[
    ("κρεββάτι", "κρεβάτι"),
    ("Κρεββάτι", "Κρεβάτι"),
    ("εξ άλλου", "εξάλλου"),
    ("Εξ άλλου", "Εξάλλου"),
    ("εξ αιτίας", "εξαιτίας"),
    ("Εξ αιτίας", "Εξαιτίας"),
];

/// Outdated spelling of strings.
///
/// Two caveats:
/// - Without regex or some more logic, this is agnostic of word boundaries
///   and could replace chunks inside words. This is fine.
/// - The const table needs manual adding of uppercase variants since the
///   prize of casting .to_lowercase() is too big, and I have not figured out
///   how to build a const array with capitalized variants at compile time.
fn outdated_spelling(text: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Probably the other order is a better choice
    for (target, destination) in OUTDATED_SPELLINGS_MULTIPLE.iter() {
        // There must be sth better without break
        if let Some((start, _)) = text.match_indices(target).next() {
            diagnostics.push(Diagnostic {
                kind: Rule::OutdatedSpelling,
                fix: Some(Fix {
                    replacement: destination.to_string(),
                    range: TextRange::new(start, start + target.len()),
                }),
            });
        }
    }
}

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

fn parse_args() -> clap::ArgMatches {
    Command::new("Greek spell checker")
        .about("Check for a variety of spelling mistakes in Greek text.")
        .arg(
            Arg::new("files")
                .help("Files to process. Anything other than .txt files will be ignored.")
                .num_args(1..)
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("fix")
                .help("Replace the input file")
                .long("fix")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("diff")
                .help("Finish me")
                .long("diff")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("select")
                .help("Specify which types of mistakes to check.")
                .long("select")
                .num_args(1)
                .value_parser(clap::builder::NonEmptyStringValueParser::new()),
        )
        .arg(
            Arg::new("ignore")
                .help("Specify which types of mistakes to ignore.")
                .long("ignore")
                .num_args(1)
                .value_parser(clap::builder::NonEmptyStringValueParser::new()),
        )
        .arg(
            Arg::new("statistics")
                .long("statistics")
                .action(clap::ArgAction::SetTrue),
        )
        // For convenience
        .arg(
            Arg::new("to-monotonic")
                .long("to-monotonic")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches()
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
    let args = parse_args();

    let text_files = args
        .get_many::<PathBuf>("files")
        .ok_or(ExitStatus::Failure)?
        .filter(|file| file.extension().and_then(|ext| ext.to_str()) == Some("txt"))
        .collect::<Vec<_>>();
    // let text_files = find_text_files_in_tests()?;

    if args.get_flag("to-monotonic") {
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

    let fix_flag = args.get_flag("fix");
    if fix_flag {
        println!("Fix flag is enabled.");
    }
    let statistics = args.get_flag("statistics");
    if statistics {
        println!("Statistics is enabled.");
    }
    let diff = args.get_flag("diff");
    if diff {
        println!("Diff is enabled.");
    }

    let mut config_str: Vec<String> = Vec::new();
    // Add all rules
    // config_str = Rule::iter().map(|rule| rule.code().to_string()).collect();
    if let Some(selection) = args.get_one::<String>("select") {
        if selection == "ALL" {
            config_str = Rule::iter().map(|rule| rule.to_string()).collect();
        } else {
            config_str = selection.split(',').map(|c| c.to_string()).collect();
        }
    }
    // Does not crash if rules to ignore were not in config.
    if let Some(selection) = args.get_one::<String>("ignore") {
        let ignore_rules: Vec<String> = selection.split(',').map(|c| c.to_string()).collect();
        config_str.retain(|rule| !ignore_rules.contains(rule));
    }

    println!("Config: {:?}", config_str);
    // Convert to a vec or rules
    let config: Vec<Rule> = config_str.iter().map(|code| rule_from_code(code)).collect();

    let mut global_statistics_counter = HashMap::new();

    for file in text_files.iter() {
        let text = std::fs::read_to_string(file)
            .unwrap_or_else(|err| panic!("Failed to read file {:?}: {}", file, err));
        let (_fixed, messages, statistics_counter) = fix(&text, &config, statistics);

        let mut had_error = false;
        for (key, value) in statistics_counter {
            had_error = true;
            *global_statistics_counter.entry(key).or_insert(0) += value;
        }

        if had_error {
            println!("{}", file.to_str().unwrap().purple());
            // header
            if diff {
                // I dont know how to remove colors
                let text_diff = CodeDiff::new(&text, &_fixed);
                println!("{}", text_diff);
            } else if !statistics {
                println!("{}", messages.join("\n"));
            }
        }

        if fix_flag {
            // Overwrite the file with the modified content
            if let Err(err) = std::fs::write(file, &_fixed) {
                eprintln!("Failed to write to file {:?}: {}", file, err);
            }
        }
    }

    if statistics {
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
    } else if fix_flag {
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
    println!("Execution time: {:?}", to.duration_since(fr));
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
