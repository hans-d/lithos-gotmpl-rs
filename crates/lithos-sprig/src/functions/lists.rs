// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_engine::{coerce_number, Error, EvalContext};
use serde_json::{Number, Value};

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
        .register("has", has)
        .register("max", max)
        .register("min", min);
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
    expect_exact_args("prepend", args, 2)?;
    let mut list = expect_array("prepend", &args[0], 1)?;
    list.insert(0, args[1].clone());
    Ok(Value::Array(list))
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

pub fn max(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    find_extreme("max", args, |candidate, current| candidate > current)
}

pub fn min(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    find_extreme("min", args, |candidate, current| candidate < current)
}

fn find_extreme<F>(name: &'static str, args: &[Value], better: F) -> Result<Value, Error>
where
    F: Fn(f64, f64) -> bool,
{
    let values = collect_numeric_inputs(name, args)?;
    let mut iter = values.into_iter();
    let mut best_score = match iter.next() {
        Some((value, position)) => to_number(name, position, &value)?,
        None => {
            return Err(Error::render(
                format!("{name} requires at least one numeric value"),
                None,
            ))
        }
    };

    for (value, position) in iter {
        let score = to_number(name, position, &value)?;
        if better(score, best_score) {
            best_score = score;
        }
    }

    Ok(score_to_value(best_score))
}

fn collect_numeric_inputs(
    name: &'static str,
    args: &[Value],
) -> Result<Vec<(Value, usize)>, Error> {
    if args.is_empty() {
        return Err(Error::render(
            format!("{name} requires at least one argument"),
            None,
        ));
    }

    Ok(args
        .iter()
        .enumerate()
        .map(|(idx, value)| (value.clone(), idx + 1))
        .collect())
}

fn to_number(name: &'static str, position: usize, value: &Value) -> Result<f64, Error> {
    if value.is_array() {
        return Ok(0.0);
    }
    coerce_number(value)
        .map_err(|_| Error::render(format!("{name} argument {position} must be numeric"), None))
}

fn score_to_value(score: f64) -> Value {
    if (score.fract() - 0.0).abs() < f64::EPSILON {
        Value::Number(Number::from(score as i64))
    } else if let Some(num) = Number::from_f64(score) {
        Value::Number(num)
    } else {
        Value::Null
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

    #[test]
    fn max_over_variadic_args() {
        let mut ctx = ctx();
        let out = super::max(&mut ctx, &[json!(1), json!(5), json!(3)]).unwrap();
        assert_eq!(out, json!(5));
    }

    #[test]
    fn min_over_list_argument() {
        let mut ctx = ctx();
        let out = super::min(&mut ctx, &[json!([4, 2, 9])]).unwrap();
        assert_eq!(out, json!(0));
    }

    #[test]
    fn min_over_variadic_args() {
        let mut ctx = ctx();
        let out = super::min(&mut ctx, &[json!(4), json!(2), json!(9)]).unwrap();
        assert_eq!(out, json!(2));
    }

    #[test]
    fn max_errors_on_non_numeric() {
        let mut ctx = ctx();
        let err = super::max(&mut ctx, &[json!("foo"), json!(1.0)]);
        assert!(err.is_err());
    }
}
