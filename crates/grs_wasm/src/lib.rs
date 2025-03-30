use grs::diagnostic::{Diagnostic, Fix};
use grs::registry::{code_to_rule, Rule};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::js_sys::Error;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &'static str = r#"
export interface Diagnostic {
  kind: string;
  range: {
    start: number;
    end: number;
  };
  fix: string;
};

export interface Token {
  text: string,
  whitespace: string,
  index: number,
  range: {
    start: number,
    end: number,
  }
  punct: boolean,
  greek: boolean,
}"#;

// TODO: Very unefficient
fn byte_range_to_char_range(text: &str, byte_range: Range<usize>) -> Range<usize> {
    let mut char_start = None;
    let mut char_end = None;

    for (idx, (char_index, _)) in text.char_indices().enumerate() {
        if char_index == byte_range.start {
            char_start = Some(idx);
        }
        if char_index == byte_range.end {
            char_end = Some(idx);
            break;
        }
    }

    // Cf. only a multisyllable non accented as input: καλη
    // (in that case we would reach EOF before assigning char_end)
    if char_end.is_none() {
        char_end = Some(text.chars().count());
    }

    match (char_start, char_end) {
        (Some(start), Some(end)) => start..end,
        _ => {
            web_sys::console::log_1(&"Warning: Invalid range, using default 0..0".into());
            0..0
        }
    }
}

fn to_fixjs(fix: &Option<Fix>) -> Option<String> {
    // The trim is hacky...
    fix.as_ref().map(|fix| fix.replacement.trim().to_string())
}

fn pascal_to_snake(s: &str) -> String {
    let mut snake_case = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                snake_case.push('_');
            }
            snake_case.push(c.to_ascii_lowercase());
        } else {
            snake_case.push(c);
        }
    }
    snake_case
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiagnosticJs {
    pub kind: String,
    pub range: Range<usize>,
    // Only the replacement at the moment.
    pub fix: Option<String>,
}

impl DiagnosticJs {
    fn new(text: &str, diagnostic: &Diagnostic) -> Self {
        let byte_range = diagnostic.range.start()..diagnostic.range.end();
        let char_range = byte_range_to_char_range(text, byte_range);
        let start = char_range.start;
        let end = char_range.end;

        let pascal_case_rule = format!("{:?}", diagnostic.kind);
        let kind = pascal_to_snake(&pascal_case_rule);

        Self {
            kind,
            range: start..end,
            fix: to_fixjs(&diagnostic.fix),
        }
    }
}

const ALL_RULES: [Rule; 10] = [
    Rule::MissingDoubleAccents,
    Rule::MissingAccentCapital,
    Rule::DuplicatedWord,
    Rule::AddFinalN,
    Rule::RemoveFinalN,
    Rule::OutdatedSpelling,
    Rule::MonosyllableAccented,
    Rule::MultisyllableNotAccented,
    Rule::MixedScripts,
    Rule::AmbiguousChar,
];

// Proxy of Config (i.e. Vec<Rule>) to be passed between rust and js.
#[derive(Serialize, Deserialize)]
struct Options {
    rules: Vec<String>,
}

fn load_config(options: JsValue) -> Vec<Rule> {
    if options.is_null() || options.is_undefined() {
        ALL_RULES.to_vec()
    } else {
        // options is expected to be: { "MDA": true, "AC": true, ... }
        let options_map: HashMap<String, bool> =
            serde_wasm_bindgen::from_value(options).unwrap_or_default();

        // Extract keys where value is true
        let codes: Vec<String> = options_map
            .into_iter()
            .filter_map(|(key, value)| if value { Some(key) } else { None })
            .collect();

        // Ignore keys that do not match any rule
        codes.iter().filter_map(|code| code_to_rule(code)).collect()
    }
}

// Reference:
// https://rustwasm.github.io/docs/wasm-bindgen/reference/arbitrary-data-with-serde.html
#[wasm_bindgen]
pub fn scan_text(text: &str, options: JsValue) -> Result<JsValue, Error> {
    let config = load_config(options);
    let diagnostics_js = grs::linter::check(text, &config.as_slice())
        .iter()
        .map(|diagnostic| DiagnosticJs::new(text, diagnostic))
        .collect::<Vec<_>>();
    serde_wasm_bindgen::to_value(&diagnostics_js).map_err(into_error)
}

#[wasm_bindgen]
pub fn fix(text: &str, options: JsValue) -> String {
    let config = load_config(options);
    let (res, _, _) = grs::linter::fix(text, &config);
    res
}

#[wasm_bindgen]
pub fn tokenize(text: &str) -> Result<JsValue, Error> {
    let tokens = grs::tokenizer::tokenize(text);
    serde_wasm_bindgen::to_value(&tokens).map_err(into_error)
}

// Grac bindings

#[wasm_bindgen]
pub fn to_monotonic(text: &str) -> String {
    grac::to_monotonic(text)
}

#[wasm_bindgen]
pub fn syllabify(text: &str, separator: &str) -> String {
    grac::syllabify_el(text).join(separator)
}

fn into_error<E: std::fmt::Display>(err: E) -> Error {
    Error::new(&err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_conversion() {
        assert_eq!(0..4, byte_range_to_char_range("Καλημέρα", 0..8));
    }
}
