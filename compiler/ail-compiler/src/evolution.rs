//! M19 source-set, semantic-graph, and impact-query implementation.

use std::collections::{BTreeMap, BTreeSet};

use crate::semantics::check_parsed_source;
use crate::{
    ArchitectureCoverage, ArchitectureEdge, ArchitecturePolicyContext, ArchitectureRevision,
    ArchitectureRevisionError, ArchitectureUnit, Block, CapabilityEnvironment, CapabilityProvider,
    ControlFlowGraph, Declaration, ExecutionFailure, ExecutionResponse, ExecutionSuccess, Expr,
    FunctionDecl, HandleKind, ParameterType, ParseResult, RuntimeFault, RuntimeValue,
    SemanticHandle, SourceUnit, Span, TypeCheckStatus, TypeRef, parse, source_digest,
};

mod transaction;

pub use transaction::{
    ArchitectureSourceChangeRequest, CandidateChangeRequest, CandidateRevision,
    ChangeCapabilitySummary, ChangeEffectSummary, ChangeFailure, ChangeResponse, ChangeSuccess,
    CompletionEvidence, PersistentIdentityChanges, PublicBehaviorFailure, SemanticChange,
    SemanticDiff, ValidationSummary,
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

/// Project-owned interpretation of one capability operation's state effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStateAccess {
    Read,
    Write,
    ReadWrite,
}

/// Project-owned state interpretation for one capability operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceOperationArchitecture {
    Stateless,
    State {
        domain: String,
        access: SourceStateAccess,
    },
}

/// Project-owned architecture facts not represented by AIL source syntax.
///
/// Keys in `capability_namespaces` are capability interface names. Keys in
/// `operations` are `Interface.operation`. Every source-visible capability
/// interface and operation must have an explicit entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceArchitectureConfig {
    pub module_groups: BTreeMap<String, String>,
    pub capability_namespaces: BTreeMap<String, String>,
    pub endpoint_groups: BTreeMap<String, String>,
    pub operations: BTreeMap<String, SourceOperationArchitecture>,
    pub policy: ArchitecturePolicyContext,
    pub semantic_model_version: String,
}

impl SourceArchitectureConfig {
    /// Return the deterministic digest of every architecture interpretation and policy field.
    #[must_use]
    pub fn stable_digest(&self) -> String {
        let operations = self
            .operations
            .iter()
            .map(|(operation, interpretation)| {
                let value = match interpretation {
                    SourceOperationArchitecture::Stateless => serde_json::json!({
                        "kind": "stateless"
                    }),
                    SourceOperationArchitecture::State { domain, access } => serde_json::json!({
                        "kind": "state",
                        "domain": domain,
                        "access": match access {
                            SourceStateAccess::Read => "read",
                            SourceStateAccess::Write => "write",
                            SourceStateAccess::ReadWrite => "read-write",
                        }
                    }),
                };
                (operation, value)
            })
            .collect::<BTreeMap<_, _>>();
        let dependencies = &self.policy.allowed_group_dependencies;
        let baseline = &self.policy.baseline_match;
        let canonical = serde_json::json!({
            "module_groups": self.module_groups,
            "capability_namespaces": self.capability_namespaces,
            "endpoint_groups": self.endpoint_groups,
            "operations": operations,
            "policy": {
                "revision": self.policy.revision,
                "allowed_group_dependencies": {
                    "contract": dependencies.contract,
                    "transport": dependencies.transport,
                    "domain": dependencies.domain,
                    "persistence_adapter": dependencies.persistence_adapter,
                    "verification": dependencies.verification,
                },
                "transport_capabilities": self.policy.transport_capabilities,
                "transport_state": self.policy.transport_state,
                "dispatch_no_growth": {
                    "control_flow_complexity": self.policy.dispatch_no_growth.control_flow_complexity,
                    "minimal_context_node_count": self.policy.dispatch_no_growth.minimal_context_node_count,
                },
                "new_unit": {
                    "control_flow_complexity_max": self.policy.new_unit.control_flow_complexity_max,
                    "minimal_context_node_count_max": self.policy.new_unit.minimal_context_node_count_max,
                },
                "new_cycles": self.policy.new_cycles,
                "coverage_required": self.policy.coverage_required,
                "baseline_match": {
                    "baseline_revision": baseline.baseline_revision,
                    "scope": baseline.scope,
                    "metrics": {
                        "control_flow_complexity": baseline.metrics.control_flow_complexity,
                        "minimal_context_node_count": baseline.metrics.minimal_context_node_count,
                    },
                    "accepted_debt": baseline.accepted_debt,
                },
            },
            "semantic_model_version": self.semantic_model_version,
        });
        source_digest(&canonical.to_string())
    }
}

/// Immutable ordered source-set metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSetRevision {
    pub workspace_id: String,
    pub revision_id: String,
    pub parent_revision_id: Option<String>,
    pub source_set_digest: String,
    /// Digest of the complete saved architecture settings, when architecture is enabled.
    pub architecture_settings_digest: Option<String>,
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

/// One compiler-visible bounded-list type in a linked function signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedListInspection {
    pub element_type: String,
    pub element_identity: Option<String>,
    pub max_length: u128,
}

/// One ordinary value parameter in linked source-set inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueParameterInspection {
    pub name: String,
    pub value_type: String,
    pub bounded_list: Option<BoundedListInspection>,
}

/// Revision-bound inspection of one linked function boundary and body dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSetFunctionInspection {
    pub revision_id: String,
    pub function_handle: SemanticHandle,
    pub module_identity: String,
    pub function_identity: String,
    pub parameters: Vec<ValueParameterInspection>,
    pub result_type: String,
    pub result_list: Option<BoundedListInspection>,
    pub effects: Vec<String>,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Failure to inspect a function in one immutable source-set revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSetInspectionFailure {
    pub code: &'static str,
    pub revision_id: String,
    pub function: String,
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
    pub diagnostics: Vec<SourceSetDiagnostic>,
}

/// One structured diagnostic produced while linking an ordered source set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSetDiagnostic {
    pub code: &'static str,
    pub path: String,
    pub span: Span,
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct StoredSourceSet {
    revision: SourceSetRevision,
    sources: Vec<EvolutionSource>,
    identities: Vec<PersistentIdentity>,
    graph: Vec<RelationshipEdge>,
    coverage: EvolutionCoverage,
    unit: SourceUnit,
    architecture_config: Option<SourceArchitectureConfig>,
}

/// Immutable source-set workspace for M20 inspection and impact queries.
#[derive(Debug, Clone)]
pub struct EvolutionWorkspace {
    id: String,
    capabilities: CapabilityEnvironment,
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
            capabilities: capabilities.clone(),
            current_revision_id: revision_id,
            revisions,
        })
    }

    /// Build an architecture-enabled workspace whose base revision owns its settings.
    ///
    /// # Errors
    /// Returns the same deterministic source-set failures as [`Self::new`].
    pub fn new_with_architecture(
        workspace_id: impl Into<String>,
        revision_id: impl Into<String>,
        sources: Vec<EvolutionSource>,
        capabilities: &CapabilityEnvironment,
        coverage: EvolutionCoverage,
        config: SourceArchitectureConfig,
    ) -> Result<Self, EvolutionBuildFailure> {
        let mut workspace = Self::new(workspace_id, revision_id, sources, capabilities, coverage)?;
        let stored = workspace
            .revisions
            .get_mut(&workspace.current_revision_id)
            .ok_or_else(|| EvolutionBuildFailure {
                causes: vec!["base revision was not retained".to_owned()],
                diagnostics: Vec::new(),
            })?;
        stored.revision.architecture_settings_digest = Some(config.stable_digest());
        stored.architecture_config = Some(config);
        Ok(workspace)
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
                diagnostics: Vec::new(),
            });
        }
        if let Some(parent) = parent_revision_id.as_deref() {
            if !self.revisions.contains_key(parent) {
                return Err(EvolutionBuildFailure {
                    causes: vec![format!("unknown parent revision {parent}")],
                    diagnostics: Vec::new(),
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

    /// Inspect one linked function without reconstructing its types or dependencies from files.
    ///
    /// # Errors
    ///
    /// Returns a stable failure for an unknown revision or a selector that does not name exactly
    /// one function in that revision.
    pub fn inspect_function(
        &self,
        revision_id: &str,
        function: &str,
    ) -> Result<SourceSetFunctionInspection, SourceSetInspectionFailure> {
        let stored = self
            .revisions
            .get(revision_id)
            .ok_or_else(|| SourceSetInspectionFailure {
                code: "AIL.PROTOCOL.STALE_REVISION",
                revision_id: revision_id.to_owned(),
                function: function.to_owned(),
            })?;
        let matches = entry_functions(&stored.unit, function);
        if matches.len() != 1 {
            return Err(SourceSetInspectionFailure {
                code: if matches.is_empty() {
                    "AIL.PROTOCOL.UNKNOWN_FUNCTION"
                } else {
                    "AIL.PROTOCOL.AMBIGUOUS_FUNCTION"
                },
                revision_id: revision_id.to_owned(),
                function: function.to_owned(),
            });
        }
        let declaration = matches[0];
        let module_identity = declaration
            .name
            .rsplit_once('.')
            .map_or_else(String::new, |(module, _)| module.to_owned());
        let path = source_path_for_function(&stored.sources, &declaration.name);
        let function_handle = handle(
            revision_id,
            path,
            HandleKind::Symbol,
            source_name(&declaration.name),
            declaration.span,
        );
        let parameters = declaration
            .parameters
            .iter()
            .filter_map(|parameter| {
                let ParameterType::Value(value_type) = &parameter.ty else {
                    return None;
                };
                Some(ValueParameterInspection {
                    name: parameter.name.clone(),
                    value_type: value_type.to_string(),
                    bounded_list: inspect_bounded_list(value_type, &stored.unit),
                })
            })
            .collect();
        let effects = declaration
            .effects
            .iter()
            .map(|effect| format!("{}.{}", effect.receiver, effect.operation))
            .collect();
        let capabilities = declaration
            .parameters
            .iter()
            .filter_map(|parameter| {
                let ParameterType::Capability(interface) = &parameter.ty else {
                    return None;
                };
                Some(format!("{}:{interface}", parameter.name))
            })
            .collect();
        let dependencies = function_dependencies(declaration);
        Ok(SourceSetFunctionInspection {
            revision_id: revision_id.to_owned(),
            function_handle,
            module_identity,
            function_identity: declaration.name.clone(),
            parameters,
            result_type: declaration.result_type.to_string(),
            result_list: inspect_bounded_list(&declaration.result_type, &stored.unit),
            effects,
            capabilities,
            dependencies,
        })
    }

    /// Execute a checked function from one retained ordered source-set revision.
    #[must_use]
    pub fn execute(
        &self,
        revision_id: &str,
        function: &str,
        arguments: Vec<RuntimeValue>,
        capabilities: &mut dyn CapabilityProvider,
    ) -> ExecutionResponse {
        let Some(stored) = self.revisions.get(revision_id) else {
            return ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: revision_id.to_owned(),
                function: function.to_owned(),
                fault: RuntimeFault::new(
                    "AIL.RUNTIME.UNKNOWN_REVISION",
                    Span::empty(0),
                    [("revision", revision_id)],
                    std::iter::empty::<(&str, &str)>(),
                ),
                calls: Vec::new(),
            });
        };
        let matching_functions = entry_functions(&stored.unit, function);
        if matching_functions.len() > 1 {
            return ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: revision_id.to_owned(),
                function: function.to_owned(),
                fault: RuntimeFault::new(
                    "AIL.RUNTIME.AMBIGUOUS_FUNCTION",
                    Span::empty(0),
                    [("selector", "module.function")],
                    [("function", function)],
                ),
                calls: Vec::new(),
            });
        }
        let Some(declaration) = matching_functions.first().copied() else {
            return ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: revision_id.to_owned(),
                function: function.to_owned(),
                fault: RuntimeFault::new(
                    "AIL.RUNTIME.UNKNOWN_FUNCTION",
                    Span::empty(0),
                    [("function", function)],
                    std::iter::empty::<(&str, &str)>(),
                ),
                calls: Vec::new(),
            });
        };
        let linked_function = &declaration.name;
        let function_handle = handle(
            revision_id,
            source_path_for_function(&stored.sources, linked_function),
            HandleKind::Symbol,
            source_name(linked_function),
            declaration.span,
        );
        match crate::interpreter::interpret(
            &stored.unit,
            linked_function,
            arguments,
            &self.capabilities,
            capabilities,
        ) {
            Ok(result) => ExecutionResponse::Completed(ExecutionSuccess {
                status: "completed",
                revision_id: revision_id.to_owned(),
                function_handle,
                value: result.value,
                calls: result.calls,
            }),
            Err(result) => ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: revision_id.to_owned(),
                function: function.to_owned(),
                fault: result.fault,
                calls: result.calls,
            }),
        }
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
    #[allow(clippy::too_many_lines)]
    fn architecture_revision(
        &self,
        workspace_id: &str,
        config: &SourceArchitectureConfig,
    ) -> Result<ArchitectureRevision, ArchitectureRevisionError> {
        let mut units = Vec::new();
        let mut edges = BTreeSet::new();
        let mut endpoint_groups = BTreeMap::new();
        for declaration in &self.unit.declarations {
            let Declaration::Function(function) = declaration else {
                continue;
            };
            let (module, local) = function
                .name
                .rsplit_once('.')
                .unwrap_or(("", &function.name));
            let id = format!("{module}:{local}");
            let group = config.module_groups.get(module).cloned().ok_or_else(|| {
                ArchitectureRevisionError(format!("no architecture group for module {module}"))
            })?;
            endpoint_groups.insert(id.clone(), group.clone());
            let receivers = function
                .parameters
                .iter()
                .filter_map(|parameter| match &parameter.ty {
                    ParameterType::Capability(interface) => {
                        Some((parameter.name.clone(), interface.clone()))
                    }
                    ParameterType::Value(_) => None,
                })
                .collect::<BTreeMap<_, _>>();
            let mut capabilities = BTreeSet::new();
            for interface in receivers.values() {
                let namespace = config.capability_namespaces.get(interface).ok_or_else(|| {
                    ArchitectureRevisionError(format!(
                        "no architecture namespace for capability interface {interface}"
                    ))
                })?;
                capabilities.insert(namespace.clone());
                let endpoint = format!("capability:{namespace}");
                let endpoint_group = config.endpoint_groups.get(&endpoint).ok_or_else(|| {
                    ArchitectureRevisionError(format!("no endpoint group for {endpoint}"))
                })?;
                endpoint_groups.insert(endpoint.clone(), endpoint_group.clone());
                edges.insert((id.clone(), endpoint, "capability-use".to_owned()));
            }
            validate_architecture_operations(
                &function.body,
                &receivers,
                config,
                &mut endpoint_groups,
            )?;
            let mut reads = BTreeSet::new();
            let mut writes = BTreeSet::new();
            let mut decisions = 0;
            derive_architecture_expr(
                &function.body,
                &id,
                &receivers,
                config,
                &mut edges,
                &mut reads,
                &mut writes,
                &mut decisions,
            );
            units.push(ArchitectureUnit {
                id,
                module: module.to_owned(),
                group,
                cfg: ControlFlowGraph {
                    nodes: decisions + 1,
                    edges: decisions * 2,
                },
                capabilities: capabilities.into_iter().collect(),
                state_reads: reads.into_iter().collect(),
                state_writes: writes.into_iter().collect(),
            });
        }
        units.sort_by(|a, b| a.id.cmp(&b.id));
        let mut analyzed_groups = units
            .iter()
            .map(|unit| unit.group.clone())
            .collect::<BTreeSet<_>>();
        analyzed_groups.extend(endpoint_groups.values().cloned());
        let required = [
            "contract",
            "transport",
            "domain",
            "persistence-adapter",
            "verification",
        ];
        let complete_for_policy = self.coverage.declared_complete
            && required
                .iter()
                .all(|group| analyzed_groups.contains(*group));
        let analyzed_groups = if complete_for_policy {
            required.iter().map(|group| (*group).to_owned()).collect()
        } else {
            analyzed_groups.into_iter().collect()
        };
        ArchitectureRevision::new(
            workspace_id.to_owned(),
            self.revision.revision_id.clone(),
            config.semantic_model_version.clone(),
            units,
            edges
                .into_iter()
                .map(|(source, target, kind)| ArchitectureEdge {
                    source,
                    target,
                    kind,
                })
                .collect(),
            endpoint_groups,
            ArchitectureCoverage {
                analyzed_groups,
                unchecked_boundaries: self
                    .coverage
                    .unchecked
                    .iter()
                    .map(|boundary| {
                        serde_json::json!({
                            "id": boundary.identity, "reason": boundary.reason
                        })
                    })
                    .collect(),
                complete_for_policy,
            },
            config.policy.clone(),
        )
    }

    #[allow(clippy::too_many_lines)]
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
            return Err(EvolutionBuildFailure {
                causes,
                diagnostics: Vec::new(),
            });
        }
        let module_diagnostics = validate_modules(&parsed_sources);
        if !module_diagnostics.is_empty() {
            return Err(EvolutionBuildFailure {
                causes: module_diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.to_owned())
                    .collect(),
                diagnostics: module_diagnostics,
            });
        }
        let merged = ParseResult {
            unit: link_source_set(&parsed_sources),
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        };
        let check = check_parsed_source(&merged, revision_id, capabilities);
        if !matches!(check.type_result.status, TypeCheckStatus::Ok) || !check.diagnostics.is_empty()
        {
            let diagnostics = check
                .diagnostics
                .iter()
                .map(|diagnostic| SourceSetDiagnostic {
                    code: diagnostic.code,
                    path: "<source-set>".to_owned(),
                    span: diagnostic.primary_span,
                    details: diagnostic
                        .expected
                        .iter()
                        .map(|(key, value)| {
                            (format!("expected.{key}"), diagnostic_value_text(value))
                        })
                        .chain(diagnostic.actual.iter().map(|(key, value)| {
                            (format!("actual.{key}"), diagnostic_value_text(value))
                        }))
                        .collect(),
                })
                .collect();
            return Err(EvolutionBuildFailure {
                causes: check
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.to_owned())
                    .collect(),
                diagnostics,
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
                architecture_settings_digest: None,
                sources: metadata,
            },
            sources,
            identities,
            graph,
            coverage,
            unit: merged.unit,
            architecture_config: None,
        })
    }
}

#[allow(clippy::too_many_lines)]
fn validate_architecture_operations(
    block: &Block,
    receivers: &BTreeMap<String, String>,
    config: &SourceArchitectureConfig,
    endpoint_groups: &mut BTreeMap<String, String>,
) -> Result<(), ArchitectureRevisionError> {
    fn visit(
        expression: &Expr,
        receivers: &BTreeMap<String, String>,
        config: &SourceArchitectureConfig,
        endpoint_groups: &mut BTreeMap<String, String>,
    ) -> Result<(), ArchitectureRevisionError> {
        let children = |expressions: &[Expr], endpoint_groups: &mut BTreeMap<String, String>| {
            for expression in expressions {
                visit(expression, receivers, config, endpoint_groups)?;
            }
            Ok(())
        };
        match expression {
            Expr::CapabilityCall {
                receiver,
                operation,
                arguments,
                ..
            } => {
                let Some(interface) = receivers.get(receiver) else {
                    let endpoint = format!("{receiver}:{operation}");
                    let group = config.endpoint_groups.get(&endpoint).ok_or_else(|| {
                        ArchitectureRevisionError(format!(
                            "no endpoint group for built-in operation {endpoint}"
                        ))
                    })?;
                    endpoint_groups.insert(endpoint, group.clone());
                    return children(arguments, endpoint_groups);
                };
                let namespace = config.capability_namespaces.get(interface).ok_or_else(|| {
                    ArchitectureRevisionError(format!(
                        "no architecture namespace for capability interface {interface}"
                    ))
                })?;
                let namespace_endpoint = format!("capability:{namespace}");
                let group = config
                    .endpoint_groups
                    .get(&namespace_endpoint)
                    .ok_or_else(|| {
                        ArchitectureRevisionError(format!(
                            "no endpoint group for {namespace_endpoint}"
                        ))
                    })?;
                let operation_key = format!("{interface}.{operation}");
                let operation_architecture =
                    config.operations.get(&operation_key).ok_or_else(|| {
                        ArchitectureRevisionError(format!(
                            "no architecture interpretation for capability operation {operation_key}"
                        ))
                    })?;
                endpoint_groups.insert(format!("{namespace_endpoint}.{operation}"), group.clone());
                if let SourceOperationArchitecture::State { domain, .. } = operation_architecture {
                    let state_endpoint = format!("state:{domain}");
                    let state_group =
                        config.endpoint_groups.get(&state_endpoint).ok_or_else(|| {
                            ArchitectureRevisionError(format!(
                                "no endpoint group for {state_endpoint}"
                            ))
                        })?;
                    endpoint_groups.insert(state_endpoint, state_group.clone());
                }
                children(arguments, endpoint_groups)
            }
            Expr::Call { arguments, .. } => children(arguments, endpoint_groups),
            Expr::Record { fields, .. } => {
                for field in fields {
                    visit(&field.value, receivers, config, endpoint_groups)?;
                }
                Ok(())
            }
            Expr::Variant { payload, .. } => {
                if let Some(payload) = payload {
                    visit(payload, receivers, config, endpoint_groups)?;
                }
                Ok(())
            }
            Expr::FieldAccess { target, .. } => visit(target, receivers, config, endpoint_groups),
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                visit(condition, receivers, config, endpoint_groups)?;
                validate_architecture_operations(then_branch, receivers, config, endpoint_groups)?;
                validate_architecture_operations(else_branch, receivers, config, endpoint_groups)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                visit(scrutinee, receivers, config, endpoint_groups)?;
                for arm in arms {
                    validate_architecture_operations(
                        &arm.body,
                        receivers,
                        config,
                        endpoint_groups,
                    )?;
                }
                Ok(())
            }
            Expr::Map { source, body, .. } => {
                visit(source, receivers, config, endpoint_groups)?;
                validate_architecture_operations(body, receivers, config, endpoint_groups)
            }
            Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => Ok(()),
        }
    }

    for binding in &block.bindings {
        visit(&binding.value, receivers, config, endpoint_groups)?;
    }
    visit(&block.tail, receivers, config, endpoint_groups)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn derive_architecture_expr(
    block: &Block,
    source: &str,
    receivers: &BTreeMap<String, String>,
    config: &SourceArchitectureConfig,
    edges: &mut BTreeSet<(String, String, String)>,
    reads: &mut BTreeSet<String>,
    writes: &mut BTreeSet<String>,
    decisions: &mut usize,
) {
    #[allow(clippy::too_many_lines)]
    fn visit(
        expression: &Expr,
        source: &str,
        receivers: &BTreeMap<String, String>,
        config: &SourceArchitectureConfig,
        edges: &mut BTreeSet<(String, String, String)>,
        reads: &mut BTreeSet<String>,
        writes: &mut BTreeSet<String>,
        decisions: &mut usize,
    ) {
        match expression {
            Expr::Call {
                function,
                arguments,
                ..
            } => {
                let (module, local) = function.rsplit_once('.').unwrap_or(("", function));
                edges.insert((
                    source.to_owned(),
                    format!("{module}:{local}"),
                    "calls".into(),
                ));
                for argument in arguments {
                    visit(
                        argument, source, receivers, config, edges, reads, writes, decisions,
                    );
                }
            }
            Expr::CapabilityCall {
                receiver,
                operation,
                arguments,
                ..
            } => {
                if let Some(interface) = receivers.get(receiver) {
                    let namespace = config
                        .capability_namespaces
                        .get(interface)
                        .expect("source architecture validation resolves interfaces");
                    edges.insert((
                        source.to_owned(),
                        format!("capability:{namespace}.{operation}"),
                        "capability-use".into(),
                    ));
                    if let SourceOperationArchitecture::State { domain, access } = config
                        .operations
                        .get(&format!("{interface}.{operation}"))
                        .expect("source architecture validation resolves operations")
                    {
                        if matches!(
                            access,
                            SourceStateAccess::Read | SourceStateAccess::ReadWrite
                        ) {
                            reads.insert(domain.clone());
                            edges.insert((
                                source.to_owned(),
                                format!("state:{domain}"),
                                "state-read".into(),
                            ));
                        }
                        if matches!(
                            access,
                            SourceStateAccess::Write | SourceStateAccess::ReadWrite
                        ) {
                            writes.insert(domain.clone());
                            edges.insert((
                                source.to_owned(),
                                format!("state:{domain}"),
                                "state-write".into(),
                            ));
                        }
                    }
                } else {
                    edges.insert((
                        source.to_owned(),
                        format!("{receiver}:{operation}"),
                        "calls".into(),
                    ));
                }
                for argument in arguments {
                    visit(
                        argument, source, receivers, config, edges, reads, writes, decisions,
                    );
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                *decisions += 1;
                visit(
                    condition, source, receivers, config, edges, reads, writes, decisions,
                );
                derive_architecture_expr(
                    then_branch,
                    source,
                    receivers,
                    config,
                    edges,
                    reads,
                    writes,
                    decisions,
                );
                derive_architecture_expr(
                    else_branch,
                    source,
                    receivers,
                    config,
                    edges,
                    reads,
                    writes,
                    decisions,
                );
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                *decisions += arms.len().saturating_sub(1);
                visit(
                    scrutinee, source, receivers, config, edges, reads, writes, decisions,
                );
                for arm in arms {
                    derive_architecture_expr(
                        &arm.body, source, receivers, config, edges, reads, writes, decisions,
                    );
                }
            }
            Expr::Map {
                source: mapped,
                body,
                ..
            } => {
                *decisions += 1;
                visit(
                    mapped, source, receivers, config, edges, reads, writes, decisions,
                );
                derive_architecture_expr(
                    body, source, receivers, config, edges, reads, writes, decisions,
                );
            }
            Expr::Record { fields, .. } => {
                for field in fields {
                    visit(
                        &field.value,
                        source,
                        receivers,
                        config,
                        edges,
                        reads,
                        writes,
                        decisions,
                    );
                }
            }
            Expr::Variant { payload, .. } => {
                if let Some(payload) = payload {
                    visit(
                        payload, source, receivers, config, edges, reads, writes, decisions,
                    );
                }
            }
            Expr::FieldAccess { target, .. } => visit(
                target, source, receivers, config, edges, reads, writes, decisions,
            ),
            Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => {}
        }
    }
    for binding in &block.bindings {
        visit(
            &binding.value,
            source,
            receivers,
            config,
            edges,
            reads,
            writes,
            decisions,
        );
    }
    visit(
        &block.tail,
        source,
        receivers,
        config,
        edges,
        reads,
        writes,
        decisions,
    );
}

#[allow(clippy::too_many_lines)]
fn validate_modules(parsed_sources: &[(String, ParseResult)]) -> Vec<SourceSetDiagnostic> {
    if parsed_sources
        .iter()
        .all(|(_, parsed)| parsed.unit.module.is_none() && parsed.unit.imports.is_empty())
    {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();
    let mut modules = BTreeMap::<String, (&str, &ParseResult)>::new();
    for (path, parsed) in parsed_sources {
        let Some(module) = &parsed.unit.module else {
            diagnostics.push(source_set_diagnostic(
                "AIL.MODULE.MISSING_IDENTITY",
                path,
                Span::empty(0),
                [("requirement", "explicit module identity")],
            ));
            continue;
        };
        if let Some((existing_path, _)) = modules.get(&module.name) {
            diagnostics.push(source_set_diagnostic(
                "AIL.MODULE.DUPLICATE_IDENTITY",
                path,
                module.span,
                [
                    ("module", module.name.as_str()),
                    ("existing_path", *existing_path),
                ],
            ));
        } else {
            modules.insert(module.name.clone(), (path, parsed));
        }
    }

    let mut declarations = BTreeMap::<String, Vec<(String, String, Span)>>::new();
    for (module, (path, parsed)) in &modules {
        for declaration in &parsed.unit.declarations {
            let (name, span) = declaration_name(declaration);
            declarations.entry(name.to_owned()).or_default().push((
                module.clone(),
                (*path).to_owned(),
                span,
            ));
        }
    }
    for (module_name, (path, parsed)) in &modules {
        let mut imported_modules = BTreeSet::new();
        let mut qualifiers = BTreeMap::from([(module_name.as_str(), module_name.as_str())]);
        let local_declarations = parsed
            .unit
            .declarations
            .iter()
            .map(|declaration| declaration_name(declaration).0.to_owned())
            .collect::<BTreeSet<_>>();
        let mut imported_declarations = BTreeMap::<String, BTreeSet<String>>::new();
        for import in &parsed.unit.imports {
            if !imported_modules.insert(import.module.as_str()) {
                diagnostics.push(source_set_diagnostic(
                    "AIL.MODULE.DUPLICATE_IMPORT",
                    path,
                    import.span,
                    [("module", import.module.as_str())],
                ));
                continue;
            }
            if let Some(existing) = qualifiers.insert(import.qualifier(), import.module.as_str()) {
                diagnostics.push(source_set_diagnostic(
                    "AIL.MODULE.DUPLICATE_QUALIFIER",
                    path,
                    import.span,
                    [
                        ("qualifier", import.qualifier()),
                        ("first_module", existing),
                        ("second_module", import.module.as_str()),
                    ],
                ));
                continue;
            }
            let Some((_, imported)) = modules.get(&import.module) else {
                diagnostics.push(source_set_diagnostic(
                    "AIL.MODULE.MISSING_IMPORT",
                    path,
                    import.span,
                    [("module", import.module.as_str())],
                ));
                continue;
            };
            if import.alias.is_none() {
                for declaration in &imported.unit.declarations {
                    let (name, _) = declaration_name(declaration);
                    imported_declarations
                        .entry(name.to_owned())
                        .or_default()
                        .insert(import.module.clone());
                }
            }
        }

        let mut visible = local_declarations.clone();
        visible.extend(
            imported_declarations
                .iter()
                .filter(|(name, modules)| modules.len() == 1 && !local_declarations.contains(*name))
                .map(|(name, _)| name.clone()),
        );
        for declaration in &parsed.unit.declarations {
            let (name, _) = declaration_name(declaration);
            visible.insert(qualified_name(module_name, name));
        }
        for import in &parsed.unit.imports {
            let Some((_, imported)) = modules.get(&import.module) else {
                continue;
            };
            for declaration in &imported.unit.declarations {
                let (name, _) = declaration_name(declaration);
                visible.insert(qualified_name(import.qualifier(), name));
            }
        }
        for (name, span, role) in source_references(&parsed.unit) {
            if !name.contains('.') && !local_declarations.contains(&name) {
                if let Some(imports) = imported_declarations.get(&name) {
                    if imports.len() > 1 {
                        let mut imports = imports.iter();
                        diagnostics.push(source_set_diagnostic(
                            "AIL.MODULE.AMBIGUOUS_IMPORT",
                            path,
                            span,
                            [
                                ("declaration", name.as_str()),
                                (
                                    "first_module",
                                    imports.next().expect("at least two imports"),
                                ),
                                (
                                    "second_module",
                                    imports.next().expect("at least two imports"),
                                ),
                            ],
                        ));
                        continue;
                    }
                }
            }
            if declarations.contains_key(source_name(&name)) && !visible.contains(&name) {
                diagnostics.push(source_set_diagnostic(
                    "AIL.MODULE.INACCESSIBLE_DECLARATION",
                    path,
                    span,
                    [
                        ("declaration", name.as_str()),
                        ("role", role),
                        ("module", module_name.as_str()),
                    ],
                ));
            }
        }
    }

    let mut cycles = BTreeMap::<String, (String, Span, Vec<String>)>::new();
    for module in modules.keys() {
        find_import_cycles(module, &modules, &mut Vec::new(), &mut cycles);
    }
    for (_, (path, span, cycle)) in cycles {
        diagnostics.push(source_set_diagnostic(
            "AIL.MODULE.IMPORT_CYCLE",
            &path,
            span,
            [("cycle", cycle.join(" -> ").as_str())],
        ));
    }

    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.span.start.cmp(&right.span.start))
            .then(left.code.cmp(right.code))
    });
    diagnostics
}

fn link_source_set(parsed_sources: &[(String, ParseResult)]) -> SourceUnit {
    if parsed_sources
        .iter()
        .all(|(_, parsed)| parsed.unit.module.is_none())
    {
        return SourceUnit {
            module: None,
            imports: Vec::new(),
            declarations: parsed_sources
                .iter()
                .flat_map(|(_, parsed)| parsed.unit.declarations.iter().cloned())
                .collect(),
            span: Span::empty(0),
            tokens: Vec::new(),
        };
    }

    let modules = parsed_sources
        .iter()
        .filter_map(|(_, parsed)| {
            parsed
                .unit
                .module
                .as_ref()
                .map(|module| (module.name.as_str(), &parsed.unit))
        })
        .collect::<BTreeMap<_, _>>();
    let mut declarations = Vec::new();
    for (_, parsed) in parsed_sources {
        let module = parsed
            .unit
            .module
            .as_ref()
            .expect("module validation requires every source identity");
        let mut scope = parsed
            .unit
            .declarations
            .iter()
            .map(|declaration| {
                let name = declaration_name(declaration).0;
                (name.to_owned(), qualified_name(&module.name, name))
            })
            .collect::<BTreeMap<_, _>>();
        for declaration in &parsed.unit.declarations {
            let name = declaration_name(declaration).0;
            scope.insert(
                qualified_name(&module.name, name),
                qualified_name(&module.name, name),
            );
        }
        let local_names = parsed
            .unit
            .declarations
            .iter()
            .map(|declaration| declaration_name(declaration).0)
            .collect::<BTreeSet<_>>();
        let mut bare_imports = BTreeMap::<String, Vec<String>>::new();
        for import in &parsed.unit.imports {
            let imported = modules
                .get(import.module.as_str())
                .expect("module validation resolves every import");
            for declaration in &imported.declarations {
                let name = declaration_name(declaration).0;
                let target = qualified_name(&import.module, name);
                scope.insert(qualified_name(import.qualifier(), name), target.clone());
                if import.alias.is_none() && !local_names.contains(name) {
                    bare_imports
                        .entry(name.to_owned())
                        .or_default()
                        .push(target);
                }
            }
        }
        for (name, targets) in bare_imports {
            if targets.len() == 1 {
                scope.insert(name, targets.into_iter().next().expect("one target"));
            }
        }
        declarations.extend(
            parsed
                .unit
                .declarations
                .iter()
                .cloned()
                .map(|declaration| qualify_declaration(declaration, &module.name, &scope)),
        );
    }
    SourceUnit {
        module: None,
        imports: Vec::new(),
        declarations,
        span: Span::empty(0),
        tokens: Vec::new(),
    }
}

fn qualify_declaration(
    mut declaration: Declaration,
    module: &str,
    scope: &BTreeMap<String, String>,
) -> Declaration {
    match &mut declaration {
        Declaration::Record(record) => {
            record.name = qualified_name(module, &record.name);
            for field in &mut record.fields {
                field.ty.qualify(&|name| resolve_linked_name(scope, name));
            }
        }
        Declaration::Variant(variant) => {
            variant.name = qualified_name(module, &variant.name);
            for case in &mut variant.cases {
                if let Some(payload) = &mut case.payload {
                    payload.qualify(&|name| resolve_linked_name(scope, name));
                }
            }
        }
        Declaration::Function(function) => {
            function.name = qualified_name(module, &function.name);
            for parameter in &mut function.parameters {
                if let ParameterType::Value(ty) = &mut parameter.ty {
                    ty.qualify(&|name| resolve_linked_name(scope, name));
                }
            }
            function
                .result_type
                .qualify(&|name| resolve_linked_name(scope, name));
            qualify_block(&mut function.body, scope);
        }
    }
    declaration
}

fn qualify_block(block: &mut Block, scope: &BTreeMap<String, String>) {
    for binding in &mut block.bindings {
        qualify_expression(&mut binding.value, scope);
    }
    qualify_expression(&mut block.tail, scope);
}

fn qualify_expression(expression: &mut Expr, scope: &BTreeMap<String, String>) {
    match expression {
        Expr::Map { source, body, .. } => {
            qualify_expression(source, scope);
            qualify_block(body, scope);
        }
        Expr::Call {
            function,
            arguments,
            ..
        } => {
            *function = resolve_linked_name(scope, function);
            for argument in arguments {
                qualify_expression(argument, scope);
            }
        }
        Expr::Record { name, fields, .. } => {
            *name = resolve_linked_name(scope, name);
            for field in fields {
                qualify_expression(&mut field.value, scope);
            }
        }
        Expr::Variant {
            type_name, payload, ..
        } => {
            *type_name = resolve_linked_name(scope, type_name);
            if let Some(payload) = payload {
                qualify_expression(payload, scope);
            }
        }
        Expr::CapabilityCall { arguments, .. } => {
            for argument in arguments {
                qualify_expression(argument, scope);
            }
        }
        Expr::FieldAccess { target, .. } => qualify_expression(target, scope),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            qualify_expression(condition, scope);
            qualify_block(then_branch, scope);
            qualify_block(else_branch, scope);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            qualify_expression(scrutinee, scope);
            for arm in arms {
                arm.type_name = resolve_linked_name(scope, &arm.type_name);
                qualify_block(&mut arm.body, scope);
            }
        }
        Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => {}
    }
}

fn resolve_linked_name(scope: &BTreeMap<String, String>, name: &str) -> String {
    scope.get(name).cloned().unwrap_or_else(|| name.to_owned())
}

fn qualified_name(module: &str, name: &str) -> String {
    format!("{module}.{name}")
}

fn source_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

fn entry_functions<'a>(unit: &'a SourceUnit, selector: &str) -> Vec<&'a FunctionDecl> {
    unit.declarations
        .iter()
        .filter_map(|declaration| {
            let Declaration::Function(candidate) = declaration else {
                return None;
            };
            (candidate.name == selector || source_name(&candidate.name) == selector)
                .then_some(candidate)
        })
        .collect()
}

fn declaration_name(declaration: &Declaration) -> (&str, Span) {
    match declaration {
        Declaration::Record(record) => (&record.name, record.span),
        Declaration::Variant(variant) => (&variant.name, variant.span),
        Declaration::Function(function) => (&function.name, function.span),
    }
}

fn source_references(unit: &SourceUnit) -> Vec<(String, Span, &'static str)> {
    let mut references = Vec::new();
    for declaration in &unit.declarations {
        match declaration {
            Declaration::Record(record) => {
                for field in &record.fields {
                    references.extend(
                        field
                            .ty
                            .named_references()
                            .into_iter()
                            .map(|(name, span)| (name.to_owned(), span, "type")),
                    );
                }
            }
            Declaration::Variant(variant) => {
                for case in &variant.cases {
                    if let Some(payload) = &case.payload {
                        references.extend(
                            payload
                                .named_references()
                                .into_iter()
                                .map(|(name, span)| (name.to_owned(), span, "type")),
                        );
                    }
                }
            }
            Declaration::Function(function) => {
                for parameter in &function.parameters {
                    if let ParameterType::Value(ty) = &parameter.ty {
                        references.extend(
                            ty.named_references()
                                .into_iter()
                                .map(|(name, span)| (name.to_owned(), span, "type")),
                        );
                    }
                }
                references.extend(
                    function
                        .result_type
                        .named_references()
                        .into_iter()
                        .map(|(name, span)| (name.to_owned(), span, "type")),
                );
                collect_source_references_block(&function.body, &mut references);
            }
        }
    }
    references
}

fn inspect_bounded_list(ty: &TypeRef, unit: &SourceUnit) -> Option<BoundedListInspection> {
    let (element, max_length) = ty.as_list()?;
    let element_type = element.to_string();
    let element_identity = element.as_named().and_then(|name| {
        unit.declarations
            .iter()
            .find_map(|declaration| match declaration {
                Declaration::Record(record) if record.name == name => record.identity.clone(),
                Declaration::Variant(variant) if variant.name == name => variant.identity.clone(),
                Declaration::Record(_) | Declaration::Variant(_) | Declaration::Function(_) => None,
            })
    });
    Some(BoundedListInspection {
        element_type,
        element_identity,
        max_length,
    })
}

fn function_dependencies(function: &FunctionDecl) -> Vec<String> {
    let mut dependencies = BTreeSet::new();
    for parameter in &function.parameters {
        match &parameter.ty {
            ParameterType::Value(ty) => {
                dependencies.extend(
                    ty.named_references()
                        .into_iter()
                        .map(|(name, _)| name.to_owned()),
                );
            }
            ParameterType::Capability(interface) => {
                dependencies.insert(interface.clone());
            }
        }
    }
    dependencies.extend(
        function
            .result_type
            .named_references()
            .into_iter()
            .map(|(name, _)| name.to_owned()),
    );
    let mut references = Vec::new();
    collect_source_references_block(&function.body, &mut references);
    dependencies.extend(references.into_iter().map(|(name, _, _)| name));
    for effect in &function.effects {
        if let Some(ParameterType::Capability(interface)) = function
            .parameters
            .iter()
            .find(|parameter| parameter.name == effect.receiver)
            .map(|parameter| &parameter.ty)
        {
            dependencies.insert(format!("{interface}.{}", effect.operation));
        }
    }
    dependencies.into_iter().collect()
}

fn collect_source_references_block(
    block: &Block,
    references: &mut Vec<(String, Span, &'static str)>,
) {
    for binding in &block.bindings {
        collect_source_references(&binding.value, references);
    }
    collect_source_references(&block.tail, references);
}

fn collect_source_references(
    expression: &Expr,
    references: &mut Vec<(String, Span, &'static str)>,
) {
    match expression {
        Expr::Map { source, body, .. } => {
            collect_source_references(source, references);
            collect_source_references_block(body, references);
        }
        Expr::Call {
            function,
            arguments,
            span,
        } => {
            references.push((function.clone(), *span, "function"));
            for argument in arguments {
                collect_source_references(argument, references);
            }
        }
        Expr::Record { name, fields, span } => {
            references.push((name.clone(), *span, "record"));
            for field in fields {
                collect_source_references(&field.value, references);
            }
        }
        Expr::Variant {
            type_name,
            payload,
            span,
            ..
        } => {
            references.push((type_name.clone(), *span, "variant"));
            if let Some(payload) = payload {
                collect_source_references(payload, references);
            }
        }
        Expr::CapabilityCall { arguments, .. } => {
            for argument in arguments {
                collect_source_references(argument, references);
            }
        }
        Expr::FieldAccess { target, .. } => collect_source_references(target, references),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_source_references(condition, references);
            collect_source_references_block(then_branch, references);
            collect_source_references_block(else_branch, references);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_source_references(scrutinee, references);
            for arm in arms {
                references.push((arm.type_name.clone(), arm.span, "variant"));
                collect_source_references_block(&arm.body, references);
            }
        }
        Expr::Text { .. } | Expr::Integer { .. } | Expr::Name { .. } => {}
    }
}

fn find_import_cycles(
    module: &str,
    modules: &BTreeMap<String, (&str, &ParseResult)>,
    stack: &mut Vec<String>,
    cycles: &mut BTreeMap<String, (String, Span, Vec<String>)>,
) {
    if stack.iter().any(|entry| entry == module) {
        return;
    }
    let Some((path, parsed)) = modules.get(module) else {
        return;
    };
    stack.push(module.to_owned());
    for import in &parsed.unit.imports {
        if let Some(start) = stack.iter().position(|entry| *entry == import.module) {
            let mut cycle = stack[start..].to_vec();
            cycle.push(import.module.clone());
            let mut members = cycle[..cycle.len() - 1].to_vec();
            members.sort();
            cycles
                .entry(members.join("\0"))
                .or_insert(((*path).to_owned(), import.span, cycle));
        } else {
            find_import_cycles(&import.module, modules, stack, cycles);
        }
    }
    stack.pop();
}

fn source_set_diagnostic<const N: usize>(
    code: &'static str,
    path: &str,
    span: Span,
    details: [(&str, &str); N],
) -> SourceSetDiagnostic {
    SourceSetDiagnostic {
        code,
        path: path.to_owned(),
        span,
        details: details
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect(),
    }
}

fn diagnostic_value_text(value: &crate::DiagnosticValue) -> String {
    match value {
        crate::DiagnosticValue::Text(value) => value.clone(),
        crate::DiagnosticValue::TextList(values) => values.join(", "),
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

fn source_path_for_function<'a>(sources: &'a [EvolutionSource], function: &str) -> &'a str {
    let (module, source_function) = function
        .rsplit_once('.')
        .map_or((None, function), |(module, function)| {
            (Some(module), function)
        });
    sources
        .iter()
        .find(|source| {
            let parsed = parse(&source.source);
            module.is_none_or(|module| {
                parsed.unit.module.as_ref().map(|item| item.name.as_str()) == Some(module)
            }) && parsed.unit.declarations.iter().any(|declaration| {
                matches!(declaration, Declaration::Function(candidate) if candidate.name == source_function)
            })
        })
        .map_or("<unknown>", |source| source.path.as_str())
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
        Err(EvolutionBuildFailure {
            causes,
            diagnostics: Vec::new(),
        })
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
    let identity_by_path_and_name = identities
        .iter()
        .filter(|identity| identity.parent_identity.is_none())
        .map(|identity| {
            (
                (
                    identity_path(identity).to_owned(),
                    identity.display_name.clone(),
                ),
                identity.identity.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let module_paths = parsed_sources
        .iter()
        .filter_map(|(path, parsed)| {
            parsed
                .unit
                .module
                .as_ref()
                .map(|module| (module.name.as_str(), path.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
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
        let local_identity_names = identity_by_path_and_name
            .keys()
            .filter_map(|(identity_path, name)| (identity_path == path).then_some(name.as_str()))
            .collect::<BTreeSet<_>>();
        let mut visible_identities = if parsed.unit.module.is_none() {
            identity_by_name
                .iter()
                .map(|(name, identity)| ((*name).to_owned(), *identity))
                .collect::<BTreeMap<_, _>>()
        } else {
            identity_by_path_and_name
                .iter()
                .filter_map(|((identity_path, name), identity)| {
                    (identity_path == path).then_some((name.clone(), *identity))
                })
                .collect::<BTreeMap<_, _>>()
        };
        if let Some(module) = &parsed.unit.module {
            for ((identity_path, name), identity) in &identity_by_path_and_name {
                if identity_path == path {
                    visible_identities.insert(qualified_name(&module.name, name), *identity);
                }
            }
        }
        for import in &parsed.unit.imports {
            let imported_path = module_paths
                .get(import.module.as_str())
                .expect("validated imports have a source path");
            for ((identity_path, name), identity) in &identity_by_path_and_name {
                if identity_path == imported_path {
                    visible_identities.insert(qualified_name(import.qualifier(), name), *identity);
                    if import.alias.is_none() && !local_identity_names.contains(name.as_str()) {
                        visible_identities.insert(name.clone(), *identity);
                    }
                }
            }
        }
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
                if let ParameterType::Value(ty) = &parameter.ty {
                    for (name, _) in ty.named_references() {
                        let Some(identity) = visible_identities.get(name) else {
                            continue;
                        };
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
            for (name, _) in function.result_type.named_references() {
                if let Some(identity) = visible_identities.get(name) {
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
                &visible_identities,
                &mut graph,
            );
            if function.name.contains("adapt_") || function.name.contains("decode_") {
                if let Some(ParameterType::Value(input)) =
                    function.parameters.first().map(|parameter| &parameter.ty)
                {
                    if let Some(identity) = input
                        .as_named()
                        .and_then(|name| visible_identities.get(name))
                    {
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
                if let Some(identity) = function
                    .result_type
                    .as_named()
                    .and_then(|name| visible_identities.get(name))
                {
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
                if let Some(identity) = function
                    .result_type
                    .as_named()
                    .and_then(|name| visible_identities.get(name))
                {
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
    identities: &BTreeMap<String, &str>,
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
    identities: &BTreeMap<String, &str>,
    graph: &mut Vec<RelationshipEdge>,
) {
    match expression {
        Expr::Map {
            source: input,
            body,
            ..
        } => {
            walk_expr(input, source, revision_id, path, identities, graph);
            walk_block(body, source, revision_id, path, identities, graph);
        }
        Expr::Call { arguments, .. } => {
            for argument in arguments {
                walk_expr(argument, source, revision_id, path, identities, graph);
            }
        }
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
    let subject_type = linked_type_name_for_identity(stored, &change.subject_identity)
        .unwrap_or(subject.display_name.as_str());
    let functions = functions_with_paths(&stored.sources, &stored.unit);
    let handler = functions.iter().find(|(_, function)| {
        function.parameters.iter().any(
            |parameter| matches!(&parameter.ty, ParameterType::Value(ty) if ty.as_named() == Some(subject_type)),
        ) && function
            .parameters
            .iter()
            .any(|parameter| matches!(parameter.ty, ParameterType::Capability(_)))
    });
    let stored_name = handler
        .and_then(|(_, function)| capability_argument_record(function))
        .unwrap_or_default();
    let stored_schema = top_identity_for_type(stored, &stored_name);
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
        named_parameter(function, subject_type)
            && function.result_type.as_named() == Some(subject_type)
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
            && function.result_type.as_named() == Some(stored_name.as_str())
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
            && function.result_type.as_named() == Some(stored_name.as_str())
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
            && function.result_type.as_named() != Some(stored_name.as_str())
            && function
                .result_type
                .as_named()
                .is_some_and(|name| expression_constructs(&function.body, name))
    });
    if let Some((path, function)) = projection {
        let output_identity = identity_for_type(
            stored,
            function
                .result_type
                .as_named()
                .expect("projection result is a named type"),
        );
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
        function.result_type.as_named() == Some(subject_type)
            && !named_parameter(function, subject_type)
            && expression_constructs(&function.body, subject_type)
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
        if let Some(result_schema) = function
            .result_type
            .as_named()
            .and_then(|name| top_identity_for_type(stored, name))
        {
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
            if let Some(outcome_identity) = top_identity_for_type(stored, &outcome_name) {
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
            let path = source_path_for_function(sources, &function.name);
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

fn linked_type_name_for_identity<'a>(
    stored: &'a StoredSourceSet,
    identity: &str,
) -> Option<&'a str> {
    stored
        .unit
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            Declaration::Record(record) if record.identity.as_deref() == Some(identity) => {
                Some(record.name.as_str())
            }
            Declaration::Variant(variant) if variant.identity.as_deref() == Some(identity) => {
                Some(variant.name.as_str())
            }
            Declaration::Record(_) | Declaration::Variant(_) | Declaration::Function(_) => None,
        })
}

fn top_identity_for_type<'a>(
    stored: &'a StoredSourceSet,
    name: &str,
) -> Option<&'a PersistentIdentity> {
    let identity = identity_for_type(stored, name);
    stored
        .identities
        .iter()
        .find(|candidate| candidate.identity == identity)
}

fn identity_for_type<'a>(stored: &'a StoredSourceSet, name: &str) -> &'a str {
    stored
        .unit
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            Declaration::Record(record) if record.name == name => record.identity.as_deref(),
            Declaration::Variant(variant) if variant.name == name => variant.identity.as_deref(),
            Declaration::Record(_) | Declaration::Variant(_) | Declaration::Function(_) => None,
        })
        .unwrap_or("")
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
        .any(|parameter| matches!(&parameter.ty, ParameterType::Value(ty) if ty.as_named() == Some(type_name)))
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
        Expr::Map { source, body, .. } => {
            expr_constructs(source, type_name) || expression_constructs(body, type_name)
        }
        Expr::Call { arguments, .. } => arguments
            .iter()
            .any(|argument| expr_constructs(argument, type_name)),
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
        Expr::Map { source, body, .. } => {
            collect_nested_bindings(source, bindings);
            collect_record_bindings(body, bindings);
        }
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
        | Expr::Call { .. }
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
            ParameterType::Value(_) => None,
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
        Expr::Map { source, body, .. } => find_capability_expr(source, receivers)
            .or_else(|| find_capability_call(body, receivers)),
        Expr::Call { arguments, .. } => arguments
            .iter()
            .find_map(|argument| find_capability_expr(argument, receivers)),
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
        Expr::Map { source, body, .. } => {
            collect_match_types(source, types);
            for binding in &body.bindings {
                collect_match_types(&binding.value, types);
            }
            collect_match_types(&body.tail, types);
        }
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
        Expr::CapabilityCall { arguments, .. } | Expr::Call { arguments, .. } => {
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
