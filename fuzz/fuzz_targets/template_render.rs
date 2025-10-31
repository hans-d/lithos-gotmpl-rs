#![no_main]

use libfuzzer_sys::fuzz_target;
use lithos_gotmpl_core::{
    FunctionRegistryBuilder, Template, install_text_template_functions,
};
use lithos_gotmpl_engine::FunctionRegistry;
use lithos_sprig::install_sprig_functions;
use once_cell::sync::Lazy;
use serde_json::Value;

static REGISTRY: Lazy<FunctionRegistry> = Lazy::new(|| {
    let mut builder = FunctionRegistryBuilder::new();
    install_text_template_functions(&mut builder);
    install_sprig_functions(&mut builder);
    builder.build()
});

fuzz_target!(|data: &[u8]| {
    let source = match std::str::from_utf8(data) {
        Ok(src) => src,
        Err(_) => return,
    };

    if let Ok(template) =
        Template::parse_with_functions("fuzz-template-render", source, REGISTRY.clone())
    {
        let _ = template.render(&Value::Null);
    }
});
