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
