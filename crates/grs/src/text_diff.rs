use colored::Colorize;
use similar::{ChangeTag, TextDiff};

// From ruff CodeDiff
pub struct CodeDiff<'a> {
    diff: TextDiff<'a, 'a, 'a, str>,
    header: Option<(&'a str, &'a str)>,
    missing_newline_hint: bool,
}

impl<'a> CodeDiff<'a> {
    pub fn new(original: &'a str, modified: &'a str) -> Self {
        let diff = TextDiff::from_words(original, modified);
        Self {
            diff,
            header: None,
            missing_newline_hint: true,
        }
    }

    #[allow(dead_code)]
    const fn header(&mut self, original: &'a str, modified: &'a str) {
        self.header = Some((original, modified));
    }
}

impl std::fmt::Display for CodeDiff<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((original, modified)) = self.header {
            writeln!(f, "--- {}", original.red())?;
            writeln!(f, "+++ {}", modified.green())?;
        }

        let mut unified = self.diff.unified_diff();
        unified.missing_newline_hint(self.missing_newline_hint);

        // Individual hunks (section of changes)
        for hunk in unified.iter_hunks() {
            // writeln!(f, "{}", hunk.header())?;

            // individual lines
            for change in hunk.iter_changes() {
                let value = change.value();
                match change.tag() {
                    ChangeTag::Equal => (), // write!(f, " {value}")?,
                    ChangeTag::Delete => writeln!(f, "{}{}", "-".red(), value.red())?,
                    ChangeTag::Insert => writeln!(f, "{}{}", "+".green(), value.green())?,
                }
            }
        }

        Ok(())
    }
}
