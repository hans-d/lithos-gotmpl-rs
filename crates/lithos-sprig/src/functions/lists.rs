// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_engine::{Error, EvalContext};
use serde_json::Value;

use super::{expect_array, expect_exact_args, expect_min_args};

pub fn register(builder: &mut lithos_gotmpl_engine::FunctionRegistryBuilder) {
    builder
        .register("list", list)
        .register("first", first)
        .register("last", last)
        .register("rest", rest)
        .register("initial", initial)
        .register("append", append)
        .register("prepend", prepend)
        .register("concat", concat)
        .register("reverse", reverse)
        .register("compact", compact)
        .register("uniq", uniq)
        .register("without", without)
        .register("has", has);
}

pub fn list(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    Ok(Value::Array(args.to_vec()))
}

pub fn first(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("first", args, 1)?;
    let list = expect_array("first", &args[0], 1)?;
    Ok(list.into_iter().next().unwrap_or(Value::Null))
}

pub fn last(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("last", args, 1)?;
    let list = expect_array("last", &args[0], 1)?;
    Ok(list.into_iter().last().unwrap_or(Value::Null))
}

pub fn rest(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("rest", args, 1)?;
    let list = expect_array("rest", &args[0], 1)?;
    Ok(Value::Array(list.into_iter().skip(1).collect()))
}

pub fn initial(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("initial", args, 1)?;
    let mut list = expect_array("initial", &args[0], 1)?;
    if !list.is_empty() {
        list.pop();
    }
    Ok(Value::Array(list))
}

pub fn append(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("append", args, 2)?;
    let mut list = expect_array("append", &args[0], 1)?;
    list.extend_from_slice(&args[1..]);
    Ok(Value::Array(list))
}

pub fn prepend(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("prepend", args, 2)?;
    let list = expect_array("prepend", &args[0], 1)?;
    let mut prefix: Vec<Value> = args[1..].to_vec();
    prefix.extend(list);
    Ok(Value::Array(prefix))
}

pub fn concat(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("concat", args, 1)?;
    let mut combined = Vec::new();
    for (idx, value) in args.iter().enumerate() {
        let mut list = expect_array("concat", value, idx + 1)?;
        combined.append(&mut list);
    }
    Ok(Value::Array(combined))
}

pub fn reverse(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("reverse", args, 1)?;
    let mut list = expect_array("reverse", &args[0], 1)?;
    list.reverse();
    Ok(Value::Array(list))
}

pub fn compact(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("compact", args, 1)?;
    let list = expect_array("compact", &args[0], 1)?;
    Ok(Value::Array(
        list.into_iter()
            .filter(|value| !super::is_empty(value))
            .collect(),
    ))
}

pub fn uniq(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("uniq", args, 1)?;
    let list = expect_array("uniq", &args[0], 1)?;
    let mut out = Vec::new();
    for value in list {
        if !out.iter().any(|existing| existing == &value) {
            out.push(value);
        }
    }
    Ok(Value::Array(out))
}

pub fn without(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("without", args, 2)?;
    let list = expect_array("without", &args[0], 1)?;
    Ok(Value::Array(
        list.into_iter()
            .filter(|item| !args[1..].iter().any(|remove| remove == item))
            .collect(),
    ))
}

pub fn has(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("has", args, 2)?;
    let target = &args[0];
    let haystack = &args[1];
    let result = match haystack {
        Value::Array(items) => items.iter().any(|value| value == target),
        Value::String(text) => text.contains(&super::value_to_string(target)),
        Value::Null => false,
        _ => {
            return Err(Error::render(
                "has expects a string or array as the second argument",
                None,
            ));
        }
    };
    Ok(Value::Bool(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> EvalContext {
        super::super::empty_context()
    }

    #[test]
    fn first_requires_array_input() {
        let mut ctx = ctx();
        let err = first(&mut ctx, &[json!("oops")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: first argument 1 must be an array, got String(\"oops\")"
        );
    }

    #[test]
    fn concat_propagates_first_non_array_error() {
        let mut ctx = ctx();
        let err = concat(&mut ctx, &[json!([1, 2]), json!({"bad": true})]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: concat argument 2 must be an array, got Object {\"bad\": Bool(true)}"
        );
    }

    #[test]
    fn without_handles_duplicates_and_nulls() {
        let mut ctx = ctx();
        let out = without(
            &mut ctx,
            &[json!([1, 2, 2, null, 3]), json!(2), Value::Null],
        )
        .unwrap();
        assert_eq!(out, json!([1, 3]));
    }

    #[test]
    fn has_rejects_invalid_haystack_type() {
        let mut ctx = ctx();
        let err = has(&mut ctx, &[json!("a"), json!({"not": "supported"})]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: has expects a string or array as the second argument"
        );
    }
}
