// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::ast::Span;
use thiserror::Error;

/// Unified error type for the template engine.
///
/// Errors carry the message, optional source error, and – when available – the
/// `Span` pointing to the offending location in the template. Prefer the
/// `*_with_span` constructors when propagating parse or render failures that
/// originate from a concrete region of the template.
#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error: {message}")]
    Parse {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        span: Option<Span>,
    },
    #[error("render error: {message}")]
    Render {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        span: Option<Span>,
    },
}

impl Error {
    pub fn parse(message: impl Into<String>, span: Option<Span>) -> Self {
        Error::Parse {
            message: message.into(),
            source: None,
            span,
        }
    }

    pub fn parse_with_span(message: impl Into<String>, span: Span) -> Self {
        Self::parse(message, Some(span))
    }

    pub fn render(message: impl Into<String>, span: Option<Span>) -> Self {
        Error::Render {
            message: message.into(),
            source: None,
            span,
        }
    }

    pub fn render_with_span(message: impl Into<String>, span: Span) -> Self {
        Self::render(message, Some(span))
    }
}
