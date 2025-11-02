// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_engine::{Error, EvalContext};
use serde_json::{Map, Value};

use super::{expect_exact_args, expect_min_args, expect_string};

pub fn register(builder: &mut lithos_gotmpl_engine::FunctionRegistryBuilder) {
    builder
        .register("dict", dict)
        .register("get", get)
        .register("set", set)
        .register("unset", unset)
        .register("merge", merge)
        .register("hasKey", has_key)
        .register("keys", keys)
        .register("values", values)
        .register("pick", pick)
        .register("omit", omit)
        .register("pluck", pluck)
        .register("dig", dig);
}

pub fn dict(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.len().rem_euclid(2) != 0 {
        return Err(Error::render(
            format!("dict expected even number of arguments, got {}", args.len()),
            None,
        ));
    }
    let mut map = Map::new();
    let mut iter = args.iter();
    let mut index = 0;
    while let Some(key_val) = iter.next() {
        let key = expect_string("dict", key_val, index + 1)?;
        let value = iter.next().expect("even number ensured").clone();
        map.insert(key, value);
        index += 2;
    }
    Ok(Value::Object(map))
}

pub fn set(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("set", args, 3)?;
    let mut map = as_object("set", &args[0])?;
    let key = expect_string("set", &args[1], 2)?;
    map.insert(key, args[2].clone());
    Ok(Value::Object(map))
}

pub fn unset(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("unset", args, 2)?;
    let mut map = as_object("unset", &args[0])?;
    let key = expect_string("unset", &args[1], 2)?;
    map.remove(&key);
    Ok(Value::Object(map))
}

pub fn has_key(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("hasKey", args, 2)?;
    let map = as_object("hasKey", &args[0])?;
    let key = expect_string("hasKey", &args[1], 2)?;
    Ok(Value::Bool(map.contains_key(&key)))
}

pub fn get(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("get", args, 2)?;
    let map = as_object("get", &args[0])?;
    let key = expect_string("get", &args[1], 2)?;
    Ok(map
        .get(&key)
        .cloned()
        .unwrap_or_else(|| Value::String(String::new())))
}

pub fn merge(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("merge", args, 1)?;
    let mut result = as_object("merge", &args[0])?;
    for value in &args[1..] {
        let other = as_object("merge", value)?;
        for (key, val) in other {
            result.insert(key, val);
        }
    }
    Ok(Value::Object(result))
}

pub fn keys(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("keys", args, 1)?;
    let map = as_object("keys", &args[0])?;
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort();
    Ok(Value::Array(keys.into_iter().map(Value::String).collect()))
}

pub fn values(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_exact_args("values", args, 1)?;
    let map = as_object("values", &args[0])?;
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort();
    Ok(Value::Array(
        keys.into_iter()
            .map(|k| map.get(&k).cloned().unwrap_or(Value::Null))
            .collect(),
    ))
}

pub fn pick(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("pick", args, 2)?;
    let map = as_object("pick", &args[0])?;
    let mut result = Map::new();
    for (idx, key_val) in args[1..].iter().enumerate() {
        let key = expect_string("pick", key_val, idx + 2)?;
        if let Some(value) = map.get(&key) {
            result.insert(key, value.clone());
        }
    }
    Ok(Value::Object(result))
}

pub fn omit(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("omit", args, 2)?;
    let mut map = as_object("omit", &args[0])?;
    for (idx, key_val) in args[1..].iter().enumerate() {
        let key = expect_string("omit", key_val, idx + 2)?;
        map.remove(&key);
    }
    Ok(Value::Object(map))
}

pub fn pluck(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    expect_min_args("pluck", args, 2)?;
    let key = expect_string("pluck", &args[0], 1)?;
    let mut result = Vec::new();
    for source in &args[1..] {
        match source {
            Value::Array(items) => {
                for item in items {
                    if let Value::Object(map) = item {
                        if let Some(value) = map.get(&key) {
                            result.push(value.clone());
                        }
                    }
                }
            }
            Value::Object(map) => {
                if let Some(value) = map.get(&key) {
                    result.push(value.clone());
                }
            }
            _ => {}
        }
    }
    Ok(Value::Array(result))
}

pub fn dig(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.len() < 3 {
        return Err(Error::render(
            format!("dig requires at least three arguments, got {}", args.len()),
            None,
        ));
    }
    let key_count = args.len() - 2;
    let mut keys = Vec::with_capacity(key_count);
    for (idx, value) in args[..key_count].iter().enumerate() {
        keys.push(expect_string("dig", value, idx + 1)?);
    }
    let default_value = args[args.len() - 2].clone();
    let data = &args[args.len() - 1];

    let mut current = data;
    for key in keys {
        match current {
            Value::Object(map) => match map.get(&key) {
                Some(next) => current = next,
                None => return Ok(default_value),
            },
            _ => return Ok(default_value),
        }
    }
    Ok(current.clone())
}

fn as_object(name: &'static str, value: &Value) -> Result<Map<String, Value>, Error> {
    match value {
        Value::Object(map) => Ok(map.clone()),
        Value::Null => Ok(Map::new()),
        _ => Err(Error::render(
            format!("{name} expects a map/object as the first argument, got {value:?}"),
            None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> EvalContext {
        super::super::empty_context()
    }

    #[test]
    fn dict_rejects_odd_argument_counts() {
        let mut ctx = ctx();
        let err = dict(&mut ctx, &[json!("key")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: dict expected even number of arguments, got 1"
        );
    }

    #[test]
    fn set_treats_null_as_empty_map() {
        let mut ctx = ctx();
        let out = set(&mut ctx, &[Value::Null, json!("foo"), json!(1)]).unwrap();
        assert_eq!(out, json!({"foo": 1}));
    }

    #[test]
    fn set_rejects_non_map_inputs() {
        let mut ctx = ctx();
        let err = set(&mut ctx, &[json!("oops"), json!("foo"), json!(1)]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: set expects a map/object as the first argument, got String(\"oops\")"
        );
    }

    #[test]
    fn merge_requires_objects_for_every_argument() {
        let mut ctx = ctx();
        let err = merge(&mut ctx, &[json!({"foo": 1}), json!([1, 2, 3])]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: merge expects a map/object as the first argument, got Array [Number(1), Number(2), Number(3)]"
        );
    }

    #[test]
    fn pluck_ignores_non_maps_and_non_arrays() {
        let mut ctx = ctx();
        let out = pluck(
            &mut ctx,
            &[
                json!("name"),
                json!(
                    [
                        {"name": "alpha"},
                        Value::Null,
                        {"other": "ignored"}
                    ]
                ),
                json!({"name": "beta"}),
                json!(["not", "an", "object"]),
            ],
        )
        .unwrap();
        assert_eq!(out, json!(["alpha", "beta"]));
    }

    #[test]
    fn dig_returns_default_when_path_hits_non_object() {
        let mut ctx = ctx();
        let data = json!({"user": "missing nested profile"});
        let out = dig(
            &mut ctx,
            &[json!("user"), json!("profile"), json!("fallback"), data],
        )
        .unwrap();
        assert_eq!(out, json!("fallback"));
    }

    #[test]
    fn dig_requires_minimum_arguments() {
        let mut ctx = ctx();
        let err = dig(&mut ctx, &[json!("too"), json!("short")]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "render error: dig requires at least three arguments, got 2"
        );
    }
}
