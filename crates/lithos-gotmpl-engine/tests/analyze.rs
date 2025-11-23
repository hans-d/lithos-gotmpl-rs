// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_engine::ControlKind;
use lithos_gotmpl_engine::{FunctionRegistryBuilder, Template};
use serde_json::Value;

#[test]
fn analysis_reports_variables_and_functions() {
    let mut builder = FunctionRegistryBuilder::new();
    builder.register("printf", |_ctx, _args| Ok(Value::Null));
    builder.register("default", |_ctx, _args| Ok(Value::Null));
    builder.register("upper", |_ctx, _args| Ok(Value::Null));
    let registry = builder.build();

    let tmpl = Template::parse_with_functions(
        "analysis",
        r#"{{ default "friend" .user.name | printf "%s" | upper }}"#,
        registry,
    )
    .unwrap();

    let report = tmpl.analyze();
    assert!(matches!(
        report.precision,
        lithos_gotmpl_engine::Precision::Precise
    ));
    assert!(!report.has_template_invocation);

    let var_paths: Vec<_> = report.variables.iter().map(|v| v.path.as_str()).collect();
    assert!(var_paths.contains(&".user.name"));

    let fn_names: Vec<_> = report.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(fn_names.contains(&"printf"));
    assert!(fn_names.contains(&"default"));
    assert!(fn_names.contains(&"upper"));
    assert!(report.unknown_functions.is_empty());
}

#[test]
fn analysis_marks_dynamic_template() {
    let tmpl = Template::parse_str("tmpl", r#"{{ template .name . }}"#).unwrap();
    let report = tmpl.analyze();
    assert!(report.has_template_invocation);
    assert!(matches!(
        report.precision,
        lithos_gotmpl_engine::Precision::Conservative
    ));
    assert!(!report.templates.is_empty());
}

#[test]
fn analysis_reports_control_structures() {
    let tmpl = Template::parse_str(
        "controls",
        "{{if .flag}}{{with .user}}{{.name}}{{end}}{{else}}{{range .items}}{{.}}{{end}}{{end}}",
    )
    .unwrap();
    let report = tmpl.analyze();

    let kinds: Vec<_> = report.controls.iter().map(|c| c.kind).collect();
    assert!(kinds.contains(&ControlKind::If));
    assert!(kinds.contains(&ControlKind::With));
    assert!(kinds.contains(&ControlKind::Range));
}

#[test]
fn analysis_collects_unknown_functions() {
    let tmpl =
        Template::parse_str("unknown-fn", "{{ customFunc .value }}").expect("parse template");
    let report = tmpl.analyze();
    assert_eq!(report.functions.len(), 1);
    assert_eq!(report.unknown_functions.len(), 1);
    assert_eq!(report.unknown_functions[0].name, "customFunc");
    assert!(matches!(
        report.unknown_functions[0].source,
        lithos_gotmpl_engine::FunctionSource::Unknown
    ));
}

#[test]
fn analysis_reports_else_if_functions() {
    let mut builder = FunctionRegistryBuilder::new();
    builder.register("helper", |_ctx, _args| Ok(Value::Bool(true)));
    let registry = builder.build();

    let tmpl = Template::parse_with_functions(
        "else-if",
        "{{ if .primary }}A{{ else if helper .secondary }}B{{ else }}C{{ end }}",
        registry,
    )
    .expect("parse template");

    let report = tmpl.analyze();
    let helper_calls: Vec<_> = report
        .functions
        .iter()
        .filter(|call| call.name == "helper")
        .collect();
    assert_eq!(helper_calls.len(), 1);
    assert!(report.unknown_functions.is_empty());
}

#[test]
fn parser_rejects_else_without_if() {
    let err = Template::parse_str("else-with", "{{ if true }}A{{ else with . }}B{{ end }}")
        .expect_err("parse should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid else-if: expected 'if' after 'else'"),
        "unexpected parse error: {}",
        msg
    );
}
