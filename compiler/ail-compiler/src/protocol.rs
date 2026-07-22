//! Transport-independent M11 revision, inspection, and rename protocol.
//!
//! This module deliberately owns no transport or persistence mechanism. A
//! [`Workspace`] retains immutable, already-parsed source revisions so callers
//! can inspect revision-scoped handles and validate a rename transaction.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use crate::semantics::check_parsed_source;
use crate::{
    CapabilityEnvironment, CapabilityProvider, Declaration, Diagnostic, DiagnosticValue, Expr,
    FunctionDecl, HandleKind, ObservedCapabilityCall, ParameterType, RecordDecl, RuntimeFault,
    RuntimeValue, SemanticHandle, SourceUnit, Span, StructuredDiagnostic, TypeCheckStatus,
    VariantDecl, parse,
};

const RESERVED_NAMES: [&str; 9] = [
    "record",
    "variant",
    "fn",
    "let",
    "capability",
    "effects",
    "if",
    "else",
    "match",
];

/// Immutable source identity returned by protocol operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    /// Owning workspace identity.
    pub workspace_id: String,
    /// Opaque workspace-local immutable revision identity.
    pub revision_id: String,
    /// Parent revision for a successful structural edit.
    pub parent_revision_id: Option<String>,
    /// SHA-256 digest of the canonical source for this M11 source unit.
    pub source_digest: String,
}

/// Request one elaborated semantic record for a revision-scoped handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionRequest {
    /// Revision containing the requested handle.
    pub revision_id: String,
    /// Handle to inspect.
    pub handle: SemanticHandle,
}

/// Elaboration exposed for one source-revision handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionResult {
    /// Revision containing the inspected node.
    pub revision_id: String,
    /// Echo of the inspected handle.
    pub handle: SemanticHandle,
    /// Stable M11 semantic category such as `function` or `let-binding`.
    pub semantic_kind: String,
    /// Explicit boundary type, when present.
    pub explicit_type: Option<String>,
    /// Inferred local or expression type, when present.
    pub inferred_type: Option<String>,
    /// Declared capability effects in canonical source order.
    pub effects: Vec<String>,
    /// Capability parameters in canonical source order.
    pub capabilities: Vec<String>,
    /// Referenced named semantic dependencies in deterministic order.
    pub dependencies: Vec<String>,
}

/// One validated rename request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameRequest {
    /// Revision against which source positions and handles are interpreted.
    pub base_revision_id: String,
    /// Symbol to rename.
    pub handle: SemanticHandle,
    /// Replacement M11 identifier.
    pub new_name: String,
}

/// One canonical source edit expressed against a base revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEdit {
    /// Source path in the base revision.
    pub path: String,
    /// Inclusive start / exclusive end byte span in the base source.
    pub span: Span,
    /// UTF-8 replacement source.
    pub replacement: String,
}

/// Relationship of an old revision-scoped handle to a child revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityClassification {
    /// The semantic or syntax node remains byte-equivalent.
    Surviving,
    /// The node has a corresponding child node but its source bytes changed.
    Replaced,
    /// The node has no corresponding child node.
    Removed,
    /// The compiler could not establish a correspondence.
    Unmapped,
}

/// One deterministic old-to-new handle relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityMapEntry {
    /// Handle in the base revision.
    pub old_handle: SemanticHandle,
    /// Correspondence classification.
    pub classification: IdentityClassification,
    /// Corresponding child handle for surviving or replaced nodes.
    pub new_handle: Option<SemanticHandle>,
}

/// Complete handle correspondence for one committed rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityMap {
    /// Source revision.
    pub from_revision_id: String,
    /// Child revision.
    pub to_revision_id: String,
    /// One entry for every indexed base-revision handle.
    pub entries: Vec<IdentityMapEntry>,
    /// Child handles without a base-revision counterpart.
    pub new_handles: Vec<SemanticHandle>,
}

/// Validation phases recorded by a committed transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameValidation {
    /// Parser result.
    pub parse: &'static str,
    /// Ordinary name and type result.
    pub types: &'static str,
    /// Capability and effect result.
    pub capabilities: &'static str,
}

/// Successful atomic rename result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameSuccess {
    /// Always `committed`.
    pub status: &'static str,
    /// Revision that supplied the requested source positions.
    pub base_revision_id: String,
    /// Newly published immutable child revision.
    pub revision: Revision,
    /// Complete ordered canonical source edits.
    pub edits: Vec<CanonicalEdit>,
    /// Complete identity correspondence.
    pub identity_map: IdentityMap,
    /// Empty after a successful validated transaction.
    pub diagnostics: Vec<StructuredDiagnostic>,
    /// Validation phases that completed before publishing.
    pub validation: RenameValidation,
}

/// Failed atomic rename result. It never publishes a child revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameFailure {
    /// Always `rejected`.
    pub status: &'static str,
    /// Requested base revision.
    pub base_revision_id: String,
    /// Workspace revision current when the operation was rejected.
    pub current_revision_id: String,
    /// Primary rejection reason.
    pub diagnostic: StructuredDiagnostic,
    /// Always empty for a failed atomic transaction.
    pub edits: Vec<CanonicalEdit>,
}

/// Outcome of a transactional rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameResponse {
    /// Transaction committed and published one child revision.
    Committed(RenameSuccess),
    /// Transaction was rejected with no child revision.
    Rejected(RenameFailure),
}

/// Invoke one function in an immutable source revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRequest {
    pub revision_id: String,
    pub function: String,
    /// Arguments for ordinary value parameters, in declaration order.
    /// Capability parameters are supplied separately by the host provider.
    pub arguments: Vec<RuntimeValue>,
}

/// Successful deterministic execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSuccess {
    pub status: &'static str,
    pub revision_id: String,
    pub function_handle: SemanticHandle,
    pub value: RuntimeValue,
    pub calls: Vec<ObservedCapabilityCall>,
}

/// Failed deterministic execution, including effects observed before failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionFailure {
    pub status: &'static str,
    pub revision_id: String,
    pub function: String,
    pub fault: RuntimeFault,
    pub calls: Vec<ObservedCapabilityCall>,
}

/// Revision-scoped execution outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionResponse {
    Completed(ExecutionSuccess),
    Failed(ExecutionFailure),
}

/// Failure to establish an immutable revision because parsing did not complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionBuildFailure {
    /// M14 parse diagnostics that prevented revision storage.
    pub parse_diagnostics: Vec<Diagnostic>,
}

/// Immutable in-memory workspace for the M11 source-unit protocol.
#[derive(Debug, Clone)]
pub struct Workspace {
    id: String,
    capabilities: CapabilityEnvironment,
    current_revision_id: String,
    revisions: BTreeMap<String, StoredRevision>,
}

impl Workspace {
    /// Create a workspace with one immutable initial revision.
    ///
    /// Valid but non-canonical input is formatted before it is stored, so every
    /// externally visible revision digest and byte span refers to canonical
    /// source. Static diagnostics are retained on the revision; parsing is the
    /// only condition that prevents storage.
    ///
    /// # Errors
    ///
    /// Returns the deterministic M14 parse diagnostics when the initial source
    /// unit cannot be represented as an immutable revision.
    pub fn new(
        workspace_id: impl Into<String>,
        revision_id: impl Into<String>,
        path: impl Into<String>,
        source: &str,
        capabilities: CapabilityEnvironment,
    ) -> Result<Self, RevisionBuildFailure> {
        let workspace_id = workspace_id.into();
        let revision_id = revision_id.into();
        let prepared = StoredRevision::build(
            &workspace_id,
            &revision_id,
            None,
            path.into(),
            source,
            &capabilities,
        )?;
        let mut revisions = BTreeMap::new();
        revisions.insert(revision_id.clone(), prepared);
        Ok(Self {
            id: workspace_id,
            capabilities,
            current_revision_id: revision_id,
            revisions,
        })
    }

    /// Current immutable revision identity.
    #[must_use]
    pub fn current_revision_id(&self) -> &str {
        &self.current_revision_id
    }

    /// Current immutable revision metadata.
    #[must_use]
    pub fn current_revision(&self) -> &Revision {
        &self.current().revision
    }

    /// Canonical source retained for one immutable revision.
    #[must_use]
    pub fn source(&self, revision_id: &str) -> Option<&str> {
        self.revisions
            .get(revision_id)
            .map(|revision| revision.source.as_str())
    }

    /// Return every deterministic handle for one immutable revision in source order.
    #[must_use]
    pub fn handles(&self, revision_id: &str) -> Option<Vec<SemanticHandle>> {
        let revision = self.revisions.get(revision_id)?;
        let mut handles = revision
            .index
            .nodes
            .iter()
            .map(|node| node.handle.clone())
            .collect::<Vec<_>>();
        handles.sort_by_key(|handle| handle_order(&revision.index, handle));
        Some(handles)
    }

    /// Inspect one elaborated handle without modifying workspace state.
    ///
    /// # Errors
    ///
    /// Returns a structured stale or invalid-handle diagnostic when the
    /// requested revision or handle is not available.
    pub fn inspect(
        &self,
        request: InspectionRequest,
    ) -> Result<InspectionResult, Box<StructuredDiagnostic>> {
        let Some(revision) = self.revisions.get(&request.revision_id) else {
            return Err(Box::new(protocol_diagnostic(
                "AIL.PROTOCOL.STALE_HANDLE",
                "protocol",
                &request.revision_id,
                request.handle,
                Span::empty(0),
                fields([("reason", text("unknown revision"))]),
                BTreeMap::new(),
                "resolve-inspection-revision",
            )));
        };
        if request.handle.revision_id != request.revision_id {
            return Err(Box::new(protocol_diagnostic(
                "AIL.PROTOCOL.STALE_HANDLE",
                "protocol",
                &request.revision_id,
                request.handle,
                Span::empty(0),
                fields([("revision_id", text(&request.revision_id))]),
                BTreeMap::new(),
                "validate-handle-revision",
            )));
        }
        let Some(node) = revision.index.node(&request.handle) else {
            return Err(Box::new(protocol_diagnostic(
                "AIL.PROTOCOL.INVALID_HANDLE",
                "protocol",
                &request.revision_id,
                request.handle,
                Span::empty(0),
                fields([("reason", text("unknown handle"))]),
                BTreeMap::new(),
                "resolve-inspection-handle",
            )));
        };
        Ok(node.inspection())
    }

    /// Execute a statically valid function from one retained immutable revision.
    #[must_use]
    pub fn execute(
        &self,
        request: ExecutionRequest,
        capabilities: &mut dyn CapabilityProvider,
    ) -> ExecutionResponse {
        let Some(revision) = self.revisions.get(&request.revision_id) else {
            return ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: request.revision_id,
                function: request.function,
                fault: RuntimeFault::new(
                    "AIL.RUNTIME.UNKNOWN_REVISION",
                    Span::empty(0),
                    [("revision", "retained")],
                    [("revision", "unknown")],
                ),
                calls: Vec::new(),
            });
        };
        if !revision.check.diagnostics.is_empty()
            || !matches!(revision.check.type_result.status, TypeCheckStatus::Ok)
        {
            return ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: request.revision_id,
                function: request.function,
                fault: RuntimeFault::new(
                    "AIL.RUNTIME.STATIC_CHECK_REQUIRED",
                    Span::empty(0),
                    [("static_check", "ok")],
                    [("static_check", "error")],
                ),
                calls: Vec::new(),
            });
        }
        let Some(function_handle) = revision.index.function_handle(&request.function) else {
            return ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: request.revision_id,
                function: request.function.clone(),
                fault: RuntimeFault::new(
                    "AIL.RUNTIME.UNKNOWN_FUNCTION",
                    Span::empty(0),
                    [("function", request.function)],
                    std::iter::empty::<(&str, String)>(),
                ),
                calls: Vec::new(),
            });
        };
        let result = crate::interpreter::interpret(
            &revision.unit,
            &request.function,
            request.arguments,
            &self.capabilities,
            capabilities,
        );
        match result {
            Ok(result) => ExecutionResponse::Completed(ExecutionSuccess {
                status: "completed",
                revision_id: request.revision_id,
                function_handle,
                value: result.value,
                calls: result.calls,
            }),
            Err(result) => ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: request.revision_id,
                function: request.function,
                fault: result.fault,
                calls: result.calls,
            }),
        }
    }

    /// Atomically rename one symbol in the current complete M11 source unit.
    ///
    /// Every precondition and validation failure returns a rejected result and
    /// leaves the workspace's current revision unchanged.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn rename(&mut self, request: RenameRequest) -> RenameResponse {
        let current_revision_id = self.current_revision_id.clone();
        if request.base_revision_id != current_revision_id {
            return RenameResponse::Rejected(self.rename_failure(
                &request,
                protocol_diagnostic(
                    "AIL.PROTOCOL.STALE_REVISION",
                    "protocol",
                    &request.base_revision_id,
                    request.handle.clone(),
                    Span::empty(0),
                    fields([("current_revision_id", text(&current_revision_id))]),
                    fields([("base_revision_id", text(&request.base_revision_id))]),
                    "compare-current-revision",
                ),
            ));
        }

        let base = self.current();
        if request.handle.revision_id != request.base_revision_id
            || request.handle.kind != HandleKind::Symbol
        {
            return RenameResponse::Rejected(self.rename_failure(
                &request,
                protocol_diagnostic(
                    "AIL.PROTOCOL.INVALID_HANDLE",
                    "protocol",
                    &request.base_revision_id,
                    request.handle.clone(),
                    Span::empty(0),
                    fields([("required_kind", text("symbol"))]),
                    BTreeMap::new(),
                    "validate-rename-handle",
                ),
            ));
        }
        let Some(symbol) = base.index.symbol(&request.handle) else {
            return RenameResponse::Rejected(self.rename_failure(
                &request,
                protocol_diagnostic(
                    "AIL.PROTOCOL.INVALID_HANDLE",
                    "protocol",
                    &request.base_revision_id,
                    request.handle.clone(),
                    Span::empty(0),
                    fields([("reason", text("unknown symbol handle"))]),
                    BTreeMap::new(),
                    "resolve-rename-symbol",
                ),
            ));
        };
        if !is_m11_identifier(&request.new_name) {
            return RenameResponse::Rejected(self.rename_failure(
                &request,
                protocol_diagnostic(
                    "AIL.PROTOCOL.INVALID_NAME",
                    "protocol",
                    &request.base_revision_id,
                    request.handle.clone(),
                    base.index.span_for(&request.handle),
                    fields([("rule", text("M11 identifier and non-keyword"))]),
                    fields([("new_name", text(&request.new_name))]),
                    "validate-rename-name",
                ),
            ));
        }
        if base
            .index
            .has_collision(&symbol.scope, &request.handle, &request.new_name)
        {
            return RenameResponse::Rejected(self.rename_failure(
                &request,
                protocol_diagnostic(
                    "AIL.PROTOCOL.NAME_COLLISION",
                    "protocol",
                    &request.base_revision_id,
                    request.handle.clone(),
                    base.index.span_for(&request.handle),
                    fields([("scope", text(&symbol.scope))]),
                    fields([("new_name", text(&request.new_name))]),
                    "check-rename-collision",
                ),
            ));
        }
        if !base.check.diagnostics.is_empty()
            || !matches!(base.check.type_result.status, TypeCheckStatus::Ok)
        {
            let diagnostic = base.check.diagnostics.first().cloned().unwrap_or_else(|| {
                protocol_diagnostic(
                    "AIL.PROTOCOL.VALIDATION_FAILED",
                    "protocol",
                    &request.base_revision_id,
                    request.handle.clone(),
                    base.index.span_for(&request.handle),
                    fields([("reason", text("base revision is not statically valid"))]),
                    BTreeMap::new(),
                    "validate-base-revision",
                )
            });
            return RenameResponse::Rejected(self.rename_failure(&request, diagnostic));
        }

        let edits = symbol
            .occurrences
            .iter()
            .copied()
            .map(|span| CanonicalEdit {
                path: base.path.clone(),
                span,
                replacement: request.new_name.clone(),
            })
            .collect::<Vec<_>>();
        let candidate_source = apply_edits(&base.source, &edits);
        let child_revision_id = self.next_revision_id(&request.base_revision_id);
        let Ok(candidate) = StoredRevision::build(
            &self.id,
            &child_revision_id,
            Some(request.base_revision_id.clone()),
            base.path.clone(),
            &candidate_source,
            &self.capabilities,
        ) else {
            return RenameResponse::Rejected(self.rename_failure(
                &request,
                protocol_diagnostic(
                    "AIL.PROTOCOL.VALIDATION_FAILED",
                    "protocol",
                    &request.base_revision_id,
                    request.handle.clone(),
                    base.index.span_for(&request.handle),
                    fields([("phase", text("parse"))]),
                    BTreeMap::new(),
                    "validate-rename-parse",
                ),
            ));
        };
        if !candidate.check.diagnostics.is_empty()
            || !matches!(candidate.check.type_result.status, TypeCheckStatus::Ok)
        {
            let diagnostic = candidate
                .check
                .diagnostics
                .first()
                .cloned()
                .unwrap_or_else(|| {
                    protocol_diagnostic(
                        "AIL.PROTOCOL.VALIDATION_FAILED",
                        "protocol",
                        &request.base_revision_id,
                        request.handle.clone(),
                        base.index.span_for(&request.handle),
                        fields([("phase", text("types"))]),
                        BTreeMap::new(),
                        "validate-rename-types",
                    )
                });
            return RenameResponse::Rejected(self.rename_failure(&request, diagnostic));
        }

        let identity_map = identity_map(
            &base.index,
            &candidate.index,
            &base.source,
            &candidate.source,
        );
        let revision = candidate.revision.clone();
        self.revisions.insert(child_revision_id.clone(), candidate);
        self.current_revision_id = child_revision_id;
        RenameResponse::Committed(RenameSuccess {
            status: "committed",
            base_revision_id: request.base_revision_id,
            revision,
            edits,
            identity_map,
            diagnostics: Vec::new(),
            validation: RenameValidation {
                parse: "ok",
                types: "ok",
                capabilities: "ok",
            },
        })
    }

    fn current(&self) -> &StoredRevision {
        self.revisions
            .get(&self.current_revision_id)
            .expect("workspace current revision is always stored")
    }

    fn rename_failure(
        &self,
        request: &RenameRequest,
        diagnostic: StructuredDiagnostic,
    ) -> RenameFailure {
        RenameFailure {
            status: "rejected",
            base_revision_id: request.base_revision_id.clone(),
            current_revision_id: self.current_revision_id.clone(),
            diagnostic,
            edits: Vec::new(),
        }
    }

    fn next_revision_id(&self, base_revision_id: &str) -> String {
        let (prefix, number) = revision_number(base_revision_id);
        let mut candidate = number.saturating_add(1);
        loop {
            let id = format!("{prefix}{candidate}");
            if !self.revisions.contains_key(&id) {
                return id;
            }
            candidate = candidate.saturating_add(1);
        }
    }
}

#[derive(Debug, Clone)]
struct StoredRevision {
    revision: Revision,
    path: String,
    source: String,
    check: crate::CheckResult,
    unit: SourceUnit,
    index: HandleIndex,
}

impl StoredRevision {
    fn build(
        workspace_id: &str,
        revision_id: &str,
        parent_revision_id: Option<String>,
        path: String,
        source: &str,
        capabilities: &CapabilityEnvironment,
    ) -> Result<Self, RevisionBuildFailure> {
        let parsed = parse(source);
        let mut check = check_parsed_source(&parsed, revision_id, capabilities);
        if !check.parse_diagnostics.is_empty() {
            return Err(RevisionBuildFailure {
                parse_diagnostics: check.parse_diagnostics,
            });
        }
        let canonical = check
            .canonical_source
            .clone()
            .expect("successful M11 parse always has canonical source");

        // A non-canonical initial request is normalized before it becomes a
        // revision. Reparse only to make all stored spans refer to those
        // canonical bytes; semantic checking itself consumes the retained AST.
        let parsed = if canonical == source {
            parsed
        } else {
            parse(&canonical)
        };
        if canonical != source {
            check = check_parsed_source(&parsed, revision_id, capabilities);
        }
        let index = HandleIndex::build(revision_id, &parsed.unit, capabilities);
        Ok(Self {
            revision: Revision {
                workspace_id: workspace_id.to_owned(),
                revision_id: revision_id.to_owned(),
                parent_revision_id,
                source_digest: source_digest(&canonical),
            },
            path,
            source: canonical,
            check,
            unit: parsed.unit,
            index,
        })
    }
}

#[derive(Debug, Clone)]
struct IndexedNode {
    handle: SemanticHandle,
    span: Span,
    semantic_kind: &'static str,
    explicit_type: Option<String>,
    inferred_type: Option<String>,
    effects: Vec<String>,
    capabilities: Vec<String>,
    dependencies: Vec<String>,
    identity_key: String,
}

impl IndexedNode {
    fn inspection(&self) -> InspectionResult {
        InspectionResult {
            revision_id: self.handle.revision_id.clone(),
            handle: self.handle.clone(),
            semantic_kind: self.semantic_kind.to_owned(),
            explicit_type: self.explicit_type.clone(),
            inferred_type: self.inferred_type.clone(),
            effects: self.effects.clone(),
            capabilities: self.capabilities.clone(),
            dependencies: self.dependencies.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct SymbolData {
    scope: String,
    declared_name: String,
    occurrences: Vec<Span>,
}

#[derive(Debug, Clone, Default)]
struct HandleIndex {
    nodes: Vec<IndexedNode>,
    node_positions: BTreeMap<SemanticHandle, usize>,
    symbols: BTreeMap<SemanticHandle, SymbolData>,
}

impl HandleIndex {
    fn build(revision_id: &str, unit: &SourceUnit, capabilities: &CapabilityEnvironment) -> Self {
        IndexBuilder::new(revision_id, unit, capabilities).build()
    }

    fn node(&self, handle: &SemanticHandle) -> Option<&IndexedNode> {
        self.node_positions
            .get(handle)
            .and_then(|position| self.nodes.get(*position))
    }

    fn symbol(&self, handle: &SemanticHandle) -> Option<&SymbolData> {
        self.symbols.get(handle)
    }

    fn span_for(&self, handle: &SemanticHandle) -> Span {
        self.node(handle).map_or(Span::empty(0), |node| node.span)
    }

    fn has_collision(&self, scope: &str, requested: &SemanticHandle, name: &str) -> bool {
        self.symbols.iter().any(|(handle, symbol)| {
            handle != requested && symbol.scope == scope && symbol.declared_name == name
        })
    }

    fn function_handle(&self, name: &str) -> Option<SemanticHandle> {
        self.symbols.iter().find_map(|(handle, symbol)| {
            let node = self.node(handle)?;
            (symbol.scope == "top"
                && symbol.declared_name == name
                && node.semantic_kind == "function")
                .then(|| handle.clone())
        })
    }
}

struct IndexBuilder<'a> {
    revision_id: &'a str,
    unit: &'a SourceUnit,
    capabilities: &'a CapabilityEnvironment,
    records: BTreeMap<&'a str, (&'a RecordDecl, usize)>,
    variants: BTreeMap<&'a str, &'a VariantDecl>,
    top_symbols: BTreeMap<String, SemanticHandle>,
    field_symbols: BTreeMap<(String, String), SemanticHandle>,
    case_symbols: BTreeMap<(String, String), SemanticHandle>,
    index: HandleIndex,
}

impl<'a> IndexBuilder<'a> {
    fn new(
        revision_id: &'a str,
        unit: &'a SourceUnit,
        capabilities: &'a CapabilityEnvironment,
    ) -> Self {
        let mut records = BTreeMap::new();
        let mut variants = BTreeMap::new();
        for (declaration_index, declaration) in unit.declarations.iter().enumerate() {
            match declaration {
                Declaration::Record(record) => {
                    records.insert(record.name.as_str(), (record, declaration_index));
                }
                Declaration::Variant(variant) => {
                    variants.insert(variant.name.as_str(), variant);
                }
                Declaration::Function(_) => {}
            }
        }
        Self {
            revision_id,
            unit,
            capabilities,
            records,
            variants,
            top_symbols: BTreeMap::new(),
            field_symbols: BTreeMap::new(),
            case_symbols: BTreeMap::new(),
            index: HandleIndex::default(),
        }
    }

    fn build(mut self) -> HandleIndex {
        self.add_top_level_symbols();
        for (declaration_index, declaration) in self.unit.declarations.iter().enumerate() {
            match declaration {
                Declaration::Record(record) => self.index_record(record, declaration_index),
                Declaration::Variant(variant) => self.index_variant(variant, declaration_index),
                Declaration::Function(function) => self.index_function(function, declaration_index),
            }
        }
        self.index
    }

    fn add_top_level_symbols(&mut self) {
        for (declaration_index, declaration) in self.unit.declarations.iter().enumerate() {
            let (name, span, kind, semantic_kind) = match declaration {
                Declaration::Record(record) => (
                    record.name.as_str(),
                    identifier_after(&self.unit.tokens, record.span.start, 0),
                    "record",
                    "record",
                ),
                Declaration::Variant(variant) => (
                    variant.name.as_str(),
                    identifier_after(&self.unit.tokens, variant.span.start, 0),
                    "variant",
                    "variant",
                ),
                Declaration::Function(function) => (
                    function.name.as_str(),
                    identifier_after(&self.unit.tokens, function.span.start, 0),
                    "function",
                    "function",
                ),
            };
            let handle = self.symbol_handle(&format!("symbol:{kind}:{declaration_index}"));
            self.add_symbol(
                handle.clone(),
                span,
                semantic_kind,
                format!("symbol:{kind}:{declaration_index}"),
                "top".to_owned(),
                name.to_owned(),
                if kind == "function" {
                    let Declaration::Function(function) = declaration else {
                        unreachable!("function kind must contain a function")
                    };
                    Some(function_type(function))
                } else {
                    None
                },
                None,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            self.top_symbols.insert(name.to_owned(), handle);
        }
    }

    fn index_record(&mut self, record: &RecordDecl, declaration_index: usize) {
        let name_span = identifier_after(&self.unit.tokens, record.span.start, 0);
        self.add_syntax(
            name_span,
            "record-declaration-name",
            format!("syntax:record-name:{declaration_index}"),
        );
        for (field_index, field) in record.fields.iter().enumerate() {
            let field_name = identifier_at(&self.unit.tokens, field.span.start);
            let field_handle =
                self.symbol_handle(&format!("symbol:field:{declaration_index}:{field_index}"));
            self.add_symbol(
                field_handle.clone(),
                field_name,
                "record-field",
                format!("symbol:field:{declaration_index}:{field_index}"),
                format!("field:{declaration_index}"),
                field.name.clone(),
                Some(field.ty.clone()),
                None,
                Vec::new(),
                Vec::new(),
                vec![field.ty.clone()],
            );
            self.field_symbols
                .insert((record.name.clone(), field.name.clone()), field_handle);
            self.add_syntax(
                field_name,
                "record-field-name",
                format!("syntax:field-name:{declaration_index}:{field_index}"),
            );
            let type_span = identifier_after(&self.unit.tokens, field.span.start, 1);
            self.add_syntax(
                type_span,
                "type-reference",
                format!("syntax:field-type:{declaration_index}:{field_index}"),
            );
            self.add_top_reference(&field.ty, type_span);
        }
    }

    fn index_variant(&mut self, variant: &VariantDecl, declaration_index: usize) {
        let name_span = identifier_after(&self.unit.tokens, variant.span.start, 0);
        self.add_syntax(
            name_span,
            "variant-declaration-name",
            format!("syntax:variant-name:{declaration_index}"),
        );
        for (case_index, case) in variant.cases.iter().enumerate() {
            let case_span = identifier_at(&self.unit.tokens, case.span.start);
            let handle = self.symbol_handle(&format!(
                "symbol:variant-case:{declaration_index}:{case_index}"
            ));
            self.add_symbol(
                handle.clone(),
                case_span,
                "variant-case",
                format!("symbol:variant-case:{declaration_index}:{case_index}"),
                format!("variant-case:{declaration_index}"),
                case.name.clone(),
                case.payload.clone(),
                None,
                Vec::new(),
                Vec::new(),
                case.payload.clone().into_iter().collect(),
            );
            self.case_symbols
                .insert((variant.name.clone(), case.name.clone()), handle);
            self.add_syntax(
                case_span,
                "variant-case-name",
                format!("syntax:variant-case-name:{declaration_index}:{case_index}"),
            );
            if case.payload.is_some() {
                let payload_span = identifier_after(&self.unit.tokens, case.span.start, 1);
                self.add_syntax(
                    payload_span,
                    "type-reference",
                    format!("syntax:variant-payload:{declaration_index}:{case_index}"),
                );
                self.add_top_reference(case.payload.as_deref().unwrap_or_default(), payload_span);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn index_function(&mut self, function: &FunctionDecl, declaration_index: usize) {
        let function_name = identifier_after(&self.unit.tokens, function.span.start, 0);
        self.add_syntax(
            function_name,
            "function-declaration-name",
            format!("syntax:function-name:{declaration_index}"),
        );
        let mut locals = BTreeMap::new();
        let mut function_capabilities = Vec::new();
        for (parameter_index, parameter) in function.parameters.iter().enumerate() {
            let name_span = identifier_at(&self.unit.tokens, parameter.span.start);
            let (explicit_type, capability) = match &parameter.ty {
                ParameterType::Named(ty) => (Some(ty.clone()), None),
                ParameterType::Capability(interface) => (
                    Some(format!("capability {interface}")),
                    Some(interface.clone()),
                ),
            };
            if let Some(interface) = &capability {
                function_capabilities.push(format!("{}:{interface}", parameter.name));
            }
            let handle = self.symbol_handle(&format!(
                "symbol:parameter:{declaration_index}:{parameter_index}"
            ));
            self.add_symbol(
                handle.clone(),
                name_span,
                "parameter",
                format!("symbol:parameter:{declaration_index}:{parameter_index}"),
                format!("function:{declaration_index}"),
                parameter.name.clone(),
                explicit_type,
                None,
                Vec::new(),
                Vec::new(),
                parameter_dependencies(parameter),
            );
            self.add_syntax(
                name_span,
                "parameter-name",
                format!("syntax:parameter-name:{declaration_index}:{parameter_index}"),
            );
            let type_span = identifier_after(&self.unit.tokens, parameter.span.start, 1);
            self.add_syntax(
                type_span,
                "type-reference",
                format!("syntax:parameter-type:{declaration_index}:{parameter_index}"),
            );
            match &parameter.ty {
                ParameterType::Named(ty) => self.add_top_reference(ty, type_span),
                ParameterType::Capability(_) => {}
            }
            locals.insert(
                parameter.name.clone(),
                LocalBindingIndex {
                    handle,
                    value_type: match &parameter.ty {
                        ParameterType::Named(ty) => Some(ty.clone()),
                        ParameterType::Capability(_) => None,
                    },
                    capability,
                },
            );
        }

        let result_span = result_type_span(&self.unit.tokens, function);
        self.add_syntax(
            result_span,
            "type-reference",
            format!("syntax:function-result:{declaration_index}"),
        );
        self.add_top_reference(&function.result_type, result_span);
        for (effect_index, effect) in function.effects.iter().enumerate() {
            let receiver = identifier_at(&self.unit.tokens, effect.span.start);
            self.add_syntax(
                receiver,
                "effect-receiver",
                format!("syntax:effect-receiver:{declaration_index}:{effect_index}"),
            );
            self.add_local_reference(&locals, &effect.receiver, receiver);
            let operation = identifier_after(&self.unit.tokens, effect.span.start, 1);
            self.add_syntax(
                operation,
                "capability-operation",
                format!("syntax:effect-operation:{declaration_index}:{effect_index}"),
            );
        }

        for (binding_index, binding) in function.body.bindings.iter().enumerate() {
            let name_span = identifier_after(&self.unit.tokens, binding.span.start, 0);
            let value_type = self.expression_type(&binding.value, &locals);
            let dependencies = self.expression_dependencies(&binding.value, &locals);
            let handle =
                self.symbol_handle(&format!("symbol:let:{declaration_index}:{binding_index}"));
            self.add_symbol(
                handle.clone(),
                name_span,
                "let-binding",
                format!("symbol:let:{declaration_index}:{binding_index}"),
                format!("function:{declaration_index}"),
                binding.name.clone(),
                None,
                value_type.clone(),
                Vec::new(),
                Vec::new(),
                dependencies,
            );
            self.add_syntax(
                name_span,
                "let-binding-name",
                format!("syntax:let-name:{declaration_index}:{binding_index}"),
            );
            self.index_expression(
                &binding.value,
                &format!("expression:function:{declaration_index}:let:{binding_index}"),
                &locals,
            );
            locals.insert(
                binding.name.clone(),
                LocalBindingIndex {
                    handle,
                    value_type,
                    capability: None,
                },
            );
        }
        self.index_expression(
            &function.body.tail,
            &format!("expression:function:{declaration_index}:tail"),
            &locals,
        );

        let function_dependencies = self.function_dependencies(function, &locals);
        let function_handle = self.symbol_handle(&format!("symbol:function:{declaration_index}"));
        if let Some(node) = self.index.node_mut(&function_handle) {
            node.effects = effect_names(function);
            node.capabilities = function_capabilities;
            node.dependencies = function_dependencies;
        }
    }

    fn index_expression(
        &mut self,
        expression: &Expr,
        identity_key: &str,
        locals: &BTreeMap<String, LocalBindingIndex>,
    ) {
        let span = expression.span();
        self.add_expression(
            span,
            expression_kind(expression),
            identity_key.to_owned(),
            self.expression_type(expression, locals),
            self.expression_dependencies(expression, locals),
        );
        match expression {
            Expr::Text { .. } | Expr::Integer { .. } => {}
            Expr::Name { name, span } => {
                self.add_syntax(*span, "name-reference", format!("{identity_key}:name"));
                self.add_local_reference(locals, name, *span);
            }
            Expr::Record { name, fields, .. } => {
                let type_span = identifier_at(&self.unit.tokens, span.start);
                self.add_syntax(
                    type_span,
                    "record-reference",
                    format!("{identity_key}:record"),
                );
                self.add_top_reference(name, type_span);
                for (field_index, field) in fields.iter().enumerate() {
                    let field_span = identifier_at(&self.unit.tokens, field.span.start);
                    self.add_syntax(
                        field_span,
                        "record-field-reference",
                        format!("{identity_key}:field:{field_index}"),
                    );
                    self.add_field_reference(name, &field.name, field_span);
                    self.index_expression(
                        &field.value,
                        &format!("{identity_key}:field:{field_index}:value"),
                        locals,
                    );
                }
            }
            Expr::Variant {
                type_name,
                case,
                payload,
                ..
            } => {
                let type_span = identifier_at(&self.unit.tokens, span.start);
                self.add_syntax(
                    type_span,
                    "variant-reference",
                    format!("{identity_key}:variant"),
                );
                self.add_top_reference(type_name, type_span);
                let case_span = identifier_after(&self.unit.tokens, span.start, 1);
                self.add_syntax(
                    case_span,
                    "variant-case-reference",
                    format!("{identity_key}:case"),
                );
                self.add_case_reference(type_name, case, case_span);
                if let Some(payload) = payload {
                    self.index_expression(payload, &format!("{identity_key}:payload"), locals);
                }
            }
            Expr::CapabilityCall {
                receiver,
                arguments,
                ..
            } => {
                let receiver_span = identifier_at(&self.unit.tokens, span.start);
                self.add_syntax(
                    receiver_span,
                    "capability-receiver-reference",
                    format!("{identity_key}:receiver"),
                );
                self.add_local_reference(locals, receiver, receiver_span);
                let operation_span = identifier_after(&self.unit.tokens, span.start, 1);
                self.add_syntax(
                    operation_span,
                    "capability-operation",
                    format!("{identity_key}:operation"),
                );
                for (argument_index, argument) in arguments.iter().enumerate() {
                    self.index_expression(
                        argument,
                        &format!("{identity_key}:argument:{argument_index}"),
                        locals,
                    );
                }
            }
            Expr::FieldAccess { target, .. } => {
                self.index_expression(target, &format!("{identity_key}:target"), locals);
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => self.index_if(condition, then_branch, else_branch, identity_key, locals),
            Expr::Match {
                scrutinee, arms, ..
            } => self.index_match(scrutinee, arms, identity_key, locals),
        }
    }

    fn index_if(
        &mut self,
        condition: &Expr,
        then_branch: &crate::Block,
        else_branch: &crate::Block,
        identity_key: &str,
        locals: &BTreeMap<String, LocalBindingIndex>,
    ) {
        self.index_expression(condition, &format!("{identity_key}:condition"), locals);
        self.index_nested_block(then_branch, &format!("{identity_key}:then"), locals);
        self.index_nested_block(else_branch, &format!("{identity_key}:else"), locals);
    }

    fn index_match(
        &mut self,
        scrutinee: &Expr,
        arms: &[crate::MatchArm],
        identity_key: &str,
        locals: &BTreeMap<String, LocalBindingIndex>,
    ) {
        self.index_expression(scrutinee, &format!("{identity_key}:scrutinee"), locals);
        let scrutinee_type = self.expression_type(scrutinee, locals);
        for (arm_index, arm) in arms.iter().enumerate() {
            let mut arm_locals = locals.clone();
            if let (Some(binding), Some(variant_name)) = (&arm.binding, scrutinee_type.as_deref()) {
                let payload_type = self
                    .variants
                    .get(variant_name)
                    .and_then(|variant| variant.cases.iter().find(|case| case.name == arm.case))
                    .and_then(|case| case.payload.clone());
                if let Some(payload_type) = payload_type {
                    self.index_match_binding(
                        arm,
                        binding,
                        &payload_type,
                        identity_key,
                        arm_index,
                        &mut arm_locals,
                    );
                }
            }
            self.index_nested_block(
                &arm.body,
                &format!("{identity_key}:arm:{arm_index}"),
                &arm_locals,
            );
        }
    }

    fn index_match_binding(
        &mut self,
        arm: &crate::MatchArm,
        binding: &str,
        payload_type: &str,
        identity_key: &str,
        arm_index: usize,
        locals: &mut BTreeMap<String, LocalBindingIndex>,
    ) {
        let binding_span = identifier_after(&self.unit.tokens, arm.span.start, 2);
        let handle =
            self.symbol_handle(&format!("symbol:match-binding:{identity_key}:{arm_index}"));
        self.add_symbol(
            handle.clone(),
            binding_span,
            "match-binding",
            format!("{identity_key}:arm:{arm_index}:binding"),
            format!("match:{identity_key}:{arm_index}"),
            binding.to_owned(),
            None,
            Some(payload_type.to_owned()),
            Vec::new(),
            Vec::new(),
            vec![payload_type.to_owned()],
        );
        self.add_syntax(
            binding_span,
            "match-binding-name",
            format!("{identity_key}:arm:{arm_index}:binding-name"),
        );
        locals.insert(
            binding.to_owned(),
            LocalBindingIndex {
                handle,
                value_type: Some(payload_type.to_owned()),
                capability: None,
            },
        );
    }

    fn index_nested_block(
        &mut self,
        block: &crate::Block,
        identity_key: &str,
        outer: &BTreeMap<String, LocalBindingIndex>,
    ) {
        let mut locals = outer.clone();
        for (binding_index, binding) in block.bindings.iter().enumerate() {
            let value_type = self.expression_type(&binding.value, &locals);
            let dependencies = self.expression_dependencies(&binding.value, &locals);
            let name_span = identifier_after(&self.unit.tokens, binding.span.start, 0);
            let handle =
                self.symbol_handle(&format!("symbol:nested-let:{identity_key}:{binding_index}"));
            self.add_symbol(
                handle.clone(),
                name_span,
                "let-binding",
                format!("{identity_key}:let:{binding_index}"),
                format!("block:{identity_key}"),
                binding.name.clone(),
                None,
                value_type.clone(),
                Vec::new(),
                Vec::new(),
                dependencies,
            );
            self.add_syntax(
                name_span,
                "let-binding-name",
                format!("{identity_key}:let:{binding_index}:name"),
            );
            self.index_expression(
                &binding.value,
                &format!("{identity_key}:let:{binding_index}:value"),
                &locals,
            );
            locals.insert(
                binding.name.clone(),
                LocalBindingIndex {
                    handle,
                    value_type,
                    capability: None,
                },
            );
        }
        self.index_expression(&block.tail, &format!("{identity_key}:tail"), &locals);
    }

    fn expression_type(
        &self,
        expression: &Expr,
        locals: &BTreeMap<String, LocalBindingIndex>,
    ) -> Option<String> {
        match expression {
            Expr::Text { .. } => Some("Text".to_owned()),
            Expr::Integer { .. } => Some("Int".to_owned()),
            Expr::Name { name, .. } => locals
                .get(name)
                .and_then(|binding| binding.value_type.clone()),
            Expr::Record { name, .. } => Some(name.clone()),
            Expr::Variant { type_name, .. } => Some(type_name.clone()),
            Expr::CapabilityCall {
                receiver,
                operation,
                ..
            } => crate::semantics::intrinsic_signature(receiver, operation)
                .map(|(_, result)| result.to_owned())
                .or_else(|| {
                    locals
                        .get(receiver)
                        .and_then(|binding| binding.capability.as_deref())
                        .and_then(|interface| self.capabilities.interface(interface))
                        .and_then(|interface| interface.operation(operation))
                        .map(|operation| operation.result.clone())
                }),
            Expr::FieldAccess { target, field, .. } => {
                let target_type = self.expression_type(target, locals)?;
                self.records
                    .get(target_type.as_str())
                    .and_then(|(record, _)| record.fields.iter().find(|item| item.name == *field))
                    .map(|field| field.ty.clone())
            }
            Expr::If { then_branch, .. } => self.block_type(then_branch, locals),
            Expr::Match {
                scrutinee, arms, ..
            } => {
                let scrutinee_type = self.expression_type(scrutinee, locals)?;
                let variant = self.variants.get(scrutinee_type.as_str())?;
                let arm = arms.first()?;
                let mut arm_locals = locals.clone();
                if let Some(binding) = &arm.binding {
                    let payload = variant
                        .cases
                        .iter()
                        .find(|case| case.name == arm.case)?
                        .payload
                        .clone()?;
                    arm_locals.insert(
                        binding.clone(),
                        LocalBindingIndex {
                            handle: self.symbol_handle("expression-type:match-binding"),
                            value_type: Some(payload),
                            capability: None,
                        },
                    );
                }
                self.block_type(&arm.body, &arm_locals)
            }
        }
    }

    fn block_type(
        &self,
        block: &crate::Block,
        outer: &BTreeMap<String, LocalBindingIndex>,
    ) -> Option<String> {
        let mut locals = outer.clone();
        for binding in &block.bindings {
            let value_type = self.expression_type(&binding.value, &locals);
            locals.insert(
                binding.name.clone(),
                LocalBindingIndex {
                    handle: self.symbol_handle("expression-type:let"),
                    value_type,
                    capability: None,
                },
            );
        }
        self.expression_type(&block.tail, &locals)
    }

    fn expression_dependencies(
        &self,
        expression: &Expr,
        locals: &BTreeMap<String, LocalBindingIndex>,
    ) -> Vec<String> {
        let mut dependencies = BTreeSet::new();
        self.collect_expression_dependencies(expression, locals, &mut dependencies);
        dependencies.into_iter().collect()
    }

    fn collect_expression_dependencies(
        &self,
        expression: &Expr,
        locals: &BTreeMap<String, LocalBindingIndex>,
        dependencies: &mut BTreeSet<String>,
    ) {
        match expression {
            Expr::Text { .. } | Expr::Integer { .. } => {}
            Expr::Name { name, .. } => {
                if let Some(Some(ty)) = locals.get(name).map(|binding| &binding.value_type) {
                    dependencies.insert(ty.clone());
                }
            }
            Expr::Record { name, fields, .. } => {
                dependencies.insert(name.clone());
                if let Some((record, _)) = self.records.get(name.as_str()) {
                    for field in &record.fields {
                        dependencies.insert(field.ty.clone());
                    }
                }
                for field in fields {
                    self.collect_expression_dependencies(&field.value, locals, dependencies);
                }
            }
            Expr::Variant {
                type_name, payload, ..
            } => {
                dependencies.insert(type_name.clone());
                if let Some(payload) = payload {
                    self.collect_expression_dependencies(payload, locals, dependencies);
                }
            }
            Expr::CapabilityCall {
                receiver,
                operation,
                arguments,
                ..
            } => {
                if let Some(binding) = locals.get(receiver) {
                    if let Some(interface) = &binding.capability {
                        dependencies.insert(interface.clone());
                        if let Some(operation) = self
                            .capabilities
                            .interface(interface)
                            .and_then(|interface| interface.operation(operation))
                        {
                            dependencies.extend(operation.parameters.iter().cloned());
                            dependencies.insert(operation.result.clone());
                        }
                    }
                }
                for argument in arguments {
                    self.collect_expression_dependencies(argument, locals, dependencies);
                }
            }
            Expr::FieldAccess { target, .. } => {
                self.collect_expression_dependencies(target, locals, dependencies);
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.collect_expression_dependencies(condition, locals, dependencies);
                self.collect_block_dependencies(then_branch, locals, dependencies);
                self.collect_block_dependencies(else_branch, locals, dependencies);
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                self.collect_expression_dependencies(scrutinee, locals, dependencies);
                let scrutinee_type = self.expression_type(scrutinee, locals);
                for arm in arms {
                    dependencies.insert(arm.type_name.clone());
                    let mut arm_locals = locals.clone();
                    if let (Some(binding), Some(variant_name)) =
                        (&arm.binding, scrutinee_type.as_deref())
                    {
                        if let Some(payload_type) = self
                            .variants
                            .get(variant_name)
                            .and_then(|variant| {
                                variant.cases.iter().find(|case| case.name == arm.case)
                            })
                            .and_then(|case| case.payload.clone())
                        {
                            arm_locals.insert(
                                binding.clone(),
                                LocalBindingIndex {
                                    handle: self.symbol_handle("dependency:match-binding"),
                                    value_type: Some(payload_type),
                                    capability: None,
                                },
                            );
                        }
                    }
                    self.collect_block_dependencies(&arm.body, &arm_locals, dependencies);
                }
            }
        }
    }

    fn collect_block_dependencies(
        &self,
        block: &crate::Block,
        locals: &BTreeMap<String, LocalBindingIndex>,
        dependencies: &mut BTreeSet<String>,
    ) {
        let mut locals = locals.clone();
        for binding in &block.bindings {
            self.collect_expression_dependencies(&binding.value, &locals, dependencies);
            let value_type = self.expression_type(&binding.value, &locals);
            locals.insert(
                binding.name.clone(),
                LocalBindingIndex {
                    handle: self.symbol_handle("dependency:let"),
                    value_type,
                    capability: None,
                },
            );
        }
        self.collect_expression_dependencies(&block.tail, &locals, dependencies);
    }

    fn function_dependencies(
        &self,
        function: &FunctionDecl,
        locals: &BTreeMap<String, LocalBindingIndex>,
    ) -> Vec<String> {
        let mut dependencies = BTreeSet::new();
        for parameter in &function.parameters {
            match &parameter.ty {
                ParameterType::Named(ty) | ParameterType::Capability(ty) => {
                    dependencies.insert(ty.clone());
                }
            }
        }
        dependencies.insert(function.result_type.clone());
        self.collect_block_dependencies(&function.body, locals, &mut dependencies);
        dependencies.into_iter().collect()
    }

    fn add_top_reference(&mut self, name: &str, span: Span) {
        if let Some(handle) = self.top_symbols.get(name).cloned() {
            self.add_occurrence(&handle, span);
        }
    }

    fn add_field_reference(&mut self, record: &str, field: &str, span: Span) {
        if let Some(handle) = self
            .field_symbols
            .get(&(record.to_owned(), field.to_owned()))
            .cloned()
        {
            self.add_occurrence(&handle, span);
        }
    }

    fn add_case_reference(&mut self, variant: &str, case: &str, span: Span) {
        if let Some(handle) = self
            .case_symbols
            .get(&(variant.to_owned(), case.to_owned()))
            .cloned()
        {
            self.add_occurrence(&handle, span);
        }
    }

    fn add_local_reference(
        &mut self,
        locals: &BTreeMap<String, LocalBindingIndex>,
        name: &str,
        span: Span,
    ) {
        if let Some(binding) = locals.get(name) {
            self.add_occurrence(&binding.handle, span);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_symbol(
        &mut self,
        handle: SemanticHandle,
        span: Span,
        semantic_kind: &'static str,
        identity_key: String,
        scope: String,
        declared_name: String,
        explicit_type: Option<String>,
        inferred_type: Option<String>,
        effects: Vec<String>,
        capabilities: Vec<String>,
        dependencies: Vec<String>,
    ) {
        self.add_node(IndexedNode {
            handle: handle.clone(),
            span,
            semantic_kind,
            explicit_type,
            inferred_type,
            effects,
            capabilities,
            dependencies,
            identity_key,
        });
        self.index.symbols.insert(
            handle,
            SymbolData {
                scope,
                declared_name,
                occurrences: vec![span],
            },
        );
    }

    fn add_syntax(&mut self, span: Span, semantic_kind: &'static str, identity_key: String) {
        let handle = SemanticHandle {
            revision_id: self.revision_id.to_owned(),
            kind: HandleKind::Syntax,
            local_id: format!("syntax:{}:{}:{}", semantic_kind, span.start, span.end),
        };
        self.add_node(IndexedNode {
            handle,
            span,
            semantic_kind,
            explicit_type: None,
            inferred_type: None,
            effects: Vec::new(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            identity_key,
        });
    }

    fn add_expression(
        &mut self,
        span: Span,
        semantic_kind: &'static str,
        identity_key: String,
        inferred_type: Option<String>,
        dependencies: Vec<String>,
    ) {
        let handle = SemanticHandle {
            revision_id: self.revision_id.to_owned(),
            kind: HandleKind::Expression,
            local_id: format!("expression:{}:{}", span.start, span.end),
        };
        self.add_node(IndexedNode {
            handle,
            span,
            semantic_kind,
            explicit_type: None,
            inferred_type,
            effects: Vec::new(),
            capabilities: Vec::new(),
            dependencies,
            identity_key,
        });
    }

    fn add_node(&mut self, node: IndexedNode) {
        let position = self.index.nodes.len();
        self.index
            .node_positions
            .insert(node.handle.clone(), position);
        self.index.nodes.push(node);
    }

    fn add_occurrence(&mut self, handle: &SemanticHandle, span: Span) {
        if let Some(symbol) = self.index.symbols.get_mut(handle) {
            symbol.occurrences.push(span);
            symbol.occurrences.sort_by_key(|span| span.start);
            symbol.occurrences.dedup();
        }
    }

    fn symbol_handle(&self, local_id: &str) -> SemanticHandle {
        SemanticHandle {
            revision_id: self.revision_id.to_owned(),
            kind: HandleKind::Symbol,
            local_id: local_id.to_owned(),
        }
    }
}

impl HandleIndex {
    fn node_mut(&mut self, handle: &SemanticHandle) -> Option<&mut IndexedNode> {
        let position = self.node_positions.get(handle).copied()?;
        self.nodes.get_mut(position)
    }
}

#[derive(Debug, Clone)]
struct LocalBindingIndex {
    handle: SemanticHandle,
    value_type: Option<String>,
    capability: Option<String>,
}

fn expression_kind(expression: &Expr) -> &'static str {
    match expression {
        Expr::Text { .. } => "text-literal",
        Expr::Integer { .. } => "integer-literal",
        Expr::Name { .. } => "name-reference",
        Expr::Record { .. } => "record-construction",
        Expr::Variant { .. } => "variant-construction",
        Expr::CapabilityCall { .. } => "capability-call",
        Expr::FieldAccess { .. } => "field-access",
        Expr::If { .. } => "if-expression",
        Expr::Match { .. } => "match-expression",
    }
}

fn parameter_dependencies(parameter: &crate::Parameter) -> Vec<String> {
    match &parameter.ty {
        ParameterType::Named(ty) | ParameterType::Capability(ty) => vec![ty.clone()],
    }
}

fn effect_names(function: &FunctionDecl) -> Vec<String> {
    function
        .effects
        .iter()
        .map(|effect| format!("{}.{}", effect.receiver, effect.operation))
        .collect()
}

fn function_type(function: &FunctionDecl) -> String {
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
        result.push_str(&effect_names(function).join(", "));
        result.push_str(" }");
    }
    result
}

fn identifier_at(tokens: &[crate::Token], start: usize) -> Span {
    tokens
        .iter()
        .find(|token| {
            token.span.start == start && matches!(token.kind, crate::TokenKind::Identifier)
        })
        .map_or(Span::empty(start), |token| token.span)
}

fn identifier_after(tokens: &[crate::Token], start: usize, ordinal: usize) -> Span {
    tokens
        .iter()
        .filter(|token| {
            token.span.start >= start && matches!(token.kind, crate::TokenKind::Identifier)
        })
        .nth(ordinal)
        .map_or(Span::empty(start), |token| token.span)
}

fn result_type_span(tokens: &[crate::Token], function: &FunctionDecl) -> Span {
    let arrow = tokens
        .iter()
        .find(|token| {
            token.span.start >= function.span.start
                && token.span.end <= function.body.span.start
                && matches!(token.kind, crate::TokenKind::Arrow)
        })
        .map_or(function.span.start, |token| token.span.end);
    identifier_after(tokens, arrow, 0)
}

fn identity_map(
    from: &HandleIndex,
    to: &HandleIndex,
    from_source: &str,
    to_source: &str,
) -> IdentityMap {
    let mut to_by_identity = BTreeMap::new();
    for node in &to.nodes {
        to_by_identity.insert(node.identity_key.as_str(), node);
    }
    let mut entries = from
        .nodes
        .iter()
        .map(|old| {
            let Some(new) = to_by_identity.get(old.identity_key.as_str()) else {
                return IdentityMapEntry {
                    old_handle: old.handle.clone(),
                    classification: IdentityClassification::Removed,
                    new_handle: None,
                };
            };
            let classification = if old.handle.kind == HandleKind::Symbol
                || source_slice(from_source, old.span) == source_slice(to_source, new.span)
            {
                IdentityClassification::Surviving
            } else {
                IdentityClassification::Replaced
            };
            IdentityMapEntry {
                old_handle: old.handle.clone(),
                classification,
                new_handle: Some(new.handle.clone()),
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        handle_order(from, &left.old_handle).cmp(&handle_order(from, &right.old_handle))
    });

    let old_keys = from
        .nodes
        .iter()
        .map(|node| node.identity_key.as_str())
        .collect::<BTreeSet<_>>();
    let mut new_handles = to
        .nodes
        .iter()
        .filter(|node| !old_keys.contains(node.identity_key.as_str()))
        .map(|node| node.handle.clone())
        .collect::<Vec<_>>();
    new_handles.sort_by_key(|handle| handle_order(to, handle));
    IdentityMap {
        from_revision_id: from
            .nodes
            .first()
            .map_or_else(String::new, |node| node.handle.revision_id.clone()),
        to_revision_id: to
            .nodes
            .first()
            .map_or_else(String::new, |node| node.handle.revision_id.clone()),
        entries,
        new_handles,
    }
}

fn handle_order(index: &HandleIndex, handle: &SemanticHandle) -> (usize, HandleKind, String) {
    let span = index.span_for(handle);
    (span.start, handle.kind, handle.local_id.clone())
}

fn source_slice(source: &str, span: Span) -> &str {
    source.get(span.start..span.end).unwrap_or_default()
}

fn apply_edits(source: &str, edits: &[CanonicalEdit]) -> String {
    let mut output = source.to_owned();
    for edit in edits.iter().rev() {
        output.replace_range(edit.span.start..edit.span.end, &edit.replacement);
    }
    output
}

fn is_m11_identifier(name: &str) -> bool {
    let mut characters = name.bytes();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == b'_')
        && !RESERVED_NAMES.contains(&name)
}

fn revision_number(revision_id: &str) -> (&str, u64) {
    let Some((before_marker, suffix)) = revision_id.rsplit_once('r') else {
        return ("revision-r", 0);
    };
    if before_marker.ends_with('-')
        && !suffix.is_empty()
        && suffix.bytes().all(|byte| byte.is_ascii_digit())
    {
        let marker_end = revision_id.len() - suffix.len();
        return (
            &revision_id[..marker_end],
            suffix.parse::<u64>().unwrap_or(0),
        );
    }
    ("revision-r", 0)
}

#[allow(clippy::too_many_arguments)]
fn protocol_diagnostic(
    code: &'static str,
    category: &'static str,
    revision_id: &str,
    primary_handle: SemanticHandle,
    primary_span: Span,
    expected: BTreeMap<String, DiagnosticValue>,
    actual: BTreeMap<String, DiagnosticValue>,
    step: &str,
) -> StructuredDiagnostic {
    StructuredDiagnostic {
        code,
        revision_id: revision_id.to_owned(),
        category,
        primary_handle: primary_handle.clone(),
        primary_span,
        expected,
        actual,
        related_handles: Vec::new(),
        causal_chain: vec![crate::CausalStep {
            step: step.to_owned(),
            handle: primary_handle,
        }],
    }
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

/// Return the canonical SHA-256 source digest used by immutable revisions.
#[must_use]
pub fn source_digest(source: &str) -> String {
    format!("sha256:{}", sha256_hex(source.as_bytes()))
}

#[allow(clippy::unreadable_literal)]
fn sha256_hex(input: &[u8]) -> String {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const ROUND: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_length = (input.len() as u64).wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    let mut state = INITIAL;
    for chunk in message.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes(chunk[start..start + 4].try_into().expect("chunk is sized"));
        }
        for index in 16..64 {
            let sigma0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let sigma1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(sigma0)
                .wrapping_add(words[index - 7])
                .wrapping_add(sigma1);
        }
        let mut working = state;
        for index in 0..64 {
            let sigma1 = working[4].rotate_right(6)
                ^ working[4].rotate_right(11)
                ^ working[4].rotate_right(25);
            let choose = (working[4] & working[5]) ^ ((!working[4]) & working[6]);
            let temporary1 = working[7]
                .wrapping_add(sigma1)
                .wrapping_add(choose)
                .wrapping_add(ROUND[index])
                .wrapping_add(words[index]);
            let sigma0 = working[0].rotate_right(2)
                ^ working[0].rotate_right(13)
                ^ working[0].rotate_right(22);
            let majority =
                (working[0] & working[1]) ^ (working[0] & working[2]) ^ (working[1] & working[2]);
            let temporary2 = sigma0.wrapping_add(majority);
            working[7] = working[6];
            working[6] = working[5];
            working[5] = working[4];
            working[4] = working[3].wrapping_add(temporary1);
            working[3] = working[2];
            working[2] = working[1];
            working[1] = working[0];
            working[0] = temporary1.wrapping_add(temporary2);
        }
        for (state_word, working_word) in state.iter_mut().zip(working) {
            *state_word = state_word.wrapping_add(working_word);
        }
    }
    let mut output = String::with_capacity(64);
    for word in state {
        write!(&mut output, "{word:08x}").expect("writing to String cannot fail");
    }
    output
}
