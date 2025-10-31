// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::fs;
use std::path::PathBuf;

use lithos_gotmpl_core::{text_template_functions, Template};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    template: String,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    expected: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[test]
fn test_cases_render_like_go_reference() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .expect("workspace root missing")
        .parent()
        .expect("workspace root missing again");
    let path = root.join("test-cases/lithos-gotmpl-core.json");
    let bytes = fs::read(&path).expect("test cases file missing");
    let cases: Vec<Fixture> = serde_json::from_slice(&bytes).expect("invalid test cases json");

    for case in cases {
        let functions = text_template_functions();
        let template = Template::parse_with_functions(&case.name, &case.template, functions)
            .unwrap_or_else(|err| panic!("parse {} failed: {}", case.name, err));

        match case.error {
            Some(expected_error) => {
                let result = template.render(&case.data);
                match result {
                    Ok(output) => panic!(
                        "{} expected error '{}' but rendered '{}'",
                        case.name, expected_error, output
                    ),
                    Err(err) => {
                        let err_text = err.to_string();
                        assert!(
                            err_text.contains(&expected_error),
                            "{} expected error containing '{}', got '{}'",
                            case.name,
                            expected_error,
                            err_text
                        );
                    }
                }
            }
            None => {
                let output = template
                    .render(&case.data)
                    .unwrap_or_else(|err| panic!("render {} failed: {}", case.name, err));

                let expected = case.expected.unwrap_or_default();
                assert_eq!(
                    output, expected,
                    "fixture {} rendered incorrectly",
                    case.name
                );
            }
        }
    }
}
