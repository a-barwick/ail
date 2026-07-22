use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Declaration, Effect, Expr, FunctionDecl, LetBinding, ParameterType, RecordDecl, SourceUnit,
    Span, VariantDecl, parse,
};

const BUILTIN_TYPES: [&str; 5] = ["Text", "Int", "Unit", "Bool", "Bytes"];

/// Capability operation signatures supplied by the embedding compiler client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityOperation {
    /// Parameter types in declaration order.
    pub parameters: Vec<String>,
    /// The exact named result type.
    pub result: String,
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

    pub(crate) fn operation(&self, name: &str) -> Option<&CapabilityOperation> {
        self.operations.get(name)
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

    pub(crate) fn interface(&self, name: &str) -> Option<&CapabilityInterface> {
        self.interfaces.get(name)
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
    Value(String),
    Capability(String),
}

struct Checker<'a> {
    revision_id: &'a str,
    capabilities: &'a CapabilityEnvironment,
    unit: &'a SourceUnit,
    records: BTreeMap<&'a str, &'a RecordDecl>,
    variants: BTreeMap<&'a str, &'a VariantDecl>,
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
            top_level_names: BTreeMap::new(),
            problems: Vec::new(),
            facts: Vec::new(),
        }
    }

    fn check(&mut self) {
        self.collect_top_level_names();
        self.check_type_references();
        for declaration in &self.unit.declarations {
            if let Declaration::Function(function) = declaration {
                self.check_function(function);
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
                Declaration::Function(_) => {}
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
                        self.require_value_type(&field.ty, field.span);
                    }
                }
                Declaration::Variant(variant) => {
                    for case in &variant.cases {
                        if let Some(payload) = &case.payload {
                            self.require_value_type(payload, case.span);
                        }
                    }
                }
                Declaration::Function(function) => {
                    for parameter in &function.parameters {
                        match &parameter.ty {
                            ParameterType::Named(ty) => self.require_value_type(ty, parameter.span),
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
                    self.require_value_type(&function.result_type, function.span);
                }
            }
        }
    }

    fn require_value_type(&mut self, name: &str, span: Span) {
        if BUILTIN_TYPES.contains(&name)
            || self.records.contains_key(name)
            || self.variants.contains_key(name)
        {
            return;
        }
        self.unresolved_name(span, name, "type");
    }

    fn check_function(&mut self, function: &FunctionDecl) {
        let mut locals = BTreeMap::new();
        for parameter in &function.parameters {
            if locals.contains_key(parameter.name.as_str()) {
                self.duplicate_declaration(parameter.span, "parameter", &parameter.name);
                continue;
            }
            let binding = match &parameter.ty {
                ParameterType::Named(ty) => LocalBinding::Value(ty.clone()),
                ParameterType::Capability(interface) => LocalBinding::Capability(interface.clone()),
            };
            locals.insert(parameter.name.clone(), binding);
        }

        self.check_effect_clause(function, &locals);
        let tail_type = self.check_block(&function.body, function, &locals);
        if let Some(actual) = tail_type {
            if actual != function.result_type {
                self.type_problem(
                    "AIL.TYPE.RESULT_MISMATCH",
                    function.body.tail.span(),
                    fields([("type", text(&function.result_type))]),
                    fields([("type", text(&actual))]),
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
    ) -> Option<String> {
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
                    inferred_type: Some(inferred),
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
    ) -> Option<String> {
        self.check_expr(&binding.value, function, locals)
    }

    fn check_expr(
        &mut self,
        expression: &Expr,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<String> {
        match expression {
            Expr::Text { .. } => Some("Text".to_owned()),
            Expr::Integer { .. } => Some("Int".to_owned()),
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
        }
    }

    fn check_field_access(
        &mut self,
        target: &Expr,
        field_name: &str,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<String> {
        let target_type = self.check_expr(target, function, locals)?;
        let Some(record) = self.records.get(target_type.as_str()).copied() else {
            self.type_problem(
                "AIL.TYPE.FIELD_TARGET",
                target.span(),
                fields([("type_kind", text("record"))]),
                fields([("type", text(target_type))]),
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
    ) -> Option<String> {
        if let Some(condition_type) = self.check_expr(condition, function, locals) {
            if condition_type != "Bool" {
                self.type_problem(
                    "AIL.TYPE.IF_CONDITION",
                    condition.span(),
                    fields([("type", text("Bool"))]),
                    fields([("type", text(condition_type))]),
                    Vec::new(),
                    "check-if-condition",
                );
            }
        }
        let then_type = self.check_block(then_branch, function, locals);
        let else_type = self.check_block(else_branch, function, locals);
        match (then_type, else_type) {
            (Some(left), Some(right)) if left == right => Some(left),
            (Some(left), Some(right)) => {
                self.type_problem(
                    "AIL.TYPE.IF_BRANCH_MISMATCH",
                    else_branch.tail.span(),
                    fields([("type", text(left))]),
                    fields([("type", text(right))]),
                    Vec::new(),
                    "check-if-branches",
                );
                None
            }
            _ => None,
        }
    }

    fn check_match_expression(
        &mut self,
        scrutinee: &Expr,
        arms: &[crate::MatchArm],
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<String> {
        let scrutinee_type = self.check_expr(scrutinee, function, locals)?;
        let Some(variant) = self.variants.get(scrutinee_type.as_str()).copied() else {
            self.type_problem(
                "AIL.TYPE.MATCH_TARGET",
                scrutinee.span(),
                fields([("type_kind", text("variant"))]),
                fields([("type", text(scrutinee_type))]),
                Vec::new(),
                "check-match-target",
            );
            return None;
        };

        let mut seen = BTreeSet::new();
        let mut result_type: Option<String> = None;
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
                    fields([("type", text(payload))]),
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
                    if *expected != arm_type {
                        self.type_problem(
                            "AIL.TYPE.MATCH_ARM_MISMATCH",
                            arm.body.tail.span(),
                            fields([("type", text(expected))]),
                            fields([("type", text(arm_type))]),
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
    ) -> Option<String> {
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
                if actual != field.ty {
                    self.push_problem_with_chain(
                        ProblemClass::Type,
                        "AIL.TYPE.FIELD_MISMATCH",
                        "type",
                        value.value.span(),
                        fields([("type", text(&field.ty))]),
                        fields([("type", text(&actual))]),
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
        Some(name.to_owned())
    }

    fn check_variant_expression(
        &mut self,
        type_name: &str,
        case_name: &str,
        payload: Option<&Expr>,
        expression_span: Span,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<String> {
        let Some(variant) = self.variants.get(type_name).copied() else {
            self.unresolved_name(expression_span, type_name, "variant");
            return None;
        };
        let Some(case) = variant.cases.iter().find(|case| case.name == case_name) else {
            self.unresolved_name(expression_span, case_name, "variant-case");
            return Some(type_name.to_owned());
        };
        match (&case.payload, payload) {
            (None, None) => {}
            (Some(expected), Some(payload)) => {
                if let Some(actual) = self.check_expr(payload, function, locals) {
                    if actual != *expected {
                        self.type_problem(
                            "AIL.TYPE.VARIANT_PAYLOAD_MISMATCH",
                            payload.span(),
                            fields([("type", text(expected))]),
                            fields([("type", text(&actual))]),
                            vec![self.symbol_handle("variant", variant.span, &variant.name)],
                            "check-variant-construction",
                        );
                    }
                }
            }
            (Some(expected), None) => self.type_problem(
                "AIL.TYPE.VARIANT_PAYLOAD_MISMATCH",
                expression_span,
                fields([("type", text(expected))]),
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
        Some(type_name.to_owned())
    }

    fn check_capability_call(
        &mut self,
        receiver: &str,
        operation: &str,
        arguments: &[Expr],
        expression_span: Span,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBinding>,
    ) -> Option<String> {
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
                if actual != *expected {
                    ordinary_types_ok = false;
                    self.type_problem(
                        "AIL.TYPE.CAPABILITY_ARGUMENT",
                        argument.span(),
                        fields([("type", text(expected))]),
                        fields([("type", text(&actual))]),
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
        Some(signature.result.clone())
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
    ) -> String {
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
                if actual != *expected {
                    self.type_problem(
                        "AIL.TYPE.INTRINSIC_ARGUMENT",
                        argument.span(),
                        fields([("type", text(*expected))]),
                        fields([("type", text(actual))]),
                        Vec::new(),
                        "check-intrinsic-arguments",
                    );
                }
            }
        }
        result.to_owned()
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
            ParameterType::Named(ty) => ty.clone(),
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
