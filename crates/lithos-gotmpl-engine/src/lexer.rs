// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::str::Chars;

use crate::ast::Span;
use crate::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(String),
    Dot,
    Pipe,
    Colon,
    Assign,
    Declare,
    Comma,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Operator(Operator),
    Keyword(Keyword),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    If,
    Else,
    End,
    Range,
    With,
    Nil,
    True,
    False,
}

impl Keyword {
    pub fn as_str(&self) -> &'static str {
        match self {
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::End => "end",
            Keyword::Range => "range",
            Keyword::With => "with",
            Keyword::Nil => "nil",
            Keyword::True => "true",
            Keyword::False => "false",
        }
    }
}

pub fn lex_action(input: &str, offset: usize) -> Result<Vec<Token>, Error> {
    let mut lexer = Lexer::new(input, offset);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token()? {
        tokens.push(token);
    }
    Ok(tokens)
}

struct Lexer<'a> {
    chars: Chars<'a>,
    pos: usize,
    offset: usize,
    peeked: Option<char>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str, offset: usize) -> Self {
        Self {
            chars: input.chars(),
            pos: 0,
            offset,
            peeked: None,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, Error> {
        self.skip_whitespace();

        let start = self.pos;
        let chr = match self.bump_char() {
            Some(c) => c,
            None => return Ok(None),
        };

        let token = match chr {
            '.' => Token {
                kind: TokenKind::Dot,
                span: self.span_from(start),
            },
            '|' => Token {
                kind: TokenKind::Pipe,
                span: self.span_from(start),
            },
            ':' => {
                if self.peek_char() == Some('=') {
                    self.bump_char();
                    Token {
                        kind: TokenKind::Declare,
                        span: self.span_from(start),
                    }
                } else {
                    Token {
                        kind: TokenKind::Colon,
                        span: self.span_from(start),
                    }
                }
            }
            '=' => {
                if self.peek_char() == Some('=') {
                    self.bump_char();
                    Token {
                        kind: TokenKind::Operator(Operator::Equal),
                        span: self.span_from(start),
                    }
                } else {
                    Token {
                        kind: TokenKind::Assign,
                        span: self.span_from(start),
                    }
                }
            }
            '!' => {
                if self.peek_char() == Some('=') {
                    self.bump_char();
                    Token {
                        kind: TokenKind::Operator(Operator::NotEqual),
                        span: self.span_from(start),
                    }
                } else {
                    return Err(Error::parse_with_span(
                        "unexpected '!' without '='",
                        self.span_from(start),
                    ));
                }
            }
            '<' => {
                if self.peek_char() == Some('=') {
                    self.bump_char();
                    Token {
                        kind: TokenKind::Operator(Operator::LessOrEqual),
                        span: self.span_from(start),
                    }
                } else {
                    Token {
                        kind: TokenKind::Operator(Operator::Less),
                        span: self.span_from(start),
                    }
                }
            }
            '>' => {
                if self.peek_char() == Some('=') {
                    self.bump_char();
                    Token {
                        kind: TokenKind::Operator(Operator::GreaterOrEqual),
                        span: self.span_from(start),
                    }
                } else {
                    Token {
                        kind: TokenKind::Operator(Operator::Greater),
                        span: self.span_from(start),
                    }
                }
            }
            '(' => Token {
                kind: TokenKind::LeftParen,
                span: self.span_from(start),
            },
            ')' => Token {
                kind: TokenKind::RightParen,
                span: self.span_from(start),
            },
            '[' => Token {
                kind: TokenKind::LeftBracket,
                span: self.span_from(start),
            },
            ']' => Token {
                kind: TokenKind::RightBracket,
                span: self.span_from(start),
            },
            ',' => Token {
                kind: TokenKind::Comma,
                span: self.span_from(start),
            },
            '"' => {
                let literal = self.read_string(start)?;
                Token {
                    kind: TokenKind::StringLiteral(literal),
                    span: self.span_from(start),
                }
            }
            '`' => {
                let literal = self.read_raw_string(start)?;
                Token {
                    kind: TokenKind::StringLiteral(literal),
                    span: self.span_from(start),
                }
            }
            c if is_identifier_start(c) => {
                let ident = self.read_identifier(c);
                let span = self.span_from(start);
                match ident.as_str() {
                    "if" => Token {
                        kind: TokenKind::Keyword(Keyword::If),
                        span,
                    },
                    "else" => Token {
                        kind: TokenKind::Keyword(Keyword::Else),
                        span,
                    },
                    "end" => Token {
                        kind: TokenKind::Keyword(Keyword::End),
                        span,
                    },
                    "range" => Token {
                        kind: TokenKind::Keyword(Keyword::Range),
                        span,
                    },
                    "with" => Token {
                        kind: TokenKind::Keyword(Keyword::With),
                        span,
                    },
                    "nil" => Token {
                        kind: TokenKind::Keyword(Keyword::Nil),
                        span,
                    },
                    "true" => Token {
                        kind: TokenKind::Keyword(Keyword::True),
                        span,
                    },
                    "false" => Token {
                        kind: TokenKind::Keyword(Keyword::False),
                        span,
                    },
                    _ => Token {
                        kind: TokenKind::Identifier(ident),
                        span,
                    },
                }
            }
            c if c.is_ascii_digit() => {
                let literal = self.read_number(c);
                Token {
                    kind: TokenKind::NumberLiteral(literal),
                    span: self.span_from(start),
                }
            }
            _ => {
                return Err(Error::parse(
                    format!("unexpected character '{}'", chr),
                    Some(self.span_from(start)),
                ));
            }
        };

        Ok(Some(token))
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.bump_char();
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self, first: char) -> String {
        let mut ident = String::new();
        ident.push(first);
        while let Some(ch) = self.peek_char() {
            if is_identifier_part(ch) {
                ident.push(self.bump_char().unwrap());
            } else {
                break;
            }
        }
        ident
    }

    fn read_string(&mut self, start: usize) -> Result<String, Error> {
        let mut literal = String::new();
        while let Some(ch) = self.bump_char() {
            match ch {
                '"' => return Ok(literal),
                '\\' => {
                    if let Some(next) = self.bump_char() {
                        let escaped = match next {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            other => other,
                        };
                        literal.push(escaped);
                    } else {
                        return Err(Error::parse_with_span(
                            "unterminated escape sequence",
                            self.span_from(start),
                        ));
                    }
                }
                other => literal.push(other),
            }
        }
        Err(Error::parse_with_span(
            "unterminated string literal",
            self.span_from(start),
        ))
    }

    fn read_number(&mut self, first: char) -> String {
        let mut literal = String::new();
        literal.push(first);

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || ch == '.' {
                literal.push(self.bump_char().unwrap());
            } else {
                break;
            }
        }
        literal
    }

    fn read_raw_string(&mut self, start: usize) -> Result<String, Error> {
        let mut literal = String::new();
        while let Some(ch) = self.bump_char() {
            match ch {
                '`' => return Ok(literal),
                _ => literal.push(ch),
            }
        }
        Err(Error::parse_with_span(
            "unterminated raw string literal",
            self.span_from(start),
        ))
    }

    fn bump_char(&mut self) -> Option<char> {
        if let Some(peek) = self.peeked.take() {
            self.pos += peek.len_utf8();
            Some(peek)
        } else {
            let ch = self.chars.next()?;
            self.pos += ch.len_utf8();
            Some(ch)
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        if self.peeked.is_none() {
            self.peeked = self.chars.next();
        }
        self.peeked
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(self.offset + start, self.offset + self.pos)
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn is_identifier_part(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn lexes_identifier_and_strings() {
        let tokens = lex_action(r#"default "value" .name"#, 0).unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Identifier("default".into()),
                TokenKind::StringLiteral("value".into()),
                TokenKind::Dot,
                TokenKind::Identifier("name".into())
            ]
        );
    }

    #[test]
    fn lexes_operators() {
        let tokens = lex_action(".a == .b", 0).unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                TokenKind::Dot,
                TokenKind::Identifier("a".into()),
                TokenKind::Operator(Operator::Equal),
                TokenKind::Dot,
                TokenKind::Identifier("b".into()),
            ]
        );
    }

    #[test]
    fn errors_on_unterminated_string() {
        let err = lex_action("\"unterminated", 0).unwrap_err();
        match err {
            Error::Parse { .. } => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
