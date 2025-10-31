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
        .register("values", values);
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
}
