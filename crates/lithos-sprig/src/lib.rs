// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Lithos Sprig provides a partial re-implementation of selected helpers from
//! the `sprig` Go template library, tailored for use with Rust-based Go template
//! interpreters such as `gitmpl`.

use lithos_gotmpl_core::{
    install_text_template_functions, FunctionRegistry, FunctionRegistryBuilder,
};

mod functions;

/// Registers the sprig helpers into an existing function registry builder.
pub fn install_sprig_functions(builder: &mut FunctionRegistryBuilder) {
    functions::install_all(builder);
}

/// Returns a registry populated with the Go core helpers plus sprig extensions.
pub fn sprig_functions() -> FunctionRegistry {
    let mut builder = FunctionRegistryBuilder::new();
    install_text_template_functions(&mut builder);
    install_sprig_functions(&mut builder);
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lithos_gotmpl_core::Template;
    use serde_json::json;

    #[test]
    fn template_with_sprig_helpers() {
        let registry = sprig_functions();
        let template = Template::parse_with_functions(
            "sprig",
            "{{default \"friend\" .name | upper}}",
            registry,
        )
        .unwrap();
        let rendered = template.render(&json!({"name": "sprig"})).unwrap();
        assert_eq!(rendered, "SPRIG");
    }
}
