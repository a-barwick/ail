use crate::{
    Declaration, Effect, Expr, FunctionDecl, LetBinding, ParameterType, RecordDecl, SourceUnit,
    Span, TypeRef, ValueType, VariantDecl, parse,
};
use std::collections::{BTreeMap, BTreeSet};

const BUILTIN_TYPES: [&str; 6] = ["Text", "Int", "Unit", "Bool", "Bytes", "Cancellation"];

/// Host classification of a capability operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityOperationKind {
    Ordinary,
    Outbound(OutboundCapabilityMetadata),
}

/// Static cooperative outbound request contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundCapabilityMetadata {
    pub timeout_argument_index: usize,
    pub cancellation_argument_index: usize,
    pub maximum_timeout_ms: u128,
    pub timed_out_case_identity: String,
    pub cancelled_case_identity: String,
}

/// Capability operation signatures supplied by the embedding compiler client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityOperation {
    /// Parameter types in declaration order.
    pub parameters: Vec<String>,
    /// The exact named result type.
    pub result: String,
    pub kind: CapabilityOperationKind,
}

impl CapabilityOperation {
    /// Build one capability operation signature.
    #[must_use]
    pub fn new(
        parameters: impl IntoIterator<Item = impl Into<String>>,
        result: impl Into<String>,
    ) -> Self {
        Self {
            parameters: parameters.into_iter().map(Into::into).collect(),
            result: result.into(),
            kind: CapabilityOperationKind::Ordinary,
        }
    }

    #[must_use]
    pub fn outbound(
        parameters: impl IntoIterator<Item = impl Into<String>>,
        result: impl Into<String>,
        metadata: OutboundCapabilityMetadata,
    ) -> Self {
        Self {
            parameters: parameters.into_iter().map(Into::into).collect(),
            result: result.into(),
            kind: CapabilityOperationKind::Outbound(metadata),
        }
    }
}

/// The operations exposed by one capability type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityInterface {
    operations: BTreeMap<String, CapabilityOperation>,
}

impl CapabilityInterface {
    /// Create an empty interface.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace one operation signature.
    pub fn insert_operation(
        &mut self,
        name: impl Into<String>,
        operation: CapabilityOperation,
    ) -> Option<CapabilityOperation> {
        self.operations.insert(name.into(), operation)
    }

    #[must_use]
    pub fn operation(&self, name: &str) -> Option<&CapabilityOperation> {
        self.operations.get(name)
    }

    pub fn operations(&self) -> impl Iterator<Item = (&str, &CapabilityOperation)> {
        self.operations
            .iter()
            .map(|(name, operation)| (name.as_str(), operation))
    }
}

/// Capability interfaces available while checking one source unit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityEnvironment {
    interfaces: BTreeMap<String, CapabilityInterface>,
}

impl CapabilityEnvironment {
    /// Create an environment with no capability interfaces.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace one capability interface.
    pub fn insert_interface(
        &mut self,
        name: impl Into<String>,
        interface: CapabilityInterface,
    ) -> Option<CapabilityInterface> {
        self.interfaces.insert(name.into(), interface)
    }

    #[must_use]
    pub fn interface(&self, name: &str) -> Option<&CapabilityInterface> {
        self.interfaces.get(name)
    }

    pub fn interfaces(&self) -> impl Iterator<Item = (&str, &CapabilityInterface)> {
        self.interfaces
            .iter()
            .map(|(name, interface)| (name.as_str(), interface))
    }

    /// Deterministic digest independent of insertion order.
    #[must_use]
    pub fn stable_digest(&self) -> String {
        fn field(encoded: &mut String, value: &str) {
            encoded.push_str(&value.len().to_string());
            encoded.push(':');
            encoded.push_str(value);
        }

        let mut canonical = String::from("ail-capability-environment-v1;");
        for (interface_name, interface) in &self.interfaces {
            field(&mut canonical, interface_name);
            canonical.push(';');
            for (operation_name, operation) in &interface.operations {
                field(&mut canonical, operation_name);
                canonical.push(';');
                canonical.push_str(&operation.parameters.len().to_string());
                canonical.push(';');
                for parameter in &operation.parameters {
                    field(&mut canonical, parameter);
                    canonical.push(';');
                }
                field(&mut canonical, &operation.result);
                canonical.push(';');
                match &operation.kind {
                    CapabilityOperationKind::Ordinary => canonical.push_str("ordinary;"),
                    CapabilityOperationKind::Outbound(metadata) => {
                        canonical.push_str("outbound;");
                        canonical.push_str(&metadata.timeout_argument_index.to_string());
                        canonical.push(';');
                        canonical.push_str(&metadata.cancellation_argument_index.to_string());
                        canonical.push(';');
                        canonical.push_str(&metadata.maximum_timeout_ms.to_string());
                        canonical.push(';');
                        field(&mut canonical, &metadata.timed_out_case_identity);
                        canonical.push(';');
                        field(&mut canonical, &metadata.cancelled_case_identity);
                        canonical.push(';');
                    }
                }
            }
        }
        crate::protocol::source_digest(&canonical)
    }
}

/// A revision-scoped semantic location used by the M15 checker.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticHandle {
    /// Caller-provided immutable revision identifier.
    pub revision_id: String,
    /// Broad class of semantic location.
    pub kind: HandleKind,
    /// Deterministic identifier within this revision.
    pub local_id: String,
}

/// Kinds of semantic locations M15 can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandleKind {
    /// A declared semantic entity.
    Symbol,
    /// A declaration or field syntax location.
    Syntax,
    /// An expression syntax location.
    Expression,
}

/// One machine-readable value in a structured diagnostic field map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticValue {
    /// One named type, effect, or identifier.
    Text(String),
    /// A deterministic ordered list of strings.
    TextList(Vec<String>),
}

/// One deterministic causal step in a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalStep {
    /// Stable checker action name.
    pub step: String,
    /// Semantic location at which the action occurred.
    pub handle: SemanticHandle,
}

/// A structured static diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredDiagnostic {
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Source revision that was checked.
    pub revision_id: String,
    /// Broad diagnostic category.
    pub category: &'static str,
    /// Primary source-revision handle.
    pub primary_handle: SemanticHandle,
    /// Exact source span of the primary handle.
    pub primary_span: Span,
    /// Expected semantic facts, keyed deterministically.
    pub expected: BTreeMap<String, DiagnosticValue>,
    /// Actual semantic facts, keyed deterministically.
    pub actual: BTreeMap<String, DiagnosticValue>,
    /// Related semantic locations in deterministic order.
    pub related_handles: Vec<SemanticHandle>,
    /// Minimal, ordered checker steps that caused the result.
    pub causal_chain: Vec<CausalStep>,
}

/// The M11 type-checking outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCheckStatus {
    /// Parsing, names, and ordinary types passed.
    Ok,
    /// A name, duplicate declaration, or ordinary type error occurred.
    Error,
    /// Parsing failed, so M11 static checking did not run.
    NotRun,
}

/// One explicit or inferred type exposed by semantic checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeFact {
    /// Revision-scoped symbol handle.
    pub handle: SemanticHandle,
    /// Explicit public type, when this fact is a function boundary.
    pub explicit_type: Option<String>,
    /// Inferred local type, when this fact is a `let` binding.
    pub inferred_type: Option<String>,
}

/// The type result portion of a semantic check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCheckResult {
    /// Aggregate type-checking status.
    pub status: TypeCheckStatus,
    /// Inferred local and explicit public type facts.
    pub facts: Vec<TypeFact>,
}

/// Complete M15 result for one immutable source revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// Canonical source when parsing succeeded.
    pub canonical_source: Option<String>,
    /// M14 parser diagnostics. A non-empty value blocks static checking.
    pub parse_diagnostics: Vec<crate::Diagnostic>,
    /// Name and type checking outcome.
    pub type_result: TypeCheckResult,
    /// Structured static diagnostics, ordered by M11 primary-diagnostic order.
    pub diagnostics: Vec<StructuredDiagnostic>,
}

/// Parse and statically check an M11 source unit.
///
/// The supplied revision identifier scopes every semantic handle in the result.
/// Capability declarations are intentionally compiler input: adding capability
/// syntax would exceed M11's fixed five constructs.
#[must_use]
pub fn check_source(
    source: &str,
    revision_id: &str,
    capabilities: &CapabilityEnvironment,
) -> CheckResult {
    let parsed = parse(source);
    check_parsed_source(&parsed, revision_id, capabilities)
}

/// Check a source unit that has already been parsed for one immutable revision.
///
/// The revision protocol uses this entry point so revision creation retains one
/// lossless parse tree for both semantic checking and later handle indexing.
pub(crate) fn check_parsed_source(
    parsed: &crate::ParseResult,
    revision_id: &str,
    capabilities: &CapabilityEnvironment,
) -> CheckResult {
    if !parsed.diagnostics.is_empty() {
        return CheckResult {
            canonical_source: None,
            parse_diagnostics: parsed.diagnostics.clone(),
            type_result: TypeCheckResult {
                status: TypeCheckStatus::NotRun,
                facts: Vec::new(),
            },
            diagnostics: Vec::new(),
        };
    }

    let mut checker = Checker::new(revision_id, capabilities, &parsed.unit);
    checker.check();
    checker.finish()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ProblemClass {
    UnresolvedName,
    DuplicateDeclaration,
    Recursion,
    Type,
    Capability,
}

impl ProblemClass {
    const fn type_result_is_error(self) -> bool {
        !matches!(self, Self::Capability)
    }
}

#[derive(Debug, Clone)]
struct Problem {
    class: ProblemClass,
    diagnostic: StructuredDiagnostic,
}

#[derive(Debug, Clone)]
enum LocalBinding {
    Value(TypeRef),
    Capability(String),
}

struct Checker<'a> {
    revision_id: &'a str,
    capabilities: &'a CapabilityEnvironment,
    unit: &'a SourceUnit,
    records: BTreeMap<&'a str, &'a RecordDecl>,
    variants: BTreeMap<&'a str, &'a VariantDecl>,
    functions: BTreeMap<&'a str, &'a FunctionDecl>,
    top_level_names: BTreeMap<&'a str, Span>,
    problems: Vec<Problem>,
    facts: Vec<TypeFact>,
}

impl<'a> Checker<'a> {
    fn new(
        revision_id: &'a str,
        capabilities: &'a CapabilityEnvironment,
        unit: &'a SourceUnit,
    ) -> Self {
        Self {
            revision_id,
            capabilities,
            unit,
            records: BTreeMap::new(),
            variants: BTreeMap::new(),
            functions: BTreeMap::new(),
            top_level_names: BTreeMap::new(),
            problems: Vec::new(),
            facts: Vec::new(),
        }
    }

    fn check(&mut self) {
        self.collect_top_level_names();
        self.check_type_references();
        self.check_recursion();
        self.check_outbound_operations();
        for declaration in &self.unit.declarations {
            if let Declaration::Function(function) = declaration {
                self.check_function(function);
            }
        }
    }

    fn check_outbound_operations(&mut self) {
        for (interface_name, interface) in self.capabilities.interfaces() {
            for (operation_name, operation) in interface.operations() {
                let CapabilityOperationKind::Outbound(metadata) = &operation.kind else {
                    continue;
                };
                let operation_label = format!("{interface_name}.{operation_name}");
                let timeout_valid = metadata.timeout_argument_index < operation.parameters.len()
                    && operation.parameters[metadata.timeout_argument_index] == "Int"
                    && (1..=u128::from(u64::MAX)).contains(&metadata.maximum_timeout_ms);
                let code = if !timeout_valid {
                    Some((
                        "AIL.CAPABILITY.OUTBOUND_TIMEOUT_CONTRACT",
                        "check-outbound-timeout-contract",
                    ))
                } else if metadata.cancellation_argument_index >= operation.parameters.len()
                    || metadata.cancellation_argument_index == metadata.timeout_argument_index
                    || operation.parameters[metadata.cancellation_argument_index] != "Cancellation"
                {
                    Some((
                        "AIL.CAPABILITY.OUTBOUND_CANCELLATION_CONTRACT",
                        "check-outbound-cancellation-contract",
                    ))
                } else {
                    let variant = self.variants.get(operation.result.as_str()).copied();
                    let valid_case = |identity: &str| {
                        variant.is_some_and(|variant| {
                            variant.cases.iter().any(|case| {
                                case.identity.as_deref() == Some(identity) && case.payload.is_none()
                            })
                        })
                    };
                    (variant
                        .and_then(|variant| variant.identity.as_deref())
                        .is_none()
                        || metadata.timed_out_case_identity == metadata.cancelled_case_identity
                        || !valid_case(&metadata.timed_out_case_identity)
                        || !valid_case(&metadata.cancelled_case_identity))
                    .then_some((
                        "AIL.CAPABILITY.OUTBOUND_RESULT_CONTRACT",
                        "check-outbound-result-contract",
                    ))
                };
                if let Some((code, step)) = code {
                    self.capability_problem(
                        code,
                        Span::empty(0),
                        fields([("operation", text(operation_label))]),
                        BTreeMap::new(),
                        Vec::new(),
                        step,
                    );
                }
            }
        }
    }

    fn finish(mut self) -> CheckResult {
        self.problems.sort_by(|left, right| {
            left.class
                .cmp(&right.class)
                .then_with(|| {
                    left.diagnostic
                        .primary_span
                        .start
                        .cmp(&right.diagnostic.primary_span.start)
                })
                .then_with(|| {
                    left.diagnostic
                        .primary_handle
                        .kind
                        .cmp(&right.diagnostic.primary_handle.kind)
                })
                .then_with(|| {
                    left.diagnostic
                        .primary_handle
                        .local_id
                        .cmp(&right.diagnostic.primary_handle.local_id)
                })
        });
        let has_type_error = self
            .problems
            .iter()
            .any(|problem| problem.class.type_result_is_error());
        let status = if has_type_error {
            TypeCheckStatus::Error
        } else {
            TypeCheckStatus::Ok
        };
        let facts = if has_type_error {
            Vec::new()
        } else {
            self.facts
        };
        CheckResult {
            canonical_source: Some(crate::formatter::format(self.unit)),
            parse_diagnostics: Vec::new(),
            type_result: TypeCheckResult { status, facts },
            diagnostics: self
                .problems
                .into_iter()
                .map(|problem| problem.diagnostic)
                .collect(),
        }
    }

    fn collect_top_level_names(&mut self) {
        for declaration in &self.unit.declarations {
            let (name, span, kind) = match declaration {
                Declaration::Record(record) => (&record.name, record.span, "record"),
                Declaration::Variant(variant) => (&variant.name, variant.span, "variant"),
                Declaration::Function(function) => (&function.name, function.span, "function"),
            };
            if self.top_level_names.insert(name, span).is_some() {
                self.duplicate_declaration(span, kind, name);
            }
            match declaration {
                Declaration::Record(record) => {
                    self.records.entry(&record.name).or_insert(record);
                    self.check_unique_fields(record);
                }
                Declaration::Variant(variant) => {
                    self.variants.entry(&variant.name).or_insert(variant);
                    self.check_unique_variant_cases(variant);
                }
                Declaration::Function(function) => {
                    self.functions.entry(&function.name).or_insert(function);
                }
            }
        }
    }

    fn check_unique_fields(&mut self, record: &RecordDecl) {
        let mut names = BTreeSet::new();
        for field in &record.fields {
            if !names.insert(field.name.as_str()) {
                self.duplicate_declaration(field.span, "field", &field.name);
            }
        }
    }

    fn check_unique_variant_cases(&mut self, variant: &VariantDecl) {
        let mut names = BTreeSet::new();
        for case in &variant.cases {
            if !names.insert(case.name.as_str()) {
                self.duplicate_declaration(case.span, "variant-case", &case.name);
            }
        }
    }

    fn check_type_references(&mut self) {
        for declaration in &self.unit.declarations {
            match declaration {
                Declaration::Record(record) => {
                    for field in &record.fields {
                        self.require_value_type(&field.ty);
                    }
                }
                Declaration::Variant(variant) => {
                    for case in &variant.cases {
                        if let Some(payload) = &case.payload {
                            self.require_value_type(payload);
                        }
                    }
                }
                Declaration::Function(function) => {
                    for parameter in &function.parameters {
                        match &parameter.ty {
                            ParameterType::Value(ty) => self.require_value_type(ty),
                            ParameterType::Capability(interface) => {
                                if self.capabilities.interface(interface).is_none() {
                                    self.capability_problem(
                                        "AIL.CAPABILITY.UNKNOWN_INTERFACE",
                                        parameter.span,
                                        fields([("capability", text(interface))]),
                                        BTreeMap::new(),
                                        Vec::new(),
                                        "resolve-capability-interface",
                                    );
                                }
                            }
                        }
                    }
                    self.require_value_type(&function.result_type);
                }
            }
        }
    }

    fn check_recursion(&mut self) {
        let mut cycles = BTreeMap::<String, (Span, Vec<String>)>::new();
        for name in self.functions.keys().copied() {
            let mut stack = Vec::new();
            self.find_cycles(name, &mut stack, &mut cycles);
        }
        for (_, (span, cycle)) in cycles {
            let related = cycle
                .iter()
                .filter_map(|name| self.functions.get(name.as_str()))
                .map(|function| self.symbol_handle("function", function.span, &function.name))
                .collect();
            self.push_problem(
                ProblemClass::Recursion,
                "AIL.CALL.RECURSIVE_CYCLE",
                "call",
                span,
                fields([("rule", text("acyclic AIL call graph"))]),
                fields([("cycle", DiagnosticValue::TextList(cycle))]),
                related,
                "detect-recursive-call-cycle",
            );
        }
    }

    fn find_cycles(
        &self,
        name: &str,
        stack: &mut Vec<String>,
        cycles: &mut BTreeMap<String, (Span, Vec<String>)>,
    ) {
        if stack.iter().any(|entry| entry == name) {
            return;
        }
        let Some(function) = self.functions.get(name).copied() else {
            return;
        };
        stack.push(name.to_owned());
        for (callee, span) in calls_in_block(&function.body) {
            if let Some(start) = stack.iter().position(|entry| *entry == callee) {
                let mut cycle = stack[start..].to_vec();
                cycle.push(callee);
                let mut members = cycle[..cycle.len() - 1].to_vec();
                members.sort();
                cycles.entry(members.join("\0")).or_insert((span, cycle));
            } else {
                self.find_cycles(&callee, stack, cycles);
            }
        }
        stack.pop();
    }

    fn require_value_type(&mut self, ty: &TypeRef) {
        let ValueType::Named(name) = &ty.value else {
            let ValueType::List {
                element,
                max_length,
                max_length_spelling,
                max_length_span,
            } = &ty.value
            else {
                unreachable!("value types are named or bounded lists")
            };
            if !(1..=u128::from(crate::syntax::MAX_LIST_LENGTH)).contains(max_length) {
                self.type_problem(
                    "AIL.TYPE.LIST_BOUND",
                    *max_length_span,
                    fields([
                        ("minimum", text("1")),
                        ("maximum", text(crate::syntax::MAX_LIST_LENGTH.to_string())),
                    ]),
                    fields([("bound", text(max_length_spelling))]),
                    Vec::new(),
                    "check-list-bound",
                );
            }
            if element.as_list().is_some() {
                self.type_problem(
                    "AIL.TYPE.LIST_ELEMENT",
                    element.span,
                    fields([("type_kind", text("named value type"))]),
                    fields([("type", text(element.to_string()))]),
                    Vec::new(),
                    "check-list-element",
                );
            } else {
                self.require_value_type(element);
            }
            return;
        };
        if BUILTIN_TYPES.contains(&name.as_str())
            || self.records.contains_key(name.as_str())
            || self.variants.contains_key(name.as_str())
        {
            return;
        }
        self.unresolved_name(ty.span, name, "type");
    }

    fn check_function(&mut self, function: &FunctionDecl) {
        let mut locals = BTreeMap::new();
        for parameter in &function.parameters {
            if locals.contains_key(parameter.name.as_str()) {
                self.duplicate_declaration(parameter.span, "parameter", &parameter.name);
                continue;
            }
            let binding = match &parameter.ty {
                ParameterType::Value(ty) => LocalBinding::Value(ty.clone()),
                ParameterType::Capability(interface) => LocalBinding::Capability(interface.clone()),
            };
            locals.insert(parameter.name.clone(), binding);
        }

        self.check_effect_clause(function, &locals);
        let tail_type = self.check_block(&function.body, function, &locals);
        if let Some(actual) = tail_type {
            if !actual.same_type(&function.result_type) {
                self.type_problem(
                    "AIL.TYPE.RESULT_MISMATCH",
                    function.body.tail.span(),
                    fields([("type", text(function.result_type.to_string()))]),
                    fields([("type", text(actual.to_string()))]),
                    vec![self.symbol_handle("function", function.span, &function.name)],
                    "check-function-result",
                );
            }
        }
        self.facts.push(TypeFact {
            handle: self.symbol_handle("function", function.span, &function.name),
            explicit_type: Some(format_function_type(function)),
            inferred_type: None,
        });
    }

    fn check_effect_clause(
        &mut self,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) {
        let mut effects = BTreeSet::new();
        for effect in &function.effects {
            let key = format!("{}.{}", effect.receiver, effect.operation);
            if !effects.insert(key.clone()) {
                self.capability_problem(
                    "AIL.CAPABILITY.DUPLICATE_EFFECT",
                    effect.span,
                    fields([("effect", text(&key))]),
                    BTreeMap::new(),
                    vec![self.symbol_handle("function", function.span, &function.name)],
                    "check-declared-effects",
                );
                continue;
            }
            let Some(LocalBinding::Capability(interface)) = locals.get(effect.receiver.as_str())
            else {
                self.capability_problem(
                    "AIL.CAPABILITY.INVALID_EFFECT",
                    effect.span,
                    fields([("capability", text(&effect.receiver))]),
                    BTreeMap::new(),
                    vec![self.symbol_handle("function", function.span, &function.name)],
                    "resolve-declared-effect",
                );
                continue;
            };
            let Some(interface) = self.capabilities.interface(interface) else {
                continue;
            };
            if interface.operation(&effect.operation).is_none() {
                self.capability_problem(
                    "AIL.CAPABILITY.UNKNOWN_OPERATION",
                    effect.span,
                    fields([("operation", text(&key))]),
                    BTreeMap::new(),
                    vec![self.symbol_handle("function", function.span, &function.name)],
                    "resolve-capability-operation",
                );
            }
        }
    }

    fn check_block(
        &mut self,
        block: &crate::Block,
        function: &FunctionDecl,
        outer: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        let mut locals = outer.clone();
        for binding in &block.bindings {
            let inferred = self.check_binding(binding, function, &locals);
            if locals.contains_key(&binding.name) {
                self.duplicate_declaration(binding.span, "let-binding", &binding.name);
                continue;
            }
            if let Some(inferred) = inferred {
                locals.insert(binding.name.clone(), LocalBinding::Value(inferred.clone()));
                self.facts.push(TypeFact {
                    handle: self.symbol_handle(
                        "let",
                        binding.span,
                        &format!(
                            "{}:{}:{}",
                            binding.name, binding.span.start, binding.span.end
                        ),
                    ),
                    explicit_type: None,
                    inferred_type: Some(inferred.to_string()),
                });
            }
        }
        self.check_expr(&block.tail, function, &locals)
    }

    fn check_binding(
        &mut self,
        binding: &LetBinding,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        self.check_expr(&binding.value, function, locals)
    }

    #[allow(clippy::too_many_lines)]
    fn check_expr(
        &mut self,
        expression: &Expr,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        match expression {
            Expr::Text { span, .. } => Some(TypeRef::named("Text", *span)),
            Expr::Integer { span, .. } => Some(TypeRef::named("Int", *span)),
            Expr::Name { name, span } => match locals.get(name.as_str()) {
                Some(LocalBinding::Value(ty)) => Some(ty.clone()),
                Some(LocalBinding::Capability(_)) => {
                    self.capability_problem(
                        "AIL.CAPABILITY.VALUE_REQUIRED",
                        *span,
                        fields([("name", text(name))]),
                        BTreeMap::new(),
                        Vec::new(),
                        "resolve-local-name",
                    );
                    None
                }
                None => {
                    self.unresolved_name(*span, name, "value");
                    None
                }
            },
            Expr::Call {
                function: callee,
                arguments,
                span,
            } => self.check_function_call(callee, arguments, *span, function, locals),
            Expr::Record {
                name,
                fields: values,
                ..
            } => self.check_record_expression(name, values, expression.span(), function, locals),
            Expr::Variant {
                type_name,
                case,
                payload,
                ..
            } => self.check_variant_expression(
                type_name,
                case,
                payload.as_deref(),
                expression.span(),
                function,
                locals,
            ),
            Expr::CapabilityCall {
                receiver,
                operation,
                arguments,
                ..
            } => self.check_capability_call(
                receiver,
                operation,
                arguments,
                expression.span(),
                function,
                locals,
            ),
            Expr::FieldAccess { target, field, .. } => {
                self.check_field_access(target, field, function, locals)
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => self.check_if_expression(condition, then_branch, else_branch, function, locals),
            Expr::Match {
                scrutinee, arms, ..
            } => self.check_match_expression(scrutinee, arms, function, locals),
            Expr::Map {
                binding,
                source,
                body,
                span,
            } => self.check_map_expression(binding, source, body, *span, function, locals),
            Expr::ParallelMap {
                binding,
                source,
                limit,
                body,
                span,
                ..
            } => {
                let source_type = self.check_expr(source, function, locals)?;
                let Some((element, max)) = source_type.as_list() else {
                    self.type_problem(
                        "AIL.TYPE.PARALLEL_MAP_SOURCE",
                        *span,
                        fields([("type_kind", text("bounded list"))]),
                        fields([("type", text(source_type.to_string()))]),
                        Vec::new(),
                        "check-parallel-map-source",
                    );
                    return None;
                };
                let direct_parameter = matches!(source.as_ref(), Expr::Name { name, .. } if function.parameters.iter().any(|parameter| parameter.name == *name && matches!(parameter.ty, ParameterType::Value(_))));
                if !direct_parameter {
                    self.type_problem(
                        "AIL.TYPE.PARALLEL_MAP_SOURCE",
                        source.span(),
                        fields([("source", text("direct bounded-list parameter"))]),
                        fields([("source", text("computed expression"))]),
                        Vec::new(),
                        "check-parallel-map-inspectable-source",
                    );
                }
                if locals.contains_key(binding) {
                    self.duplicate_declaration(*span, "parallel-map-binding", binding);
                    return None;
                }
                if *limit == 0 || *limit > max {
                    self.type_problem(
                        "AIL.TYPE.PARALLEL_MAP_LIMIT",
                        *span,
                        fields([("range", text(format!("1..={max}")))]),
                        fields([("limit", text(limit.to_string()))]),
                        Vec::new(),
                        "check-parallel-map-limit",
                    );
                }
                if !body.bindings.is_empty() {
                    self.type_problem(
                        "AIL.TYPE.PARALLEL_MAP_BODY",
                        body.span,
                        fields([("body", text("one direct outbound capability call"))]),
                        fields([("let_bindings", text(body.bindings.len().to_string()))]),
                        Vec::new(),
                        "check-parallel-map-body",
                    );
                }
                let Expr::CapabilityCall {
                    receiver,
                    operation,
                    arguments,
                    ..
                } = &body.tail
                else {
                    self.type_problem(
                        "AIL.TYPE.PARALLEL_MAP_BODY",
                        body.tail.span(),
                        fields([("body", text("one direct outbound capability call"))]),
                        fields([("body", text("other expression"))]),
                        Vec::new(),
                        "check-parallel-map-body",
                    );
                    return None;
                };
                let outbound = locals
                    .get(receiver)
                    .and_then(|binding| {
                        if let LocalBinding::Capability(interface) = binding {
                            self.capabilities
                                .interface(interface)
                                .and_then(|i| i.operation(operation))
                        } else {
                            None
                        }
                    })
                    .is_some_and(|operation| {
                        matches!(operation.kind, CapabilityOperationKind::Outbound(_))
                    });
                if !outbound {
                    self.capability_problem(
                        "AIL.CAPABILITY.PARALLEL_MAP_OUTBOUND",
                        body.tail.span(),
                        fields([("operation_kind", text("outbound"))]),
                        BTreeMap::new(),
                        Vec::new(),
                        "check-parallel-map-operation",
                    );
                }
                let mut nested = locals.clone();
                nested.insert(binding.clone(), LocalBinding::Value(element.clone()));
                for argument in arguments {
                    if self.expression_reaches_capability_operation(argument, &mut BTreeSet::new())
                    {
                        self.capability_problem(
                            "AIL.CAPABILITY.PARALLEL_MAP_ARGUMENT_EFFECT",
                            argument.span(),
                            fields([("argument", text("effect-free expression"))]),
                            fields([("argument", text("outside operation"))]),
                            Vec::new(),
                            "check-parallel-map-argument-effects",
                        );
                    }
                }
                let result = self.check_expr(&body.tail, function, &nested)?;
                Some(TypeRef::list(result, max, *span))
            }
        }
    }

    fn check_map_expression(
        &mut self,
        binding: &str,
        source: &Expr,
        body: &crate::Block,
        span: Span,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        let source_type = self.check_expr(source, function, locals)?;
        let Some((element, max)) = source_type.as_list() else {
            self.type_problem(
                "AIL.TYPE.MAP_SOURCE",
                span,
                fields([("type_kind", text("bounded list"))]),
                fields([("type", text(source_type.to_string()))]),
                Vec::new(),
                "check-map-source",
            );
            return None;
        };
        if locals.contains_key(binding) {
            self.duplicate_declaration(span, "map-binding", binding);
            return None;
        }
        let mut nested = locals.clone();
        nested.insert(binding.to_owned(), LocalBinding::Value(element.clone()));
        let result = self.check_block(body, function, &nested)?;
        if result.as_list().is_some() {
            self.type_problem(
                "AIL.TYPE.LIST_ELEMENT",
                body.tail.span(),
                fields([("type_kind", text("named value type"))]),
                fields([("type", text(result.to_string()))]),
                Vec::new(),
                "check-map-result-element",
            );
            return None;
        }
        Some(TypeRef::list(result, max, span))
    }

    #[allow(clippy::too_many_lines)]
    fn check_function_call(
        &mut self,
        callee_name: &str,
        arguments: &[Expr],
        span: Span,
        caller: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        let Some(target_function) = self.functions.get(callee_name).copied() else {
            self.push_problem(
                ProblemClass::UnresolvedName,
                "AIL.NAME.UNKNOWN_FUNCTION",
                "name",
                span,
                fields([("function", text(callee_name))]),
                BTreeMap::new(),
                Vec::new(),
                "resolve-function-call",
            );
            for argument in arguments {
                let _ = self.check_expr(argument, caller, locals);
            }
            return None;
        };

        let value_parameters = target_function
            .parameters
            .iter()
            .filter_map(|parameter| match &parameter.ty {
                ParameterType::Value(ty) => Some((parameter, ty)),
                ParameterType::Capability(_) => None,
            })
            .collect::<Vec<_>>();
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_expr(argument, caller, locals))
            .collect::<Vec<_>>();
        if value_parameters.len() != arguments.len() {
            self.type_problem(
                "AIL.TYPE.FUNCTION_ARGUMENTS",
                span,
                fields([("count", text(value_parameters.len().to_string()))]),
                fields([("count", text(arguments.len().to_string()))]),
                vec![self.symbol_handle("function", target_function.span, &target_function.name)],
                "check-function-arguments",
            );
        }
        for ((argument, actual), (_, expected)) in
            arguments.iter().zip(argument_types).zip(&value_parameters)
        {
            if let Some(actual) = actual {
                if !actual.same_type(expected) {
                    self.type_problem(
                        "AIL.TYPE.FUNCTION_ARGUMENT",
                        argument.span(),
                        fields([("type", text(expected.to_string()))]),
                        fields([("type", text(actual.to_string()))]),
                        vec![self.symbol_handle(
                            "function",
                            target_function.span,
                            &target_function.name,
                        )],
                        "check-function-arguments",
                    );
                }
            }
        }

        for parameter in &target_function.parameters {
            let ParameterType::Capability(expected_interface) = &parameter.ty else {
                continue;
            };
            if !matches!(
                locals.get(&parameter.name),
                Some(LocalBinding::Capability(actual)) if actual == expected_interface
            ) {
                self.capability_problem(
                    "AIL.CAPABILITY.MISSING_TRANSITIVE_CAPABILITY",
                    span,
                    fields([
                        ("receiver", text(&parameter.name)),
                        ("interface", text(expected_interface)),
                    ]),
                    BTreeMap::new(),
                    vec![self.symbol_handle(
                        "function",
                        target_function.span,
                        &target_function.name,
                    )],
                    "resolve-transitive-capability",
                );
            }
        }

        let declared = effect_names(&caller.effects)
            .into_iter()
            .collect::<BTreeSet<_>>();
        for required in self.transitive_effects(callee_name, &mut BTreeSet::new()) {
            if !declared.contains(&required) {
                self.push_problem_with_chain(
                    ProblemClass::Capability,
                    "AIL.CAPABILITY.UNDECLARED_TRANSITIVE_EFFECT",
                    "capability",
                    span,
                    fields([(
                        "declared_effects",
                        DiagnosticValue::TextList(declared.iter().cloned().collect()),
                    )]),
                    fields([("required_effect", text(required))]),
                    vec![
                        self.symbol_handle("function", caller.span, &caller.name),
                        self.symbol_handle("function", target_function.span, &target_function.name),
                    ],
                    vec![
                        CausalStep {
                            step: "resolve-function-call".to_owned(),
                            handle: self.expression_handle(span),
                        },
                        CausalStep {
                            step: "compare-transitive-effects".to_owned(),
                            handle: self.symbol_handle("function", caller.span, &caller.name),
                        },
                    ],
                );
            }
        }
        Some(target_function.result_type.clone())
    }

    fn transitive_effects(
        &self,
        function_name: &str,
        visiting: &mut BTreeSet<String>,
    ) -> BTreeSet<String> {
        if !visiting.insert(function_name.to_owned()) {
            return BTreeSet::new();
        }
        let Some(function) = self.functions.get(function_name).copied() else {
            return BTreeSet::new();
        };
        let mut effects = effect_names(&function.effects)
            .into_iter()
            .collect::<BTreeSet<_>>();
        for (callee, _) in calls_in_block(&function.body) {
            effects.extend(self.transitive_effects(&callee, visiting));
        }
        visiting.remove(function_name);
        effects
    }

    fn expression_reaches_capability_operation(
        &self,
        expression: &Expr,
        visiting: &mut BTreeSet<String>,
    ) -> bool {
        match expression {
            Expr::CapabilityCall {
                receiver,
                operation,
                arguments,
                ..
            } => {
                intrinsic_signature(receiver, operation).is_none()
                    || arguments.iter().any(|argument| {
                        self.expression_reaches_capability_operation(argument, visiting)
                    })
            }
            Expr::Call {
                function,
                arguments,
                ..
            } => {
                let arguments_have_operations = arguments.iter().any(|argument| {
                    self.expression_reaches_capability_operation(argument, visiting)
                });
                if arguments_have_operations || !visiting.insert(function.clone()) {
                    return arguments_have_operations;
                }
                let result = self
                    .functions
                    .get(function.as_str())
                    .is_some_and(|function| {
                        !function.effects.is_empty()
                            || self.block_reaches_capability_operation(&function.body, visiting)
                    });
                visiting.remove(function);
                result
            }
            Expr::Record { fields, .. } => fields
                .iter()
                .any(|field| self.expression_reaches_capability_operation(&field.value, visiting)),
            Expr::Variant { payload, .. } => payload.as_deref().is_some_and(|payload| {
                self.expression_reaches_capability_operation(payload, visiting)
            }),
            Expr::FieldAccess { target, .. } => {
                self.expression_reaches_capability_operation(target, visiting)
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.expression_reaches_capability_operation(condition, visiting)
                    || self.block_reaches_capability_operation(then_branch, visiting)
                    || self.block_reaches_capability_operation(else_branch, visiting)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                self.expression_reaches_capability_operation(scrutinee, visiting)
                    || arms
                        .iter()
                        .any(|arm| self.block_reaches_capability_operation(&arm.body, visiting))
            }
            Expr::Map { source, body, .. } | Expr::ParallelMap { source, body, .. } => {
                self.expression_reaches_capability_operation(source, visiting)
                    || self.block_reaches_capability_operation(body, visiting)
            }
            Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => false,
        }
    }

    fn block_reaches_capability_operation(
        &self,
        block: &crate::Block,
        visiting: &mut BTreeSet<String>,
    ) -> bool {
        block
            .bindings
            .iter()
            .any(|binding| self.expression_reaches_capability_operation(&binding.value, visiting))
            || self.expression_reaches_capability_operation(&block.tail, visiting)
    }

    fn check_field_access(
        &mut self,
        target: &Expr,
        field_name: &str,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        let target_type = self.check_expr(target, function, locals)?;
        let Some(record_name) = target_type.as_named() else {
            self.type_problem(
                "AIL.TYPE.FIELD_TARGET",
                target.span(),
                fields([("type_kind", text("record"))]),
                fields([("type", text(target_type.to_string()))]),
                Vec::new(),
                "check-field-access",
            );
            return None;
        };
        let Some(record) = self.records.get(record_name).copied() else {
            self.type_problem(
                "AIL.TYPE.FIELD_TARGET",
                target.span(),
                fields([("type_kind", text("record"))]),
                fields([("type", text(target_type.to_string()))]),
                Vec::new(),
                "check-field-access",
            );
            return None;
        };
        let Some(field) = record.fields.iter().find(|field| field.name == field_name) else {
            self.type_problem(
                "AIL.TYPE.UNKNOWN_FIELD",
                target.span(),
                fields([(
                    "fields",
                    DiagnosticValue::TextList(
                        record
                            .fields
                            .iter()
                            .map(|field| field.name.clone())
                            .collect(),
                    ),
                )]),
                fields([("field", text(field_name))]),
                vec![self.symbol_handle("record", record.span, &record.name)],
                "check-field-access",
            );
            return None;
        };
        Some(field.ty.clone())
    }

    fn check_if_expression(
        &mut self,
        condition: &Expr,
        then_branch: &crate::Block,
        else_branch: &crate::Block,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        if let Some(condition_type) = self.check_expr(condition, function, locals) {
            if condition_type.as_named() != Some("Bool") {
                self.type_problem(
                    "AIL.TYPE.IF_CONDITION",
                    condition.span(),
                    fields([("type", text("Bool"))]),
                    fields([("type", text(condition_type.to_string()))]),
                    Vec::new(),
                    "check-if-condition",
                );
            }
        }
        let then_type = self.check_block(then_branch, function, locals);
        let else_type = self.check_block(else_branch, function, locals);
        match (then_type, else_type) {
            (Some(left), Some(right)) if left.same_type(&right) => Some(left),
            (Some(left), Some(right)) => {
                self.type_problem(
                    "AIL.TYPE.IF_BRANCH_MISMATCH",
                    else_branch.tail.span(),
                    fields([("type", text(left.to_string()))]),
                    fields([("type", text(right.to_string()))]),
                    Vec::new(),
                    "check-if-branches",
                );
                None
            }
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn check_match_expression(
        &mut self,
        scrutinee: &Expr,
        arms: &[crate::MatchArm],
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        let scrutinee_type = self.check_expr(scrutinee, function, locals)?;
        let Some(variant_name) = scrutinee_type.as_named() else {
            self.type_problem(
                "AIL.TYPE.MATCH_TARGET",
                scrutinee.span(),
                fields([("type_kind", text("variant"))]),
                fields([("type", text(scrutinee_type.to_string()))]),
                Vec::new(),
                "check-match-target",
            );
            return None;
        };
        let Some(variant) = self.variants.get(variant_name).copied() else {
            self.type_problem(
                "AIL.TYPE.MATCH_TARGET",
                scrutinee.span(),
                fields([("type_kind", text("variant"))]),
                fields([("type", text(scrutinee_type.to_string()))]),
                Vec::new(),
                "check-match-target",
            );
            return None;
        };

        let mut seen = BTreeSet::new();
        let mut result_type: Option<TypeRef> = None;
        for arm in arms {
            if arm.type_name != variant.name {
                self.type_problem(
                    "AIL.TYPE.MATCH_VARIANT",
                    arm.span,
                    fields([("variant", text(&variant.name))]),
                    fields([("variant", text(&arm.type_name))]),
                    Vec::new(),
                    "check-match-arm",
                );
                continue;
            }
            let Some(case) = variant.cases.iter().find(|case| case.name == arm.case) else {
                self.unresolved_name(arm.span, &arm.case, "variant-case");
                continue;
            };
            if !seen.insert(case.name.as_str()) {
                self.duplicate_declaration(arm.span, "match-arm", &case.name);
                continue;
            }
            let mut arm_locals = locals.clone();
            match (&case.payload, &arm.binding) {
                (Some(payload), Some(binding)) => {
                    if arm_locals.contains_key(binding) {
                        self.duplicate_declaration(arm.span, "match-binding", binding);
                    } else {
                        arm_locals.insert(binding.clone(), LocalBinding::Value(payload.clone()));
                    }
                }
                (Some(payload), None) => self.type_problem(
                    "AIL.TYPE.MATCH_BINDING",
                    arm.span,
                    fields([("type", text(payload.to_string()))]),
                    fields([("binding", text("missing"))]),
                    Vec::new(),
                    "check-match-pattern",
                ),
                (None, Some(binding)) => self.type_problem(
                    "AIL.TYPE.MATCH_BINDING",
                    arm.span,
                    fields([("type", text("Unit"))]),
                    fields([("binding", text(binding))]),
                    Vec::new(),
                    "check-match-pattern",
                ),
                (None, None) => {}
            }
            if let Some(arm_type) = self.check_block(&arm.body, function, &arm_locals) {
                if let Some(expected) = &result_type {
                    if !expected.same_type(&arm_type) {
                        self.type_problem(
                            "AIL.TYPE.MATCH_ARM_MISMATCH",
                            arm.body.tail.span(),
                            fields([("type", text(expected.to_string()))]),
                            fields([("type", text(arm_type.to_string()))]),
                            Vec::new(),
                            "check-match-arm-result",
                        );
                    }
                } else {
                    result_type = Some(arm_type);
                }
            }
        }
        let missing = variant
            .cases
            .iter()
            .filter(|case| !seen.contains(case.name.as_str()))
            .map(|case| case.name.clone())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            self.type_problem(
                "AIL.TYPE.NON_EXHAUSTIVE_MATCH",
                scrutinee.span(),
                fields([("cases", DiagnosticValue::TextList(missing))]),
                BTreeMap::new(),
                vec![self.symbol_handle("variant", variant.span, &variant.name)],
                "check-match-exhaustiveness",
            );
        }
        result_type
    }

    fn check_record_expression(
        &mut self,
        name: &str,
        values: &[crate::RecordFieldValue],
        expression_span: Span,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        let Some(record) = self.records.get(name).copied() else {
            self.unresolved_name(expression_span, name, "record");
            return None;
        };
        let declared = record
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field))
            .collect::<BTreeMap<_, _>>();
        let mut seen = BTreeSet::new();
        for value in values {
            if !seen.insert(value.name.as_str()) {
                self.duplicate_declaration(value.span, "record-field-initializer", &value.name);
                continue;
            }
            let actual = self.check_expr(&value.value, function, locals);
            let Some(field) = declared.get(value.name.as_str()) else {
                self.type_problem(
                    "AIL.TYPE.RECORD_FIELD_SET",
                    value.span,
                    fields([("field_set", text("declared record fields"))]),
                    fields([("field", text(&value.name))]),
                    vec![self.symbol_handle("record", record.span, &record.name)],
                    "check-record-initializer",
                );
                continue;
            };
            if let Some(actual) = actual {
                if !actual.same_type(&field.ty) {
                    self.push_problem_with_chain(
                        ProblemClass::Type,
                        "AIL.TYPE.FIELD_MISMATCH",
                        "type",
                        value.value.span(),
                        fields([("type", text(field.ty.to_string()))]),
                        fields([("type", text(actual.to_string()))]),
                        vec![
                            self.symbol_handle("record", record.span, &record.name),
                            self.symbol_handle(
                                "field",
                                field.span,
                                &format!("{}:{}", record.name, field.name),
                            ),
                        ],
                        vec![CausalStep {
                            step: "check-record-initializer".to_owned(),
                            handle: self.expression_handle(expression_span),
                        }],
                    );
                }
            }
        }
        for field in &record.fields {
            if !seen.contains(field.name.as_str()) {
                self.type_problem(
                    "AIL.TYPE.RECORD_FIELD_SET",
                    expression_span,
                    fields([("field", text(&field.name))]),
                    fields([("field", text("missing"))]),
                    vec![self.symbol_handle(
                        "field",
                        field.span,
                        &format!("{}:{}", record.name, field.name),
                    )],
                    "check-record-initializer",
                );
            }
        }
        Some(TypeRef::named(name, expression_span))
    }

    fn check_variant_expression(
        &mut self,
        type_name: &str,
        case_name: &str,
        payload: Option<&Expr>,
        expression_span: Span,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        let Some(variant) = self.variants.get(type_name).copied() else {
            self.unresolved_name(expression_span, type_name, "variant");
            return None;
        };
        let Some(case) = variant.cases.iter().find(|case| case.name == case_name) else {
            self.unresolved_name(expression_span, case_name, "variant-case");
            return Some(TypeRef::named(type_name, expression_span));
        };
        match (&case.payload, payload) {
            (None, None) => {}
            (Some(expected), Some(payload)) => {
                if let Some(actual) = self.check_expr(payload, function, locals) {
                    if !actual.same_type(expected) {
                        self.type_problem(
                            "AIL.TYPE.VARIANT_PAYLOAD_MISMATCH",
                            payload.span(),
                            fields([("type", text(expected.to_string()))]),
                            fields([("type", text(actual.to_string()))]),
                            vec![self.symbol_handle("variant", variant.span, &variant.name)],
                            "check-variant-construction",
                        );
                    }
                }
            }
            (Some(expected), None) => self.type_problem(
                "AIL.TYPE.VARIANT_PAYLOAD_MISMATCH",
                expression_span,
                fields([("type", text(expected.to_string()))]),
                fields([("type", text("missing"))]),
                vec![self.symbol_handle("variant", variant.span, &variant.name)],
                "check-variant-construction",
            ),
            (None, Some(payload)) => {
                let _ = self.check_expr(payload, function, locals);
                self.type_problem(
                    "AIL.TYPE.VARIANT_PAYLOAD_MISMATCH",
                    payload.span(),
                    fields([("type", text("Unit"))]),
                    fields([("type", text("unexpected"))]),
                    vec![self.symbol_handle("variant", variant.span, &variant.name)],
                    "check-variant-construction",
                );
            }
        }
        Some(TypeRef::named(type_name, expression_span))
    }

    #[allow(clippy::too_many_lines)]
    fn check_capability_call(
        &mut self,
        receiver: &str,
        operation: &str,
        arguments: &[Expr],
        expression_span: Span,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<TypeRef> {
        if let Some(signature) = intrinsic_signature(receiver, operation) {
            return Some(self.check_intrinsic_call(
                arguments,
                expression_span,
                function,
                locals,
                signature,
            ));
        }
        let Some(binding) = locals.get(receiver) else {
            self.unresolved_name(expression_span, receiver, "capability");
            return None;
        };
        let LocalBinding::Capability(interface_name) = binding else {
            self.capability_problem(
                "AIL.CAPABILITY.INVALID_RECEIVER",
                expression_span,
                fields([("receiver", text(receiver))]),
                BTreeMap::new(),
                Vec::new(),
                "resolve-capability-operation",
            );
            return None;
        };
        let interface = self.capabilities.interface(interface_name)?;
        let Some(signature) = interface.operation(operation) else {
            self.capability_problem(
                "AIL.CAPABILITY.UNKNOWN_OPERATION",
                expression_span,
                fields([("operation", text(format!("{receiver}.{operation}")))]),
                BTreeMap::new(),
                Vec::new(),
                "resolve-capability-operation",
            );
            return None;
        };

        if let CapabilityOperationKind::Outbound(metadata) = &signature.kind {
            if metadata.timeout_argument_index < arguments.len()
                && metadata.cancellation_argument_index < arguments.len()
                && metadata.timeout_argument_index != metadata.cancellation_argument_index
            {
                let timeout_is_control = arguments
                    .get(metadata.timeout_argument_index)
                    .is_some_and(|argument| {
                        matches!(argument, Expr::Name { .. } | Expr::Integer { .. })
                    });
                let cancellation_is_control = arguments
                    .get(metadata.cancellation_argument_index)
                    .is_some_and(|argument| matches!(argument, Expr::Name { .. }));
                if !timeout_is_control || !cancellation_is_control {
                    self.capability_problem(
                        "AIL.CAPABILITY.OUTBOUND_CONTROL_EFFECT",
                        expression_span,
                        fields([("controls", text("effect-free names or timeout literal"))]),
                        fields([("controls", text("computed expression"))]),
                        Vec::new(),
                        "validate-outbound-controls-before-effects",
                    );
                }
            }
        }

        let mut ordinary_types_ok = true;
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_expr(argument, function, locals))
            .collect::<Vec<_>>();
        if signature.parameters.len() != arguments.len() {
            ordinary_types_ok = false;
            self.type_problem(
                "AIL.TYPE.CAPABILITY_ARGUMENTS",
                expression_span,
                fields([("count", text(signature.parameters.len().to_string()))]),
                fields([("count", text(arguments.len().to_string()))]),
                Vec::new(),
                "check-capability-arguments",
            );
        }
        for ((argument, actual), expected) in arguments
            .iter()
            .zip(argument_types)
            .zip(&signature.parameters)
        {
            if let Some(actual) = actual {
                if actual.as_named() != Some(expected.as_str()) {
                    ordinary_types_ok = false;
                    self.type_problem(
                        "AIL.TYPE.CAPABILITY_ARGUMENT",
                        argument.span(),
                        fields([("type", text(expected))]),
                        fields([("type", text(actual.to_string()))]),
                        Vec::new(),
                        "check-capability-arguments",
                    );
                }
            } else {
                ordinary_types_ok = false;
            }
        }
        if ordinary_types_ok {
            self.check_call_effect(receiver, operation, expression_span, function);
        }
        Some(TypeRef::named(&signature.result, expression_span))
    }

    fn check_call_effect(
        &mut self,
        receiver: &str,
        operation: &str,
        expression_span: Span,
        function: &FunctionDecl,
    ) {
        if function
            .effects
            .iter()
            .any(|effect| effect.receiver == receiver && effect.operation == operation)
        {
            return;
        }
        let call_handle = self.expression_handle(expression_span);
        let function_handle = self.symbol_handle("function", function.span, &function.name);
        self.push_problem_with_chain(
            ProblemClass::Capability,
            "AIL.CAPABILITY.UNDECLARED_EFFECT",
            "capability",
            expression_span,
            fields([(
                "declared_effects",
                DiagnosticValue::TextList(effect_names(&function.effects)),
            )]),
            fields([("required_effect", text(format!("{receiver}.{operation}")))]),
            vec![
                function_handle.clone(),
                self.parameter_handle(function, receiver),
            ],
            vec![
                CausalStep {
                    step: "resolve-capability-operation".to_owned(),
                    handle: call_handle,
                },
                CausalStep {
                    step: "compare-declared-effects".to_owned(),
                    handle: function_handle,
                },
            ],
        );
    }

    fn check_intrinsic_call(
        &mut self,
        arguments: &[Expr],
        expression_span: Span,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
        (parameters, result): (&'static [&'static str], &'static str),
    ) -> TypeRef {
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_expr(argument, function, locals))
            .collect::<Vec<_>>();
        if parameters.len() != arguments.len() {
            self.type_problem(
                "AIL.TYPE.INTRINSIC_ARGUMENTS",
                expression_span,
                fields([("count", text(parameters.len().to_string()))]),
                fields([("count", text(arguments.len().to_string()))]),
                Vec::new(),
                "check-intrinsic-arguments",
            );
        }
        for ((argument, actual), expected) in arguments.iter().zip(argument_types).zip(parameters) {
            if let Some(actual) = actual {
                if actual.as_named() != Some(*expected) {
                    self.type_problem(
                        "AIL.TYPE.INTRINSIC_ARGUMENT",
                        argument.span(),
                        fields([("type", text(*expected))]),
                        fields([("type", text(actual.to_string()))]),
                        Vec::new(),
                        "check-intrinsic-arguments",
                    );
                }
            }
        }
        TypeRef::named(result, expression_span)
    }

    fn unresolved_name(&mut self, span: Span, name: &str, role: &str) {
        self.push_problem(
            ProblemClass::UnresolvedName,
            "AIL.NAME.UNRESOLVED",
            "name",
            span,
            fields([("name", text(name)), ("role", text(role))]),
            BTreeMap::new(),
            Vec::new(),
            "resolve-name",
        );
    }

    fn duplicate_declaration(&mut self, span: Span, kind: &str, name: &str) {
        self.push_problem(
            ProblemClass::DuplicateDeclaration,
            "AIL.NAME.DUPLICATE_DECLARATION",
            "name",
            span,
            fields([("name", text(name)), ("kind", text(kind))]),
            BTreeMap::new(),
            Vec::new(),
            "declare-name",
        );
    }

    fn type_problem(
        &mut self,
        code: &'static str,
        span: Span,
        expected: BTreeMap<String, DiagnosticValue>,
        actual: BTreeMap<String, DiagnosticValue>,
        related_handles: Vec<SemanticHandle>,
        step: &str,
    ) {
        self.push_problem(
            ProblemClass::Type,
            code,
            "type",
            span,
            expected,
            actual,
            related_handles,
            step,
        );
    }

    fn capability_problem(
        &mut self,
        code: &'static str,
        span: Span,
        expected: BTreeMap<String, DiagnosticValue>,
        actual: BTreeMap<String, DiagnosticValue>,
        related_handles: Vec<SemanticHandle>,
        step: &str,
    ) {
        self.push_problem(
            ProblemClass::Capability,
            code,
            "capability",
            span,
            expected,
            actual,
            related_handles,
            step,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_problem(
        &mut self,
        class: ProblemClass,
        code: &'static str,
        category: &'static str,
        span: Span,
        expected: BTreeMap<String, DiagnosticValue>,
        actual: BTreeMap<String, DiagnosticValue>,
        related_handles: Vec<SemanticHandle>,
        step: &str,
    ) {
        let primary_handle = self.expression_handle(span);
        self.push_problem_with_chain(
            class,
            code,
            category,
            span,
            expected,
            actual,
            related_handles,
            vec![CausalStep {
                step: step.to_owned(),
                handle: primary_handle,
            }],
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_problem_with_chain(
        &mut self,
        class: ProblemClass,
        code: &'static str,
        category: &'static str,
        span: Span,
        expected: BTreeMap<String, DiagnosticValue>,
        actual: BTreeMap<String, DiagnosticValue>,
        related_handles: Vec<SemanticHandle>,
        causal_chain: Vec<CausalStep>,
    ) {
        let primary_handle = self.expression_handle(span);
        let diagnostic = StructuredDiagnostic {
            code,
            revision_id: self.revision_id.to_owned(),
            category,
            primary_handle: primary_handle.clone(),
            primary_span: span,
            expected,
            actual,
            related_handles,
            causal_chain,
        };
        self.problems.push(Problem { class, diagnostic });
    }

    fn expression_handle(&self, span: Span) -> SemanticHandle {
        self.handle(
            HandleKind::Expression,
            span,
            &format!("expression:{}:{}", span.start, span.end),
        )
    }

    fn symbol_handle(&self, kind: &str, span: Span, name: &str) -> SemanticHandle {
        self.handle(HandleKind::Symbol, span, &format!("{kind}:{name}"))
    }

    fn parameter_handle(&self, function: &FunctionDecl, name: &str) -> SemanticHandle {
        let span = function
            .parameters
            .iter()
            .find(|parameter| parameter.name == name)
            .map_or(function.span, |parameter| parameter.span);
        self.symbol_handle("parameter", span, &format!("{}:{name}", function.name))
    }

    fn handle(&self, kind: HandleKind, _span: Span, local_id: &str) -> SemanticHandle {
        SemanticHandle {
            revision_id: self.revision_id.to_owned(),
            kind,
            local_id: local_id.to_owned(),
        }
    }
}

fn calls_in_block(block: &crate::Block) -> Vec<(String, Span)> {
    let mut calls = Vec::new();
    for binding in &block.bindings {
        collect_calls(&binding.value, &mut calls);
    }
    collect_calls(&block.tail, &mut calls);
    calls
}

fn collect_calls(expression: &Expr, calls: &mut Vec<(String, Span)>) {
    match expression {
        Expr::Call {
            function,
            arguments,
            span,
        } => {
            calls.push((function.clone(), *span));
            for argument in arguments {
                collect_calls(argument, calls);
            }
        }
        Expr::Record { fields, .. } => {
            for field in fields {
                collect_calls(&field.value, calls);
            }
        }
        Expr::Variant { payload, .. } => {
            if let Some(payload) = payload {
                collect_calls(payload, calls);
            }
        }
        Expr::CapabilityCall { arguments, .. } => {
            for argument in arguments {
                collect_calls(argument, calls);
            }
        }
        Expr::FieldAccess { target, .. } => collect_calls(target, calls),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_calls(condition, calls);
            calls.extend(calls_in_block(then_branch));
            calls.extend(calls_in_block(else_branch));
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_calls(scrutinee, calls);
            for arm in arms {
                calls.extend(calls_in_block(&arm.body));
            }
        }
        Expr::Map { source, body, .. } | Expr::ParallelMap { source, body, .. } => {
            collect_calls(source, calls);
            calls.extend(calls_in_block(body));
        }
        Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => {}
    }
}

fn effect_names(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .map(|effect| format!("{}.{}", effect.receiver, effect.operation))
        .collect()
}

pub(crate) fn intrinsic_signature(
    namespace: &str,
    operation: &str,
) -> Option<(&'static [&'static str], &'static str)> {
    match (namespace, operation) {
        ("text", "is_empty" | "first_ascii_alphanumeric" | "contains_control") => {
            Some((&["Text"], "Bool"))
        }
        ("text", "byte_length_between") => Some((&["Text", "Int", "Int"], "Bool")),
        ("text", "rest_ascii_alphanumeric_or") => Some((&["Text", "Text"], "Bool")),
        ("text", "scalar_count_gt") => Some((&["Text", "Int"], "Bool")),
        ("bytes", "length_gt") => Some((&["Bytes", "Int"], "Bool")),
        _ => None,
    }
}

fn format_function_type(function: &FunctionDecl) -> String {
    let parameters = function
        .parameters
        .iter()
        .map(|parameter| match &parameter.ty {
            ParameterType::Value(ty) => ty.to_string(),
            ParameterType::Capability(ty) => format!("capability {ty}"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut result = format!("fn({parameters}) -> {}", function.result_type);
    if !function.effects.is_empty() {
        result.push_str(" effects { ");
        result.push_str(&effect_names(&function.effects).join(", "));
        result.push_str(" }");
    }
    result
}

fn text(value: impl Into<String>) -> DiagnosticValue {
    DiagnosticValue::Text(value.into())
}

fn fields<const N: usize>(
    values: [(&str, DiagnosticValue); N],
) -> BTreeMap<String, DiagnosticValue> {
    values
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}
