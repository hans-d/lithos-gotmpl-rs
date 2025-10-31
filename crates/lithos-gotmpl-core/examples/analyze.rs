// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_core::{install_text_template_functions, FunctionRegistryBuilder, Template};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = FunctionRegistryBuilder::new();
    install_text_template_functions(&mut builder);
    let registry = builder.build();

    let template = Template::parse_with_functions(
        "example",
        "{{ default \"friend\" .user.name | printf \"Hello, %s!\" }}",
        registry,
    )?;

    let analysis = template.analyze();

    println!("analysis precision: {:?}", analysis.precision);
    for var in &analysis.variables {
        println!(
            "variable {} @ {:?} (kind={:?}, certainty={:?})",
            var.path, var.span, var.kind, var.certainty
        );
    }
    for func in &analysis.functions {
        println!(
            "function {} @ {:?} ({:?})",
            func.name, func.span, func.source
        );
    }

    let rendered = template.render(&json!({"user": {"name": "Hydros"}}))?;
    println!("rendered: {rendered}");
    Ok(())
}
