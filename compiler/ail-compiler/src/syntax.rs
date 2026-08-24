use crate::{Span, Token};

pub const MAX_LIST_LENGTH: u32 = u32::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnit {
    pub module: Option<ModuleDecl>,
    pub imports: Vec<ImportDecl>,
    pub declarations: Vec<Declaration>,
    pub span: Span,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDecl {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    pub module: String,
    pub alias: Option<String>,
    pub span: Span,
}

impl ImportDecl {
    #[must_use]
    pub fn qualifier(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.module)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
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
    pub ty: TypeRef,
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
    pub payload: Option<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDecl {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result_type: TypeRef,
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
    Value(TypeRef),
    Capability(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub value: ValueType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Named(String),
    List {
        element: Box<TypeRef>,
        max_length: u128,
        max_length_spelling: String,
        max_length_span: Span,
    },
}

impl std::fmt::Display for TypeRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            ValueType::Named(name) => formatter.write_str(name),
            ValueType::List {
                element,
                max_length_spelling,
                ..
            } => write!(formatter, "List<{element}, {max_length_spelling}>"),
        }
    }
}

impl TypeRef {
    #[must_use]
    pub fn named(name: impl Into<String>, span: Span) -> Self {
        Self {
            value: ValueType::Named(name.into()),
            span,
        }
    }

    #[must_use]
    pub fn list(element: Self, max_length: u128, span: Span) -> Self {
        Self::list_with_bound_span(element, &max_length.to_string(), span, span)
    }

    #[must_use]
    pub(crate) fn list_with_bound_span(
        element: Self,
        max_length_spelling: &str,
        span: Span,
        max_length_span: Span,
    ) -> Self {
        let max_length_spelling = canonical_unsigned_decimal(max_length_spelling);
        let max_length = max_length_spelling.parse().unwrap_or(u128::MAX);
        Self {
            value: ValueType::List {
                element: Box::new(element),
                max_length,
                max_length_spelling,
                max_length_span,
            },
            span,
        }
    }

    #[must_use]
    pub fn as_named(&self) -> Option<&str> {
        let ValueType::Named(name) = &self.value else {
            return None;
        };
        Some(name)
    }

    #[must_use]
    pub fn as_list(&self) -> Option<(&Self, u128)> {
        let ValueType::List {
            element,
            max_length,
            ..
        } = &self.value
        else {
            return None;
        };
        Some((element, *max_length))
    }

    #[must_use]
    pub fn same_type(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (ValueType::Named(left), ValueType::Named(right)) => left == right,
            (
                ValueType::List {
                    element: left,
                    max_length: left_max,
                    ..
                },
                ValueType::List {
                    element: right,
                    max_length: right_max,
                    ..
                },
            ) => left_max == right_max && left.same_type(right),
            (ValueType::Named(_), ValueType::List { .. })
            | (ValueType::List { .. }, ValueType::Named(_)) => false,
        }
    }

    pub(crate) fn qualify(&mut self, resolve: &impl Fn(&str) -> String) {
        match &mut self.value {
            ValueType::Named(name) => *name = resolve(name),
            ValueType::List { element, .. } => element.qualify(resolve),
        }
    }

    pub(crate) fn named_references(&self) -> Vec<(&str, Span)> {
        match &self.value {
            ValueType::Named(name) => vec![(name, self.span)],
            ValueType::List { element, .. } => element.named_references(),
        }
    }
}

fn canonical_unsigned_decimal(spelling: &str) -> String {
    let canonical = spelling.trim_start_matches('0');
    if canonical.is_empty() {
        "0".to_owned()
    } else {
        canonical.to_owned()
    }
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
    Call {
        function: String,
        arguments: Vec<Expr>,
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
    Map {
        binding: String,
        source: Box<Expr>,
        body: Box<Block>,
        span: Span,
    },
    ParallelMap {
        binding: String,
        source: Box<Expr>,
        limit: u128,
        limit_spelling: String,
        body: Box<Block>,
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
            | Self::Call { span, .. }
            | Self::Record { span, .. }
            | Self::Variant { span, .. }
            | Self::CapabilityCall { span, .. }
            | Self::FieldAccess { span, .. }
            | Self::If { span, .. }
            | Self::Match { span, .. }
            | Self::Map { span, .. }
            | Self::ParallelMap { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFieldValue {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

/// Move every span in a syntax subtree by a fixed byte offset.
///
/// Linking an ordered source set into one unit loses which file a span came
/// from, because every file numbers its own bytes from zero. Shifting each
/// file's declarations into a disjoint range of one virtual buffer keeps that
/// mapping exact, so a diagnostic on the linked unit still names its file.
pub(crate) trait ShiftSpans {
    fn shift_spans(&mut self, delta: usize);
}

fn shift(span: &mut Span, delta: usize) {
    span.start += delta;
    span.end += delta;
}

impl<T: ShiftSpans> ShiftSpans for Vec<T> {
    fn shift_spans(&mut self, delta: usize) {
        for item in self {
            item.shift_spans(delta);
        }
    }
}

impl<T: ShiftSpans> ShiftSpans for Option<T> {
    fn shift_spans(&mut self, delta: usize) {
        if let Some(item) = self {
            item.shift_spans(delta);
        }
    }
}

impl<T: ShiftSpans> ShiftSpans for Box<T> {
    fn shift_spans(&mut self, delta: usize) {
        self.as_mut().shift_spans(delta);
    }
}

impl ShiftSpans for SourceUnit {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.module.shift_spans(delta);
        self.imports.shift_spans(delta);
        self.declarations.shift_spans(delta);
        for token in &mut self.tokens {
            shift(&mut token.span, delta);
        }
    }
}

impl ShiftSpans for ModuleDecl {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
    }
}

impl ShiftSpans for ImportDecl {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
    }
}

impl ShiftSpans for Declaration {
    fn shift_spans(&mut self, delta: usize) {
        match self {
            Self::Record(record) => record.shift_spans(delta),
            Self::Variant(variant) => variant.shift_spans(delta),
            Self::Function(function) => function.shift_spans(delta),
        }
    }
}

impl ShiftSpans for RecordDecl {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.fields.shift_spans(delta);
    }
}

impl ShiftSpans for Field {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.ty.shift_spans(delta);
    }
}

impl ShiftSpans for VariantDecl {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.cases.shift_spans(delta);
    }
}

impl ShiftSpans for VariantCase {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.payload.shift_spans(delta);
    }
}

impl ShiftSpans for FunctionDecl {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.parameters.shift_spans(delta);
        self.result_type.shift_spans(delta);
        self.effects.shift_spans(delta);
        self.body.shift_spans(delta);
    }
}

impl ShiftSpans for Parameter {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        match &mut self.ty {
            ParameterType::Value(ty) => ty.shift_spans(delta),
            ParameterType::Capability(_) => {}
        }
    }
}

impl ShiftSpans for TypeRef {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        match &mut self.value {
            ValueType::Named(_) => {}
            ValueType::List {
                element,
                max_length_span,
                ..
            } => {
                element.shift_spans(delta);
                shift(max_length_span, delta);
            }
        }
    }
}

impl ShiftSpans for Effect {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
    }
}

impl ShiftSpans for Block {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.bindings.shift_spans(delta);
        self.tail.shift_spans(delta);
    }
}

impl ShiftSpans for LetBinding {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.value.shift_spans(delta);
    }
}

impl ShiftSpans for MatchArm {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.body.shift_spans(delta);
    }
}

impl ShiftSpans for RecordFieldValue {
    fn shift_spans(&mut self, delta: usize) {
        shift(&mut self.span, delta);
        self.value.shift_spans(delta);
    }
}

impl ShiftSpans for Expr {
    fn shift_spans(&mut self, delta: usize) {
        match self {
            Self::Text { span, .. } | Self::Integer { span, .. } | Self::Name { span, .. } => {
                shift(span, delta);
            }
            Self::Call {
                arguments, span, ..
            }
            | Self::CapabilityCall {
                arguments, span, ..
            } => {
                shift(span, delta);
                arguments.shift_spans(delta);
            }
            Self::Record { fields, span, .. } => {
                shift(span, delta);
                fields.shift_spans(delta);
            }
            Self::Variant { payload, span, .. } => {
                shift(span, delta);
                payload.shift_spans(delta);
            }
            Self::FieldAccess { target, span, .. } => {
                shift(span, delta);
                target.shift_spans(delta);
            }
            Self::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                shift(span, delta);
                condition.shift_spans(delta);
                then_branch.shift_spans(delta);
                else_branch.shift_spans(delta);
            }
            Self::Match {
                scrutinee,
                arms,
                span,
            } => {
                shift(span, delta);
                scrutinee.shift_spans(delta);
                arms.shift_spans(delta);
            }
            Self::Map {
                source, body, span, ..
            }
            | Self::ParallelMap {
                source, body, span, ..
            } => {
                shift(span, delta);
                source.shift_spans(delta);
                body.shift_spans(delta);
            }
        }
    }
}
