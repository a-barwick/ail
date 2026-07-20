use crate::Span;

/// Structured parser diagnostic delivered by M14.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub category: &'static str,
    pub span: Span,
    pub expected: String,
    pub actual: String,
}

impl Diagnostic {
    pub(crate) fn expected_token(span: Span, expected: &str, actual: &str) -> Self {
        Self {
            code: "AIL.PARSE.EXPECTED_TOKEN",
            category: "parse",
            span,
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        }
    }
}
