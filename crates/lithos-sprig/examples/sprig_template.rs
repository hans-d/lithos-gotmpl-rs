// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_core::{install_text_template_functions, FunctionRegistryBuilder, Template};
use lithos_sprig::install_sprig_functions;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = FunctionRegistryBuilder::new();
    install_text_template_functions(&mut builder);
    install_sprig_functions(&mut builder);
    let registry = builder.build();

    let template = Template::parse_with_functions(
        "sprig",
        "{{ default \"friend\" .name | upper }}",
        registry,
    )?;

    let rendered = template.render(&json!({"name": "sprig"}))?;
    println!("{}", rendered);
    Ok(())
}
