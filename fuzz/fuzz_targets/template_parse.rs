#![no_main]

use libfuzzer_sys::fuzz_target;
use lithos_gotmpl_engine::Template;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        let _ = Template::parse_str("fuzz-template", source);
    }
});
