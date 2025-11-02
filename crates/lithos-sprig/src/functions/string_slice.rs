// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_engine::{Error, EvalContext};
use serde_json::{Map, Value};

use super::value_to_string;
use super::{expect_array, expect_exact_args, expect_string};

pub fn register(builder: &mut lithos_gotmpl_engine::FunctionRegistryBuilder) {
    builder
        .register("splitList", split_list)
        .register("split", split_map)
        .register("splitn", splitn)
        .register("join", join)
        .register("sortAlpha", sort_alpha);
}

pub fn split_list(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("splitList", args, 2)?;
    let sep = expect_string("splitList", &args[0], 1)?;
    let text = expect_string("splitList", &args[1], 2)?;
    Ok(Value::Array(
        text.split(&sep)
            .map(|s| Value::String(s.to_string()))
            .collect(),
    ))
}

pub fn split_map(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("split", args, 2)?;
    let sep = expect_string("split", &args[0], 1)?;
    let text = expect_string("split", &args[1], 2)?;
    let mut map = Map::new();
    for (idx, part) in text.split(&sep).enumerate() {
        map.insert(format!("_{idx}"), Value::String(part.to_string()));
    }
    Ok(Value::Object(map))
}

pub fn splitn(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("splitn", args, 3)?;
    let sep = expect_string("splitn", &args[0], 1)?;
    let text = expect_string("splitn", &args[1], 2)?;
    let count = super::expect_usize("splitn", &args[2], 3)?;
    Ok(Value::Array(
        text.splitn(count, &sep)
            .map(|s| Value::String(s.to_string()))
            .collect(),
    ))
}

pub fn join(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("join", args, 2)?;
    let sep = expect_string("join", &args[0], 1)?;
    let list = expect_array("join", &args[1], 2)?;
    let mut result = String::new();
    for (idx, value) in list.iter().enumerate() {
        if idx > 0 {
            result.push_str(&sep);
        }
        result.push_str(&value_to_string(value));
    }
    Ok(Value::String(result))
}

pub fn sort_alpha(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("sortAlpha", args, 1)?;
    let mut list = expect_array("sortAlpha", &args[0], 1)?;
    list.sort_by_key(value_to_string);
    Ok(Value::Array(list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> EvalContext {
        super::super::empty_context()
    }

    #[test]
    fn split_list_requires_string_separator() {
        let mut ctx = ctx();
        let err = split_list(&mut ctx, &[json!({"oops": true}), json!("a,b")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: splitList argument 1 must be coercible to string, got Object {\"oops\": Bool(true)}"
        );
    }

    #[test]
    fn split_map_uses_incrementing_keys() {
        let mut ctx = ctx();
        let out = split_map(&mut ctx, &[json!(":"), json!("a:b")]).unwrap();
        assert_eq!(out, json!({"_0": "a", "_1": "b"}));
    }

    #[test]
    fn splitn_truncates_to_requested_segments() {
        let mut ctx = ctx();
        let out = splitn(&mut ctx, &[json!(","), json!("a,b,c"), json!(2)]).unwrap();
        assert_eq!(out, json!(["a", "b,c"]));
    }
}
