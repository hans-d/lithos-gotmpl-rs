// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::fmt;

/// Byte offsets into the original template source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Root AST structure for a parsed template.
#[derive(Debug, Clone)]
pub struct Ast {
    pub name: String,
    pub root: Block,
}

impl Ast {
    pub fn new(name: impl Into<String>, root: Block) -> Self {
        Self {
            name: name.into(),
            root,
        }
    }
}

/// A sequential block of nodes (equivalent to Go's `parse.ListNode`).
#[derive(Debug, Clone, Default)]
pub struct Block {
    pub nodes: Vec<Node>,
}

impl Block {
    pub fn push(&mut self, node: Node) {
        self.nodes.push(node);
    }
}

/// Node types recognised by the parser.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Node {
    Text(TextNode),
    Action(ActionNode),
    Comment(CommentNode),
    If(IfNode),
    Range(RangeNode),
    With(WithNode),
}

impl Node {
    pub fn span(&self) -> Span {
        match self {
            Node::Text(node) => node.span,
            Node::Action(node) => node.span,
            Node::Comment(node) => node.span,
            Node::If(node) => node.span,
            Node::Range(node) => node.span,
            Node::With(node) => node.span,
        }
    }
}

/// Raw text literal.
#[derive(Debug, Clone)]
pub struct TextNode {
    pub span: Span,
    pub text: String,
}

impl TextNode {
    pub fn new(span: Span, text: impl Into<String>) -> Self {
        Self {
            span,
            text: text.into(),
        }
    }
}

/// Template action with parsed pipeline information.
#[derive(Debug, Clone)]
pub struct ActionNode {
    pub span: Span,
    pub source: String,
    pub tokens: Vec<crate::lexer::Token>,
    pub pipeline: Pipeline,
    pub trim_left: bool,
    pub trim_right: bool,
}

impl ActionNode {
    pub fn new(
        span: Span,
        source: impl Into<String>,
        tokens: Vec<crate::lexer::Token>,
        pipeline: Pipeline,
        trim_left: bool,
        trim_right: bool,
    ) -> Self {
        Self {
            span,
            source: source.into(),
            tokens,
            pipeline,
            trim_left,
            trim_right,
        }
    }

    pub fn to_template_fragment(&self) -> String {
        let mut out = String::from("{{");
        if self.trim_left {
            out.push('-');
        }
        out.push_str(&self.source);
        if self.trim_right {
            out.push('-');
        }
        out.push_str("}}");
        out
    }
}

/// Template comment (e.g. `{{/* comment */}}`).
#[derive(Debug, Clone)]
pub struct CommentNode {
    pub span: Span,
    pub text: String,
    pub trim_left: bool,
    pub trim_right: bool,
}

impl CommentNode {
    pub fn new(span: Span, text: impl Into<String>, trim_left: bool, trim_right: bool) -> Self {
        Self {
            span,
            text: text.into(),
            trim_left,
            trim_right,
        }
    }

    pub fn to_template_fragment(&self) -> String {
        let mut out = String::from("{{");
        if self.trim_left {
            out.push('-');
        }
        out.push_str("/*");
        out.push_str(&self.text);
        out.push_str("*/");
        if self.trim_right {
            out.push('-');
        }
        out.push_str("}}");
        out
    }
}

/// Conditional branch node (mirrors Go's `parse.IfNode`).
#[derive(Debug, Clone)]
pub struct IfNode {
    pub span: Span,
    pub tokens: Vec<crate::lexer::Token>,
    pub pipeline: Pipeline,
    pub then_block: Block,
    pub else_if_branches: Vec<ElseIfBranch>,
    pub else_block: Option<Block>,
}

impl IfNode {
    pub fn new(
        span: Span,
        tokens: Vec<crate::lexer::Token>,
        pipeline: Pipeline,
        then_block: Block,
        else_if_branches: Vec<ElseIfBranch>,
        else_block: Option<Block>,
    ) -> Self {
        Self {
            span,
            tokens,
            pipeline,
            then_block,
            else_if_branches,
            else_block,
        }
    }
}

/// Captures an `{{else if ...}}` branch.
#[derive(Debug, Clone)]
pub struct ElseIfBranch {
    pub span: Span,
    pub tokens: Vec<crate::lexer::Token>,
    pub pipeline: Pipeline,
    pub block: Block,
}

impl ElseIfBranch {
    pub fn new(
        span: Span,
        tokens: Vec<crate::lexer::Token>,
        pipeline: Pipeline,
        block: Block,
    ) -> Self {
        Self {
            span,
            tokens,
            pipeline,
            block,
        }
    }
}

/// Range iteration node.
#[derive(Debug, Clone)]
pub struct RangeNode {
    pub span: Span,
    pub tokens: Vec<crate::lexer::Token>,
    pub pipeline: Pipeline,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

impl RangeNode {
    pub fn new(
        span: Span,
        tokens: Vec<crate::lexer::Token>,
        pipeline: Pipeline,
        then_block: Block,
        else_block: Option<Block>,
    ) -> Self {
        Self {
            span,
            tokens,
            pipeline,
            then_block,
            else_block,
        }
    }
}

/// Scoped context node (`with`).
#[derive(Debug, Clone)]
pub struct WithNode {
    pub span: Span,
    pub tokens: Vec<crate::lexer::Token>,
    pub pipeline: Pipeline,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

impl WithNode {
    pub fn new(
        span: Span,
        tokens: Vec<crate::lexer::Token>,
        pipeline: Pipeline,
        then_block: Block,
        else_block: Option<Block>,
    ) -> Self {
        Self {
            span,
            tokens,
            pipeline,
            then_block,
            else_block,
        }
    }
}

/// A complete pipeline inside an action.
#[derive(Debug, Clone)]
pub struct Pipeline {
    pub declarations: Option<PipelineDeclarations>,
    pub commands: Vec<Command>,
}

impl Pipeline {
    pub fn new(declarations: Option<PipelineDeclarations>, commands: Vec<Command>) -> Self {
        Self {
            declarations,
            commands,
        }
    }
}

/// Variable declarations leading a pipeline (e.g. `{{$x := ...}}`).
#[derive(Debug, Clone)]
pub struct PipelineDeclarations {
    pub kind: BindingKind,
    pub variables: Vec<String>,
}

impl PipelineDeclarations {
    pub fn new(kind: BindingKind, variables: Vec<String>) -> Self {
        Self { kind, variables }
    }
}

/// Whether the pipeline introduces (`:=`) or assigns (`=`) variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BindingKind {
    Declare,
    Assign,
}

/// Individual command in a pipeline.
#[derive(Debug, Clone)]
pub struct Command {
    pub target: Expression,
    pub args: Vec<Expression>,
}

impl Command {
    pub fn new(target: Expression, args: Vec<Expression>) -> Self {
        Self { target, args }
    }
}

/// Expression node.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Expression {
    Identifier(String),
    Field(Vec<String>),
    Variable(String),
    PipelineExpr(Pipeline),
    StringLiteral(String),
    NumberLiteral(String),
    BoolLiteral(bool),
    Nil,
}

impl Expression {
    pub fn identifier(name: impl Into<String>) -> Self {
        Expression::Identifier(name.into())
    }

    pub fn field(path: Vec<String>) -> Self {
        Expression::Field(path)
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Text(node) => write!(f, "Text({:?})", node.text),
            Node::Action(node) => write!(f, "Action({:?})", node.source),
            Node::Comment(_) => write!(f, "Comment"),
            Node::If(_) => write!(f, "If"),
            Node::Range(_) => write!(f, "Range"),
            Node::With(_) => write!(f, "With"),
        }
    }
}
