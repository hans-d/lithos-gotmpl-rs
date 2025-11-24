#![forbid(unsafe_code)]
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Core utilities for parsing and rendering Go-style text templates in Rust.
//!
//! This crate is work-in-progress: it currently provides AST inspection and a
//! placeholder renderer while the full evaluator is implemented.

pub mod analyze;
pub mod ast;
mod error;
pub mod lexer;
mod parser;
mod runtime;

pub use analyze::{
    analyze_template, AnalysisIssue, Certainty, ControlKind, ControlUsage, FunctionCall,
    FunctionSource, Precision, TemplateAnalysis, TemplateCall, VariableAccess, VariableKind,
};
pub use ast::{
    ActionNode, Ast, BindingKind, Block, Command, CommentNode, ElseIfBranch, Expression, IfNode,
    Node, Pipeline, PipelineDeclarations, RangeNode, Span, TextNode, WithNode,
};
pub use error::Error;
pub use lexer::{Keyword, Operator, Token, TokenKind};
pub use runtime::{
    coerce_number, is_empty, is_truthy, value_to_string, EvalContext, Function, FunctionRegistry,
    FunctionRegistryBuilder,
};

use serde_json::{Number, Value};
use std::fmt;

/// Parsed template with associated AST and original source.
#[derive(Clone)]
pub struct Template {
    name: String,
    source: String,
    ast: Ast,
    functions: FunctionRegistry,
}

impl fmt::Debug for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Template")
            .field("name", &self.name)
            .field("source", &self.source)
            .finish()
    }
}

impl Template {
    /// Parses template source into an AST representation.
    pub fn parse_str(name: &str, source: &str) -> Result<Self, Error> {
        Self::parse_with_functions(name, source, FunctionRegistry::empty())
    }

    /// Parses template source and associates it with a registry of functions.
    pub fn parse_with_functions(
        name: &str,
        source: &str,
        functions: FunctionRegistry,
    ) -> Result<Self, Error> {
        let ast = parser::parse_template(name, source)?;
        Ok(Self {
            name: name.to_string(),
            source: source.to_string(),
            ast,
            functions,
        })
    }

    /// Returns a clone of the function registry in use.
    pub fn functions(&self) -> FunctionRegistry {
        self.functions.clone()
    }

    /// Replaces the function registry associated with this template.
    pub fn set_functions(&mut self, functions: FunctionRegistry) {
        self.functions = functions;
    }

    /// Consumes the template and returns a new instance with the provided function registry.
    pub fn with_functions(mut self, functions: FunctionRegistry) -> Self {
        self.functions = functions;
        self
    }

    /// Returns the original template name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the original template source.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns a reference to the parsed AST.
    pub fn ast(&self) -> &Ast {
        &self.ast
    }

    /// Runs structural analysis over the template and returns helper usage metadata.
    pub fn analyze(&self) -> TemplateAnalysis {
        analyze::analyze_template(&self.ast, Some(&self.functions))
    }

    /// Returns a canonical string representation of the parsed template, similar to Go's
    /// `parse.Tree.Root.String()` output.
    pub fn to_template_string(&self) -> String {
        let mut out = String::new();
        Self::write_block(&mut out, &self.ast.root);
        out
    }

    fn write_block(out: &mut String, block: &Block) {
        for node in &block.nodes {
            match node {
                Node::Text(text) => out.push_str(&text.text),
                Node::Comment(comment) => out.push_str(&comment.to_template_fragment()),
                Node::Action(action) => out.push_str(&action.to_template_fragment()),
                Node::If(if_node) => {
                    out.push_str("{{if ");
                    out.push_str(&pipeline_to_string(&if_node.pipeline));
                    out.push_str("}}");
                    Self::write_block(out, &if_node.then_block);
                    for branch in &if_node.else_if_branches {
                        out.push_str("{{else if ");
                        out.push_str(&pipeline_to_string(&branch.pipeline));
                        out.push_str("}}");
                        Self::write_block(out, &branch.block);
                    }
                    if let Some(else_block) = &if_node.else_block {
                        out.push_str("{{else}}");
                        Self::write_block(out, else_block);
                    }
                    out.push_str("{{end}}");
                }
                Node::Range(range_node) => {
                    out.push_str("{{range ");
                    out.push_str(&pipeline_to_string(&range_node.pipeline));
                    out.push_str("}}");
                    Self::write_block(out, &range_node.then_block);
                    if let Some(else_block) = &range_node.else_block {
                        out.push_str("{{else}}");
                        Self::write_block(out, else_block);
                    }
                    out.push_str("{{end}}");
                }
                Node::With(with_node) => {
                    out.push_str("{{with ");
                    out.push_str(&pipeline_to_string(&with_node.pipeline));
                    out.push_str("}}");
                    Self::write_block(out, &with_node.then_block);
                    if let Some(else_block) = &with_node.else_block {
                        out.push_str("{{else}}");
                        Self::write_block(out, else_block);
                    }
                    out.push_str("{{end}}");
                }
            }
        }
    }

    /// Renders the template against the provided data.
    pub fn render(&self, data: &Value) -> Result<String, Error> {
        let mut ctx = runtime::EvalContext::new(data.clone(), self.functions.clone());
        let mut output = String::new();
        Self::render_block(&mut ctx, &self.ast.root, &mut output)?;
        Ok(output)
    }

    fn render_block(
        ctx: &mut runtime::EvalContext,
        block: &Block,
        output: &mut String,
    ) -> Result<(), Error> {
        for node in &block.nodes {
            match node {
                Node::Text(text) => output.push_str(&text.text),
                Node::Comment(_) => {}
                Node::Action(action) => {
                    let value = ctx.eval_pipeline(&action.pipeline)?;
                    ctx.apply_bindings(&action.pipeline, &value)?;
                    if action.pipeline.declarations.is_none() {
                        output.push_str(&runtime::value_to_string(&value));
                    }
                }
                Node::If(if_node) => Self::render_if(ctx, if_node, output)?,
                Node::Range(range_node) => Self::render_range(ctx, range_node, output)?,
                Node::With(with_node) => Self::render_with(ctx, with_node, output)?,
            }
        }
        Ok(())
    }

    fn render_if(
        ctx: &mut runtime::EvalContext,
        node: &crate::ast::IfNode,
        output: &mut String,
    ) -> Result<(), Error> {
        let value = ctx.eval_pipeline(&node.pipeline)?;
        ctx.apply_bindings(&node.pipeline, &value)?;
        if runtime::is_truthy(&value) {
            Self::render_block(ctx, &node.then_block, output)?;
        } else {
            for branch in &node.else_if_branches {
                let branch_value = ctx.eval_pipeline(&branch.pipeline)?;
                ctx.apply_bindings(&branch.pipeline, &branch_value)?;
                if runtime::is_truthy(&branch_value) {
                    Self::render_block(ctx, &branch.block, output)?;
                    return Ok(());
                }
            }
            if let Some(else_block) = &node.else_block {
                Self::render_block(ctx, else_block, output)?;
            }
        }
        Ok(())
    }

    fn render_range(
        ctx: &mut runtime::EvalContext,
        node: &crate::ast::RangeNode,
        output: &mut String,
    ) -> Result<(), Error> {
        ctx.predeclare_bindings(&node.pipeline);
        let value = ctx.eval_pipeline(&node.pipeline)?;

        let mut iterated = false;

        match value {
            Value::Array(items) => {
                if items.is_empty() {
                    // handled later for else
                } else {
                    for (index, item) in items.iter().enumerate() {
                        let key_value = Value::Number(Number::from(index as u64));
                        ctx.assign_range_bindings(&node.pipeline, Some(key_value), item.clone())?;
                        ctx.push_scope(item.clone());
                        let render_result = Self::render_block(ctx, &node.then_block, output);
                        ctx.pop_scope();
                        render_result?;
                        iterated = true;
                    }
                }
            }
            Value::Object(map) => {
                if map.is_empty() {
                    // handled later
                } else {
                    for (key, val) in map.iter() {
                        let key_value = Value::String(key.clone());
                        ctx.assign_range_bindings(&node.pipeline, Some(key_value), val.clone())?;
                        ctx.push_scope(val.clone());
                        let render_result = Self::render_block(ctx, &node.then_block, output);
                        ctx.pop_scope();
                        render_result?;
                        iterated = true;
                    }
                }
            }
            _ => {}
        }

        if !iterated {
            ctx.assign_range_bindings(&node.pipeline, None, Value::Null)?;
            if let Some(else_block) = &node.else_block {
                Self::render_block(ctx, else_block, output)?;
            }
        }

        Ok(())
    }

    fn render_with(
        ctx: &mut runtime::EvalContext,
        node: &crate::ast::WithNode,
        output: &mut String,
    ) -> Result<(), Error> {
        let value = ctx.eval_pipeline(&node.pipeline)?;
        ctx.apply_bindings(&node.pipeline, &value)?;
        if runtime::is_truthy(&value) {
            ctx.push_scope(value.clone());
            let render_result = Self::render_block(ctx, &node.then_block, output);
            ctx.pop_scope();
            render_result?;
        } else if let Some(else_block) = &node.else_block {
            Self::render_block(ctx, else_block, output)?;
        }
        Ok(())
    }
}

fn pipeline_to_string(pipeline: &Pipeline) -> String {
    let mut out = String::new();
    if let Some(decls) = &pipeline.declarations {
        out.push_str(&decls.variables.join(", "));
        out.push(' ');
        out.push_str(match decls.kind {
            BindingKind::Declare => ":=",
            BindingKind::Assign => "=",
        });
        out.push(' ');
    }

    for (idx, command) in pipeline.commands.iter().enumerate() {
        if idx > 0 {
            out.push_str(" | ");
        }
        out.push_str(&expression_to_string(&command.target));
        for arg in &command.args {
            out.push(' ');
            out.push_str(&expression_to_string(arg));
        }
    }

    out
}

fn expression_to_string(expr: &Expression) -> String {
    match expr {
        Expression::Identifier(name) => name.clone(),
        Expression::Field(parts) => {
            if parts.is_empty() {
                ".".to_string()
            } else {
                format!(".{}", parts.join("."))
            }
        }
        Expression::Variable(name) => name.clone(),
        Expression::PipelineExpr(pipeline) => {
            format!("({})", pipeline_to_string(pipeline))
        }
        Expression::StringLiteral(value) => format!("\"{}\"", value),
        Expression::NumberLiteral(value) => value.clone(),
        Expression::BoolLiteral(flag) => flag.to_string(),
        Expression::Nil => "nil".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn renders_with_custom_registry() {
        let mut builder = FunctionRegistry::builder();
        builder.register("greet", |_ctx, args| {
            let name = args
                .first()
                .cloned()
                .unwrap_or_else(|| Value::String("friend".into()));
            Ok(Value::String(format!("Hello, {}!", value_to_string(&name))))
        });
        let registry = builder.build();

        let tmpl = Template::parse_with_functions("test", "{{greet .name}}", registry).unwrap();
        let rendered = tmpl.render(&json!({"name": "Hans"})).unwrap();
        assert_eq!(rendered, "Hello, Hans!");
    }

    #[test]
    fn missing_function_is_error() {
        let tmpl = Template::parse_str("missing", "{{unknown .}} ").unwrap();
        let err = tmpl.render(&json!(1)).unwrap_err();
        assert!(err.to_string().contains("unknown function"));
    }

    #[test]
    fn parse_error_on_unclosed_action() {
        let err = Template::parse_str("bad", "{{ \"d\" }").unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
        assert!(err.to_string().contains("unclosed action"));
    }

    #[test]
    fn raw_string_literal_roundtrip() {
        let tmpl = Template::parse_str("raw", "{{ `{{ \"d\" }` }}").unwrap();
        let output = tmpl.render(&json!({})).unwrap();
        assert_eq!(output, "{{ \"d\" }");
    }

    #[test]
    fn renders_if_else_branches() {
        let tmpl = Template::parse_str("if", "{{if .flag}}yes{{else}}no{{end}}").unwrap();
        let rendered_true = tmpl.render(&json!({"flag": true})).unwrap();
        let rendered_false = tmpl.render(&json!({"flag": false})).unwrap();
        assert_eq!(rendered_true, "yes");
        assert_eq!(rendered_false, "no");
    }

    #[test]
    fn renders_range_over_arrays() {
        let tmpl =
            Template::parse_str("range", "{{range .items}}{{.}},{{else}}empty{{end}}").unwrap();
        let rendered = tmpl.render(&json!({"items": ["a", "b"]})).unwrap();
        assert_eq!(rendered, "a,b,");

        let empty = tmpl.render(&json!({"items": []})).unwrap();
        assert_eq!(empty, "empty");
    }

    #[test]
    fn renders_with_changes_context() {
        let tmpl =
            Template::parse_str("with", "{{with .user}}{{.name}}{{else}}missing{{end}}").unwrap();
        let rendered = tmpl.render(&json!({"user": {"name": "Lithos"}})).unwrap();
        assert_eq!(rendered, "Lithos");

        let missing = tmpl.render(&json!({"user": null})).unwrap();
        assert_eq!(missing, "missing");
    }

    #[test]
    fn trims_whitespace_around_actions() {
        let tmpl = Template::parse_str("trim", "Line1\n{{- \"Line2\" -}}\nLine3").unwrap();
        let output = tmpl.render(&json!({})).unwrap();
        assert_eq!(output, "Line1Line2Line3");
    }

    #[test]
    fn variable_binding_inside_if() {
        let tmpl = Template::parse_str("if-var", "{{if $val := .value}}{{$val}}{{end}}").unwrap();
        let output = tmpl.render(&json!({"value": "ok"})).unwrap();
        assert_eq!(output, "ok");
    }

    #[test]
    fn range_assigns_iteration_variables() {
        let tmpl = Template::parse_str(
            "range-vars",
            "{{range $i, $v := .items}}{{$i}}:{{$v}};{{end}}",
        )
        .unwrap();
        let output = tmpl.render(&json!({"items": ["zero", "one"]})).unwrap();
        assert_eq!(output, "0:zero;1:one;");
    }

    #[test]
    fn comment_trimming_matches_go() {
        let left = Template::parse_str("comment-left", "x \r\n\t{{- /* hi */}}").unwrap();
        assert_eq!(left.render(&json!({})).unwrap(), "x");
        assert_eq!(left.to_template_string(), "x{{-/*hi*/}}");

        let right = Template::parse_str("comment-right", "{{/* hi */ -}}\n\n\ty").unwrap();
        assert_eq!(right.render(&json!({})).unwrap(), "y");
        assert_eq!(right.to_template_string(), "{{/*hi*/-}}y");

        let both =
            Template::parse_str("comment-both", "left \n{{- /* trim */ -}}\n right").unwrap();
        assert_eq!(both.render(&json!({})).unwrap(), "leftright");
        assert_eq!(both.to_template_string(), "left{{-/*trim*/-}}right");
    }

    #[test]
    fn comment_only_renders_empty_string() {
        let tmpl = Template::parse_str("comment-only", "{{/* comment */}}").unwrap();
        assert_eq!(tmpl.render(&json!({})).unwrap(), "");
    }

    #[test]
    fn root_variable_resolves_to_input() {
        let tmpl = Template::parse_str("root", "{{ $.name }}").unwrap();
        let rendered = tmpl.render(&json!({"name": "Lithos"})).unwrap();
        assert_eq!(rendered.trim(), "Lithos");
    }

    #[test]
    fn nested_scope_shadowing_preserves_outer() {
        let tmpl = Template::parse_str(
            "shadow",
            "{{ $x := \"outer\" }}{{ with .inner }}{{ $x := \"inner\" }}{{ $x }}{{ end }}{{ $x }}",
        )
        .unwrap();
        let rendered: String = tmpl
            .render(&json!({"inner": {"value": 1}}))
            .unwrap()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        assert_eq!(rendered, "innerouter");
    }

    #[test]
    fn assignment_updates_existing_variable() {
        let tmpl = Template::parse_str(
            "assign",
            "{{ $v := \"first\" }}{{ $v = \"second\" }}{{ $v }}",
        )
        .unwrap();
        let rendered = tmpl.render(&json!({})).unwrap();
        assert_eq!(rendered, "second");
    }

    #[test]
    fn assignment_to_unknown_variable_fails() {
        let tmpl = Template::parse_str("assign", "{{ $v = .value }}")
            .expect("assignment pipeline should parse");
        let err = tmpl.render(&json!({"value": 1})).unwrap_err();
        assert!(err.to_string().contains("variable $v not defined"));
    }

    #[test]
    fn pipeline_expression_inside_if() {
        let mut builder = FunctionRegistry::builder();
        builder
            .register("default", |_ctx, args| {
                let fallback = args.first().cloned().unwrap_or(Value::Null);
                let value = args.get(1).cloned().unwrap_or(Value::Null);
                if is_empty(&value) {
                    Ok(fallback)
                } else {
                    Ok(value)
                }
            })
            .register("ge", |_ctx, args| {
                if args.len() != 2 {
                    return Err(Error::render("ge expects two arguments", None));
                }
                let left = coerce_number(&args[0])?;
                let right = coerce_number(&args[1])?;
                Ok(Value::Bool(left >= right))
            });
        let registry = builder.build();

        let tmpl = Template::parse_with_functions(
            "pipeline-if",
            "# {{ if ge (.x | default 1) 1 }}\nyes \n# {{ end }}",
            registry,
        )
        .unwrap();

        let rendered = tmpl.render(&json!({})).unwrap();
        assert_eq!(rendered, "# \nyes \n# ");
        assert!(tmpl
            .to_template_string()
            .contains("{{if ge (.x | default 1) 1}}"));
    }
}
