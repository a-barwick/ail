use crate::{Span, Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnit {
    pub declarations: Vec<Declaration>,
    pub span: Span,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declaration {
    Record(RecordDecl),
    Variant(VariantDecl),
    Function(FunctionDecl),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDecl {
    pub name: String,
    pub identity: Option<String>,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub identity: Option<String>,
    pub ty: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantDecl {
    pub name: String,
    pub identity: Option<String>,
    pub cases: Vec<VariantCase>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantCase {
    pub name: String,
    pub identity: Option<String>,
    pub payload: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDecl {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result_type: String,
    pub effects: Vec<Effect>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub ty: ParameterType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    Named(String),
    Capability(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub receiver: String,
    pub operation: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub bindings: Vec<LetBinding>,
    pub tail: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetBinding {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub type_name: String,
    pub case: String,
    pub binding: Option<String>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Text {
        value: String,
        span: Span,
    },
    Integer {
        spelling: String,
        span: Span,
    },
    Name {
        name: String,
        span: Span,
    },
    Record {
        name: String,
        fields: Vec<RecordFieldValue>,
        span: Span,
    },
    Variant {
        type_name: String,
        case: String,
        payload: Option<Box<Expr>>,
        span: Span,
    },
    CapabilityCall {
        receiver: String,
        operation: String,
        arguments: Vec<Expr>,
        span: Span,
    },
    FieldAccess {
        target: Box<Expr>,
        field: String,
        span: Span,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        else_branch: Box<Block>,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
}

impl Expr {
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Text { span, .. }
            | Self::Integer { span, .. }
            | Self::Name { span, .. }
            | Self::Record { span, .. }
            | Self::Variant { span, .. }
            | Self::CapabilityCall { span, .. }
            | Self::FieldAccess { span, .. }
            | Self::If { span, .. }
            | Self::Match { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFieldValue {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}
