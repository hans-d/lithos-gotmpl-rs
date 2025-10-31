// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use lithos_gotmpl_core::Template;
use lithos_sprig::sprig_functions;
use serde::Deserialize;
use serde_json::{json, Value};
use tempfile::NamedTempFile;

#[derive(Debug, Deserialize)]
struct GoSanityCase {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    function: Option<String>,
    #[serde(default)]
    args: Option<Vec<Value>>,
    #[serde(default)]
    output: Option<Value>,
    #[serde(default)]
    rendered: Option<String>,
    #[serde(default)]
    expected: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[test]
fn go_sanity_matches_sprig_examples() {
    if Command::new("go").arg("version").output().is_err() {
        eprintln!("skipping go-sanity sprig check because `go` was not found in PATH");
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("missing crates directory")
        .parent()
        .expect("missing workspace root");
    let runner_dir = workspace_root.join("go-sanity");
    let cases_path = manifest_dir
        .parent()
        .expect("missing crates directory")
        .parent()
        .expect("missing workspace root")
        .join("test-cases/lithos-sprig.json");

    let output = Command::new("go")
        .arg("run")
        .arg(".")
        .arg("-cases")
        .arg(&cases_path)
        .current_dir(&runner_dir)
        .output()
        .expect("failed to invoke go-sanity runner");

    assert!(
        output.status.success(),
        "go-sanity execution failed: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let cases: Vec<GoSanityCase> =
        serde_json::from_slice(&output.stdout).expect("failed to parse go-sanity output");

    let registry = sprig_functions();

    for case in cases {
        let label = case
            .name
            .clone()
            .or_else(|| case.function.clone())
            .unwrap_or_else(|| "<anonymous>".to_string());

        if let Some(err) = case.error {
            panic!("go-sanity case {} returned an error: {}", label, err);
        }

        if let (Some(rendered), Some(expected)) = (case.rendered.as_ref(), case.expected.as_ref()) {
            assert_eq!(rendered, expected, "template mismatch for case {}", label);
        }

        let Some(function) = case.function.as_ref() else {
            continue;
        };

        let args = case
            .args
            .as_ref()
            .unwrap_or_else(|| panic!("go-sanity case {} missing args", label));
        let expected = case
            .output
            .as_ref()
            .unwrap_or_else(|| panic!("go-sanity case {} missing function output", label));

        let mut template = format!("{{{{ {}", function);
        let mut data = serde_json::Map::new();
        for (idx, value) in args.iter().enumerate() {
            let key = format!("arg{}", idx);
            template.push(' ');
            template.push_str(&format!(".{}", key));
            data.insert(key, value.clone());
        }
        template.push_str(" }}");

        let template = Template::parse_with_functions(&label, &template, registry.clone())
            .unwrap_or_else(|e| panic!("parse failed for {}: {}", label, e));
        let rendered = template
            .render(&Value::Object(data))
            .unwrap_or_else(|e| panic!("render failed for {}: {}", label, e));

        let actual_value = match expected {
            Value::Bool(_) => Value::Bool(match rendered.as_str() {
                "true" => true,
                "false" => false,
                other => panic!("unexpected bool literal {other} for {label}"),
            }),
            Value::Number(_) => {
                if let Ok(i) = rendered.parse::<i64>() {
                    Value::Number(serde_json::Number::from(i))
                } else if let Ok(u) = rendered.parse::<u64>() {
                    Value::Number(serde_json::Number::from(u))
                } else if let Ok(f) = rendered.parse::<f64>() {
                    Value::Number(serde_json::Number::from_f64(f).expect("invalid float"))
                } else {
                    Value::String(rendered.clone())
                }
            }
            Value::Array(_) | Value::Object(_) => match serde_json::from_str::<Value>(&rendered) {
                Ok(parsed) => parsed,
                Err(_) => Value::String(rendered.clone()),
            },
            _ => Value::String(rendered.clone()),
        };

        if matches!(function.as_str(), "keys" | "values") {
            assert_json_multiset_eq(&actual_value, expected, &label);
        } else {
            assert_eq!(actual_value, *expected, "mismatch for function {}", label);
        }
    }

    verify_directory_cases(&registry, runner_dir.as_path(), workspace_root);
}

fn verify_directory_cases(
    registry: &lithos_gotmpl_engine::FunctionRegistry,
    runner_dir: &Path,
    workspace_root: &Path,
) {
    let dir_root = workspace_root.join("test-cases/sprig");
    if !dir_root.exists() {
        return;
    }

    for entry in fs::read_dir(&dir_root).expect("read sprig test-cases directory") {
        let entry = entry.expect("read sprig test case entry");
        let file_type = entry.file_type().expect("fetch entry type");
        if !file_type.is_dir() {
            continue;
        }

        let case_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        let template_src = fs::read_to_string(case_path.join("input.tmpl"))
            .unwrap_or_else(|err| panic!("{name}: failed to read input.tmpl: {err}"));

        let data_path = case_path.join("input.json");
        let data_value = if data_path.exists() {
            let raw = fs::read_to_string(&data_path)
                .unwrap_or_else(|err| panic!("{name}: failed to read input.json: {err}"));
            if raw.trim().is_empty() {
                Value::Null
            } else {
                serde_json::from_str(&raw)
                    .unwrap_or_else(|err| panic!("{name}: invalid input.json: {err}"))
            }
        } else {
            Value::Null
        };

        let expected_text = fs::read_to_string(case_path.join("expected.txt"))
            .unwrap_or_else(|err| panic!("{name}: failed to read expected.txt: {err}"));

        let template = Template::parse_with_functions(&name, &template_src, registry.clone())
            .unwrap_or_else(|err| panic!("{name}: parse failed: {err}"));
        let rendered = template
            .render(&data_value)
            .unwrap_or_else(|err| panic!("{name}: render failed: {err}"));

        assert_eq!(rendered, expected_text, "{name}: template output mismatch");

        let case_json = json!([{
            "name": name.clone(),
            "template": template_src,
            "data": data_value,
            "expected": expected_text.clone(),
        }]);

        let mut temp = NamedTempFile::new().expect("create temp test-cases file");
        serde_json::to_writer(&mut temp, &case_json).expect("write temp test case");
        temp.flush().expect("flush temp test case");

        let output = Command::new("go")
            .arg("run")
            .arg(".")
            .arg("-cases")
            .arg(temp.path())
            .current_dir(runner_dir)
            .output()
            .expect("failed to invoke go-sanity runner for directory case");

        assert!(
            output.status.success(),
            "go-sanity execution failed for {}: {}\nstdout: {}\nstderr: {}",
            case_path.display(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let cases: Vec<GoSanityCase> =
            serde_json::from_slice(&output.stdout).expect("failed to parse go-sanity output");
        assert_eq!(cases.len(), 1, "{}: expected single go case", name);
        let go_case = &cases[0];

        if let Some(err) = &go_case.error {
            panic!("go-sanity reported error for {}: {}", name, err);
        }

        if let Some(rendered) = &go_case.rendered {
            assert_eq!(rendered, &expected_text, "{}: go template mismatch", name);
        } else {
            panic!("go-sanity did not return rendered output for {}", name);
        }
    }
}

fn assert_json_multiset_eq(actual: &Value, expected: &Value, label: &str) {
    match (actual, expected) {
        (Value::Array(actual_arr), Value::Array(expected_arr)) => {
            let mut actual_norm = canonical_json_vec(actual_arr);
            let mut expected_norm = canonical_json_vec(expected_arr);
            actual_norm.sort();
            expected_norm.sort();
            assert_eq!(
                actual_norm, expected_norm,
                "unordered mismatch for {}",
                label
            );
        }
        _ => panic!("{label} expected arrays for unordered comparison"),
    }
}

fn canonical_json_vec(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| format!("{:?}", v)))
        .collect()
}

#[test]
fn multiset_comparison_allows_unordered_arrays() {
    let actual = json!(["b", "a"]);
    let expected = json!(["a", "b"]);
    assert_json_multiset_eq(&actual, &expected, "keys-basic");
}

#[test]
fn multiset_comparison_detects_different_lengths() {
    let actual = json!(["a"]);
    let expected = json!(["a", "b"]);
    let result = std::panic::catch_unwind(|| {
        assert_json_multiset_eq(&actual, &expected, "keys-basic");
    });
    assert!(
        result.is_err(),
        "expected comparison to fail for mismatched elements"
    );
}
