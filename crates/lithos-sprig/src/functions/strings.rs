// SPDX-License-Identifier: Apache-2.0 OR MIT
use heck::{ToKebabCase, ToSnakeCase, ToUpperCamelCase};
use lithos_gotmpl_engine::{Error, EvalContext};
use serde_json::{json, Value};

use super::{
    clamp_char_range, expect_exact_args, expect_min_args, expect_string, expect_usize,
    value_to_string,
};

type StringFunction = fn(&mut EvalContext, &[Value]) -> Result<Value, Error>;

const CASE_CONVERSION_FUNCS: &[(&str, StringFunction)] = &[
    ("upper", upper),
    ("lower", lower),
    ("title", title),
    ("snakecase", snakecase),
    ("camelcase", camelcase),
    ("kebabcase", kebabcase),
    ("swapcase", swapcase),
];

const TRIMMING_FUNCS: &[(&str, StringFunction)] = &[
    ("trim", trim),
    ("trimAll", trim_all),
    ("trimPrefix", trim_prefix),
    ("trimSuffix", trim_suffix),
    ("hasPrefix", has_prefix),
    ("hasSuffix", has_suffix),
];

const SEARCH_FUNCS: &[(&str, StringFunction)] = &[
    ("contains", contains),
    ("replace", replace),
    ("substr", substr),
    ("trunc", trunc),
];

const FORMATTING_FUNCS: &[(&str, StringFunction)] = &[
    ("wrap", wrap),
    ("indent", indent),
    ("nindent", nindent),
    ("nospace", nospace),
    ("repeat", repeat),
    ("cat", cat),
    ("quote", quote),
    ("squote", squote),
];

pub fn register(builder: &mut lithos_gotmpl_engine::FunctionRegistryBuilder) {
    for &(name, func) in CASE_CONVERSION_FUNCS
        .iter()
        .chain(TRIMMING_FUNCS.iter())
        .chain(SEARCH_FUNCS.iter())
        .chain(FORMATTING_FUNCS.iter())
    {
        builder.register(name, func);
    }
}

fn title_case(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for (idx, word) in input.split_whitespace().enumerate() {
        if idx > 0 {
            result.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.push_str(&chars.collect::<String>().to_lowercase());
        }
    }
    result
}

fn strip_prefix(input: &str, prefix: &str) -> String {
    if input.starts_with(prefix) {
        input[prefix.len()..].to_string()
    } else {
        input.to_string()
    }
}

fn strip_suffix(input: &str, suffix: &str) -> String {
    if input.ends_with(suffix) {
        let end = input.len() - suffix.len();
        input[..end].to_string()
    } else {
        input.to_string()
    }
}

fn wrap_text(width: usize, text: &str) -> String {
    if width == 0 {
        return text.to_string();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let current_len = current.chars().count();
        if current.is_empty() {
            current.push_str(word);
        } else if current_len + 1 + word_len <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines.join("\n")
}

fn indent_text(spaces: usize, input: &str) -> String {
    let pad = " ".repeat(spaces);
    input
        .lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_whitespace(input: &str) -> String {
    input.chars().filter(|c| !c.is_whitespace()).collect()
}

fn truncate_chars(text: &str, length: usize) -> String {
    text.chars().take(length).collect()
}

fn render_non_null(args: &[Value], mut render: impl FnMut(&str) -> String) -> String {
    args.iter()
        .filter(|value| !value.is_null())
        .map(|value| render(&value_to_string(value)))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn upper(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("upper", args, 1)?;
    let s = expect_string("upper", &args[0], 1)?;
    Ok(json!(s.to_uppercase()))
}

pub fn lower(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("lower", args, 1)?;
    let s = expect_string("lower", &args[0], 1)?;
    Ok(json!(s.to_lowercase()))
}

pub fn title(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("title", args, 1)?;
    let input = expect_string("title", &args[0], 1)?;
    Ok(json!(title_case(&input)))
}

pub fn trim(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("trim", args, 1)?;
    let input = expect_string("trim", &args[0], 1)?;
    Ok(json!(input.trim()))
}

pub fn trim_all(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("trimAll", args, 2)?;
    let cutset = expect_string("trimAll", &args[0], 1)?;
    let input = expect_string("trimAll", &args[1], 2)?;
    Ok(json!(input.trim_matches(|c| cutset.contains(c))))
}

pub fn trim_prefix(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("trimPrefix", args, 2)?;
    let prefix = expect_string("trimPrefix", &args[0], 1)?;
    let input = expect_string("trimPrefix", &args[1], 2)?;
    Ok(json!(strip_prefix(&input, &prefix)))
}

pub fn trim_suffix(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("trimSuffix", args, 2)?;
    let suffix = expect_string("trimSuffix", &args[0], 1)?;
    let input = expect_string("trimSuffix", &args[1], 2)?;
    Ok(json!(strip_suffix(&input, &suffix)))
}

pub fn has_prefix(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("hasPrefix", args, 2)?;
    let prefix = expect_string("hasPrefix", &args[0], 1)?;
    let input = expect_string("hasPrefix", &args[1], 2)?;
    Ok(Value::Bool(input.starts_with(&prefix)))
}

pub fn has_suffix(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("hasSuffix", args, 2)?;
    let suffix = expect_string("hasSuffix", &args[0], 1)?;
    let input = expect_string("hasSuffix", &args[1], 2)?;
    Ok(Value::Bool(input.ends_with(&suffix)))
}

pub fn contains(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("contains", args, 2)?;
    let needle = expect_string("contains", &args[0], 1)?;
    let haystack = expect_string("contains", &args[1], 2)?;
    Ok(Value::Bool(haystack.contains(&needle)))
}

pub fn replace(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("replace", args, 3)?;
    let old = expect_string("replace", &args[0], 1)?;
    let new = expect_string("replace", &args[1], 2)?;
    let text = expect_string("replace", &args[2], 3)?;
    let replaced = if args.len() > 3 {
        let count = expect_usize("replace", &args[3], 4)?;
        text.replacen(&old, &new, count)
    } else {
        text.replace(&old, &new)
    };
    Ok(Value::String(replaced))
}

pub fn substr(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("substr", args, 2)?;
    let start_chars = expect_usize("substr", &args[0], 1)?;
    let (len_chars, string_idx) = if args.len() == 3 {
        let end = expect_usize("substr", &args[1], 2)?;
        (Some(end.saturating_sub(start_chars)), 2)
    } else {
        (None, 1)
    };
    let input = expect_string("substr", &args[string_idx], string_idx + 1)?;
    let (start_idx, end_idx) = clamp_char_range(&input, start_chars, len_chars);
    Ok(Value::String(input[start_idx..end_idx].to_string()))
}

pub fn trunc(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("trunc", args, 2)?;
    let length = expect_usize("trunc", &args[0], 1)?;
    let text = expect_string("trunc", &args[1], 2)?;
    Ok(Value::String(truncate_chars(&text, length)))
}

pub fn wrap(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("wrap", args, 2)?;
    let width = expect_usize("wrap", &args[0], 1)?;
    let text = expect_string("wrap", &args[1], 2)?;
    Ok(Value::String(wrap_text(width, &text)))
}

pub fn indent(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("indent", args, 2)?;
    let spaces = expect_usize("indent", &args[0], 1)?;
    let input = expect_string("indent", &args[1], 2)?;
    Ok(Value::String(indent_text(spaces, &input)))
}

pub fn nindent(ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    let mut output = indent(ctx, args)?;
    if let Value::String(s) = &mut output {
        s.insert(0, '\n');
    }
    Ok(output)
}

pub fn nospace(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("nospace", args, 1)?;
    let input = expect_string("nospace", &args[0], 1)?;
    Ok(Value::String(remove_whitespace(&input)))
}

pub fn repeat(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("repeat", args, 2)?;
    let count = expect_usize("repeat", &args[0], 1)?;
    let s = expect_string("repeat", &args[1], 2)?;
    Ok(json!(s.repeat(count)))
}

pub fn cat(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    let parts: Vec<String> = args
        .iter()
        .filter(|value| !value.is_null())
        .map(|value| value_to_string(value))
        .collect();
    Ok(Value::String(parts.join(" ")))
}

pub fn quote(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    Ok(Value::String(render_non_null(args, |raw| {
        serde_json::to_string(raw).unwrap_or_else(|_| format!("\"{}\"", raw.replace('"', "\\\"")))
    })))
}

pub fn squote(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    Ok(Value::String(render_non_null(args, |raw| {
        format!("'{}'", raw)
    })))
}

pub fn snakecase(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("snakecase", args, 1)?;
    let input = expect_string("snakecase", &args[0], 1)?;
    Ok(Value::String(input.to_snake_case()))
}

pub fn camelcase(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("camelcase", args, 1)?;
    let input = expect_string("camelcase", &args[0], 1)?;
    Ok(Value::String(input.to_upper_camel_case()))
}

pub fn kebabcase(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("kebabcase", args, 1)?;
    let input = expect_string("kebabcase", &args[0], 1)?;
    Ok(Value::String(input.to_kebab_case()))
}

pub fn swapcase(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("swapcase", args, 1)?;
    let input = expect_string("swapcase", &args[0], 1)?;
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_lowercase() {
            output.extend(ch.to_uppercase());
        } else if ch.is_uppercase() {
            output.extend(ch.to_lowercase());
        } else {
            output.push(ch);
        }
    }
    Ok(Value::String(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> EvalContext {
        super::super::empty_context()
    }

    #[test]
    fn upper_rejects_uncoercible_input() {
        let mut ctx = ctx();
        let err = super::upper(&mut ctx, &[json!(["oops"])])
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "render error: upper argument 1 must be coercible to string, got Array [String(\"oops\")]"
        );
    }

    #[test]
    fn substr_returns_empty_string_when_start_exceeds_length() {
        let mut ctx = ctx();
        let out = super::substr(&mut ctx, &[json!(5), json!("hey")]).unwrap();
        assert_eq!(out, json!(""));
    }

    #[test]
    fn cat_skips_null_arguments() {
        let mut ctx = ctx();
        let out = super::cat(&mut ctx, &[Value::Null, json!("foo"), json!("bar")]).unwrap();
        assert_eq!(out, json!("foo bar"));
    }

    #[test]
    fn repeat_rejects_negative_counts() {
        let mut ctx = ctx();
        let err = super::repeat(&mut ctx, &[json!(-1), json!("foo")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: repeat argument 1 must be a non-negative integer, got Number(-1)"
        );
    }

    #[test]
    fn strip_prefix_and_suffix_are_safe() {
        assert_eq!(super::strip_prefix("foobar", "foo"), "bar");
        assert_eq!(super::strip_prefix("foobar", "baz"), "foobar");
        assert_eq!(super::strip_suffix("foobar", "bar"), "foo");
        assert_eq!(super::strip_suffix("foobar", "qux"), "foobar");
    }

    #[test]
    fn wrap_text_respects_word_boundaries() {
        let wrapped = super::wrap_text(4, "foo bar baz");
        assert_eq!(wrapped, "foo\nbar\nbaz");
        let zero_width = super::wrap_text(0, "no change");
        assert_eq!(zero_width, "no change");
    }

    #[test]
    fn render_non_null_skips_missing_values() {
        let rendered = super::render_non_null(&[Value::Null, json!("foo"), json!("bar")], |raw| {
            format!("<{raw}>")
        });
        assert_eq!(rendered, "<foo> <bar>");
    }
}
