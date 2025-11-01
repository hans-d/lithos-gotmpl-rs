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
    fn dict_builds_map() {
        let mut ctx = ctx();
        let out = dict(&mut ctx, &[json!("foo"), json!(1)]).unwrap();
        assert_eq!(out, json!({"foo": 1}));
    }

    #[test]
    fn set_updates_map() {
        let mut ctx = ctx();
        let input = json!({"foo": 1});
        let out = set(&mut ctx, &[input, json!("bar"), json!(2)]).unwrap();
        assert_eq!(out, json!({"foo":1, "bar":2}));
    }

    #[test]
    fn get_returns_value_when_present() {
        let mut ctx = ctx();
        let map = json!({"foo": "bar"});
        let out = get(&mut ctx, &[map, json!("foo")]).unwrap();
        assert_eq!(out, json!("bar"));
    }

    #[test]
    fn get_returns_empty_string_when_missing() {
        let mut ctx = ctx();
        let map = json!({"foo": "bar"});
        let out = get(&mut ctx, &[map, json!("missing")]).unwrap();
        assert_eq!(out, json!(""));
    }

    #[test]
    fn keys_are_sorted() {
        let mut ctx = ctx();
        let map = dict(&mut ctx, &[json!("b"), json!(2), json!("a"), json!(1)]).unwrap();
        let out = keys(&mut ctx, &[map]).unwrap();
        assert_eq!(out, json!(["a", "b"]));
    }

    #[test]
    fn merge_overrides_values() {
        let mut ctx = ctx();
        let out = merge(
            &mut ctx,
            &[json!({"foo": 1}), json!({"bar": 2}), json!({"foo": 3})],
        )
        .unwrap();
        assert_eq!(out, json!({"foo": 3, "bar": 2}));
    }

    #[test]
    fn values_follow_sorted_keys() {
        let mut ctx = ctx();
        let map = dict(
            &mut ctx,
            &[
                json!("b"),
                json!(2),
                json!("a"),
                json!(1),
                json!("c"),
                json!(3),
            ],
        )
        .unwrap();
        let out = values(&mut ctx, &[map]).unwrap();
        assert_eq!(out, json!([1, 2, 3]));
    }

    #[test]
    fn pick_extracts_selected_keys() {
        let mut ctx = ctx();
        let map = json!({"foo": 1, "bar": 2, "baz": 3});
        let out = pick(&mut ctx, &[map, json!("foo"), json!("baz")]).unwrap();
        assert_eq!(out, json!({"foo": 1, "baz": 3}));
    }

    #[test]
    fn omit_removes_keys() {
        let mut ctx = ctx();
        let map = json!({"foo": 1, "bar": 2, "baz": 3});
        let out = omit(&mut ctx, &[map, json!("bar")]).unwrap();
        assert_eq!(out, json!({"foo": 1, "baz": 3}));
    }

    #[test]
    fn pluck_collects_values_across_arrays() {
        let mut ctx = ctx();
        let list = json!([
            {"name": "alpha"},
            {"name": "beta"},
            {"other": "ignored"}
        ]);
        let out = pluck(&mut ctx, &[json!("name"), list]).unwrap();
        assert_eq!(out, json!(["alpha", "beta"]));

        let out_maps = pluck(
            &mut ctx,
            &[
                json!("name"),
                json!({"name": "alpha"}),
                json!({"name": "beta"}),
            ],
        )
        .unwrap();
        assert_eq!(out_maps, json!(["alpha", "beta"]));
    }

    #[test]
    fn dig_returns_nested_value_or_default() {
        let mut ctx = ctx();
        let map = json!({"user": {"profile": {"id": 42}}});
        let found = dig(
            &mut ctx,
            &[json!("user"), json!("profile"), json!(0), map.clone()],
        )
        .unwrap();
        assert_eq!(found, json!({"id": 42}));

        let missing = dig(
            &mut ctx,
            &[json!("user"), json!("missing"), json!("fallback"), map],
        )
        .unwrap();
        assert_eq!(missing, json!("fallback"));
    }
}
