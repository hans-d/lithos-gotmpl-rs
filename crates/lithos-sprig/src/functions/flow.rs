// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_engine::{Error, EvalContext};
use serde_json::Value;

use super::{expect_exact_args, expect_min_args, expect_string};
use super::{is_empty, value_to_string};

pub fn register(builder: &mut lithos_gotmpl_engine::FunctionRegistryBuilder) {
    builder
        .register("default", default)
        .register("coalesce", coalesce)
        .register("ternary", ternary)
        .register("empty", empty)
        .register("fail", fail)
        .register("fromJson", from_json)
        .register("mustFromJson", must_from_json)
        .register("toJson", to_json)
        .register("mustToJson", must_to_json)
        .register("toPrettyJson", to_pretty_json)
        .register("mustToPrettyJson", must_to_pretty_json)
        .register("toRawJson", to_raw_json)
        .register("mustToRawJson", must_to_raw_json);
}

// NOTE: Every helper takes `&mut EvalContext` even when the body does not need
// to touch the context. This keeps the function signature uniform with the
// engine's `Function` trait, making registration and invocation consistent.

pub fn default(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("default", args, 2)?;
    let fallback = args[0].clone();
    let candidate = args[1].clone();
    if is_empty(&candidate) {
        Ok(fallback)
    } else {
        Ok(candidate)
    }
}

pub fn coalesce(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    for value in args {
        if !is_empty(value) {
            return Ok(value.clone());
        }
    }
    Ok(Value::Null)
}

pub fn ternary(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("ternary", args, 3)?;
    if super::is_truthy(&args[2]) {
        Ok(args[0].clone())
    } else {
        Ok(args[1].clone())
    }
}

pub fn empty(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("empty", args, 1)?;
    Ok(Value::Bool(is_empty(&args[0])))
}

pub fn fail(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("fail", args, 1)?;
    let mut message = String::new();
    for (idx, value) in args.iter().enumerate() {
        if idx > 0 {
            message.push(' ');
        }
        message.push_str(&value_to_string(value));
    }
    Err(Error::render(message, None))
}

pub fn from_json(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("fromJson", args, 1)?;
    let text = expect_string("fromJson", &args[0], 1)?;
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}

pub fn must_from_json(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("mustFromJson", args, 1)?;
    let text = expect_string("mustFromJson", &args[0], 1)?;
    serde_json::from_str(&text)
        .map_err(|err| Error::render(format!("mustFromJson failed: {err}"), None))
}

fn serialize_json(value: &Value, pretty: bool) -> Result<String, serde_json::Error> {
    if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
}

pub fn to_json(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("toJson", args, 1)?;
    match serialize_json(&args[0], false) {
        Ok(text) => Ok(Value::String(text)),
        Err(_) => Ok(Value::String(String::new())),
    }
}

pub fn must_to_json(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("mustToJson", args, 1)?;
    serialize_json(&args[0], false)
        .map(Value::String)
        .map_err(|err| Error::render(format!("mustToJson failed: {err}"), None))
}

pub fn to_pretty_json(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("toPrettyJson", args, 1)?;
    match serialize_json(&args[0], true) {
        Ok(text) => Ok(Value::String(text)),
        Err(_) => Ok(Value::String(String::new())),
    }
}

pub fn must_to_pretty_json(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("mustToPrettyJson", args, 1)?;
    serialize_json(&args[0], true)
        .map(Value::String)
        .map_err(|err| Error::render(format!("mustToPrettyJson failed: {err}"), None))
}

pub fn to_raw_json(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    to_json(_ctx, args)
}

pub fn must_to_raw_json(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    must_to_json(_ctx, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> EvalContext {
        super::super::empty_context()
    }

    #[test]
    fn default_requires_two_arguments() {
        let mut ctx = ctx();
        let err = default(&mut ctx, &[json!("fallback")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: default expected at least 2 arguments, got 1"
        );
    }

    #[test]
    fn ternary_rejects_wrong_argument_count() {
        let mut ctx = ctx();
        let err = ternary(&mut ctx, &[json!("true"), json!("false")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: ternary expected 3 arguments, got 2"
        );
    }

    #[test]
    fn must_from_json_surfaces_parse_errors() {
        let mut ctx = ctx();
        let err = must_from_json(&mut ctx, &[json!("{invalid}")]).unwrap_err();
        assert!(
            err
                .to_string()
                .starts_with("render error: mustFromJson failed:"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn fail_joins_arguments_with_spaces() {
        let mut ctx = ctx();
        let err = fail(&mut ctx, &[json!("boom"), json!(123)]).unwrap_err();
        assert_eq!(err.to_string(), "render error: boom 123");
    }
}
