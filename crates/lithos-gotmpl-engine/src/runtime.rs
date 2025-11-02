// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Number, Value};

use crate::ast::{BindingKind, Command, Expression, Pipeline};
use crate::error::Error;

/// Signature implemented by helper functions invoked from templates.
pub type Function = dyn Fn(&mut EvalContext, &[Value]) -> Result<Value, Error> + Send + Sync;

/// Registry that maps helper names to callable functions.
#[derive(Clone, Default)]
pub struct FunctionRegistry {
    map: Arc<HashMap<String, Arc<Function>>>,
}

impl FunctionRegistry {
    /// Creates an empty registry.
    pub fn empty() -> Self {
        Self {
            map: Arc::new(HashMap::new()),
        }
    }

    /// Returns a new builder for constructing registries.
    pub fn builder() -> FunctionRegistryBuilder {
        FunctionRegistryBuilder::new()
    }

    /// Builds a registry from a previously configured builder.
    pub fn from_builder(builder: FunctionRegistryBuilder) -> Self {
        builder.build()
    }

    /// Fetches a helper function by name.
    pub fn get(&self, name: &str) -> Option<Arc<Function>> {
        self.map.get(name).cloned()
    }

    /// Reports whether the registry contains no helper functions.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns a sorted list of the registered function names.
    pub fn function_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.map.keys().cloned().collect();
        names.sort();
        names
    }
}

/// Helper for constructing registries before freezing them into an immutable map.
#[derive(Default)]
pub struct FunctionRegistryBuilder {
    map: HashMap<String, Arc<Function>>,
}

impl FunctionRegistryBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Registers a helper function under the provided name.
    pub fn register<F>(&mut self, name: impl Into<String>, func: F) -> &mut Self
    where
        F: Fn(&mut EvalContext, &[Value]) -> Result<Value, Error> + Send + Sync + 'static,
    {
        self.map.insert(name.into(), Arc::new(func));
        self
    }

    /// Extends the builder with all helpers from another registry.
    pub fn extend(&mut self, other: &FunctionRegistry) -> &mut Self {
        for (key, value) in other.map.iter() {
            self.map.insert(key.clone(), value.clone());
        }
        self
    }

    /// Finalises the builder into an immutable registry.
    pub fn build(self) -> FunctionRegistry {
        FunctionRegistry {
            map: Arc::new(self.map),
        }
    }
}

/// Execution context threaded through template evaluation.
pub struct EvalContext {
    stack: Vec<Value>,
    root: Value,
    variables: Vec<HashMap<String, Value>>,
    functions: FunctionRegistry,
}

enum CommandResolution {
    Function(Arc<Function>),
    Identifier(String),
    Expression,
}

impl EvalContext {
    /// Creates a new evaluation context seeded with the input data and helper registry.
    pub fn new(data: Value, functions: FunctionRegistry) -> Self {
        let mut variables = Vec::new();
        let mut scope = HashMap::new();
        scope.insert("$".to_string(), data.clone());
        variables.push(scope);

        Self {
            stack: vec![data.clone()],
            root: data,
            variables,
            functions,
        }
    }

    /// Retrieves a helper function by name, if registered.
    pub fn function(&self, name: &str) -> Option<Arc<Function>> {
        self.functions.get(name)
    }

    /// Pushes a new scope with the provided value at the top of the stack.
    pub fn push_scope(&mut self, value: Value) {
        self.stack.push(value);
        self.variables.push(self.new_scope());
    }

    /// Pops the current scope, restoring the previous context.
    pub fn pop_scope(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
        if self.variables.len() > 1 {
            self.variables.pop();
        }
    }

    fn new_scope(&self) -> HashMap<String, Value> {
        let mut scope = HashMap::new();
        scope.insert("$".to_string(), self.root.clone());
        scope
    }

    /// Evaluates a pipeline in the context and returns the resulting value.
    pub fn eval_pipeline(&mut self, pipeline: &Pipeline) -> Result<Value, Error> {
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

    fn eval_command(&mut self, command: &Command, input: Option<Value>) -> Result<Value, Error> {
        let resolution = self.resolve_command_target(command);
        let args = self.prepare_command_args(command, input, &resolution)?;
        self.execute_prepared_command(command, resolution, args)
    }

    fn resolve_command_target(&self, command: &Command) -> CommandResolution {
        if let Expression::Identifier(name) = &command.target {
            if let Some(func) = self.functions.get(name.as_str()) {
                CommandResolution::Function(func)
            } else {
                CommandResolution::Identifier(name.clone())
            }
        } else {
            CommandResolution::Expression
        }
    }

    fn prepare_command_args(
        &mut self,
        command: &Command,
        input: Option<Value>,
        resolution: &CommandResolution,
    ) -> Result<Vec<Value>, Error> {
        match resolution {
            CommandResolution::Function(_) => {
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
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match resolution {
            CommandResolution::Function(func) => func(self, &args),
            CommandResolution::Identifier(_) | CommandResolution::Expression => {
                debug_assert!(args.is_empty());
                self.eval_expression(&command.target)
            }
        }
    }

    fn eval_expression(&mut self, expr: &Expression) -> Result<Value, Error> {
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
            Expression::StringLiteral(value) => Ok(Value::String(value.clone())),
            Expression::NumberLiteral(text) => parse_number(text)
                .map(Value::Number)
                .ok_or_else(|| Error::render(format!("invalid number literal {text}"), None)),
            Expression::BoolLiteral(flag) => Ok(Value::Bool(*flag)),
            Expression::Nil => Ok(Value::Null),
        }
    }

    fn resolve_identifier(&self, name: &str) -> Value {
        for value in self.stack.iter().rev() {
            if let Value::Object(map) = value {
                if let Some(found) = map.get(name) {
                    return found.clone();
                }
            }
        }
        Value::Null
    }

    fn resolve_field(&self, parts: &[String]) -> Result<Value, Error> {
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
                    value = Self::project_field_segment(value, part)?;
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
            value = Self::project_field_segment(value, part)?;
        }

        Ok(value)
    }

    fn resolve_variable(&self, name: &str) -> Value {
        if name == "$" {
            return self.root.clone();
        }

        for scope in self.variables.iter().rev() {
            if let Some(value) = scope.get(name) {
                return value.clone();
            }
        }

        Value::Null
    }

    fn set_variable(&mut self, name: &str, kind: BindingKind, value: Value) -> Result<(), Error> {
        if name == "$" {
            return Err(Error::render("cannot assign to root variable", None));
        }

        match kind {
            BindingKind::Declare => {
                self.variables
                    .last_mut()
                    .expect("scope stack is non-empty")
                    .insert(name.to_string(), value);
                Ok(())
            }
            BindingKind::Assign => {
                for scope in self.variables.iter_mut().rev() {
                    if scope.contains_key(name) {
                        scope.insert(name.to_string(), value);
                        return Ok(());
                    }
                }
                Err(Error::render(format!("variable {name} not defined"), None))
            }
        }
    }

    fn project_field_segment(value: Value, part: &str) -> Result<Value, Error> {
        match value {
            Value::Object(map) => Ok(map.get(part).cloned().unwrap_or(Value::Null)),
            Value::Array(list) => {
                let index = part.parse::<usize>().map_err(|_| {
                    Error::render(format!("array index must be integer, got {part}"), None)
                })?;
                Ok(list.get(index).cloned().unwrap_or(Value::Null))
            }
            _ => Err(Error::render(
                format!("cannot access field {part} on non-container value"),
                None,
            )),
        }
    }

    pub(crate) fn apply_bindings(
        &mut self,
        pipeline: &Pipeline,
        value: &Value,
    ) -> Result<(), Error> {
        if let Some(decls) = &pipeline.declarations {
            if decls.variables.is_empty() {
                return Ok(());
            }

            if decls.variables.len() == 1 {
                self.set_variable(&decls.variables[0], decls.kind, value.clone())?;
            } else if let Value::Array(items) = value {
                for (idx, name) in decls.variables.iter().enumerate() {
                    let assigned = items.get(idx).cloned().unwrap_or(Value::Null);
                    self.set_variable(name, decls.kind, assigned)?;
                }
            } else {
                for name in &decls.variables {
                    self.set_variable(name, decls.kind, value.clone())?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn predeclare_bindings(&mut self, pipeline: &Pipeline) {
        if let Some(decls) = &pipeline.declarations {
            if decls.kind == BindingKind::Declare {
                for name in &decls.variables {
                    self.variables
                        .last_mut()
                        .expect("scope stack is non-empty")
                        .entry(name.clone())
                        .or_insert(Value::Null);
                }
            }
        }
    }

    pub(crate) fn assign_range_bindings(
        &mut self,
        pipeline: &Pipeline,
        key: Option<Value>,
        value: Value,
    ) -> Result<(), Error> {
        if let Some(decls) = &pipeline.declarations {
            match decls.variables.len() {
                0 => {}
                1 => {
                    self.set_variable(&decls.variables[0], decls.kind, value)?;
                }
                _ => {
                    let key_value = key.unwrap_or(Value::Null);
                    self.set_variable(&decls.variables[0], decls.kind, key_value)?;
                    if let Some(second) = decls.variables.get(1) {
                        self.set_variable(second, decls.kind, value)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registry_with_echo() -> FunctionRegistry {
        let mut builder = FunctionRegistry::builder();
        builder.register("echo", |_, args| {
            Ok(args.first().cloned().unwrap_or(Value::Null))
        });
        FunctionRegistry::from_builder(builder)
    }

    #[test]
    fn resolve_command_target_detects_function() {
        let registry = registry_with_echo();
        let ctx = EvalContext::new(Value::Null, registry);
        let command = Command::new(Expression::Identifier("echo".into()), Vec::new());

        assert!(matches!(
            ctx.resolve_command_target(&command),
            CommandResolution::Function(_)
        ));
    }

    #[test]
    fn resolve_command_target_identifies_expression() {
        let registry = FunctionRegistry::empty();
        let mut ctx = EvalContext::new(json!({"name": "lithos"}), registry);
        let command = Command::new(Expression::Identifier("name".into()), Vec::new());

        let resolution = ctx.resolve_command_target(&command);
        let args = ctx
            .prepare_command_args(&command, None, &resolution)
            .expect("identifier without args should succeed");
        let value = ctx
            .execute_prepared_command(&command, resolution, args)
            .expect("expression should evaluate");

        assert_eq!(value, json!("lithos"));
    }

    #[test]
    fn prepare_command_args_errors_on_unknown_function_with_args() {
        let registry = FunctionRegistry::empty();
        let mut ctx = EvalContext::new(Value::Null, registry);
        let command = Command::new(
            Expression::Identifier("missing".into()),
            vec![Expression::StringLiteral("arg".into())],
        );

        let resolution = ctx.resolve_command_target(&command);
        let err = ctx
            .prepare_command_args(&command, None, &resolution)
            .expect_err("should reject unknown function with args");
        assert!(err.to_string().contains("unknown function"));
    }

    #[test]
    fn prepare_command_args_collects_values_for_functions() {
        let registry = registry_with_echo();
        let mut ctx = EvalContext::new(Value::Null, registry);
        let command = Command::new(
            Expression::Identifier("echo".into()),
            vec![Expression::NumberLiteral("7".into())],
        );

        let resolution = ctx.resolve_command_target(&command);
        let args = ctx
            .prepare_command_args(&command, Some(Value::Bool(false)), &resolution)
            .expect("function arguments should prepare");

        assert_eq!(args.len(), 2);
        assert_eq!(args[1], Value::Bool(false));
    }

    #[test]
    fn execute_prepared_command_invokes_function() {
        let mut builder = FunctionRegistry::builder();
        builder.register("count", |_, args| {
            Ok(Value::Number(Number::from(args.len())))
        });
        let registry = FunctionRegistry::from_builder(builder);
        let mut ctx = EvalContext::new(Value::Null, registry);
        let command = Command::new(Expression::Identifier("count".into()), Vec::new());

        let resolution = ctx.resolve_command_target(&command);
        let args = ctx
            .prepare_command_args(&command, Some(Value::Null), &resolution)
            .expect("function arguments should prepare");
        let value = ctx
            .execute_prepared_command(&command, resolution, args)
            .expect("function should execute");

        assert_eq!(value, Value::Number(Number::from(1))); // includes piped value
    }

    #[test]
    fn prepare_command_args_rejects_piped_expression() {
        let registry = FunctionRegistry::empty();
        let mut ctx = EvalContext::new(Value::Null, registry);
        let command = Command::new(Expression::BoolLiteral(true), Vec::new());

        let resolution = ctx.resolve_command_target(&command);
        let err = ctx
            .prepare_command_args(&command, Some(Value::Null), &resolution)
            .expect_err("piping into expression should error");

        assert!(err
            .to_string()
            .contains("cannot pipe value into non-function expression"));
    }
}

pub fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                let mut s = n.to_string();
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
        }
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

pub fn parse_number(text: &str) -> Option<Number> {
    if !text.contains(['.', 'e', 'E']) {
        if let Ok(value) = text.parse::<i64>() {
            return Some(Number::from(value));
        }
        if let Ok(value) = text.parse::<u64>() {
            return Some(Number::from(value));
        }
    }

    text.parse::<f64>().ok().and_then(Number::from_f64)
}

pub fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i == 0
            } else if let Some(u) = n.as_u64() {
                u == 0
            } else {
                n.as_f64().map(|f| f == 0.0).unwrap_or(false)
            }
        }
        Value::String(s) => s.is_empty(),
        Value::Array(arr) => arr.iter().all(is_empty),
        Value::Object(map) => map.is_empty(),
    }
}

pub fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i != 0
            } else if let Some(u) = n.as_u64() {
                u != 0
            } else {
                n.as_f64().map(|f| f != 0.0).unwrap_or(false)
            }
        }
        Value::String(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

pub fn coerce_number(value: &Value) -> Result<f64, Error> {
    if let Some(i) = value.as_i64() {
        Ok(i as f64)
    } else if let Some(u) = value.as_u64() {
        Ok(u as f64)
    } else if let Some(f) = value.as_f64() {
        Ok(f)
    } else if let Some(s) = value.as_str() {
        s.parse::<f64>()
            .map_err(|_| Error::render("cannot convert string to number", None))
    } else {
        Err(Error::render("expected numeric value for comparison", None))
    }
}
