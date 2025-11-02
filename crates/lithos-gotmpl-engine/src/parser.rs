// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::ast::{
    ActionNode, Ast, BindingKind, Block, Command, CommentNode, Expression, IfNode, Node, Pipeline,
    PipelineDeclarations, RangeNode, Span, TextNode, WithNode,
};
use crate::error::Error;
use crate::lexer;
use crate::lexer::{Keyword, Operator, Token, TokenKind};

/// Primary entry point for parsing template sources.
///
/// The parser walks the input once, splitting it into literal text and action
/// blocks. To mimic Go's `text/template` behaviour we keep two stacks:
///
/// - `control_stack` stores the open `if`/`range`/`with` frames so we can
///   populate their bodies when a matching `{{end}}` is seen.
/// - `target_stack` tracks where the next node should be appended (root block,
///   current `then` block, or current `else` block).
///
/// Keeping the structure explicit helps when trimming whitespace and recording
/// byte spans.
pub fn parse_template(name: &str, source: &str) -> Result<Ast, Error> {
    let mut root = Block::default();
    let mut cursor = 0usize;
    let bytes = source.as_bytes();
    let mut control_stack: Vec<ControlFrame> = Vec::new();
    let mut target_stack: Vec<AppendTarget> = vec![AppendTarget::Root];

    while cursor < bytes.len() {
        let Some(open) = find_action_start(bytes, cursor) else {
            let text = &source[cursor..];
            if !text.is_empty() {
                push_node(
                    &mut root,
                    &mut control_stack,
                    &target_stack,
                    Node::Text(TextNode::new(
                        Span::new(cursor, source.len()),
                        text.to_string(),
                    )),
                );
            }
            break;
        };

        if open > cursor {
            let text = &source[cursor..open];
            if !text.is_empty() {
                push_node(
                    &mut root,
                    &mut control_stack,
                    &target_stack,
                    Node::Text(TextNode::new(Span::new(cursor, open), text.to_string())),
                );
            }
        }

        match find_action_end(bytes, open + 2) {
            Some(close) => {
                let window = trim_action_delimiters(source, bytes, open, close);

                if window.trim_left {
                    let block = current_block_mut(&mut root, &mut control_stack, &target_stack);
                    trim_trailing_whitespace(block);
                }

                if is_potential_comment(window.body) && !window.body.ends_with("*/") {
                    return Err(Error::parse_with_span("unclosed comment", window.span));
                }

                if is_comment(window.body) {
                    push_node(
                        &mut root,
                        &mut control_stack,
                        &target_stack,
                        Node::Comment(CommentNode::new(
                            window.span,
                            strip_comment(window.body),
                            window.trim_left,
                            window.trim_right,
                        )),
                    );
                } else {
                    let tokens = lexer::lex_action(window.body, window.body_start)?;

                    if tokens.is_empty() {
                        return Err(Error::parse_with_span("empty action", window.span));
                    }

                    match classify_action(&tokens)? {
                        ActionKind::If => {
                            let condition_tokens: Vec<_> = tokens[1..].to_vec();
                            let condition_pipeline = parse_action_pipeline(&condition_tokens)?;
                            let frame = ControlFrame::new(
                                ControlKind::If,
                                window.span,
                                condition_tokens,
                                condition_pipeline,
                            );
                            push_control_frame(&mut control_stack, &mut target_stack, frame);
                        }
                        ActionKind::Range => {
                            let condition_tokens: Vec<_> = tokens[1..].to_vec();
                            let condition_pipeline = parse_action_pipeline(&condition_tokens)?;
                            let frame = ControlFrame::new(
                                ControlKind::Range,
                                window.span,
                                condition_tokens,
                                condition_pipeline,
                            );
                            push_control_frame(&mut control_stack, &mut target_stack, frame);
                        }
                        ActionKind::With => {
                            let condition_tokens: Vec<_> = tokens[1..].to_vec();
                            let condition_pipeline = parse_action_pipeline(&condition_tokens)?;
                            let frame = ControlFrame::new(
                                ControlKind::With,
                                window.span,
                                condition_tokens,
                                condition_pipeline,
                            );
                            push_control_frame(&mut control_stack, &mut target_stack, frame);
                        }
                        ActionKind::Else => {
                            handle_else(&mut control_stack, &mut target_stack, window.span)?;
                        }
                        ActionKind::End => {
                            close_control_frame(
                                &mut root,
                                &mut control_stack,
                                &mut target_stack,
                                window.span,
                            )?;
                        }
                        ActionKind::Regular => {
                            let pipeline = parse_action_pipeline(&tokens)?;
                            let node = build_action_node(
                                window.span,
                                window.body,
                                tokens,
                                pipeline,
                                window.trim_left,
                                window.trim_right,
                            );
                            push_node(&mut root, &mut control_stack, &target_stack, node);
                        }
                    }
                }

                cursor = close + 2;
                if window.trim_right {
                    cursor = skip_leading_whitespace(bytes, cursor);
                }
            }
            None => {
                let mut remainder = &source[open + 2..];
                remainder = remainder.trim_start();
                if let Some(rest) = remainder.strip_prefix('-') {
                    remainder = rest.trim_start();
                }

                let span = Span::new(open, source.len());
                if remainder.starts_with("/*") {
                    return Err(Error::parse_with_span("unclosed comment", span));
                }

                return Err(Error::parse_with_span("unclosed action", span));
            }
        }
    }

    if bytes.is_empty() {
        push_node(
            &mut root,
            &mut control_stack,
            &target_stack,
            Node::Text(TextNode::new(Span::new(0, 0), String::new())),
        );
    }

    if let Some(frame) = control_stack.last() {
        return Err(Error::parse(
            "unterminated control structure",
            Some(frame.start_span),
        ));
    }

    Ok(Ast::new(name, root))
}

#[derive(Debug, Clone, Copy)]
struct ActionWindow<'a> {
    span: Span,
    body_start: usize,
    body: &'a str,
    trim_left: bool,
    trim_right: bool,
}

fn trim_action_delimiters<'a>(
    source: &'a str,
    bytes: &[u8],
    open: usize,
    close: usize,
) -> ActionWindow<'a> {
    let mut body_start = open + 2;
    let mut body_end = close;
    let mut trim_left = false;
    let mut trim_right = false;

    if body_start < close && bytes[body_start] == b'-' {
        trim_left = true;
        body_start += 1;
    }
    if body_start < body_end && bytes[body_end - 1] == b'-' {
        trim_right = true;
        body_end -= 1;
    }

    let span = Span::new(open, close + 2);
    let raw = &source[body_start..body_end];
    let trimmed_start = raw.trim_start();
    let prefix_len = raw.len() - trimmed_start.len();
    let body = trimmed_start.trim_end();
    body_start += prefix_len;

    ActionWindow {
        span,
        body_start,
        body,
        trim_left,
        trim_right,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionKind {
    If,
    Range,
    With,
    Else,
    End,
    Regular,
}

fn classify_action(tokens: &[Token]) -> Result<ActionKind, Error> {
    let first = tokens
        .first()
        .ok_or_else(|| Error::parse("empty action", None))?;
    match &first.kind {
        TokenKind::Keyword(Keyword::If) => {
            if tokens.len() < 2 {
                return Err(Error::parse_with_span("if requires a pipeline", first.span));
            }
            Ok(ActionKind::If)
        }
        TokenKind::Keyword(Keyword::Range) => {
            if tokens.len() < 2 {
                return Err(Error::parse_with_span(
                    "range requires a pipeline",
                    first.span,
                ));
            }
            Ok(ActionKind::Range)
        }
        TokenKind::Keyword(Keyword::With) => {
            if tokens.len() < 2 {
                return Err(Error::parse_with_span(
                    "with requires a pipeline",
                    first.span,
                ));
            }
            Ok(ActionKind::With)
        }
        TokenKind::Keyword(Keyword::Else) => {
            if tokens.len() > 1 {
                return Err(Error::parse(
                    "else-if is not yet supported",
                    Some(tokens[1].span),
                ));
            }
            Ok(ActionKind::Else)
        }
        TokenKind::Keyword(Keyword::End) => {
            if tokens.len() > 1 {
                return Err(Error::parse(
                    "unexpected tokens after end",
                    Some(tokens[1].span),
                ));
            }
            Ok(ActionKind::End)
        }
        _ => Ok(ActionKind::Regular),
    }
}

#[derive(Debug, Clone, Copy)]
enum AppendTarget {
    Root,
    Then(usize),
    Else(usize),
}

#[derive(Debug)]
struct ControlFrame {
    kind: ControlKind,
    start_span: Span,
    tokens: Vec<Token>,
    pipeline: Pipeline,
    then_block: Block,
    else_block: Option<Block>,
}

impl ControlFrame {
    fn new(kind: ControlKind, span: Span, tokens: Vec<Token>, pipeline: Pipeline) -> Self {
        Self {
            kind,
            start_span: span,
            tokens,
            pipeline,
            then_block: Block::default(),
            else_block: None,
        }
    }
}

#[derive(Debug)]
enum ControlKind {
    If,
    Range,
    With,
}

fn current_block_mut<'a>(
    root: &'a mut Block,
    controls: &'a mut [ControlFrame],
    targets: &[AppendTarget],
) -> &'a mut Block {
    match targets.last().copied().unwrap_or(AppendTarget::Root) {
        AppendTarget::Root => root,
        AppendTarget::Then(idx) => &mut controls[idx].then_block,
        AppendTarget::Else(idx) => controls[idx]
            .else_block
            .as_mut()
            .expect("else block should be initialised"),
    }
}

fn trim_trailing_whitespace(block: &mut Block) {
    if let Some(Node::Text(text)) = block.nodes.last_mut() {
        while text
            .text
            .chars()
            .last()
            .map(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r'))
            .unwrap_or(false)
        {
            text.text.pop();
        }
    }
}

fn skip_leading_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r') {
        index += 1;
    }
    index
}

fn push_node(
    root: &mut Block,
    controls: &mut [ControlFrame],
    targets: &[AppendTarget],
    node: Node,
) {
    let block = current_block_mut(root, controls, targets);
    block.push(node);
}

fn build_action_node(
    span: Span,
    body: &str,
    tokens: Vec<Token>,
    pipeline: Pipeline,
    trim_left: bool,
    trim_right: bool,
) -> Node {
    Node::Action(ActionNode::new(
        span,
        body.to_string(),
        tokens,
        pipeline,
        trim_left,
        trim_right,
    ))
}

fn push_control_frame(
    controls: &mut Vec<ControlFrame>,
    targets: &mut Vec<AppendTarget>,
    frame: ControlFrame,
) {
    controls.push(frame);
    let idx = controls.len() - 1;
    targets.push(AppendTarget::Then(idx));
}

fn handle_else(
    controls: &mut [ControlFrame],
    targets: &mut [AppendTarget],
    span: Span,
) -> Result<(), Error> {
    if targets.len() <= 1 {
        return Err(Error::parse_with_span("unexpected else", span));
    }

    let current = targets
        .last_mut()
        .ok_or_else(|| Error::parse_with_span("unexpected else", span))?;

    let idx = match current {
        AppendTarget::Then(idx) => *idx,
        AppendTarget::Else(_) => {
            return Err(Error::parse_with_span("duplicate else block", span));
        }
        AppendTarget::Root => return Err(Error::parse_with_span("unexpected else", span)),
    };

    let frame = controls
        .get_mut(idx)
        .ok_or_else(|| Error::parse_with_span("mismatched else", span))?;

    if frame.else_block.is_some() {
        return Err(Error::parse_with_span("multiple else blocks", span));
    }

    frame.else_block = Some(Block::default());
    *current = AppendTarget::Else(idx);
    Ok(())
}

#[allow(clippy::ptr_arg)]
fn close_control_frame(
    root: &mut Block,
    controls: &mut Vec<ControlFrame>,
    targets: &mut Vec<AppendTarget>,
    span: Span,
) -> Result<(), Error> {
    let top = targets
        .pop()
        .ok_or_else(|| Error::parse_with_span("unexpected end", span))?;

    let idx = match top {
        AppendTarget::Then(idx) | AppendTarget::Else(idx) => idx,
        AppendTarget::Root => return Err(Error::parse_with_span("unexpected end", span)),
    };

    if controls.len() <= idx {
        return Err(Error::parse_with_span("mismatched end", span));
    }

    if controls.len() - 1 != idx {
        return Err(Error::parse_with_span(
            "nested block closed out of order",
            span,
        ));
    }

    let frame = controls
        .pop()
        .ok_or_else(|| Error::parse_with_span("unexpected end", span))?;
    let full_span = Span::new(frame.start_span.start, span.end);
    let ControlFrame {
        kind,
        tokens,
        pipeline,
        then_block,
        else_block,
        ..
    } = frame;

    let node = match kind {
        ControlKind::If => Node::If(IfNode::new(
            full_span, tokens, pipeline, then_block, else_block,
        )),
        ControlKind::Range => Node::Range(RangeNode::new(
            full_span, tokens, pipeline, then_block, else_block,
        )),
        ControlKind::With => Node::With(WithNode::new(
            full_span, tokens, pipeline, then_block, else_block,
        )),
    };

    push_node(root, controls, targets.as_slice(), node);

    Ok(())
}

fn is_comment(body: &str) -> bool {
    let trimmed = body.trim();
    trimmed.starts_with("/*") && trimmed.ends_with("*/")
}

fn strip_comment(body: &str) -> String {
    let trimmed = body.trim();
    trimmed
        .strip_prefix("/*")
        .and_then(|b| b.strip_suffix("*/"))
        .map(|inner| inner.trim().to_string())
        .unwrap_or_else(|| body.to_string())
}

fn is_potential_comment(body: &str) -> bool {
    let trimmed = body.trim();
    trimmed.starts_with("/*")
}

fn parse_action_pipeline(tokens: &[crate::lexer::Token]) -> Result<Pipeline, Error> {
    let mut parser = ActionParser::new(tokens);
    parser.parse_pipeline()
}

struct ActionParser<'a> {
    tokens: &'a [crate::lexer::Token],
    index: usize,
}

impl<'a> ActionParser<'a> {
    fn new(tokens: &'a [crate::lexer::Token]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_pipeline(&mut self) -> Result<Pipeline, Error> {
        let declarations = self.parse_declarations()?;
        let mut commands = Vec::new();
        if self.is_eof() {
            return Err(Error::parse("empty action", None));
        }

        commands.push(self.parse_command()?);

        while self.consume_pipe()? {
            commands.push(self.parse_command()?);
        }

        if !self.is_eof() {
            let token = &self.tokens[self.index];
            return Err(Error::parse(
                format!("unexpected token {:?}", token.kind),
                Some(token.span),
            ));
        }

        Ok(Pipeline::new(declarations, commands))
    }

    fn parse_declarations(&mut self) -> Result<Option<PipelineDeclarations>, Error> {
        let save = self.index;
        let mut names = Vec::new();

        loop {
            let Some(token) = self.peek_token() else {
                break;
            };
            match &token.kind {
                TokenKind::Identifier(name) if name.starts_with('$') => {
                    names.push(name.clone());
                    self.index += 1;
                }
                _ => break,
            }

            if let Some(next) = self.peek_token() {
                if matches!(next.kind, TokenKind::Comma) {
                    self.index += 1;
                    continue;
                }
            }
            break;
        }

        if names.is_empty() {
            self.index = save;
            return Ok(None);
        }

        let kind = match self.peek_token() {
            Some(token) => match token.kind {
                TokenKind::Declare => BindingKind::Declare,
                TokenKind::Assign => BindingKind::Assign,
                _ => {
                    self.index = save;
                    return Ok(None);
                }
            },
            None => {
                self.index = save;
                return Ok(None);
            }
        };

        self.index += 1; // consume := or =

        Ok(Some(PipelineDeclarations::new(kind, names)))
    }

    fn parse_command(&mut self) -> Result<Command, Error> {
        let first_expr = self.parse_expression()?;

        if let Some(operator) = self.consume_operator()? {
            let rhs = self.parse_expression()?;
            let op_name = match operator {
                Operator::Equal => "eq",
                Operator::NotEqual => "ne",
                Operator::Less => "lt",
                Operator::LessOrEqual => "le",
                Operator::Greater => "gt",
                Operator::GreaterOrEqual => "ge",
            };
            return Ok(Command::new(
                Expression::Identifier(op_name.to_string()),
                vec![first_expr, rhs],
            ));
        }

        let mut args = Vec::new();

        loop {
            self.skip_commas();
            if self.peek_pipe() || self.is_eof() {
                break;
            }
            args.push(self.parse_expression()?);
        }

        Ok(Command::new(first_expr, args))
    }

    fn parse_expression(&mut self) -> Result<Expression, Error> {
        let token = self
            .next_token()
            .ok_or_else(|| Error::parse("unexpected end of action", None))?;
        let expr = match &token.kind {
            TokenKind::Identifier(name) => {
                if name.starts_with('$') {
                    let mut parts = vec![name.clone()];
                    self.extend_field_segments(&mut parts, token.span);
                    if parts.len() > 1 {
                        Expression::Field(parts)
                    } else {
                        Expression::Variable(name.clone())
                    }
                } else {
                    Expression::Identifier(name.clone())
                }
            }
            TokenKind::Dot => self.parse_field(token.span)?,
            TokenKind::StringLiteral(value) => Expression::StringLiteral(value.clone()),
            TokenKind::NumberLiteral(value) => Expression::NumberLiteral(value.clone()),
            TokenKind::Keyword(Keyword::Nil) => Expression::Nil,
            TokenKind::Keyword(Keyword::True) => Expression::BoolLiteral(true),
            TokenKind::Keyword(Keyword::False) => Expression::BoolLiteral(false),
            TokenKind::Keyword(keyword) => Expression::Identifier(keyword.as_str().to_string()),
            TokenKind::LeftParen => self.parse_parenthesized_pipeline()?,
            other => {
                return Err(Error::parse(
                    format!("unexpected token in expression: {:?}", other),
                    Some(token.span),
                ));
            }
        };
        Ok(expr)
    }

    fn parse_field(&mut self, _start_span: Span) -> Result<Expression, Error> {
        let Some(token) = self.peek_token() else {
            return Ok(Expression::Field(Vec::new()));
        };

        let mut parts = Vec::new();
        let span = token.span;
        match &token.kind {
            TokenKind::Identifier(name) => {
                parts.push(name.clone());
                self.index += 1;
            }
            TokenKind::NumberLiteral(num) => {
                parts.push(num.clone());
                self.index += 1;
            }
            _ => {}
        }

        if parts.is_empty() {
            return Ok(Expression::Field(Vec::new()));
        }

        self.extend_field_segments(&mut parts, span);
        Ok(Expression::Field(parts))
    }

    fn skip_commas(&mut self) {
        while let Some(token) = self.peek_token() {
            if matches!(token.kind, TokenKind::Comma) {
                self.index += 1;
            } else {
                break;
            }
        }
    }

    fn consume_pipe(&mut self) -> Result<bool, Error> {
        if let Some(token) = self.peek_token() {
            if matches!(token.kind, TokenKind::Pipe) {
                self.index += 1;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn peek_pipe(&self) -> bool {
        self.peek_token()
            .map(|token| matches!(token.kind, TokenKind::Pipe))
            .unwrap_or(false)
    }

    fn next_token(&mut self) -> Option<&'a crate::lexer::Token> {
        if self.index >= self.tokens.len() {
            None
        } else {
            let token = &self.tokens[self.index];
            self.index += 1;
            Some(token)
        }
    }

    fn peek_token(&self) -> Option<&'a crate::lexer::Token> {
        self.tokens.get(self.index)
    }

    fn is_eof(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn consume_operator(&mut self) -> Result<Option<Operator>, Error> {
        if let Some(token) = self.peek_token() {
            if let TokenKind::Operator(op) = &token.kind {
                self.index += 1;
                return Ok(Some(op.clone()));
            }
        }
        Ok(None)
    }

    fn parse_parenthesized_pipeline(&mut self) -> Result<Expression, Error> {
        let mut depth = 1usize;
        let mut end = self.index;
        while end < self.tokens.len() {
            match self.tokens[end].kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            end += 1;
        }

        if depth != 0 {
            return Err(Error::parse("expected ')'", None));
        }

        let sub_tokens = &self.tokens[self.index..end];
        if sub_tokens.is_empty() {
            return Err(Error::parse(
                "empty pipeline",
                Some(self.tokens[self.index - 1].span),
            ));
        }

        let mut sub_parser = ActionParser::new(sub_tokens);
        let pipeline = sub_parser.parse_pipeline()?;
        if sub_parser.index != sub_tokens.len() {
            let token = &sub_tokens[sub_parser.index];
            return Err(Error::parse(
                format!("unexpected token {:?}", token.kind),
                Some(token.span),
            ));
        }
        if pipeline.declarations.is_some() {
            return Err(Error::parse(
                "pipeline declarations not allowed in expression",
                Some(self.tokens[self.index - 1].span),
            ));
        }

        self.index = end + 1; // skip the closing ')'
        Ok(Expression::PipelineExpr(pipeline))
    }

    fn extend_field_segments(&mut self, parts: &mut Vec<String>, mut last_span: Span) {
        loop {
            let Some(dot_token) = self.tokens.get(self.index) else {
                break;
            };

            if !matches!(dot_token.kind, TokenKind::Dot) || dot_token.span.start != last_span.end {
                break;
            }

            let Some(segment) = self.tokens.get(self.index + 1) else {
                break;
            };

            match &segment.kind {
                TokenKind::Identifier(name) => {
                    self.index += 2;
                    parts.push(name.clone());
                    last_span = segment.span;
                }
                TokenKind::NumberLiteral(num) => {
                    self.index += 2;
                    parts.push(num.clone());
                    last_span = segment.span;
                }
                _ => break,
            }
        }
    }
}

fn find_action_start(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_action_end(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    let mut in_raw = false;
    let mut in_string = false;
    let mut in_comment = false;
    while i + 1 < bytes.len() {
        let current = bytes[i];

        if in_comment {
            if current == b'*' && bytes[i + 1] == b'/' {
                in_comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        if in_raw {
            if current == b'`' {
                in_raw = false;
            }
            i += 1;
            continue;
        }

        if in_string {
            if current == b'\\' {
                i += 2;
                continue;
            }
            if current == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if current == b'/' && bytes[i + 1] == b'*' {
            in_comment = true;
            i += 2;
            continue;
        }

        match current {
            b'`' => {
                in_raw = true;
                i += 1;
                continue;
            }
            b'"' => {
                in_string = true;
                i += 1;
                continue;
            }
            _ => {}
        }

        if current == b'}' && bytes[i + 1] == b'}' {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_action_delimiters_reports_flags() {
        let source = "{{- foo -}}";
        let bytes = source.as_bytes();
        let open = find_action_start(bytes, 0).expect("missing action start");
        let close = find_action_end(bytes, open + 2).expect("missing action end");
        let window = trim_action_delimiters(source, bytes, open, close);

        assert!(window.trim_left);
        assert!(window.trim_right);
        assert_eq!(window.body, "foo");
        assert_eq!(window.body_start, 4);
        assert_eq!(window.span, Span::new(0, source.len()));
    }

    #[test]
    fn control_frame_helpers_manage_stack() {
        let mut root = Block::default();
        let mut controls = Vec::new();
        let mut targets = vec![AppendTarget::Root];
        let pipeline = Pipeline::new(
            None,
            vec![Command::new(Expression::BoolLiteral(true), Vec::new())],
        );
        let frame = ControlFrame::new(ControlKind::If, Span::new(0, 10), Vec::new(), pipeline);

        push_control_frame(&mut controls, &mut targets, frame);
        assert_eq!(controls.len(), 1);
        assert!(matches!(targets.last(), Some(AppendTarget::Then(0))));

        controls[0]
            .then_block
            .push(Node::Text(TextNode::new(Span::new(10, 12), "ok")));

        close_control_frame(&mut root, &mut controls, &mut targets, Span::new(12, 20))
            .expect("closing control frame should succeed");

        assert!(controls.is_empty());
        assert!(matches!(targets.as_slice(), [AppendTarget::Root]));
        assert!(matches!(root.nodes.first(), Some(Node::If(_))));
    }

    #[test]
    fn find_action_end_handles_comment_with_quotes() {
        let input = b"{{/* comment with \" unmatched */}} tail";
        let start = find_action_start(input, 0).expect("missing action start");
        let end = find_action_end(input, start + 2).expect("should find closing braces");
        assert_eq!(&input[end..end + 2], b"}}");
    }

    #[test]
    fn find_action_end_handles_comment_with_backticks() {
        let input = b"{{/* comment with ` unmatched */}} tail";
        let start = find_action_start(input, 0).expect("missing action start");
        let end = find_action_end(input, start + 2).expect("should find closing braces");
        assert_eq!(&input[end..end + 2], b"}}");
    }

    #[test]
    fn parses_text_and_actions() {
        let src = "hello {{world}}!";
        let ast = parse_template("test", src).unwrap();
        assert_eq!(ast.root.nodes.len(), 3);
    }

    #[test]
    fn parses_pipeline_into_individual_commands() {
        let src = "{{ .name | default \"lithos\" | upper }}";
        let ast = parse_template("pipe", src).unwrap();
        assert_eq!(ast.root.nodes.len(), 1);

        let action = match &ast.root.nodes[0] {
            Node::Action(node) => node,
            other => panic!("expected action node, found {other:?}"),
        };

        assert_eq!(action.pipeline.commands.len(), 3);

        let first = &action.pipeline.commands[0];
        match &first.target {
            Expression::Field(parts) => assert_eq!(parts, &["name".to_string()]),
            other => panic!("expected field expression, found {other:?}"),
        }
        assert!(first.args.is_empty());

        let second = &action.pipeline.commands[1];
        match &second.target {
            Expression::Identifier(name) => assert_eq!(name, "default"),
            other => panic!("expected identifier, found {other:?}"),
        }
        match second.args.as_slice() {
            [Expression::StringLiteral(value)] => assert_eq!(value, "lithos"),
            other => panic!("unexpected argument list: {other:?}"),
        }

        let third = &action.pipeline.commands[2];
        match &third.target {
            Expression::Identifier(name) => assert_eq!(name, "upper"),
            other => panic!("expected identifier, found {other:?}"),
        }
        assert!(third.args.is_empty());
    }

    #[test]
    fn parses_if_with_else_branch() {
        let src = "{{if .enabled}}yes{{else}}no{{end}}";
        let ast = parse_template("if", src).unwrap();
        assert_eq!(ast.root.nodes.len(), 1);

        let if_node = match &ast.root.nodes[0] {
            Node::If(node) => node,
            other => panic!("expected If node, got {other:?}"),
        };

        assert_eq!(if_node.pipeline.commands.len(), 1);
        let command = &if_node.pipeline.commands[0];
        match &command.target {
            Expression::Field(parts) => assert_eq!(parts, &["enabled".to_string()]),
            other => panic!("unexpected pipeline target: {other:?}"),
        }
        assert!(command.args.is_empty());
        assert!(if_node
            .then_block
            .nodes
            .iter()
            .any(|node| matches!(node, Node::Text(_))));
        assert!(if_node.else_block.is_some());
    }

    #[test]
    fn parses_nested_field_access() {
        let src = "{{ .project.name }}";
        let ast = parse_template("field", src).unwrap();
        let action = match &ast.root.nodes[0] {
            Node::Action(node) => node,
            other => panic!("expected action node, got {other:?}"),
        };
        assert_eq!(action.pipeline.commands.len(), 1);
        let command = &action.pipeline.commands[0];
        match &command.target {
            Expression::Field(parts) => {
                assert_eq!(parts, &["project".to_string(), "name".to_string()])
            }
            other => panic!("unexpected pipeline target {other:?}"),
        }
        assert!(command.args.is_empty());
    }

    #[test]
    fn parse_error_on_unclosed_comment() {
        let err = parse_template("bad-comment", "{{/*}} ")
            .expect_err("expected parser error for unterminated comment");
        assert!(err.to_string().contains("unclosed comment"));
    }

    #[test]
    fn parses_raw_string_with_nested_braces() {
        let src = "{{`{{.settings.bucket_name_suffix}}`}}";
        parse_template("raw", src).expect("raw string literal with braces should parse");
    }

    #[test]
    fn spans_cover_action_body() {
        let src = "{{- if .cond }}ok{{ else }}no{{ end -}}";
        let ast = parse_template("span", src).unwrap();
        let if_node = match &ast.root.nodes[0] {
            Node::If(node) => node,
            other => panic!("expected If node, got {other:?}"),
        };
        assert_eq!(if_node.span.start, 0);
        assert_eq!(if_node.span.end, src.len());
        assert_eq!(if_node.pipeline.commands.len(), 1);
        assert!(matches!(
            if_node.pipeline.commands[0].target,
            Expression::Field(_)
        ));
    }

    #[test]
    fn range_pipeline_captures_declarations() {
        let src = "{{range $i, $v := .items}}{{$i}}{{$v}}{{end}}";
        let ast = parse_template("range-decls", src).unwrap();
        let range_node = match &ast.root.nodes[0] {
            Node::Range(node) => node,
            other => panic!("expected Range node, got {other:?}"),
        };
        let decls = range_node
            .pipeline
            .declarations
            .as_ref()
            .expect("range should declare variables");
        assert_eq!(decls.kind, BindingKind::Declare);
        assert_eq!(decls.variables, vec!["$i".to_string(), "$v".to_string()]);
        if let Expression::Field(parts) = &range_node.pipeline.commands[0].target {
            assert_eq!(parts, &vec!["items".to_string()]);
        } else {
            panic!("expected field expression for range data");
        }
    }

    #[test]
    fn assignment_pipeline_marks_bindings() {
        let src = "{{ $x := .value }}{{ $x = .other }}";
        let ast = parse_template("assign", src).unwrap();
        let first = match &ast.root.nodes[0] {
            Node::Action(node) => node,
            other => panic!("expected action node, got {other:?}"),
        };
        let first_decl = first
            .pipeline
            .declarations
            .as_ref()
            .expect("missing declaration on :=");
        assert_eq!(first_decl.kind, BindingKind::Declare);
        assert_eq!(first_decl.variables, vec!["$x".to_string()]);

        let second = match &ast.root.nodes[1] {
            Node::Action(node) => node,
            other => panic!("expected action node, got {other:?}"),
        };
        let second_decl = second
            .pipeline
            .declarations
            .as_ref()
            .expect("missing declaration on =");
        assert_eq!(second_decl.kind, BindingKind::Assign);
        assert_eq!(second_decl.variables, vec!["$x".to_string()]);
        assert!(matches!(
            second.pipeline.commands[0].target,
            Expression::Field(_)
        ));
    }
}
