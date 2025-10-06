use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
use colored::Colorize;
use itertools::Itertools;

use grs::cli::{Args, CheckCommand, Command};
use grs::linter::{fix, lint_only};
use grs::registry::Rule;
use grs::text_diff::CodeDiff;

#[derive(Copy, Clone)]
pub enum ExitStatus {
    Success,
    Failure,
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        match status {
            ExitStatus::Success => Self::from(0),
            ExitStatus::Failure => Self::from(1),
        }
    }
}

fn read_file(path: &PathBuf) -> Result<String, ExitStatus> {
    std::fs::read_to_string(path).map_err(|err| {
        eprintln!("Failed to read file {}: {err}", path.display());
        ExitStatus::Failure
    })
}

fn write_file(path: &PathBuf, content: &str) -> Result<(), ExitStatus> {
    std::fs::write(path, content).map_err(|err| {
        eprintln!("Failed to write to file {}: {err}", path.display());
        ExitStatus::Failure
    })
}

fn get_text_files(files: Vec<PathBuf>) -> Result<Vec<PathBuf>, ExitStatus> {
    let text_files = files
        .into_iter()
        .filter(|file| file.extension().and_then(|ext| ext.to_str()) == Some("txt"))
        .collect::<Vec<_>>();
    if text_files.is_empty() {
        Err(ExitStatus::Success)
    } else {
        Ok(text_files)
    }
}

fn time_it<T, F: FnOnce() -> T>(label: &str, f: F) -> T {
    let start = std::time::Instant::now();
    let result = f();
    println!("{}: {:.2?}", label, start.elapsed());
    result
}

fn run() -> Result<ExitStatus, ExitStatus> {
    let args = Args::parse();

    match args.command {
        Command::Check(check_args) => time_it("Execution time", || run_check_command(check_args)),
        Command::ToMonotonic { files } => {
            time_it("Execution time", || run_to_monotonic_command(files))
        }
        Command::GenerateCompletions { shell } => {
            // https://github.com/BurntSushi/ripgrep/blob/master/FAQ.md#complete
            // grs generate-completions fish > ~/.config/fish/completions/grs.fish
            let mut cmd = Args::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
            Ok(ExitStatus::Success)
        }
    }
}

fn run_to_monotonic_command(files: Vec<PathBuf>) -> Result<ExitStatus, ExitStatus> {
    let text_files = get_text_files(files)?;
    for file in &text_files {
        let text = read_file(file)?;
        let monotonic = grac::to_monotonic(&text);
        write_file(file, &monotonic)?;
    }
    println!("Successfully converted to monotonic.");
    Ok(ExitStatus::Success)
}

fn run_check_command(args: CheckCommand) -> Result<ExitStatus, ExitStatus> {
    let text_files = get_text_files(args.files)?;

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
        let text = read_file(file)?;

        let statistics_counter = if args.diff {
            let (fixed, _messages, statistics_counter) = fix(&text, &config);
            // I dont know how to remove colors
            let text_diff = CodeDiff::new(&text, &fixed);
            println!("{text_diff}");
            statistics_counter
        } else if args.fix {
            let (fixed, _messages, statistics_counter) = fix(&text, &config);
            write_file(file, &fixed)?;
            statistics_counter
        } else {
            let (messages, statistics_counter) = lint_only(&text, &config);
            if !args.statistics && !messages.is_empty() {
                // Header
                // println!("{}", file.to_str().unwrap().purple());
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

fn main() -> ExitCode {
    run().unwrap_or_else(Into::into).into()
}
