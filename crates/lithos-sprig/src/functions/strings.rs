// SPDX-License-Identifier: Apache-2.0 OR MIT
use heck::{ToKebabCase, ToSnakeCase, ToUpperCamelCase};
use lithos_gotmpl_engine::{Error, EvalContext};
use serde_json::{json, Value};

use super::{
    clamp_char_range, expect_exact_args, expect_min_args, expect_string, expect_usize,
    value_to_string,
};

pub fn register(builder: &mut lithos_gotmpl_engine::FunctionRegistryBuilder) {
    builder
        .register("upper", upper)
        .register("lower", lower)
        .register("title", title)
        .register("trim", trim)
        .register("trimAll", trim_all)
        .register("trimPrefix", trim_prefix)
        .register("trimSuffix", trim_suffix)
        .register("hasPrefix", has_prefix)
        .register("hasSuffix", has_suffix)
        .register("contains", contains)
        .register("replace", replace)
        .register("substr", substr)
        .register("trunc", trunc)
        .register("wrap", wrap)
        .register("indent", indent)
        .register("nindent", nindent)
        .register("nospace", nospace)
        .register("repeat", repeat)
        .register("cat", cat)
        .register("quote", quote)
        .register("squote", squote)
        .register("snakecase", snakecase)
        .register("camelcase", camelcase)
        .register("kebabcase", kebabcase)
        .register("swapcase", swapcase);
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
    Ok(json!(result))
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
    let result = if input.starts_with(&prefix) {
        input[prefix.len()..].to_string()
    } else {
        input
    };
    Ok(json!(result))
}

pub fn trim_suffix(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("trimSuffix", args, 2)?;
    let suffix = expect_string("trimSuffix", &args[0], 1)?;
    let input = expect_string("trimSuffix", &args[1], 2)?;
    let result = if input.ends_with(&suffix) {
        let end = input.len() - suffix.len();
        input[..end].to_string()
    } else {
        input
    };
    Ok(json!(result))
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
    let chars: Vec<char> = text.chars().collect();
    let end = length.min(chars.len());
    let mut out = String::with_capacity(text.len());
    for ch in &chars[..end] {
        out.push(*ch);
    }
    Ok(Value::String(out))
}

pub fn wrap(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("wrap", args, 2)?;
    let width = expect_usize("wrap", &args[0], 1)?;
    let text = expect_string("wrap", &args[1], 2)?;
    if width == 0 {
        return Ok(Value::String(text));
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
    Ok(Value::String(lines.join("\n")))
}

pub fn indent(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("indent", args, 2)?;
    let spaces = expect_usize("indent", &args[0], 1)?;
    let input = expect_string("indent", &args[1], 2)?;
    let pad = " ".repeat(spaces);
    let result = input
        .lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Value::String(result))
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
    Ok(Value::String(
        input.chars().filter(|c| !c.is_whitespace()).collect(),
    ))
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
    let mut parts = Vec::new();
    for value in args {
        if value.is_null() {
            continue;
        }
        let raw = value_to_string(value);
        let quoted = serde_json::to_string(&raw)
            .unwrap_or_else(|_| format!("\"{}\"", raw.replace('"', "\\\"")));
        parts.push(quoted);
    }
    Ok(Value::String(parts.join(" ")))
}

pub fn squote(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    let mut parts = Vec::new();
    for value in args {
        if value.is_null() {
            continue;
        }
        let raw = value_to_string(value);
        parts.push(format!("'{}'", raw));
    }
    Ok(Value::String(parts.join(" ")))
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
    fn uppercases_text() {
        let mut ctx = ctx();
        let out = super::upper(&mut ctx, &[json!("hello")]).unwrap();
        assert_eq!(out, json!("HELLO"));
    }

    #[test]
    fn substr_handles_unicode() {
        let mut ctx = ctx();
        let out = super::substr(&mut ctx, &[json!(1), json!(4), json!("héllo")]).unwrap();
        assert_eq!(out, json!("éll"));
    }

    #[test]
    fn cat_skips_null_arguments() {
        let mut ctx = ctx();
        let out = super::cat(&mut ctx, &[Value::Null, json!("foo"), json!("bar")]).unwrap();
        assert_eq!(out, json!("foo bar"));
    }

    #[test]
    fn quote_and_squote_wrap_values() {
        let mut ctx = ctx();
        let quote_out = super::quote(&mut ctx, &[json!("foo"), json!("bar baz")]).unwrap();
        assert_eq!(quote_out, json!("\"foo\" \"bar baz\""));
        let squote_out = super::squote(&mut ctx, &[json!("foo"), json!(123)]).unwrap();
        assert_eq!(squote_out, json!("'foo' '123'"));
    }

    #[test]
    fn case_conversions_follow_sprig_conventions() {
        let mut ctx = ctx();
        assert_eq!(
            super::snakecase(&mut ctx, &[json!("FirstName")]).unwrap(),
            json!("first_name")
        );
        assert_eq!(
            super::camelcase(&mut ctx, &[json!("first_name")]).unwrap(),
            json!("FirstName")
        );
        assert_eq!(
            super::kebabcase(&mut ctx, &[json!("First Name")]).unwrap(),
            json!("first-name")
        );
        assert_eq!(
            super::swapcase(&mut ctx, &[json!("FirstName")]).unwrap(),
            json!("fIRSTnAME")
        );
    }
}
