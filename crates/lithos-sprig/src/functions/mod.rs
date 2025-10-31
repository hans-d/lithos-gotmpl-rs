// SPDX-License-Identifier: Apache-2.0 OR MIT
#[cfg(test)]
use lithos_gotmpl_engine::EvalContext;
use lithos_gotmpl_engine::{is_empty, is_truthy, value_to_string, Error, FunctionRegistryBuilder};
use serde_json::Value;

mod dict;
mod flow;
mod lists;
mod string_slice;
mod strings;

pub fn install_all(builder: &mut FunctionRegistryBuilder) {
    flow::register(builder);
    strings::register(builder);
    string_slice::register(builder);
    lists::register(builder);
    dict::register(builder);
}

pub(crate) fn expect_min_args(name: &'static str, args: &[Value], min: usize) -> Result<(), Error> {
    if args.len() < min {
        return Err(Error::render(
            format!(
                "{name} expected at least {min} arguments, got {}",
                args.len()
            ),
            None,
        ));
    }
    Ok(())
}

pub(crate) fn expect_exact_args(
    name: &'static str,
    args: &[Value],
    expected: usize,
) -> Result<(), Error> {
    if args.len() != expected {
        return Err(Error::render(
            format!(
                "{name} expected {expected} argument{}, got {}",
                if expected == 1 { "" } else { "s" },
                args.len()
            ),
            None,
        ));
    }
    Ok(())
}

pub(crate) fn expect_string(
    name: &'static str,
    value: &Value,
    position: usize,
) -> Result<String, Error> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Ok(String::new()),
        _ => Err(Error::render(
            format!("{name} argument {position} must be coercible to string, got {value:?}"),
            None,
        )),
    }
}

pub(crate) fn expect_array(
    name: &'static str,
    value: &Value,
    position: usize,
) -> Result<Vec<Value>, Error> {
    match value {
        Value::Array(items) => Ok(items.clone()),
        Value::Null => Ok(Vec::new()),
        _ => Err(Error::render(
            format!("{name} argument {position} must be an array, got {value:?}"),
            None,
        )),
    }
}

pub(crate) fn expect_usize(
    name: &'static str,
    value: &Value,
    position: usize,
) -> Result<usize, Error> {
    if let Some(idx) = value.as_u64().or_else(|| {
        value
            .as_i64()
            .and_then(|v| if v >= 0 { Some(v as u64) } else { None })
    }) {
        return Ok(idx as usize);
    }
    if let Some(text) = value.as_str() {
        return text.parse::<usize>().map_err(|_| {
            Error::render(
                format!("{name} argument {position} must be a non-negative integer, got {value:?}"),
                None,
            )
        });
    }
    Err(Error::render(
        format!("{name} argument {position} must be a non-negative integer, got {value:?}"),
        None,
    ))
}

pub(crate) fn clamp_char_range(
    s: &str,
    start_chars: usize,
    len_chars: Option<usize>,
) -> (usize, usize) {
    let mut indices: Vec<usize> = s.char_indices().map(|(idx, _)| idx).collect();
    indices.push(s.len());
    let total = indices.len() - 1;
    let start = start_chars.min(total);
    let end = match len_chars {
        Some(len) => (start + len).min(total),
        None => total,
    };
    (indices[start], indices[end])
}

#[cfg(test)]
pub(crate) fn empty_context() -> EvalContext {
    EvalContext::new(Value::Null, FunctionRegistryBuilder::new().build())
}
