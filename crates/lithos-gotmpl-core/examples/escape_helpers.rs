// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Demonstrates html/js/urlquery/print helpers following Go's text/template docs:
//! https://pkg.go.dev/text/template#hdr-Text_and_spaces

use lithos_gotmpl_core::{text_template_functions, Template};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html_registry = text_template_functions();
    let html =
        Template::parse_with_functions("html", r#"{{html "<b>\"Bob\"</b>"}}"#, html_registry)?;
    let html_out = html.render(&json!({}))?;
    println!("html => {}", html_out);

    let js_registry = text_template_functions();
    let js = Template::parse_with_functions("js", r#"{{js "</script>"}}"#, js_registry)?;
    let js_out = js.render(&json!({}))?;
    println!("js => {}", js_out);

    let url_registry = text_template_functions();
    let url = Template::parse_with_functions(
        "urlquery",
        r#"{{urlquery "Hello, world!"}}"#,
        url_registry,
    )?;
    let url_out = url.render(&json!({}))?;
    println!("urlquery => {}", url_out);

    let print_registry = text_template_functions();
    let print_tmpl =
        Template::parse_with_functions("print", r#"{{print "Hello" 23}}"#, print_registry.clone())?;
    let println_tmpl =
        Template::parse_with_functions("println", r#"{{println "Hello" 23}}"#, print_registry)?;
    let print_out = print_tmpl.render(&json!({}))?;
    let println_out = println_tmpl.render(&json!({}))?;
    println!("print => {}", print_out);
    println!("println => {:?}", println_out);

    let len_registry = text_template_functions();
    let len_tmpl = Template::parse_with_functions("len", r#"{{len .items}}"#, len_registry)?;
    let len_out = len_tmpl.render(&json!({"items": [1, 2, 3]}))?;
    println!("len => {}", len_out);

    let slice_registry = text_template_functions();
    let slice_tmpl =
        Template::parse_with_functions("slice", r#"{{slice "gopher" "1" "3"}}"#, slice_registry)?;
    let slice_out = slice_tmpl.render(&json!({}))?;
    println!("slice => {}", slice_out);

    let call_registry = text_template_functions();
    let call_tmpl =
        Template::parse_with_functions("call", r#"{{call "print" "world"}}"#, call_registry)?;
    let call_out = call_tmpl.render(&json!({}))?;
    println!("call => {}", call_out);

    Ok(())
}
