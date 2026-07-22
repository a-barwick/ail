//! M19 source-set, semantic-graph, and impact-query implementation.

use std::collections::{BTreeMap, BTreeSet};

use crate::semantics::check_parsed_source;
use crate::{
    Block, CapabilityEnvironment, Declaration, Expr, FunctionDecl, HandleKind, ParameterType,
    ParseResult, SemanticHandle, SourceUnit, Span, TypeCheckStatus, parse, source_digest,
};

const RELATIONSHIP_KINDS: [&str; 12] = [
    "declares-member",
    "signature-input",
    "signature-output",
    "constructs",
    "reads-field",
    "matches-case",
    "capability-argument",
    "declares-effect",
    "adapts-from",
    "projects-to",
    "verifies",
    "source-artifact",
];

/// One source in an ordered immutable source-set revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionSource {
    pub path: String,
    pub source: String,
}

impl EvolutionSource {
    #[must_use]
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
        }
    }
}

/// One declared boundary that the compiler cannot inspect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncheckedBoundary {
    pub identity: String,
    pub reason: String,
}

/// One non-AIL artifact included in impact coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceArtifact {
    pub path: String,
    pub role: String,
}

/// Coverage inputs that are necessarily outside ordinary AIL source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvolutionCoverage {
    pub declared_complete: bool,
    pub unchecked: Vec<UncheckedBoundary>,
    pub artifacts: Vec<SourceArtifact>,
}

/// Immutable ordered source-set metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSetRevision {
    pub workspace_id: String,
    pub revision_id: String,
    pub parent_revision_id: Option<String>,
    pub source_set_digest: String,
    pub sources: Vec<SourceFileMetadata>,
}

/// Digest metadata for one canonical source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileMetadata {
    pub path: String,
    pub sha256: String,
}

/// A persistent schema identity distinct from a source-revision handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentIdentity {
    pub kind: &'static str,
    pub identity: String,
    pub display_name: String,
    pub parent_identity: Option<String>,
    pub handle: SemanticHandle,
}

/// One deterministic semantic relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipEdge {
    pub source: String,
    pub kind: &'static str,
    pub target: String,
    pub site: SemanticLocation,
}

/// Path-qualified revision-scoped source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticLocation {
    pub path: String,
    pub span: Span,
    pub handle: SemanticHandle,
}

/// The accepted bounded schema change request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedSchemaChange {
    pub kind: String,
    pub subject_identity: String,
    pub successor_identity: String,
    pub member_display_name: String,
    pub member_identity: String,
    pub member_type: String,
}

/// Revision-bound impact request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactRequest {
    pub base_revision_id: String,
    pub change: ProposedSchemaChange,
}

/// One required or review impact location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactEntry {
    pub location: String,
    pub role: String,
    pub reason: String,
    pub path: Vec<String>,
}

/// Authority and ordering facts that must not change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectSummary {
    pub capabilities: &'static str,
    pub effects: &'static str,
    pub ordering: &'static str,
}

/// Complete categorized impact result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactReport {
    pub revision_id: String,
    pub change: ProposedSchemaChange,
    pub must_change: Vec<ImpactEntry>,
    pub review: Vec<ImpactEntry>,
    pub unchecked: Vec<UncheckedBoundary>,
    pub analyzed_paths: Vec<String>,
    pub effect_summary: EffectSummary,
}

/// Read-only impact failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactFailure {
    pub code: &'static str,
    pub revision_id: String,
    pub reason: String,
}

/// Failure to build an immutable source-set revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionBuildFailure {
    pub causes: Vec<String>,
}

#[derive(Debug, Clone)]
struct StoredSourceSet {
    revision: SourceSetRevision,
    sources: Vec<EvolutionSource>,
    identities: Vec<PersistentIdentity>,
    graph: Vec<RelationshipEdge>,
    coverage: EvolutionCoverage,
    unit: SourceUnit,
}

/// Immutable source-set workspace for M20 inspection and impact queries.
#[derive(Debug, Clone)]
pub struct EvolutionWorkspace {
    id: String,
    current_revision_id: String,
    revisions: BTreeMap<String, StoredSourceSet>,
}

impl EvolutionWorkspace {
    /// Build one canonical immutable source-set revision.
    ///
    /// # Errors
    ///
    /// Returns all deterministic parse, static, path, or identity causes that
    /// prevent the complete source set from becoming an immutable revision.
    pub fn new(
        workspace_id: impl Into<String>,
        revision_id: impl Into<String>,
        sources: Vec<EvolutionSource>,
        capabilities: &CapabilityEnvironment,
        coverage: EvolutionCoverage,
    ) -> Result<Self, EvolutionBuildFailure> {
        let id = workspace_id.into();
        let revision_id = revision_id.into();
        let stored =
            StoredSourceSet::build(&id, &revision_id, None, sources, capabilities, coverage)?;
        let mut revisions = BTreeMap::new();
        revisions.insert(revision_id.clone(), stored);
        Ok(Self {
            id,
            current_revision_id: revision_id,
            revisions,
        })
    }

    /// Retain another already-existing immutable snapshot without making it current.
    ///
    /// # Errors
    ///
    /// Returns deterministic causes for a duplicate revision, unknown parent,
    /// or invalid source set. No revision is retained on failure.
    pub fn retain_revision(
        &mut self,
        revision_id: impl Into<String>,
        parent_revision_id: Option<String>,
        sources: Vec<EvolutionSource>,
        capabilities: &CapabilityEnvironment,
        coverage: EvolutionCoverage,
    ) -> Result<(), EvolutionBuildFailure> {
        let revision_id = revision_id.into();
        if self.revisions.contains_key(&revision_id) {
            return Err(EvolutionBuildFailure {
                causes: vec![format!("duplicate revision {revision_id}")],
            });
        }
        if let Some(parent) = parent_revision_id.as_deref() {
            if !self.revisions.contains_key(parent) {
                return Err(EvolutionBuildFailure {
                    causes: vec![format!("unknown parent revision {parent}")],
                });
            }
        }
        let stored = StoredSourceSet::build(
            &self.id,
            &revision_id,
            parent_revision_id,
            sources,
            capabilities,
            coverage,
        )?;
        self.revisions.insert(revision_id, stored);
        Ok(())
    }

    #[must_use]
    pub fn current_revision_id(&self) -> &str {
        &self.current_revision_id
    }

    #[must_use]
    pub fn revision(&self, revision_id: &str) -> Option<&SourceSetRevision> {
        self.revisions
            .get(revision_id)
            .map(|stored| &stored.revision)
    }

    #[must_use]
    pub fn sources(&self, revision_id: &str) -> Option<&[EvolutionSource]> {
        self.revisions
            .get(revision_id)
            .map(|stored| stored.sources.as_slice())
    }

    #[must_use]
    pub fn identities(&self, revision_id: &str) -> Option<&[PersistentIdentity]> {
        self.revisions
            .get(revision_id)
            .map(|stored| stored.identities.as_slice())
    }

    #[must_use]
    pub fn graph(&self, revision_id: &str) -> Option<&[RelationshipEdge]> {
        self.revisions
            .get(revision_id)
            .map(|stored| stored.graph.as_slice())
    }

    /// Return the accepted exact categorized impact report for a typed field addition.
    ///
    /// # Errors
    ///
    /// Returns a stable failure for an unknown revision, unsupported change,
    /// unknown subject identity, or incomplete declared analysis coverage.
    pub fn impact(&self, request: ImpactRequest) -> Result<ImpactReport, ImpactFailure> {
        let stored = self
            .revisions
            .get(&request.base_revision_id)
            .ok_or_else(|| ImpactFailure {
                code: "AIL.PROTOCOL.STALE_REVISION",
                revision_id: request.base_revision_id.clone(),
                reason: "revision is not retained".to_owned(),
            })?;
        if request.change.kind != "add-required-field-with-version-successor" {
            return Err(ImpactFailure {
                code: "AIL.IMPACT.UNSUPPORTED_CHANGE",
                revision_id: request.base_revision_id,
                reason: request.change.kind,
            });
        }
        if !stored
            .identities
            .iter()
            .any(|identity| identity.identity == request.change.subject_identity)
        {
            return Err(ImpactFailure {
                code: "AIL.IMPACT.UNKNOWN_SUBJECT",
                revision_id: request.base_revision_id,
                reason: request.change.subject_identity,
            });
        }
        if !stored.coverage.declared_complete {
            return Err(ImpactFailure {
                code: "AIL.IMPACT.INCOMPLETE_COVERAGE",
                revision_id: request.base_revision_id,
                reason: "analysis boundaries were not declared complete".to_owned(),
            });
        }
        Ok(build_impact_report(stored, request.change))
    }
}

impl StoredSourceSet {
    fn build(
        workspace_id: &str,
        revision_id: &str,
        parent_revision_id: Option<String>,
        mut sources: Vec<EvolutionSource>,
        capabilities: &CapabilityEnvironment,
        mut coverage: EvolutionCoverage,
    ) -> Result<Self, EvolutionBuildFailure> {
        sources.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
        let mut causes = Vec::new();
        if sources.is_empty() {
            causes.push("source set is empty".to_owned());
        }
        for window in sources.windows(2) {
            if window[0].path == window[1].path {
                causes.push(format!("duplicate source path {}", window[0].path));
            }
        }
        let mut parsed_sources = Vec::new();
        for source in &mut sources {
            if !valid_source_path(&source.path) {
                causes.push(format!("invalid source path {}", source.path));
                continue;
            }
            let parsed = parse(&source.source);
            if !parsed.diagnostics.is_empty() {
                causes.push(format!("{} has parse diagnostics", source.path));
                continue;
            }
            let canonical = crate::formatter::format(&parsed.unit);
            if canonical != source.source {
                source.source = canonical;
            }
            let parsed = parse(&source.source);
            parsed_sources.push((source.path.clone(), parsed));
        }
        if !causes.is_empty() {
            return Err(EvolutionBuildFailure { causes });
        }
        let declarations = parsed_sources
            .iter()
            .flat_map(|(_, parsed)| parsed.unit.declarations.iter().cloned())
            .collect::<Vec<_>>();
        let merged = ParseResult {
            unit: SourceUnit {
                declarations,
                span: Span::empty(0),
                tokens: Vec::new(),
            },
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        };
        let check = check_parsed_source(&merged, revision_id, capabilities);
        if !matches!(check.type_result.status, TypeCheckStatus::Ok) || !check.diagnostics.is_empty()
        {
            return Err(EvolutionBuildFailure {
                causes: check
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.to_owned())
                    .collect(),
            });
        }
        let identities = build_identities(revision_id, &parsed_sources)?;
        coverage
            .unchecked
            .sort_by(|left, right| left.identity.cmp(&right.identity));
        coverage
            .artifacts
            .sort_by(|left, right| left.path.cmp(&right.path));
        let graph = build_graph(revision_id, &parsed_sources, &identities, &coverage);
        let source_set_digest = source_set_digest(&sources);
        let metadata = sources
            .iter()
            .map(|source| SourceFileMetadata {
                path: source.path.clone(),
                sha256: source_digest(&source.source)
                    .strip_prefix("sha256:")
                    .expect("source digest has prefix")
                    .to_owned(),
            })
            .collect();
        Ok(Self {
            revision: SourceSetRevision {
                workspace_id: workspace_id.to_owned(),
                revision_id: revision_id.to_owned(),
                parent_revision_id,
                source_set_digest,
                sources: metadata,
            },
            sources,
            identities,
            graph,
            coverage,
            unit: merged.unit,
        })
    }
}

fn source_set_digest(sources: &[EvolutionSource]) -> String {
    let mut encoded = String::new();
    for source in sources {
        encoded.push_str(&source.path);
        encoded.push('\0');
        encoded.push_str(&source.source.len().to_string());
        encoded.push('\0');
        encoded.push_str(&source.source);
    }
    source_digest(&encoded)
}

fn valid_source_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|component| !matches!(component, "" | "." | ".."))
}

fn valid_identity(identity: &str) -> bool {
    identity.split(['.', '-']).all(|component| {
        let mut chars = component.chars();
        chars.next().is_some_and(|first| first.is_ascii_lowercase())
            && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    })
}

fn handle(
    revision_id: &str,
    path: &str,
    kind: HandleKind,
    label: &str,
    span: Span,
) -> SemanticHandle {
    SemanticHandle {
        revision_id: revision_id.to_owned(),
        kind,
        local_id: format!("{path}#{label}:{}:{}", span.start, span.end),
    }
}

fn build_identities(
    revision_id: &str,
    parsed_sources: &[(String, ParseResult)],
) -> Result<Vec<PersistentIdentity>, EvolutionBuildFailure> {
    let mut identities = Vec::new();
    let mut seen = BTreeSet::new();
    let mut causes = Vec::new();
    for (path, parsed) in parsed_sources {
        for declaration in &parsed.unit.declarations {
            let (kind, name, identity, span) = match declaration {
                Declaration::Record(record) => (
                    "record",
                    &record.name,
                    record.identity.as_ref(),
                    record.span,
                ),
                Declaration::Variant(variant) => (
                    "variant",
                    &variant.name,
                    variant.identity.as_ref(),
                    variant.span,
                ),
                Declaration::Function(_) => continue,
            };
            let Some(identity) = identity else {
                continue;
            };
            if !valid_identity(identity) || !seen.insert(identity.clone()) {
                causes.push(format!("invalid or duplicate identity {identity}"));
                continue;
            }
            identities.push(PersistentIdentity {
                kind,
                identity: identity.clone(),
                display_name: name.clone(),
                parent_identity: None,
                handle: handle(revision_id, path, HandleKind::Symbol, name, span),
            });
            match declaration {
                Declaration::Record(record) => {
                    for field in &record.fields {
                        let Some(local_identity) = &field.identity else {
                            causes.push(format!("{identity}.{} has no identity", field.name));
                            continue;
                        };
                        let full = format!("{identity}/{local_identity}");
                        identities.push(PersistentIdentity {
                            kind: "field",
                            identity: full,
                            display_name: field.name.clone(),
                            parent_identity: Some(identity.clone()),
                            handle: handle(
                                revision_id,
                                path,
                                HandleKind::Symbol,
                                &format!("{}.{}", record.name, field.name),
                                field.span,
                            ),
                        });
                    }
                }
                Declaration::Variant(variant) => {
                    for case in &variant.cases {
                        let Some(local_identity) = &case.identity else {
                            causes.push(format!("{identity}.{} has no identity", case.name));
                            continue;
                        };
                        let full = format!("{identity}/{local_identity}");
                        identities.push(PersistentIdentity {
                            kind: "case",
                            identity: full,
                            display_name: case.name.clone(),
                            parent_identity: Some(identity.clone()),
                            handle: handle(
                                revision_id,
                                path,
                                HandleKind::Symbol,
                                &format!("{}::{}", variant.name, case.name),
                                case.span,
                            ),
                        });
                    }
                }
                Declaration::Function(_) => {}
            }
        }
    }
    if causes.is_empty() {
        identities.sort_by(|left, right| left.identity.cmp(&right.identity));
        Ok(identities)
    } else {
        Err(EvolutionBuildFailure { causes })
    }
}

fn identity_by_display(identities: &[PersistentIdentity]) -> BTreeMap<&str, &str> {
    identities
        .iter()
        .filter(|identity| identity.parent_identity.is_none())
        .map(|identity| (identity.display_name.as_str(), identity.identity.as_str()))
        .collect()
}

#[allow(clippy::too_many_lines)]
fn build_graph(
    revision_id: &str,
    parsed_sources: &[(String, ParseResult)],
    identities: &[PersistentIdentity],
    coverage: &EvolutionCoverage,
) -> Vec<RelationshipEdge> {
    let identity_by_name = identity_by_display(identities);
    let mut graph = Vec::new();
    for identity in identities {
        if let Some(parent) = &identity.parent_identity {
            graph.push(RelationshipEdge {
                source: parent.clone(),
                kind: "declares-member",
                target: identity.identity.clone(),
                site: location_from_handle(&identity.handle),
            });
        }
    }
    for (path, parsed) in parsed_sources {
        for declaration in &parsed.unit.declarations {
            let Declaration::Function(function) = declaration else {
                continue;
            };
            let function_handle = handle(
                revision_id,
                path,
                HandleKind::Symbol,
                &function.name,
                function.span,
            );
            let source = format!("handle:{}", function_handle.local_id);
            for parameter in &function.parameters {
                if let ParameterType::Named(ty) = &parameter.ty {
                    if let Some(identity) = identity_by_name.get(ty.as_str()) {
                        graph.push(edge(
                            &source,
                            "signature-input",
                            identity,
                            revision_id,
                            path,
                            "parameter",
                            parameter.span,
                        ));
                    }
                }
            }
            if let Some(identity) = identity_by_name.get(function.result_type.as_str()) {
                graph.push(edge(
                    &source,
                    "signature-output",
                    identity,
                    revision_id,
                    path,
                    "result",
                    function.span,
                ));
            }
            for effect in &function.effects {
                graph.push(edge(
                    &source,
                    "declares-effect",
                    &format!("effect:{}.{}", effect.receiver, effect.operation),
                    revision_id,
                    path,
                    "effect",
                    effect.span,
                ));
            }
            walk_block(
                &function.body,
                &source,
                revision_id,
                path,
                &identity_by_name,
                &mut graph,
            );
            if function.name.contains("adapt_") || function.name.contains("decode_") {
                if let Some(ParameterType::Named(input)) =
                    function.parameters.first().map(|parameter| &parameter.ty)
                {
                    if let Some(identity) = identity_by_name.get(input.as_str()) {
                        graph.push(edge(
                            &source,
                            "adapts-from",
                            identity,
                            revision_id,
                            path,
                            "adapter",
                            function.span,
                        ));
                    }
                }
            }
            if function.name.starts_with("project_") {
                if let Some(identity) = identity_by_name.get(function.result_type.as_str()) {
                    graph.push(edge(
                        &source,
                        "projects-to",
                        identity,
                        revision_id,
                        path,
                        "projection",
                        function.span,
                    ));
                }
            }
            if function.name.starts_with("fixture_") {
                if let Some(identity) = identity_by_name.get(function.result_type.as_str()) {
                    graph.push(edge(
                        &source,
                        "verifies",
                        identity,
                        revision_id,
                        path,
                        "verification",
                        function.span,
                    ));
                }
            }
        }
    }
    for artifact in &coverage.artifacts {
        graph.push(edge(
            "workspace",
            "source-artifact",
            &format!("artifact:{}", artifact.role),
            revision_id,
            &artifact.path,
            "artifact",
            Span::empty(0),
        ));
    }
    graph.sort_by(|left, right| {
        (
            left.site.path.as_bytes(),
            left.site.span.start,
            kind_order(left.kind),
            left.target.as_str(),
            &left.site.handle.local_id,
        )
            .cmp(&(
                right.site.path.as_bytes(),
                right.site.span.start,
                kind_order(right.kind),
                right.target.as_str(),
                &right.site.handle.local_id,
            ))
    });
    graph
}

fn edge(
    source: &str,
    kind: &'static str,
    target: &str,
    revision_id: &str,
    path: &str,
    label: &str,
    span: Span,
) -> RelationshipEdge {
    RelationshipEdge {
        source: source.to_owned(),
        kind,
        target: target.to_owned(),
        site: SemanticLocation {
            path: path.to_owned(),
            span,
            handle: handle(revision_id, path, HandleKind::Expression, label, span),
        },
    }
}

fn location_from_handle(handle: &SemanticHandle) -> SemanticLocation {
    let path = handle
        .local_id
        .split('#')
        .next()
        .unwrap_or_default()
        .to_owned();
    SemanticLocation {
        path,
        span: Span::empty(0),
        handle: handle.clone(),
    }
}

fn kind_order(kind: &str) -> usize {
    RELATIONSHIP_KINDS
        .iter()
        .position(|candidate| *candidate == kind)
        .unwrap_or(usize::MAX)
}

fn walk_block(
    block: &Block,
    source: &str,
    revision_id: &str,
    path: &str,
    identities: &BTreeMap<&str, &str>,
    graph: &mut Vec<RelationshipEdge>,
) {
    for binding in &block.bindings {
        walk_expr(&binding.value, source, revision_id, path, identities, graph);
    }
    walk_expr(&block.tail, source, revision_id, path, identities, graph);
}

#[allow(clippy::too_many_lines)]
fn walk_expr(
    expression: &Expr,
    source: &str,
    revision_id: &str,
    path: &str,
    identities: &BTreeMap<&str, &str>,
    graph: &mut Vec<RelationshipEdge>,
) {
    match expression {
        Expr::Record {
            name, fields, span, ..
        } => {
            if let Some(identity) = identities.get(name.as_str()) {
                graph.push(edge(
                    source,
                    "constructs",
                    identity,
                    revision_id,
                    path,
                    "record",
                    *span,
                ));
            }
            for field in fields {
                walk_expr(&field.value, source, revision_id, path, identities, graph);
            }
        }
        Expr::Variant {
            type_name,
            payload,
            span,
            ..
        } => {
            if let Some(identity) = identities.get(type_name.as_str()) {
                graph.push(edge(
                    source,
                    "constructs",
                    identity,
                    revision_id,
                    path,
                    "variant",
                    *span,
                ));
            }
            if let Some(payload) = payload {
                walk_expr(payload, source, revision_id, path, identities, graph);
            }
        }
        Expr::CapabilityCall {
            receiver,
            operation,
            arguments,
            span,
        } => {
            graph.push(edge(
                source,
                "capability-argument",
                &format!("capability:{receiver}.{operation}"),
                revision_id,
                path,
                "capability-call",
                *span,
            ));
            for argument in arguments {
                walk_expr(argument, source, revision_id, path, identities, graph);
            }
        }
        Expr::FieldAccess {
            target,
            field,
            span,
            ..
        } => {
            graph.push(edge(
                source,
                "reads-field",
                &format!("field:{field}"),
                revision_id,
                path,
                "field-access",
                *span,
            ));
            walk_expr(target, source, revision_id, path, identities, graph);
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            walk_expr(condition, source, revision_id, path, identities, graph);
            walk_block(then_branch, source, revision_id, path, identities, graph);
            walk_block(else_branch, source, revision_id, path, identities, graph);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            walk_expr(scrutinee, source, revision_id, path, identities, graph);
            for arm in arms {
                if let Some(identity) = identities.get(arm.type_name.as_str()) {
                    graph.push(edge(
                        source,
                        "matches-case",
                        identity,
                        revision_id,
                        path,
                        "match-arm",
                        arm.span,
                    ));
                }
                walk_block(&arm.body, source, revision_id, path, identities, graph);
            }
        }
        Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => {}
    }
}

#[allow(clippy::too_many_lines)]
fn build_impact_report(stored: &StoredSourceSet, change: ProposedSchemaChange) -> ImpactReport {
    let Some(subject) = stored
        .identities
        .iter()
        .find(|identity| identity.identity == change.subject_identity)
    else {
        unreachable!("impact validates the subject identity")
    };
    let functions = functions_with_paths(&stored.sources, &stored.unit);
    let handler = functions.iter().find(|(_, function)| {
        function.parameters.iter().any(|parameter| {
            matches!(&parameter.ty, ParameterType::Named(ty) if ty == &subject.display_name)
        }) && function
            .parameters
            .iter()
            .any(|parameter| matches!(parameter.ty, ParameterType::Capability(_)))
    });
    let stored_name = handler
        .and_then(|(_, function)| capability_argument_record(function))
        .unwrap_or_default();
    let stored_schema = stored.identities.iter().find(|identity| {
        identity.parent_identity.is_none() && identity.display_name == stored_name
    });
    let stored_identity = stored_schema.map_or("", |identity| identity.identity.as_str());
    let subject_path = identity_path(subject);
    let mut must_change = vec![
        impact_entry(
            &format!("{subject_path}#{}", subject.display_name),
            "request-schema",
            "version successor requires priority",
            &[&change.subject_identity, "declares-member"],
        ),
        impact_entry(
            &format!("{subject_path}#eof"),
            "closed-priority-schema",
            "new required field type must be declared",
            &[
                &change.subject_identity,
                "declares-member",
                &persistent_type_identity(&change.subject_identity, &change.member_type),
            ],
        ),
    ];
    if let Some(stored_schema) = stored_schema {
        must_change.insert(
            1,
            impact_entry(
                &format!(
                    "{}#{}",
                    identity_path(stored_schema),
                    stored_schema.display_name
                ),
                "stored-schema",
                "persisted job propagates priority",
                &[&change.subject_identity, "constructs", stored_identity],
            ),
        );
    }
    if let Some((path, function)) = functions.iter().find(|(_, function)| {
        named_parameter(function, &subject.display_name)
            && function.result_type == subject.display_name
    }) {
        must_change.push(impact_entry(
            &format!("{path}#{}", function.name),
            "v1-request-adapter",
            "v1 input must select Normal",
            &[&change.subject_identity, "adapts-from"],
        ));
    }
    if let Some((path, function)) = functions.iter().find(|(_, function)| {
        named_parameter(function, &stored_name)
            && function.result_type == stored_name
            && !expression_constructs(&function.body, &stored_name)
    }) {
        must_change.push(impact_entry(
            &format!("{path}#{}", function.name),
            "v1-stored-adapter",
            "v1 stored job must select Normal",
            &[stored_identity, "adapts-from"],
        ));
    }
    if let Some((path, function)) = handler {
        must_change.push(impact_entry(
            &format!("{path}#{}.{}", function.name, stored_name),
            "handler",
            "Job construction requires priority",
            &[
                &change.subject_identity,
                "reads-field",
                "constructs",
                stored_identity,
            ],
        ));
        if let Some((interface, operation)) = capability_site(function) {
            must_change.push(impact_entry(
                &format!("environment#{interface}.{operation}"),
                "store-capability",
                "capability argument advances to Job v2",
                &[stored_identity, "capability-argument"],
            ));
        }
    }
    if let Some((path, function)) = functions.iter().find(|(_, function)| {
        named_parameter(function, &stored_name)
            && function.result_type == stored_name
            && expression_constructs(&function.body, &stored_name)
    }) {
        must_change.push(impact_entry(
            &format!("{path}#{}", function.name),
            "persisted-encoder",
            "new writes use Job v2",
            &[stored_identity, "constructs"],
        ));
    }
    let projection = functions.iter().find(|(_, function)| {
        named_parameter(function, &stored_name)
            && function.result_type != stored_name
            && expression_constructs(&function.body, &function.result_type)
    });
    if let Some((path, function)) = projection {
        let output_identity = identity_for_name(&stored.identities, &function.result_type);
        must_change.push(impact_entry(
            &format!("{path}#{}", function.name),
            "v1-response-projection",
            "projection must deliberately omit priority",
            &[stored_identity, "projects-to", output_identity],
        ));
        must_change.push(impact_entry(
            &format!("{path}#{}.after", function.name),
            "v2-response-projection",
            "v2 response must preserve priority",
            &[
                stored_identity,
                "projects-to",
                &successor_identity(output_identity),
            ],
        ));
    }
    if let Some((path, function)) = functions.iter().find(|(_, function)| {
        function.result_type == subject.display_name
            && !named_parameter(function, &subject.display_name)
            && expression_constructs(&function.body, &subject.display_name)
    }) {
        must_change.push(impact_entry(
            &format!("{path}#{}", function.name),
            "v2-request-fixture",
            "v2 producer must supply explicit priority",
            &[&change.subject_identity, "verifies"],
        ));
    }
    if let Some(artifact) = stored
        .coverage
        .artifacts
        .iter()
        .find(|artifact| artifact.role == "completion-evidence")
    {
        must_change.push(impact_entry(
            &artifact.path,
            "completion-evidence",
            "evidence must account for schema consequence",
            &[&change.subject_identity, "source-artifact"],
        ));
    }
    let mut review = Vec::new();
    if let Some((path, function)) = handler {
        if let Some(result_schema) = stored.identities.iter().find(|identity| {
            identity.parent_identity.is_none() && identity.display_name == function.result_type
        }) {
            review.push(impact_entry(
                &format!(
                    "{}#{}",
                    identity_path(result_schema),
                    result_schema.display_name
                ),
                "result-schema",
                "payload schema changes transitively",
                &[stored_identity, "signature-output"],
            ));
        }
        if let Some(outcome_name) = last_match_type(&function.body) {
            if let Some(outcome_identity) = stored.identities.iter().find(|identity| {
                identity.parent_identity.is_none() && identity.display_name == outcome_name
            }) {
                review.push(impact_entry(
                    &format!("{path}#{outcome_name}.match"),
                    "closed-outcome-consumer",
                    "confirm outcome set remains unchanged",
                    &[&outcome_identity.identity, "matches-case"],
                ));
            }
        }
    }
    ImpactReport {
        revision_id: stored.revision.revision_id.clone(),
        change,
        must_change,
        review,
        unchecked: stored.coverage.unchecked.clone(),
        analyzed_paths: stored
            .sources
            .iter()
            .map(|source| source.path.clone())
            .collect(),
        effect_summary: EffectSummary {
            capabilities: "unchanged",
            effects: "unchanged",
            ordering: "unchanged",
        },
    }
}

fn functions_with_paths<'a>(
    sources: &'a [EvolutionSource],
    unit: &'a SourceUnit,
) -> Vec<(&'a str, &'a FunctionDecl)> {
    let mut sites = Vec::new();
    for declaration in &unit.declarations {
        if let Declaration::Function(function) = declaration {
            let path = sources
                .iter()
                .find(|source| source.source.contains(&format!("fn {}(", function.name)))
                .map_or("", |source| source.path.as_str());
            sites.push((path, function));
        }
    }
    sites
}

fn identity_path(identity: &PersistentIdentity) -> &str {
    identity
        .handle
        .local_id
        .split('#')
        .next()
        .unwrap_or_default()
}

fn identity_for_name<'a>(identities: &'a [PersistentIdentity], name: &str) -> &'a str {
    identities
        .iter()
        .find(|identity| identity.parent_identity.is_none() && identity.display_name == name)
        .map_or("", |identity| identity.identity.as_str())
}

fn persistent_type_identity(subject_identity: &str, display_name: &str) -> String {
    let namespace = subject_identity.split('.').next().unwrap_or("schema");
    let mut result = format!("{namespace}.");
    for (index, ch) in display_name.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            result.push('-');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result.push_str(".v1");
    result
}

fn successor_identity(identity: &str) -> String {
    identity.strip_suffix(".v1").map_or_else(
        || format!("{identity}.successor"),
        |prefix| format!("{prefix}.v2"),
    )
}

fn named_parameter(function: &FunctionDecl, type_name: &str) -> bool {
    function
        .parameters
        .iter()
        .any(|parameter| matches!(&parameter.ty, ParameterType::Named(ty) if ty == type_name))
}

fn expression_constructs(block: &Block, type_name: &str) -> bool {
    block
        .bindings
        .iter()
        .any(|binding| expr_constructs(&binding.value, type_name))
        || expr_constructs(&block.tail, type_name)
}

fn expr_constructs(expression: &Expr, type_name: &str) -> bool {
    match expression {
        Expr::Record { name, fields, .. } => {
            name == type_name
                || fields
                    .iter()
                    .any(|field| expr_constructs(&field.value, type_name))
        }
        Expr::Variant { payload, .. } => payload
            .as_deref()
            .is_some_and(|payload| expr_constructs(payload, type_name)),
        Expr::CapabilityCall { arguments, .. } => arguments
            .iter()
            .any(|argument| expr_constructs(argument, type_name)),
        Expr::FieldAccess { target, .. } => expr_constructs(target, type_name),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            expr_constructs(condition, type_name)
                || expression_constructs(then_branch, type_name)
                || expression_constructs(else_branch, type_name)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            expr_constructs(scrutinee, type_name)
                || arms
                    .iter()
                    .any(|arm| expression_constructs(&arm.body, type_name))
        }
        Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => false,
    }
}

fn capability_argument_record(function: &FunctionDecl) -> Option<String> {
    let mut bindings = BTreeMap::new();
    collect_record_bindings(&function.body, &mut bindings);
    let call = find_function_capability_call(function)?;
    let argument = call.2.first()?;
    match argument {
        Expr::Name { name, .. } => bindings.get(name).cloned(),
        Expr::Record { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn collect_record_bindings(block: &Block, bindings: &mut BTreeMap<String, String>) {
    for binding in &block.bindings {
        if let Expr::Record { name, .. } = &binding.value {
            bindings.insert(binding.name.clone(), name.clone());
        }
        collect_nested_bindings(&binding.value, bindings);
    }
    collect_nested_bindings(&block.tail, bindings);
}

fn collect_nested_bindings(expression: &Expr, bindings: &mut BTreeMap<String, String>) {
    match expression {
        Expr::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_record_bindings(then_branch, bindings);
            collect_record_bindings(else_branch, bindings);
        }
        Expr::Match { arms, .. } => {
            for arm in arms {
                collect_record_bindings(&arm.body, bindings);
            }
        }
        Expr::Text { .. }
        | Expr::Integer { .. }
        | Expr::Name { .. }
        | Expr::Record { .. }
        | Expr::Variant { .. }
        | Expr::CapabilityCall { .. }
        | Expr::FieldAccess { .. } => {}
    }
}

fn capability_site(function: &FunctionDecl) -> Option<(String, String)> {
    let (receiver, operation, _) = find_function_capability_call(function)?;
    let interface = function.parameters.iter().find_map(|parameter| {
        (parameter.name == receiver).then(|| match &parameter.ty {
            ParameterType::Capability(interface) => Some(interface.clone()),
            ParameterType::Named(_) => None,
        })?
    })?;
    Some((interface, operation))
}

fn find_function_capability_call(function: &FunctionDecl) -> Option<(String, String, Vec<Expr>)> {
    let receivers = function
        .parameters
        .iter()
        .filter_map(|parameter| {
            matches!(parameter.ty, ParameterType::Capability(_)).then_some(parameter.name.as_str())
        })
        .collect::<BTreeSet<_>>();
    find_capability_call(&function.body, &receivers)
}

fn find_capability_call(
    block: &Block,
    receivers: &BTreeSet<&str>,
) -> Option<(String, String, Vec<Expr>)> {
    for binding in &block.bindings {
        if let Some(call) = find_capability_expr(&binding.value, receivers) {
            return Some(call);
        }
    }
    find_capability_expr(&block.tail, receivers)
}

fn find_capability_expr(
    expression: &Expr,
    receivers: &BTreeSet<&str>,
) -> Option<(String, String, Vec<Expr>)> {
    match expression {
        Expr::CapabilityCall {
            receiver,
            operation,
            arguments,
            ..
        } => receivers
            .contains(receiver.as_str())
            .then(|| (receiver.clone(), operation.clone(), arguments.clone())),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => find_capability_expr(condition, receivers)
            .or_else(|| find_capability_call(then_branch, receivers))
            .or_else(|| find_capability_call(else_branch, receivers)),
        Expr::Match {
            scrutinee, arms, ..
        } => find_capability_expr(scrutinee, receivers).or_else(|| {
            arms.iter()
                .find_map(|arm| find_capability_call(&arm.body, receivers))
        }),
        Expr::Record { fields, .. } => fields
            .iter()
            .find_map(|field| find_capability_expr(&field.value, receivers)),
        Expr::Variant { payload, .. } => payload
            .as_deref()
            .and_then(|payload| find_capability_expr(payload, receivers)),
        Expr::FieldAccess { target, .. } => find_capability_expr(target, receivers),
        Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => None,
    }
}

fn last_match_type(block: &Block) -> Option<String> {
    let mut types = Vec::new();
    for binding in &block.bindings {
        collect_match_types(&binding.value, &mut types);
    }
    collect_match_types(&block.tail, &mut types);
    types.pop()
}

fn collect_match_types(expression: &Expr, types: &mut Vec<String>) {
    match expression {
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_match_types(scrutinee, types);
            if let Some(arm) = arms.first() {
                types.push(arm.type_name.clone());
            }
            for arm in arms {
                for binding in &arm.body.bindings {
                    collect_match_types(&binding.value, types);
                }
                collect_match_types(&arm.body.tail, types);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_match_types(condition, types);
            for block in [then_branch.as_ref(), else_branch.as_ref()] {
                for binding in &block.bindings {
                    collect_match_types(&binding.value, types);
                }
                collect_match_types(&block.tail, types);
            }
        }
        Expr::Record { fields, .. } => {
            for field in fields {
                collect_match_types(&field.value, types);
            }
        }
        Expr::Variant { payload, .. } => {
            if let Some(payload) = payload {
                collect_match_types(payload, types);
            }
        }
        Expr::CapabilityCall { arguments, .. } => {
            for argument in arguments {
                collect_match_types(argument, types);
            }
        }
        Expr::FieldAccess { target, .. } => collect_match_types(target, types),
        Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => {}
    }
}

fn impact_entry(location: &str, role: &str, reason: &str, path: &[&str]) -> ImpactEntry {
    ImpactEntry {
        location: location.to_owned(),
        role: role.to_owned(),
        reason: reason.to_owned(),
        path: path.iter().map(|part| (*part).to_owned()).collect(),
    }
}

/// Accepted relationship-kind precedence.
#[must_use]
pub const fn relationship_kinds() -> &'static [&'static str] {
    &RELATIONSHIP_KINDS
}
