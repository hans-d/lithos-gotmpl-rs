#![forbid(unsafe_code)]
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub use lithos_gotmpl_engine::{
    analyze_template, coerce_number, is_empty, is_truthy, value_to_string, AnalysisIssue,
    Certainty, ControlKind, ControlUsage, Error, EvalContext, FunctionCall, FunctionRegistry,
    FunctionRegistryBuilder, FunctionSource, Precision, Template, TemplateAnalysis, TemplateCall,
    VariableAccess, VariableKind,
};
use serde_json::Number;
use serde_json::Value;

struct SliceIndices {
    start: usize,
    end: usize,
}

struct ParsedSpecifier {
    strategy: FormatStrategy,
}

impl ParsedSpecifier {
    fn parse(chars: &mut std::str::Chars<'_>) -> Result<Self, Error> {
        let Some(next) = chars.next() else {
            return Err(Error::render("incomplete format specifier", None));
        };

        let strategy = match next {
            '%' => FormatStrategy::PercentLiteral,
            's' | 'v' => FormatStrategy::StringLike,
            'd' | 'b' | 'o' | 'x' | 'X' => FormatStrategy::Integer,
            'f' | 'g' | 'e' | 'E' => FormatStrategy::Float,
            other => FormatStrategy::Fallback(other),
        };

        Ok(Self { strategy })
    }

    fn needs_argument(&self) -> bool {
        !matches!(self.strategy, FormatStrategy::PercentLiteral)
    }

    fn format(&self, arg: Option<&Value>) -> Result<String, Error> {
        match self.strategy {
            FormatStrategy::PercentLiteral => Ok("%".to_string()),
            FormatStrategy::StringLike => {
                let value =
                    arg.ok_or_else(|| Error::render("not enough arguments for printf", None))?;
                Ok(value_to_string(value))
            }
            FormatStrategy::Integer => {
                let value =
                    arg.ok_or_else(|| Error::render("not enough arguments for printf", None))?;
                format_integer(value)
            }
            FormatStrategy::Float => {
                let value =
                    arg.ok_or_else(|| Error::render("not enough arguments for printf", None))?;
                format_float(value)
            }
            FormatStrategy::Fallback(specifier) => {
                let value =
                    arg.ok_or_else(|| Error::render("not enough arguments for printf", None))?;
                let mut formatted = String::from("%");
                formatted.push(specifier);
                formatted.push_str(&value_to_string(value));
                Ok(formatted)
            }
        }
    }
}

enum FormatStrategy {
    PercentLiteral,
    StringLike,
    Integer,
    Float,
    Fallback(char),
}

/// Builds a registry that mirrors the helper set Go's `text/template` exposes by
/// default, keeping Lithos-compatible templates aligned with the reference
/// implementation.
pub fn text_template_functions() -> FunctionRegistry {
    let mut builder = FunctionRegistryBuilder::new();
    install_text_template_functions(&mut builder);
    builder.build()
}

/// Installs the standard Go text/template helper functions into an existing registry builder.
pub fn install_text_template_functions(builder: &mut FunctionRegistryBuilder) {
    builder
        .register("and", builtin_and)
        .register("call", builtin_call)
        .register("html", builtin_html)
        .register("eq", builtin_eq)
        .register("ge", builtin_ge)
        .register("gt", builtin_gt)
        .register("index", builtin_index)
        .register("js", builtin_js)
        .register("len", builtin_len)
        .register("le", builtin_le)
        .register("lt", builtin_lt)
        .register("ne", builtin_ne)
        .register("not", builtin_not)
        .register("print", builtin_print)
        .register("println", builtin_println)
        .register("or", builtin_or)
        .register("printf", builtin_printf)
        .register("slice", builtin_slice)
        .register("urlquery", builtin_urlquery);
}

fn builtin_eq(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.len() < 2 {
        return Err(Error::render("eq expects two arguments", None));
    }
    Ok(Value::Bool(args[0] == args[1]))
}

fn builtin_ne(ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    builtin_eq(ctx, args).map(|v| Value::Bool(!v.as_bool().unwrap()))
}

fn builtin_lt(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    comparison(args, |a, b| a < b, |a, b| a < b)
}

fn builtin_le(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    comparison(args, |a, b| a <= b, |a, b| a <= b)
}

fn builtin_gt(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    comparison(args, |a, b| a > b, |a, b| a > b)
}

fn builtin_ge(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    comparison(args, |a, b| a >= b, |a, b| a >= b)
}

fn comparison<F, G>(args: &[Value], num: F, str_op: G) -> Result<Value, Error>
where
    F: Fn(f64, f64) -> bool,
    G: Fn(&str, &str) -> bool,
{
    if args.len() < 2 {
        return Err(Error::render("comparison expects two arguments", None));
    }
    let lhs = &args[0];
    let rhs = &args[1];
    if lhs.is_number() && rhs.is_number() {
        compare_numbers(lhs, rhs, num)
    } else if lhs.is_string() && rhs.is_string() {
        Ok(Value::Bool(str_op(
            lhs.as_str().unwrap(),
            rhs.as_str().unwrap(),
        )))
    } else {
        Err(Error::render(
            "comparison requires both arguments to be numbers or strings",
            None,
        ))
    }
}

fn compare_numbers<F>(lhs: &Value, rhs: &Value, cmp: F) -> Result<Value, Error>
where
    F: Fn(f64, f64) -> bool,
{
    let left = coerce_number(lhs)?;
    let right = coerce_number(rhs)?;
    Ok(Value::Bool(cmp(left, right)))
}

fn builtin_printf(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.is_empty() {
        return Err(Error::render("printf expects format string", None));
    }
    let format = args[0]
        .as_str()
        .ok_or_else(|| Error::render("printf expects format string as first argument", None))?;

    let mut output = String::new();
    let mut chars = format.chars();
    let mut arg_index = 1usize;

    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }

        let specifier = ParsedSpecifier::parse(&mut chars)?;

        if specifier.needs_argument() {
            let arg = args
                .get(arg_index)
                .ok_or_else(|| Error::render("not enough arguments for printf", None))?;
            arg_index += 1;
            output.push_str(&specifier.format(Some(arg))?);
        } else {
            output.push_str(&specifier.format(None)?);
        }
    }

    append_extra_args(&mut output, &args[arg_index..]);

    Ok(Value::String(output))
}

fn append_extra_args(output: &mut String, extra_args: &[Value]) {
    let mut extras = extra_args.iter();
    if let Some(first) = extras.next() {
        output.push_str(&value_to_string(first));
        for arg in extras {
            output.push(' ');
            output.push_str(&value_to_string(arg));
        }
    }
}

fn builtin_print(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    let mut output = String::new();
    for value in args {
        output.push_str(&value_to_string(value));
    }
    Ok(Value::String(output))
}

fn builtin_println(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    let mut output = String::new();
    let mut first = true;
    for value in args {
        if !first {
            output.push(' ');
        }
        first = false;
        output.push_str(&value_to_string(value));
    }
    output.push('\n');
    Ok(Value::String(output))
}

fn builtin_html(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.len() != 1 {
        return Err(Error::render("html expects exactly one argument", None));
    }
    let input = value_to_string(&args[0]);
    Ok(Value::String(escape_html(&input)))
}

fn builtin_js(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.len() != 1 {
        return Err(Error::render("js expects exactly one argument", None));
    }
    let input = value_to_string(&args[0]);
    Ok(Value::String(escape_js(&input)))
}

fn builtin_urlquery(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.len() != 1 {
        return Err(Error::render("urlquery expects exactly one argument", None));
    }
    let input = value_to_string(&args[0]);
    Ok(Value::String(escape_urlquery(&input)))
}

fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&#34;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
    }
    output
}

fn escape_js(input: &str) -> String {
    let mut json = serde_json::to_string(input).unwrap_or_else(|_| String::from("\"\""));
    // strip surrounding quotes
    if json.len() >= 2 {
        json = json[1..json.len() - 1].to_string();
    }
    let mut result = String::with_capacity(json.len());
    for ch in json.chars() {
        match ch {
            '<' => result.push_str("\\u003C"),
            '>' => result.push_str("\\u003E"),
            '&' => result.push_str("\\u0026"),
            '=' => result.push_str("\\u003D"),
            '\'' => result.push_str("\\u0027"),
            '"' => result.push_str("\\u0022"),
            '\u{2028}' => result.push_str("\\u2028"),
            '\u{2029}' => result.push_str("\\u2029"),
            _ => result.push(ch),
        }
    }
    result
}

fn escape_urlquery(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(b as char)
            }
            b' ' => output.push('+'),
            _ => {
                output.push('%');
                output.push_str(&format!("{:02X}", b));
            }
        }
    }
    output
}

fn builtin_index(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.is_empty() {
        return Err(Error::render("index expects at least one argument", None));
    }

    let mut current = args[0].clone();
    for key in &args[1..] {
        current = match (&current, key) {
            (Value::Object(map), Value::String(s)) => map.get(s).cloned().unwrap_or(Value::Null),
            (Value::Object(map), Value::Number(num)) => {
                let key = num.to_string();
                map.get(&key).cloned().unwrap_or(Value::Null)
            }
            (Value::Array(list), Value::Number(num)) => {
                let idx = num
                    .as_u64()
                    .ok_or_else(|| Error::render("array index must be unsigned integer", None))?
                    as usize;
                list.get(idx).cloned().unwrap_or(Value::Null)
            }
            (Value::Array(list), Value::String(s)) => {
                let idx = s
                    .parse::<usize>()
                    .map_err(|_| Error::render("array index must be integer", None))?;
                list.get(idx).cloned().unwrap_or(Value::Null)
            }
            _ => return Err(Error::render("index expects map or array container", None)),
        };
    }

    Ok(current)
}

fn builtin_and(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    for value in args {
        if !is_truthy(value) {
            return Ok(value.clone());
        }
    }
    Ok(args.last().cloned().unwrap_or(Value::Bool(true)))
}

fn builtin_or(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    for value in args {
        if is_truthy(value) {
            return Ok(value.clone());
        }
    }
    Ok(args.last().cloned().unwrap_or(Value::Bool(false)))
}

fn builtin_len(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.len() != 1 {
        return Err(Error::render("len expects exactly one argument", None));
    }
    let len = match &args[0] {
        Value::Null => 0,
        Value::String(s) => s.len(),
        Value::Array(list) => list.len(),
        Value::Object(map) => map.len(),
        Value::Bool(_) | Value::Number(_) => {
            return Err(Error::render("len expects array, map, or string", None));
        }
    };
    Ok(Value::Number(Number::from(len as u64)))
}

fn builtin_slice(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.is_empty() {
        return Err(Error::render("slice expects at least one argument", None));
    }
    let target = &args[0];

    match target {
        Value::String(s) => {
            let len = s.len();
            let SliceIndices { start, end } = parse_slice_indices(&args[1..], len)?;
            let slice = s
                .get(start..end)
                .ok_or_else(|| Error::render("slice indices not on char boundaries", None))?;
            Ok(Value::String(slice.to_string()))
        }
        Value::Array(list) => {
            let len = list.len();
            let SliceIndices { start, end } = parse_slice_indices(&args[1..], len)?;
            Ok(Value::Array(list[start..end].to_vec()))
        }
        Value::Null => Ok(Value::Array(Vec::new())),
        _ => Err(Error::render(
            "slice expects string or array as first argument",
            None,
        )),
    }
}

fn parse_slice_indices(indices: &[Value], len: usize) -> Result<SliceIndices, Error> {
    let parsed: Result<Vec<usize>, Error> = indices
        .iter()
        .map(|arg| {
            if let Some(idx) = arg.as_u64().or_else(|| {
                arg.as_i64()
                    .and_then(|v| if v >= 0 { Some(v as u64) } else { None })
            }) {
                Ok(idx as usize)
            } else if let Some(text) = arg.as_str() {
                text.parse::<usize>()
                    .map_err(|_| Error::render("slice indices must be non-negative integers", None))
            } else {
                Err(Error::render(
                    "slice indices must be non-negative integers",
                    None,
                ))
            }
        })
        .collect();
    let parsed = parsed?;
    if parsed.len() > 2 {
        return Err(Error::render("slice supports at most two indices", None));
    }
    let (start, end) = slice_bounds(&parsed, len)?;
    Ok(SliceIndices { start, end })
}

fn slice_bounds(indices: &[usize], len: usize) -> Result<(usize, usize), Error> {
    let start = indices.first().copied().unwrap_or(0);
    let end = indices.get(1).copied().unwrap_or(len);
    if start > end || end > len {
        return Err(Error::render("slice indices out of range", None));
    }
    Ok((start, end))
}

fn builtin_call(ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.is_empty() {
        return Err(Error::render("call expects at least one argument", None));
    }
    let func_name = args[0]
        .as_str()
        .ok_or_else(|| Error::render("call expects function name as string", None))?;
    let func = ctx
        .function(func_name)
        .ok_or_else(|| Error::render(format!("unknown function \"{func_name}\""), None))?;
    func(ctx, &args[1..])
}

fn builtin_not(_ctx: &mut EvalContext, args: &[Value]) -> Result<Value, Error> {
    if args.len() != 1 {
        return Err(Error::render("not expects exactly one argument", None));
    }
    Ok(Value::Bool(!is_truthy(&args[0])))
}

fn format_integer(value: &Value) -> Result<String, Error> {
    if let Some(i) = value.as_i64() {
        return Ok(i.to_string());
    }
    if let Some(u) = value.as_u64() {
        return Ok(u.to_string());
    }
    if let Some(s) = value.as_str() {
        if let Ok(parsed) = s.parse::<i128>() {
            return Ok(parsed.to_string());
        }
    }
    let coerced = coerce_number(value)?;
    if coerced.fract() == 0.0 {
        Ok(format!("{:.0}", coerced))
    } else {
        Ok(coerced.to_string())
    }
}

fn format_float(value: &Value) -> Result<String, Error> {
    if let Some(f) = value.as_f64() {
        return Ok(trim_trailing_zeros(f));
    }
    if let Some(i) = value.as_i64() {
        return Ok(trim_trailing_zeros(i as f64));
    }
    if let Some(u) = value.as_u64() {
        return Ok(trim_trailing_zeros(u as f64));
    }
    if let Some(s) = value.as_str() {
        if let Ok(parsed) = s.parse::<f64>() {
            return Ok(trim_trailing_zeros(parsed));
        }
    }
    Ok(trim_trailing_zeros(coerce_number(value)?))
}

fn trim_trailing_zeros(value: f64) -> String {
    let mut s = format!("{}", value);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn html_escapes_like_go_docs() {
        let functions = text_template_functions();
        let tmpl =
            Template::parse_with_functions("html", r#"{{html "<b>\"Bob\"</b>"}}"#, functions)
                .unwrap();
        let result = tmpl.render(&json!({})).unwrap();
        assert_eq!(result, "&lt;b&gt;&#34;Bob&#34;&lt;/b&gt;");
    }

    #[test]
    fn js_escapes_quotes_and_tags() {
        let functions = text_template_functions();
        let tmpl =
            Template::parse_with_functions("js", r#"{{js "</script>"}}"#, functions).unwrap();
        let result = tmpl.render(&json!({})).unwrap();
        assert_eq!(result, "\\u003C/script\\u003E");
    }

    #[test]
    fn urlquery_encodes_spaces_as_plus() {
        let functions = text_template_functions();
        let tmpl = Template::parse_with_functions(
            "urlquery",
            r#"{{urlquery "Hello, world!"}}"#,
            functions,
        )
        .unwrap();
        let result = tmpl.render(&json!({})).unwrap();
        assert_eq!(result, "Hello%2C+world%21");
    }

    #[test]
    fn print_concatenates_arguments() {
        let functions = text_template_functions();
        let tmpl =
            Template::parse_with_functions("print", r#"{{print "Hello" 23}}"#, functions).unwrap();
        let result = tmpl.render(&json!({})).unwrap();
        assert_eq!(result, "Hello23");
    }

    #[test]
    fn println_adds_spaces_and_newline() {
        let functions = text_template_functions();
        let tmpl =
            Template::parse_with_functions("println", r#"{{println "Hello" 23}}"#, functions)
                .unwrap();
        let result = tmpl.render(&json!({})).unwrap();
        assert_eq!(result, "Hello 23\n");
    }

    #[test]
    fn len_counts_elements() {
        let functions = text_template_functions();
        let tmpl = Template::parse_with_functions("len", r#"{{len .items}}"#, functions).unwrap();
        let result = tmpl.render(&json!({ "items": [1, 2, 3] })).unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn slice_subsets_array() {
        let functions = text_template_functions();
        let tmpl = Template::parse_with_functions("slice", r#"{{slice .word "1" "3"}}"#, functions)
            .unwrap();
        let result = tmpl.render(&json!({ "word": "rustacean" })).unwrap();
        assert_eq!(result, "us");
    }

    #[test]
    fn parsed_specifier_formats_percent_literal_without_argument() {
        let mut chars = "%".chars();
        let specifier = ParsedSpecifier::parse(&mut chars).unwrap();
        assert!(!specifier.needs_argument());
        assert_eq!(specifier.format(None).unwrap(), "%");
    }

    #[test]
    fn parsed_specifier_uses_expected_strategy() {
        let mut string_chars = "s".chars();
        let string_spec = ParsedSpecifier::parse(&mut string_chars).unwrap();
        assert!(string_spec.needs_argument());
        assert_eq!(string_spec.format(Some(&json!("value"))).unwrap(), "value");

        let mut int_chars = "d".chars();
        let int_spec = ParsedSpecifier::parse(&mut int_chars).unwrap();
        assert_eq!(int_spec.format(Some(&json!(42))).unwrap(), "42");

        let mut float_chars = "f".chars();
        let float_spec = ParsedSpecifier::parse(&mut float_chars).unwrap();
        assert_eq!(float_spec.format(Some(&json!(2.5000))).unwrap(), "2.5");

        let mut fallback_chars = "q".chars();
        let fallback_spec = ParsedSpecifier::parse(&mut fallback_chars).unwrap();
        assert_eq!(
            fallback_spec.format(Some(&json!("fallback"))).unwrap(),
            "%qfallback"
        );
    }

    #[test]
    fn parsed_specifier_requires_argument_when_missing() {
        let mut chars = "s".chars();
        let specifier = ParsedSpecifier::parse(&mut chars).unwrap();
        assert!(specifier.format(None).is_err());
    }

    #[test]
    fn parse_slice_indices_accepts_numeric_and_string_indices() {
        let indices = vec![json!(1), json!("3")];
        let SliceIndices { start, end } = parse_slice_indices(&indices, 10).unwrap();
        assert_eq!((start, end), (1, 3));
    }

    #[test]
    fn parse_slice_indices_rejects_negative_indices() {
        let indices = vec![json!(-1)];
        assert!(parse_slice_indices(&indices, 5).is_err());
    }

    #[test]
    fn call_invokes_registered_function() {
        let mut builder = FunctionRegistryBuilder::new();
        install_text_template_functions(&mut builder);
        builder.register("greet", |_ctx, args| {
            let name = args.first().and_then(|v| v.as_str()).unwrap_or("friend");
            Ok(Value::String(format!("Hello, {name}!")))
        });
        let registry = builder.build();
        let tmpl =
            Template::parse_with_functions("call", r#"{{call "greet" "Rust"}}"#, registry).unwrap();
        let result = tmpl.render(&json!({})).unwrap();
        assert_eq!(result, "Hello, Rust!");
    }
}
