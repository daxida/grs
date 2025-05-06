/// Macro to test a rule that requires tokenizing.
#[macro_export]
macro_rules! test_rule {
    ($name:ident, $rule_fn:expr, $text:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let text = $text;
            let doc = $crate::tokenizer::tokenize(text);
            let mut diagnostics = Vec::new();
            for token in &doc {
                // for token in crate::linter::doc_iter(&doc) {
                $rule_fn(&token, &doc, &mut diagnostics);
            }
            assert_eq!(diagnostics.is_empty(), $expected, "(text: {text})");
        }
    };
}

/// Macro to test a rule that DOES NOT require tokenizing.
#[macro_export]
macro_rules! test_rule_no_token {
    ($name:ident, $rule_fn:expr, $text:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let text = $text;
            let mut diagnostics = Vec::new();
            $rule_fn(&text, &mut diagnostics);
            assert_eq!(diagnostics.is_empty(), $expected, "(text: {text})");
        }
    };
}

/// Macro to test a rule fix.
#[macro_export]
macro_rules! test_fix {
    ($name:ident, $config:expr, $text:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let text = $text;
            let res = $crate::linter::fix(text, $config);
            let received = res.0;
            assert_eq!(received, $expected, "(text: {text})");
        }
    };
}
