// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::collections::HashSet;

use crate::ast::{ActionNode, Ast, Command, Expression, IfNode, Node, RangeNode, Span, WithNode};
use crate::lexer::{Token, TokenKind};
use crate::runtime::FunctionRegistry;

pub fn analyze_template(ast: &Ast, registry: Option<&FunctionRegistry>) -> TemplateAnalysis {
    let mut analyzer = Analyzer::new(registry);
    analyzer.walk_block(&ast.root);
    analyzer.finish()
}

#[derive(Debug, Clone)]
pub struct TemplateAnalysis {
    pub version: &'static str,
    pub precision: Precision,
    pub has_template_invocation: bool,
    pub variables: Vec<VariableAccess>,
    pub functions: Vec<FunctionCall>,
    pub templates: Vec<TemplateCall>,
    pub controls: Vec<ControlUsage>,
    pub issues: Vec<AnalysisIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    Precise,
    Conservative,
}

#[derive(Debug, Clone)]
pub struct VariableAccess {
    pub path: String,
    pub span: Span,
    pub kind: VariableKind,
    pub certainty: Certainty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    Dot,
    Dollar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Certainty {
    Certain,
    Uncertain,
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub span: Span,
    pub source: FunctionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSource {
    Registered,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TemplateCall {
    pub span: Span,
    pub name: Option<String>,
    pub indirect: bool,
}

#[derive(Debug, Clone)]
pub struct ControlUsage {
    pub kind: ControlKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    If,
    Range,
    With,
    Block,
    Define,
    Else,
    End,
}

#[derive(Debug, Clone)]
pub struct AnalysisIssue {
    pub message: String,
    pub span: Option<Span>,
}

struct Analyzer<'a> {
    registry: Option<&'a FunctionRegistry>,
    variables: Vec<VariableAccess>,
    functions: Vec<FunctionCall>,
    templates: Vec<TemplateCall>,
    controls: Vec<ControlUsage>,
    issues: Vec<AnalysisIssue>,
    has_template: bool,
    conservative: bool,
    seen_vars: HashSet<(String, Span)>,
}

impl<'a> Analyzer<'a> {
    fn new(registry: Option<&'a FunctionRegistry>) -> Self {
        Self {
            registry,
            variables: Vec::new(),
            functions: Vec::new(),
            templates: Vec::new(),
            controls: Vec::new(),
            issues: Vec::new(),
            has_template: false,
            conservative: false,
            seen_vars: HashSet::new(),
        }
    }

    fn finish(self) -> TemplateAnalysis {
        TemplateAnalysis {
            version: env!("CARGO_PKG_VERSION"),
            precision: if self.conservative {
                Precision::Conservative
            } else {
                Precision::Precise
            },
            has_template_invocation: self.has_template,
            variables: self.variables,
            functions: self.functions,
            templates: self.templates,
            controls: self.controls,
            issues: self.issues,
        }
    }

    fn walk_block(&mut self, block: &crate::ast::Block) {
        for node in &block.nodes {
            match node {
                Node::Action(action) => self.visit_action(action),
                Node::If(if_node) => self.visit_if(if_node),
                Node::Range(range_node) => self.visit_range(range_node),
                Node::With(with_node) => self.visit_with(with_node),
                Node::Text(_) | Node::Comment(_) => {}
            }
        }
    }

    fn visit_action(&mut self, action: &ActionNode) {
        self.inspect_tokens(&action.tokens);
        self.visit_pipeline(&action.pipeline, action.span);
    }

    fn visit_if(&mut self, node: &IfNode) {
        self.inspect_tokens(&node.tokens);
        self.controls.push(ControlUsage {
            kind: ControlKind::If,
            span: node.span,
        });
        self.visit_pipeline(&node.pipeline, node.span);
        self.walk_block(&node.then_block);
        if let Some(else_block) = &node.else_block {
            self.walk_block(else_block);
        }
    }

    fn visit_range(&mut self, node: &RangeNode) {
        self.inspect_tokens(&node.tokens);
        self.controls.push(ControlUsage {
            kind: ControlKind::Range,
            span: node.span,
        });
        self.visit_pipeline(&node.pipeline, node.span);
        self.walk_block(&node.then_block);
        if let Some(else_block) = &node.else_block {
            self.walk_block(else_block);
        }
    }

    fn visit_with(&mut self, node: &WithNode) {
        self.inspect_tokens(&node.tokens);
        self.controls.push(ControlUsage {
            kind: ControlKind::With,
            span: node.span,
        });
        self.visit_pipeline(&node.pipeline, node.span);
        self.walk_block(&node.then_block);
        if let Some(else_block) = &node.else_block {
            self.walk_block(else_block);
        }
    }

    fn inspect_tokens(&mut self, tokens: &[Token]) {
        for token in tokens {
            match token.kind {
                TokenKind::LeftBracket
                | TokenKind::RightBracket
                | TokenKind::Declare
                | TokenKind::Assign => {
                    self.mark_conservative(
                        "indexing or assignments are not fully analysed",
                        Some(token.span),
                    );
                }
                _ => {}
            }
        }
    }

    fn visit_pipeline(&mut self, pipeline: &crate::ast::Pipeline, span: Span) {
        for command in &pipeline.commands {
            self.visit_command(command, span);
        }
    }

    fn visit_command(&mut self, command: &Command, span: Span) {
        self.collect_expr(&command.target, span);

        match &command.target {
            Expression::Identifier(name) => {
                let lowered = name.as_str();
                if lowered == "template" || lowered == "block" {
                    self.record_template(command, span, lowered == "block");
                } else if let Some(control) = control_kind(lowered) {
                    self.controls.push(ControlUsage {
                        kind: control,
                        span,
                    });
                } else {
                    self.record_function(name.clone(), span);
                }
            }
            Expression::Variable(name) => {
                self.record_variable(name.clone(), span, VariableKind::Dollar, Certainty::Certain);
            }
            _ => {}
        }

        for arg in &command.args {
            self.collect_expr(arg, span);
        }
    }

    fn collect_expr(&mut self, expr: &Expression, span: Span) {
        match expr {
            Expression::Field(parts) => {
                let (path, certainty) = normalize_field(parts);
                self.record_variable(path, span, VariableKind::Dot, certainty);
            }
            Expression::Identifier(name) if name.starts_with('$') => {
                self.record_variable(name.clone(), span, VariableKind::Dollar, Certainty::Certain);
            }
            Expression::PipelineExpr(pipeline) => {
                self.visit_pipeline(pipeline, span);
            }
            Expression::Variable(name) => {
                self.record_variable(name.clone(), span, VariableKind::Dollar, Certainty::Certain);
            }
            _ => {}
        }
    }

    fn record_variable(
        &mut self,
        path: String,
        span: Span,
        kind: VariableKind,
        certainty: Certainty,
    ) {
        let key = (path.clone(), span);
        if self.seen_vars.insert(key) {
            self.variables.push(VariableAccess {
                path,
                span,
                kind,
                certainty,
            });
        }
    }

    fn record_function(&mut self, name: String, span: Span) {
        let source = if self
            .registry
            .map(|reg| reg.get(&name).is_some())
            .unwrap_or(false)
        {
            FunctionSource::Registered
        } else {
            FunctionSource::Unknown
        };
        self.functions.push(FunctionCall { name, span, source });
    }

    fn record_template(&mut self, command: &Command, span: Span, is_block: bool) {
        self.has_template = true;
        let template_name = command.args.first();
        let (name, indirect) = match template_name {
            Some(Expression::StringLiteral(lit)) => (Some(lit.clone()), false),
            Some(_) => (None, true),
            None => (None, true),
        };
        if indirect {
            self.mark_conservative("dynamic template invocation", Some(span));
        }
        self.templates.push(TemplateCall {
            span,
            name,
            indirect,
        });
        if is_block {
            self.controls.push(ControlUsage {
                kind: ControlKind::Block,
                span,
            });
        }
    }

    fn mark_conservative(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.conservative = true;
        self.issues.push(AnalysisIssue {
            message: message.into(),
            span,
        });
    }
}

fn control_kind(name: &str) -> Option<ControlKind> {
    match name {
        "if" => Some(ControlKind::If),
        "range" => Some(ControlKind::Range),
        "with" => Some(ControlKind::With),
        "block" => Some(ControlKind::Block),
        "define" => Some(ControlKind::Define),
        "else" => Some(ControlKind::Else),
        "end" => Some(ControlKind::End),
        _ => None,
    }
}

fn normalize_field(parts: &[String]) -> (String, Certainty) {
    if parts.is_empty() {
        return (".".to_string(), Certainty::Certain);
    }
    let mut certainty = Certainty::Certain;
    let mut normalized_parts = Vec::with_capacity(parts.len());
    for part in parts {
        if part.chars().all(|c| c.is_alphanumeric() || c == '_') {
            normalized_parts.push(part.clone());
        } else {
            certainty = Certainty::Uncertain;
            normalized_parts.push(part.clone());
        }
    }
    (format!(".{}", normalized_parts.join(".")), certainty)
}
