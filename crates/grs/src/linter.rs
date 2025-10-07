//! Named linter for archaic reasons: it should be a checker.
use colored::Colorize;
use itertools::Itertools;
use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, Fix};
use crate::range::TextRange;
use crate::registry::Rule;
use crate::tokenizer::{Doc, Token, tokenize};

#[allow(clippy::wildcard_imports)]
use crate::rules::*;

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
    if config.contains(&Rule::ForbiddenAccent) {
        forbidden_accent(token, doc, &mut diagnostics);
    }
    if config.contains(&Rule::ForbiddenDoubleAccent) {
        forbidden_double_accent(token, doc, &mut diagnostics);
    }
    if config.contains(&Rule::Punctuation) {
        punctuation(token, doc, &mut diagnostics);
    }

    diagnostics
}

fn check_raw(text: &str, config: Config) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    if config.contains(&Rule::OutdatedSpelling) {
        outdated_spelling(text, &mut diagnostics);
    }
    if config.contains(&Rule::AmbiguousChar) {
        ambiguous_char(text, &mut diagnostics);
    }
    if config.contains(&Rule::ForbiddenChar) {
        forbidden_char(text, &mut diagnostics);
    }
    diagnostics
}

pub fn check(text: &str, config: Config) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Raw replacements that need no tokenizing.
    diagnostics.extend(check_raw(text, config));

    let rules_requiring_doc: Vec<_> = config
        .iter()
        .copied()
        .filter(super::registry::Rule::requires_tokenizing)
        .collect();

    // Early exit if we do not need tokenizing.
    if rules_requiring_doc.is_empty() {
        return diagnostics;
    }

    let doc = tokenize(text);

    // Run the token-context-based rules.
    for token in &doc {
        // A match should be better.
        if token.is_whitespace() || token.is_punctuation() {
            // No rules at the moment concern these two
        } else if token.is_greek_word() {
            diagnostics.extend(check_token_with_context(token, &doc, &rules_requiring_doc));
        } else {
            // Does not use doc
            if rules_requiring_doc.contains(&Rule::MixedScripts) {
                mixed_scripts(token, &doc, &mut diagnostics);
            }
        }
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
/// TODO: replace \n with something less intrusive (cf. if the text is only "Χωρίς\n")
fn get_context_message(text: &str, range: &TextRange) -> String {
    let start = range.start();
    let end = range.end();

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

fn get_rich_context_message(text: &str, range: &TextRange, rule: Rule) -> String {
    let ctx = get_context_message(text, range);
    let fixable = if rule.has_fix() {
        format!("[{}]", "*".to_string().cyan())
    } else {
        "   ".to_string()
    };
    let message = format!("{:<3}: {} {}", format!("{rule}").cyan(), fixable, ctx);
    message
}

const MAX_ITERATIONS: usize = 100;

type Counter = HashMap<Rule, usize>;

/// Repeatedly fix text until stable.
//
// Should return result
//
// cf
// ruff_linter/src/linter.rs::lint_fix
// https://github.com/astral-sh/ruff/blob/main/crates/ruff_linter/src/linter.rs
//
// ruff_linter/src/fix/mod.rs
// https://github.com/astral-sh/ruff/blob/main/crates/ruff_linter/src/fix/mod.rs
//
// NOTE:
// * Should do statistics always, for safety // and it's cheap
// * Uses rules with no fixes. We should remove those from the config
//   since they are not printed nor, obviously, fixable.
pub fn fix(text: &str, config: Config) -> (String, Vec<String>, Counter) {
    let mut transformed = text.to_string();
    // For debugging. To remove eventually.
    #[allow(unused_mut)]
    let mut messages = Vec::new();
    let mut fixed = Counter::new();
    let mut iterations = 0;

    // These rules have no fixes: remove them from the config.
    // TODO: do this before reaching this function
    let rules_with_fixes = config
        .iter()
        .copied()
        .filter(super::registry::Rule::has_fix)
        .collect::<Vec<_>>();
    let config: Config = &rules_with_fixes;

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
        //         "At iteration {}. {} diagnostics. {} with_fixes. {} text length.",
        //         iterations,
        //         diagnostics.len().to_string().red().bold(),
        //         with_fixes.clone().count().to_string().red().bold(),
        //         transformed.len().to_string().red().bold(),
        //     )
        //     .italic()
        // );

        let rfixes = with_fixes
            .filter_map(|diagnostic| diagnostic.fix.as_ref().map(|fix| (diagnostic.kind, fix)))
            .sorted_by(|(rule1, fix1), (rule2, fix2)| cmp_fix(*rule1, *rule2, fix1, fix2));

        let mut first_fix = true;
        let mut transformed_this_iter = String::with_capacity(transformed.len());

        for (rule, fix) in rfixes {
            // May happen if we push some rules due to priority
            // We skip this fix.
            if let Some(last_pos) = last_pos
                && last_pos > fix.range.start()
            {
                eprintln!("Break due to disordered fixes");
                break;
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
            // May not be true if the text is composed of only one word: "Χωρίς\n"
            // assert!(last_pos < transformed.len());
            transformed_this_iter.push_str(&transformed[last_pos..]);
        }

        transformed = transformed_this_iter;

        iterations += 1;
        if iterations == MAX_ITERATIONS {
            eprintln!("Warning: exceeded maximum iterations in fix.");
            break;
        }
    }

    final_transformed.push_str(&transformed);

    (final_transformed, messages, fixed)
}

// https://github.com/astral-sh/ruff/blob/fc59e1b17f0a538a0150ea5a63de6305a8810c62/crates/ruff_linter/src/linter.rs#L382
pub fn lint_only(text: &str, config: Config) -> (Vec<String>, Counter) {
    let diagnostics = check(text, config);
    let mut statistics = Counter::new();
    let messages = diagnostics
        .iter()
        .map(|diagnostic| {
            *statistics.entry(diagnostic.kind).or_insert(0) += 1;
            get_rich_context_message(text, &diagnostic.range, diagnostic.kind)
        })
        .collect();

    (messages, statistics)
}
