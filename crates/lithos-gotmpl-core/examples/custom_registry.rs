// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_core::{install_text_template_functions, FunctionRegistryBuilder, Template};
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start from the stock Go helpers.
    let mut builder = FunctionRegistryBuilder::new();
    install_text_template_functions(&mut builder);

    // Register a simple custom helper.
    builder.register("shout", |_ctx, args| {
        let input = args.first().and_then(|value| value.as_str()).unwrap_or("");
        Ok(Value::String(format!("{}!", input.to_uppercase())))
    });

    let registry = builder.build();
    let template = Template::parse_with_functions("custom", "{{shout .phrase}}", registry)?;
    let output = template.render(&json!({"phrase": "hello core"}))?;

    println!("{}", output);
    assert_eq!(output, "HELLO CORE!");
    Ok(())
}
