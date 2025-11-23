// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::fs;
use std::path::PathBuf;

use lithos_gotmpl_engine::{FunctionRegistry, FunctionRegistryBuilder, Template};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct EngineCase {
    name: String,
    template: String,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    expected: Option<String>,
    #[serde(default)]
    error: Option<ExpectedError>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ExpectedError {
    Single(String),
    Multiple(Vec<String>),
}

impl ExpectedError {
    fn matches(&self, message: &str) -> bool {
        match self {
            ExpectedError::Single(pattern) => message.contains(pattern),
            ExpectedError::Multiple(patterns) =>
                patterns.iter().any(|pattern| message.contains(pattern)),
        }
    }

    fn description(&self) -> String {
        match self {
            ExpectedError::Single(pattern) => pattern.clone(),
            ExpectedError::Multiple(patterns) =>
                patterns.join("' or '"),
        }
    }
}

fn registry() -> FunctionRegistry {
    FunctionRegistryBuilder::new().build()
}

#[test]
fn engine_test_cases_align_with_go_semantics() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .expect("workspace root missing")
        .parent()
        .expect("workspace root missing");
    let path = root.join("test-cases/lithos-gotmpl-engine.json");
    let bytes = fs::read(&path).expect("missing engine test cases");
    let cases: Vec<EngineCase> = serde_json::from_slice(&bytes).expect("invalid engine test cases");

    for case in cases {
        let parse_result = Template::parse_with_functions(&case.name, &case.template, registry());

        let template = match parse_result {
            Ok(template) => template,
            Err(err) => {
                if let Some(expected_err) = case.error.as_ref() {
                    let msg = err.to_string();
                    assert!(
                        expected_err.matches(&msg),
                        "{} expected parse error containing '{}', got '{}'",
                        case.name,
                        expected_err.description(),
                        msg
                    );
                    continue;
                }

                panic!("parse {} failed: {}", case.name, err);
            }
        };

        if let Some(expected_err) = case.error.as_ref() {
            let result = template.render(&case.data);
            match result {
                Ok(output) => panic!(
                    "{} expected error '{}' but rendered '{}'",
                    case.name,
                    expected_err.description(),
                    output
                ),
                Err(err) => {
                    let msg = err.to_string();
                    assert!(
                        expected_err.matches(&msg),
                        "{} expected error containing '{}', got '{}'",
                        case.name,
                        expected_err.description(),
                        msg
                    );
                }
            }
            continue;
        }

        let rendered = template
            .render(&case.data)
            .unwrap_or_else(|err| panic!("render {} failed: {}", case.name, err));
        let expected = case.expected.unwrap_or_default();
        assert_eq!(rendered, expected, "case {} mismatch", case.name);
    }
}
