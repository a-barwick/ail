//! Authoritative AIL compiler library.
//!
//! M14 implements the lossless syntax and canonical formatting boundary from
//! the fixed M11 contract. Static semantics and revision operations belong to
//! later milestones.

mod diagnostic;
mod formatter;
mod lexer;
mod parser;
mod syntax;

pub use diagnostic::Diagnostic;
pub use lexer::{Keyword, Span, Token, TokenKind, lex, reconstruct};
pub use parser::{ParseResult, parse};
pub use syntax::{
    Block, Declaration, Effect, Expr, Field, FunctionDecl, LetBinding, Parameter, ParameterType,
    RecordDecl, RecordFieldValue, SourceUnit, VariantCase, VariantDecl,
};

/// Parse and canonically format one M11 source unit.
///
/// Formatting is unavailable when parsing produced a diagnostic because M11
/// forbids static or canonical processing after a parse error.
///
/// # Errors
///
/// Returns all parse diagnostics when the source is not a valid M11 source
/// unit.
pub fn format_source(source: &str) -> Result<String, Vec<Diagnostic>> {
    let parsed = parse(source);
    if parsed.diagnostics.is_empty() {
        Ok(formatter::format(&parsed.unit))
    } else {
        Err(parsed.diagnostics)
    }
}
