// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct EngineCase {
    name: String,
    #[serde(default)]
    expected: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoSanityCase {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    rendered: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[test]
fn go_reference_confirms_engine_cases() {
    if Command::new("go").arg("version").output().is_err() {
        eprintln!("skipping go reference check because `go` is not available");
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("missing crates directory")
        .parent()
        .expect("missing workspace root");

    let cases_path = workspace_root.join("test-cases/lithos-gotmpl-engine.json");
    let bytes = fs::read(&cases_path).expect("missing engine test cases");
    let engine_cases: Vec<EngineCase> =
        serde_json::from_slice(&bytes).expect("invalid engine test cases");

    let runner_dir = workspace_root.join("go-sanity");
    let go_cache = workspace_root.join("target/go-cache");
    let _ = fs::create_dir_all(&go_cache);

    let output = Command::new("go")
        .arg("run")
        .arg(".")
        .arg("-cases")
        .arg(&cases_path)
        .current_dir(&runner_dir)
        .env("GOCACHE", &go_cache)
        .output()
        .expect("failed to invoke go-sanity runner");

    assert!(
        output.status.success(),
        "go-sanity execution failed: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let go_cases: Vec<GoSanityCase> =
        serde_json::from_slice(&output.stdout).expect("failed to parse go-sanity output");

    let go_map: HashMap<_, _> = go_cases
        .into_iter()
        .map(|case| {
            let name = case.name.clone().unwrap_or_else(|| "<unnamed>".to_string());
            (name, case)
        })
        .collect();

    for case in engine_cases {
        let go_case = go_map
            .get(&case.name)
            .unwrap_or_else(|| panic!("missing go reference result for {}", case.name));

        match (case.expected.as_ref(), case.error.as_ref()) {
            (Some(expected), None) => {
                let rendered = go_case
                    .rendered
                    .as_ref()
                    .unwrap_or_else(|| panic!("go did not render {}", case.name));
                assert_eq!(rendered, expected, "go output mismatch for {}", case.name);
                assert!(
                    go_case.error.is_none(),
                    "go reported unexpected error for {}: {:?}",
                    case.name,
                    go_case.error
                );
            }
            (None, Some(_)) => {
                let err = go_case
                    .error
                    .as_ref()
                    .unwrap_or_else(|| panic!("go did not error for {}", case.name));
                assert!(
                    err.contains("unexpected <with>"),
                    "go error for {} did not mention unexpected <with>: {}",
                    case.name,
                    err
                );
            }
            _ => panic!(
                "case {} must have either expected output or error",
                case.name
            ),
        }
    }
}
