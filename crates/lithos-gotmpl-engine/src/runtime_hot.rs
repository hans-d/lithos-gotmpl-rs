// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use smallvec::SmallVec;

use crate::ast::{BindingKind, Command, Expression, Pipeline};
use crate::error::Error;
use crate::runtime::{self, EvalContext, FunctionRegistry, HelperEntry};
use crate::telemetry;

/// Borrow-aware value holder used by the hot runtime.
#[derive(Debug, Clone)]
pub enum ValueSlot<'a> {
    Borrowed(&'a Value),
    Owned(Value),
    Temp(Cow<'a, Value>),
}

impl<'a> ValueSlot<'a> {
    pub fn borrowed(value: &'a Value) -> Self {
        Self::Borrowed(value)
    }

    pub fn owned(value: Value) -> Self {
        Self::Owned(value)
    }

    pub fn as_value(&self) -> &Value {
        match self {
            ValueSlot::Borrowed(value) => value,
            ValueSlot::Owned(value) => value,
            ValueSlot::Temp(cow) => cow.as_ref(),
        }
    }

    pub fn into_owned(self) -> Value {
        match self {
            ValueSlot::Borrowed(value) => value.clone(),
            ValueSlot::Owned(value) => value,
            ValueSlot::Temp(cow) => cow.into_owned(),
        }
    }
}

/// Read-only view handed to fast helper functions.
#[derive(Debug, Clone)]
pub struct ValueView<'a> {
    slot: ValueSlot<'a>,
}

impl<'a> ValueView<'a> {
    pub fn new(slot: ValueSlot<'a>) -> Self {
        Self { slot }
    }

    pub fn as_value(&self) -> &Value {
        self.slot.as_value()
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.as_value().as_bool()
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_value().as_str()
    }

    pub fn as_number(&self) -> Option<&serde_json::Number> {
        self.as_value().as_number()
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        self.as_value().as_array()
    }

    pub fn as_object(&self) -> Option<&serde_json::Map<String, Value>> {
        self.as_value().as_object()
    }

    pub fn to_string_fast(&self) -> String {
        runtime::value_to_string(self.as_value())
    }

    pub fn into_owned(self) -> Value {
        self.slot.into_owned()
    }
}

/// Snapshot of the legacy context used when bridging helper invocations.
pub struct LegacySnapshot {
    pub root: Value,
    pub stack: Vec<Value>,
    pub variables: Vec<HashMap<String, Value>>,
}

#[derive(Clone)]
enum CommandResolution {
    Helper { entry: HelperEntry, name: String },
    Identifier(String),
    Expression,
}

/// Signature implemented by hot helper functions.
pub type FastFunction = dyn for<'a> Fn(&mut EvalContextHot<'a>, &[ValueView<'a>]) -> Result<ValueSlot<'a>, Error>
    + Send
    + Sync;

/// Execution context for the hot runtime.
#[allow(dead_code)]
pub struct EvalContextHot<'a> {
    root: &'a Value,
    stack: SmallVec<[ValueSlot<'a>; 4]>,
    variables: Vec<HashMap<String, ValueSlot<'a>>>,
    functions: FunctionRegistry,
    #[allow(dead_code)]
    scratch: Vec<Value>,
}

impl<'a> EvalContextHot<'a> {
    pub fn new(root: &'a Value, functions: FunctionRegistry) -> Self {
        let mut stack = SmallVec::new();
        stack.push(ValueSlot::borrowed(root));

        let mut variables = Vec::new();
        let mut scope = HashMap::new();
        scope.insert("$".to_string(), ValueSlot::borrowed(root));
        variables.push(scope);

        Self {
            root,
            stack,
            variables,
            functions,
            scratch: Vec::new(),
        }
    }

    pub fn root(&self) -> &'a Value {
        self.root
    }

    pub fn functions(&self) -> FunctionRegistry {
        self.functions.clone()
    }

    fn legacy_args(&mut self, args: &[ValueView<'a>]) -> &[Value] {
        self.scratch.clear();
        self.scratch
            .extend(args.iter().map(|view| view.as_value().clone()));
        self.scratch.as_slice()
    }

    pub fn push_scope_slot(&mut self, slot: ValueSlot<'a>) {
        self.stack.push(slot.clone());
        let mut scope = HashMap::new();
        scope.insert("$".to_string(), ValueSlot::borrowed(self.root));
        scope.insert(".".to_string(), slot);
        self.variables.push(scope);
    }

    pub fn pop_scope(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
        if self.variables.len() > 1 {
            self.variables.pop();
        }
    }

    pub fn assign_variable(
        &mut self,
        name: &str,
        kind: BindingKind,
        value: ValueSlot<'a>,
    ) -> Result<(), Error> {
        if name == "$" {
            return Err(Error::render("cannot assign to root variable", None));
        }
        match kind {
            BindingKind::Declare => {
                if let Some(scope) = self.variables.last_mut() {
                    scope.insert(name.to_string(), value);
                    Ok(())
                } else {
                    Err(Error::render("scope stack is empty", None))
                }
            }
            BindingKind::Assign => {
                for scope in self.variables.iter_mut().rev() {
                    if scope.contains_key(name) {
                        scope.insert(name.to_string(), value.clone());
                        return Ok(());
                    }
                }
                Err(Error::render(format!("variable {name} not defined"), None))
            }
        }
    }

    pub fn apply_bindings(
        &mut self,
        pipeline: &Pipeline,
        value: &ValueSlot<'a>,
    ) -> Result<(), Error> {
        if let Some(decls) = &pipeline.declarations {
            match decls.variables.len() {
                0 => {}
                1 => self.assign_variable(&decls.variables[0], decls.kind, value.clone())?,
                _ => match value.as_value() {
                    Value::Array(items) => {
                        for (idx, name) in decls.variables.iter().enumerate() {
                            let assigned = items.get(idx).cloned().unwrap_or(Value::Null);
                            self.assign_variable(name, decls.kind, ValueSlot::owned(assigned))?;
                        }
                    }
                    _ => {
                        for name in &decls.variables {
                            self.assign_variable(
                                name,
                                decls.kind,
                                ValueSlot::owned(value.as_value().clone()),
                            )?;
                        }
                    }
                },
            }
        }
        Ok(())
    }

    pub fn predeclare_bindings(&mut self, pipeline: &Pipeline) {
        if let Some(decls) = &pipeline.declarations {
            if decls.kind == BindingKind::Declare {
                for name in &decls.variables {
                    self.variables
                        .last_mut()
                        .expect("scope stack is non-empty")
                        .entry(name.clone())
                        .or_insert(ValueSlot::owned(Value::Null));
                }
            }
        }
    }

    pub fn assign_range_bindings(
        &mut self,
        pipeline: &Pipeline,
        key: Option<Value>,
        value: Value,
    ) -> Result<(), Error> {
        if let Some(decls) = &pipeline.declarations {
            match decls.variables.len() {
                0 => {}
                1 => {
                    self.assign_variable(
                        &decls.variables[0],
                        decls.kind,
                        ValueSlot::owned(value.clone()),
                    )?;
                }
                _ => {
                    if let Some(key_name) = decls.variables.get(0) {
                        let key_value = key.clone().unwrap_or(Value::Null);
                        self.assign_variable(key_name, decls.kind, ValueSlot::owned(key_value))?;
                    }
                    if let Some(value_name) = decls.variables.get(1) {
                        self.assign_variable(value_name, decls.kind, ValueSlot::owned(value))?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn eval_pipeline(&mut self, pipeline: &Pipeline) -> Result<ValueSlot<'a>, Error> {
        let mut iter = pipeline.commands.iter();
        let first = iter
            .next()
            .ok_or_else(|| Error::render("empty pipeline", None))?;
        let mut value = self.eval_command(first, None)?;

        for command in iter {
            value = self.eval_command(command, Some(value))?;
        }

        Ok(value)
    }

    pub fn variables(&self) -> &Vec<HashMap<String, ValueSlot<'a>>> {
        &self.variables
    }

    pub fn snapshot(&self) -> LegacySnapshot {
        let stack = self
            .stack
            .iter()
            .map(|slot| slot.as_value().clone())
            .collect();
        let variables = self
            .variables
            .iter()
            .map(|scope| {
                scope
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_value().clone()))
                    .collect::<HashMap<_, _>>()
            })
            .collect();
        LegacySnapshot {
            root: self.root.clone(),
            stack,
            variables,
        }
    }

    fn eval_command(
        &mut self,
        command: &Command,
        input: Option<ValueSlot<'a>>,
    ) -> Result<ValueSlot<'a>, Error> {
        let resolution = self.resolve_command_target(command);
        let args = self.prepare_command_args(command, input, &resolution)?;
        self.execute_prepared_command(command, resolution, args)
    }

    fn resolve_command_target(&self, command: &Command) -> CommandResolution {
        if let Expression::Identifier(name) = &command.target {
            if let Some(entry) = self.functions.get_entry(name) {
                return CommandResolution::Helper {
                    entry,
                    name: name.clone(),
                };
            }
            return CommandResolution::Identifier(name.clone());
        }
        CommandResolution::Expression
    }

    fn prepare_command_args(
        &mut self,
        command: &Command,
        input: Option<ValueSlot<'a>>,
        resolution: &CommandResolution,
    ) -> Result<Vec<ValueSlot<'a>>, Error> {
        match resolution {
            CommandResolution::Helper { .. } => {
                let mut args =
                    Vec::with_capacity(command.args.len() + usize::from(input.is_some()));
                for expr in &command.args {
                    args.push(self.eval_expression(expr)?);
                }
                if let Some(prev) = input {
                    args.push(prev);
                }
                Ok(args)
            }
            CommandResolution::Identifier(name) => {
                if !command.args.is_empty() || input.is_some() {
                    return Err(Error::render(format!("unknown function \"{name}\""), None));
                }
                Ok(Vec::new())
            }
            CommandResolution::Expression => {
                if !command.args.is_empty() {
                    return Err(Error::render(
                        "arguments supplied to non-function expression",
                        None,
                    ));
                }
                if input.is_some() {
                    return Err(Error::render(
                        "cannot pipe value into non-function expression",
                        None,
                    ));
                }
                Ok(Vec::new())
            }
        }
    }

    fn execute_prepared_command(
        &mut self,
        command: &Command,
        resolution: CommandResolution,
        args: Vec<ValueSlot<'a>>,
    ) -> Result<ValueSlot<'a>, Error> {
        match resolution {
            CommandResolution::Helper { entry, name } => {
                let views: Vec<ValueView<'a>> = args.into_iter().map(ValueView::new).collect();
                let kind = entry.telemetry_kind();
                let result = entry.invoke_hot(self, &views);
                telemetry::record_helper_invocation(&name, kind, result.is_ok());
                result
            }
            CommandResolution::Identifier(_) | CommandResolution::Expression => {
                debug_assert!(args.is_empty());
                self.eval_expression(&command.target)
            }
        }
    }

    fn eval_expression(&mut self, expr: &Expression) -> Result<ValueSlot<'a>, Error> {
        match expr {
            Expression::Identifier(name) => Ok(self.resolve_identifier(name)),
            Expression::Field(parts) => self.resolve_field(parts),
            Expression::Variable(name) => Ok(self.resolve_variable(name)),
            Expression::PipelineExpr(pipeline) => {
                if pipeline.declarations.is_some() {
                    return Err(Error::render(
                        "pipeline declarations not allowed in expression",
                        None,
                    ));
                }
                self.eval_pipeline(pipeline)
            }
            Expression::StringLiteral(value) => Ok(ValueSlot::owned(Value::String(value.clone()))),
            Expression::NumberLiteral(text) => runtime::parse_number(text)
                .map(|n| ValueSlot::owned(Value::Number(n)))
                .ok_or_else(|| Error::render(format!("invalid number literal {text}"), None)),
            Expression::BoolLiteral(flag) => Ok(ValueSlot::owned(Value::Bool(*flag))),
            Expression::Nil => Ok(ValueSlot::owned(Value::Null)),
        }
    }

    fn resolve_identifier(&self, name: &str) -> ValueSlot<'a> {
        for slot in self.stack.iter().rev() {
            if let Value::Object(map) = slot.as_value() {
                if let Some(found) = map.get(name) {
                    return ValueSlot::owned(found.clone());
                }
            }
        }
        ValueSlot::owned(Value::Null)
    }

    fn resolve_field(&self, parts: &[String]) -> Result<ValueSlot<'a>, Error> {
        if parts.is_empty() {
            return self
                .stack
                .last()
                .cloned()
                .ok_or_else(|| Error::render("dot resolution failed", None));
        }
        if let Some(first) = parts.first() {
            if first.starts_with('$') {
                let mut value = self.resolve_variable(first);
                for part in parts.iter().skip(1) {
                    value = self.project_field_segment(value.as_value(), part)?;
                }
                return Ok(value);
            }
        }

        let mut value = self
            .stack
            .last()
            .cloned()
            .ok_or_else(|| Error::render("dot resolution failed", None))?;

        for part in parts {
            value = self.project_field_segment(value.as_value(), part)?;
        }

        Ok(value)
    }

    fn project_field_segment(&self, value: &Value, part: &str) -> Result<ValueSlot<'a>, Error> {
        match value {
            Value::Object(map) => Ok(ValueSlot::owned(
                map.get(part).cloned().unwrap_or(Value::Null),
            )),
            Value::Array(list) => {
                let index = part.parse::<usize>().map_err(|_| {
                    Error::render(format!("array index must be integer, got {part}"), None)
                })?;
                Ok(ValueSlot::owned(
                    list.get(index).cloned().unwrap_or(Value::Null),
                ))
            }
            _ => Err(Error::render(
                format!("cannot access field {part} on non-container value"),
                None,
            )),
        }
    }

    fn resolve_variable(&self, name: &str) -> ValueSlot<'a> {
        for scope in self.variables.iter().rev() {
            if let Some(value) = scope.get(name) {
                return value.clone();
            }
        }
        ValueSlot::owned(Value::Null)
    }
}

pub(crate) fn invoke_legacy_helper<'a>(
    func: Arc<runtime::Function>,
    ctx: &mut EvalContextHot<'a>,
    args: &[ValueView<'a>],
) -> Result<ValueSlot<'a>, Error> {
    let snapshot = ctx.snapshot();
    let mut legacy_ctx = EvalContext::from_snapshot(snapshot, ctx.functions());
    let owned_args = ctx.legacy_args(args);
    func(&mut legacy_ctx, owned_args).map(ValueSlot::owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BindingKind, Command, Expression, Pipeline, PipelineDeclarations};
    use crate::runtime::FunctionRegistryBuilder;
    use serde_json::json;

    fn pipeline_for_helper(name: &str, args: Vec<Expression>) -> Pipeline {
        Pipeline::new(
            None,
            vec![Command::new(Expression::Identifier(name.into()), args)],
        )
    }

    #[test]
    fn apply_bindings_destructures_arrays() -> Result<(), Error> {
        let data = json!({});
        let mut ctx = EvalContextHot::new(&data, FunctionRegistry::empty());
        let decls =
            PipelineDeclarations::new(BindingKind::Declare, vec!["first".into(), "second".into()]);
        let pipeline = Pipeline::new(Some(decls), Vec::new());
        let value = ValueSlot::owned(Value::Array(vec![Value::from(1), Value::from(2)]));
        ctx.apply_bindings(&pipeline, &value)?;
        let scope = ctx.variables().last().unwrap();
        assert_eq!(scope.get("first").unwrap().as_value(), &Value::from(1));
        assert_eq!(scope.get("second").unwrap().as_value(), &Value::from(2));
        Ok(())
    }

    #[test]
    fn assign_range_bindings_sets_key_and_value() -> Result<(), Error> {
        let data = json!({});
        let mut ctx = EvalContextHot::new(&data, FunctionRegistry::empty());
        let decls = PipelineDeclarations::new(BindingKind::Declare, vec!["k".into(), "v".into()]);
        let pipeline = Pipeline::new(Some(decls), Vec::new());
        ctx.assign_range_bindings(&pipeline, Some(Value::from(5)), Value::from(42))?;
        let scope = ctx.variables().last().unwrap();
        assert_eq!(scope.get("k").unwrap().as_value(), &Value::from(5));
        assert_eq!(scope.get("v").unwrap().as_value(), &Value::from(42));
        Ok(())
    }

    #[test]
    fn legacy_helper_invocation_round_trips() -> Result<(), Error> {
        let mut builder = FunctionRegistryBuilder::new();
        builder.register("echo", |_, args| {
            let joined = args
                .iter()
                .map(|v| v.as_str().unwrap_or_default())
                .collect::<String>();
            Ok(Value::String(joined))
        });
        let registry = FunctionRegistry::from_builder(builder);
        let data = json!({});
        let mut ctx = EvalContextHot::new(&data, registry);
        let pipeline = pipeline_for_helper(
            "echo",
            vec![
                Expression::StringLiteral("foo".into()),
                Expression::StringLiteral("bar".into()),
            ],
        );
        let result = ctx.eval_pipeline(&pipeline)?;
        assert_eq!(result.as_value(), &Value::String("foobar".into()));
        Ok(())
    }

    #[test]
    fn fast_helper_receives_borrowed_view() -> Result<(), Error> {
        let mut builder = FunctionRegistryBuilder::new();
        builder.register_fast("shout", |_ctx, args| {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].as_str(), Some("world"));
            Ok(ValueSlot::owned(Value::String(
                args[0].to_string_fast().to_uppercase(),
            )))
        });
        let registry = FunctionRegistry::from_builder(builder);
        let data = json!({
            "name": "world"
        });
        let mut ctx = EvalContextHot::new(&data, registry);
        let pipeline = pipeline_for_helper("shout", vec![Expression::Field(vec!["name".into()])]);
        let result = ctx.eval_pipeline(&pipeline)?;
        assert_eq!(result.as_value(), &Value::String("WORLD".into()));
        Ok(())
    }
}
