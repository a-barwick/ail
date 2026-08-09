use std::collections::{BTreeMap, BTreeSet};

use crate::interpreter::interpret;
use crate::{
    ArchitectureChangeResult, ArchitectureEvaluationInput, ArchitectureRequestError,
    ArchitectureRequestErrorKind, ArchitectureWorkspace, BehaviorValidation, CanonicalEdit,
    CapabilityProvider, CausalStep, Declaration, DiagnosticValue, ExecutionFailure,
    ExecutionResponse, ExecutionSuccess, HandleKind, IdentityClassification, IdentityMap,
    IdentityMapEntry, ParameterType, RuntimeFault, RuntimeValue, SemanticHandle, Span,
    StructuredDiagnostic,
};

use super::{
    EvolutionSource, EvolutionWorkspace, ImpactEntry, ImpactReport, ProposedSchemaChange,
    SourceArchitectureConfig, SourceSetRevision, StoredSourceSet, UncheckedBoundary,
    entry_functions, handle, source_name, source_path_for_function,
};

/// Complete source candidate for source-backed architecture validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitectureSourceChangeRequest {
    pub base_revision_id: String,
    pub candidate_sources: Vec<EvolutionSource>,
}

/// Complete source-set candidate associated with one already-computed impact report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateChangeRequest {
    pub base_revision_id: String,
    pub candidate_sources: Vec<EvolutionSource>,
    pub required_impact_ids: Vec<String>,
}

/// One typed semantic consequence in a committed source-set change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticChange {
    pub kind: String,
    pub identity: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Observable effect facts before and after a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeEffectSummary {
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub ordering: &'static str,
}

/// Capability authority facts before and after a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeCapabilitySummary {
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub authority: &'static str,
}

/// Deterministic typed source-set diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticDiff {
    pub from_revision_id: String,
    pub to_revision_id: String,
    pub changes: Vec<SemanticChange>,
    pub effect_summary: ChangeEffectSummary,
    pub capability_summary: ChangeCapabilitySummary,
}

/// Persistent identity classifications remain separate from revision handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentIdentityChanges {
    pub preserved: Vec<String>,
    pub added: Vec<String>,
    pub retired: Vec<String>,
}

/// Every validation phase bound to the committed child revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSummary {
    pub revision_id: String,
    pub parse: &'static str,
    pub types: &'static str,
    pub capabilities: &'static str,
    pub impact: &'static str,
    pub public_behavior: String,
}

/// Revision-bound evidence proving the complete accepted change loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEvidence {
    pub base_revision_id: String,
    pub revision_id: String,
    pub impact_report: ImpactReport,
    pub edits: Vec<CanonicalEdit>,
    pub identity_map: IdentityMap,
    pub persistent_identities: PersistentIdentityChanges,
    pub semantic_diff: SemanticDiff,
    pub validation: ValidationSummary,
    pub analyzed_paths: Vec<String>,
    pub unchecked: Vec<UncheckedBoundary>,
}

/// A committed atomic candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSuccess {
    pub status: &'static str,
    pub base_revision_id: String,
    pub revision: SourceSetRevision,
    pub completion: CompletionEvidence,
}

/// A rejected candidate. Rejections never contain edits or publish a revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeFailure {
    pub status: &'static str,
    pub base_revision_id: String,
    pub current_revision_id: String,
    pub phase: &'static str,
    pub diagnostic: StructuredDiagnostic,
    pub edits: Vec<CanonicalEdit>,
}

/// Result of whole-workspace candidate validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeResponse {
    Committed(Box<ChangeSuccess>),
    Rejected(Box<ChangeFailure>),
}

/// Public-behavior failure returned by the caller's accepted fixture oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicBehaviorFailure {
    pub failing_case: String,
    pub location: String,
}

/// Read-only access to the exact uncommitted candidate used by behavior validation.
pub struct CandidateRevision<'a> {
    stored: &'a StoredSourceSet,
}

impl CandidateRevision<'_> {
    #[must_use]
    pub fn revision(&self) -> &SourceSetRevision {
        &self.stored.revision
    }

    #[must_use]
    pub fn sources(&self) -> &[EvolutionSource] {
        &self.stored.sources
    }

    /// Execute a checked function from this exact candidate before publication.
    #[must_use]
    pub fn execute(
        &self,
        function: &str,
        arguments: Vec<RuntimeValue>,
        capabilities: &mut dyn CapabilityProvider,
    ) -> ExecutionResponse {
        let matching_functions = entry_functions(&self.stored.unit, function);
        if matching_functions.len() > 1 {
            return ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: self.stored.revision.revision_id.clone(),
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
                revision_id: self.stored.revision.revision_id.clone(),
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
            &self.stored.revision.revision_id,
            source_path_for_function(&self.stored.sources, linked_function),
            HandleKind::Symbol,
            source_name(linked_function),
            declaration.span,
        );
        match interpret(
            &self.stored.unit,
            linked_function,
            arguments,
            &self.stored.capabilities,
            capabilities,
        ) {
            Ok(result) => ExecutionResponse::Completed(ExecutionSuccess {
                status: "completed",
                revision_id: self.stored.revision.revision_id.clone(),
                function_handle,
                value: result.value,
                calls: result.calls,
            }),
            Err(result) => ExecutionResponse::Failed(ExecutionFailure {
                status: "failed",
                revision_id: self.stored.revision.revision_id.clone(),
                function: function.to_owned(),
                fault: result.fault,
                calls: result.calls,
            }),
        }
    }
}

impl EvolutionWorkspace {
    /// Build, check, evaluate, and atomically publish a source-backed architecture candidate.
    ///
    /// # Errors
    /// Returns a request error for stale or mismatched revision identities, invalid source,
    /// invalid derived architecture facts, or evaluator request failures. No error publishes.
    pub fn validate_source_architecture_change<F>(
        &mut self,
        request: ArchitectureSourceChangeRequest,
        config: &SourceArchitectureConfig,
        input: &ArchitectureEvaluationInput,
        validate_behavior: F,
    ) -> Result<ArchitectureChangeResult, ArchitectureRequestError>
    where
        F: FnOnce(&CandidateRevision<'_>) -> Result<BehaviorValidation, ArchitectureRequestError>,
    {
        if request.base_revision_id != self.current_revision_id {
            return Err(ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::StaleRevision,
                message: "base source revision is not current".into(),
            });
        }
        let base = self
            .revisions
            .get(&request.base_revision_id)
            .ok_or_else(|| ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::StaleRevision,
                message: "base source revision is not retained".into(),
            })?;
        let saved_config =
            base.architecture_config
                .clone()
                .ok_or_else(|| ArchitectureRequestError {
                    kind: ArchitectureRequestErrorKind::InvalidRevision,
                    message: "base source revision has no saved architecture settings".into(),
                })?;
        if saved_config.stable_digest() != config.stable_digest() || saved_config != *config {
            return Err(ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::InvalidRevision,
                message: "supplied architecture settings differ from the base revision".into(),
            });
        }
        let child_revision_id = self.next_revision_id(&request.base_revision_id);
        if input.request.base_revision_id != request.base_revision_id
            || input.request.candidate_revision_id != child_revision_id
        {
            return Err(ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::InvalidRevision,
                message: "architecture request is not bound to generated source revisions".into(),
            });
        }
        let coverage = base.coverage.clone();
        let base_architecture = base
            .architecture_revision(&self.id, &saved_config)
            .map_err(|error| ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::InvalidRevision,
                message: error.0,
            })?;
        let base_paths = base
            .sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<Vec<_>>();
        let mut candidate_paths = request
            .candidate_sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<Vec<_>>();
        candidate_paths.sort_unstable();
        if candidate_paths != base_paths {
            return Err(ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::InvalidRevision,
                message: "architecture candidate is not the complete source set".into(),
            });
        }
        let mut candidate = StoredSourceSet::build(
            &self.id,
            &child_revision_id,
            Some(request.base_revision_id.clone()),
            request.candidate_sources,
            &base.capabilities,
            coverage,
        )
        .map_err(|failure| ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: failure.causes.join(","),
        })?;
        candidate
            .revision
            .architecture_settings_digest
            .clone_from(&base.revision.architecture_settings_digest);
        candidate.architecture_config = Some(saved_config.clone());
        let candidate_architecture = candidate
            .architecture_revision(&self.id, &saved_config)
            .map_err(|error| ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::InvalidRevision,
                message: error.0,
            })?;
        let candidate_view = CandidateRevision { stored: &candidate };
        let mut architecture_workspace = ArchitectureWorkspace::new(base_architecture);
        let result = architecture_workspace.validate_architecture_change(
            candidate_architecture,
            input,
            |_| validate_behavior(&candidate_view),
        )?;
        if matches!(result, ArchitectureChangeResult::Success(_)) {
            self.revisions.insert(child_revision_id.clone(), candidate);
            self.current_revision_id = child_revision_id;
        }
        Ok(result)
    }

    /// Canonicalize and validate a complete candidate before publishing one child revision.
    ///
    /// The supplied behavior oracle receives read-only execution access to the exact candidate.
    /// A failure in any phase leaves the current revision and retained revision set unchanged.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn validate_change<F>(
        &mut self,
        request: CandidateChangeRequest,
        impact_report: &ImpactReport,
        validate_public_behavior: F,
    ) -> ChangeResponse
    where
        F: FnOnce(&CandidateRevision<'_>) -> Result<String, PublicBehaviorFailure>,
    {
        if request.base_revision_id != self.current_revision_id {
            return ChangeResponse::Rejected(self.failure(
                &request,
                "revision",
                "AIL.PROTOCOL.STALE_REVISION",
                &format!("workspace:{}", request.base_revision_id),
                [("revision", request.base_revision_id.as_str())],
                [("revision", self.current_revision_id.as_str())],
                "validate-current-revision",
            ));
        }
        let Some(base) = self.revisions.get(&request.base_revision_id) else {
            return ChangeResponse::Rejected(self.failure(
                &request,
                "revision",
                "AIL.PROTOCOL.STALE_REVISION",
                &format!("workspace:{}", request.base_revision_id),
                [("revision", request.base_revision_id.as_str())],
                [("revision", "not retained")],
                "resolve-base-revision",
            ));
        };

        let recomputed = match self.impact(super::ImpactRequest {
            base_revision_id: request.base_revision_id.clone(),
            change: impact_report.change.clone(),
        }) {
            Ok(report) => report,
            Err(failure) => {
                return ChangeResponse::Rejected(self.failure(
                    &request,
                    "impact",
                    failure.code,
                    "workspace:impact",
                    [("impact", "complete")],
                    [("impact", failure.reason.as_str())],
                    "recompute-impact",
                ));
            }
        };
        if &recomputed != impact_report {
            return ChangeResponse::Rejected(self.failure(
                &request,
                "impact",
                "AIL.IMPACT.MISSED_CONSUMER",
                "workspace:impact",
                [("impact_report", "current base report")],
                [("impact_report", "mismatched report")],
                "bind-impact-report",
            ));
        }
        let expected_ids = recomputed
            .must_change
            .iter()
            .map(impact_id)
            .collect::<Vec<_>>();
        if request.required_impact_ids != expected_ids {
            let missing = expected_ids
                .iter()
                .find(|id| !request.required_impact_ids.contains(id))
                .map_or("impact-report", String::as_str);
            let location = recomputed
                .must_change
                .iter()
                .find(|entry| impact_id(entry) == missing)
                .map_or("workspace:impact", |entry| entry.location.as_str());
            return ChangeResponse::Rejected(self.failure(
                &request,
                "impact",
                "AIL.IMPACT.MISSED_CONSUMER",
                location,
                [("required_impact_id", missing)],
                [("required_impact_id", "missing or out of order")],
                "validate-impact-accounting",
            ));
        }

        let base_paths = base
            .sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<Vec<_>>();
        let mut candidate_paths = request
            .candidate_sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<Vec<_>>();
        candidate_paths.sort_unstable();
        if candidate_paths != base_paths {
            return ChangeResponse::Rejected(self.failure(
                &request,
                "impact",
                "AIL.IMPACT.MISSED_CONSUMER",
                "workspace:sources",
                [("analyzed_paths", base_paths.join(",").as_str())],
                [("candidate_paths", candidate_paths.join(",").as_str())],
                "validate-complete-source-set",
            ));
        }

        let child_revision_id = self.next_revision_id(&request.base_revision_id);
        let mut candidate = match StoredSourceSet::build(
            &self.id,
            &child_revision_id,
            Some(request.base_revision_id.clone()),
            request.candidate_sources.clone(),
            &base.capabilities,
            base.coverage.clone(),
        ) {
            Ok(candidate) => candidate,
            Err(failure) => {
                let actual = failure.causes.join(",");
                return ChangeResponse::Rejected(self.failure(
                    &request,
                    "static",
                    "AIL.PROTOCOL.VALIDATION_FAILED",
                    "workspace:candidate",
                    [("validation", "parse and types ok")],
                    [("diagnostics", actual.as_str())],
                    "validate-candidate-source-set",
                ));
            }
        };
        candidate
            .revision
            .architecture_settings_digest
            .clone_from(&base.revision.architecture_settings_digest);
        candidate
            .architecture_config
            .clone_from(&base.architecture_config);

        if let Some((identity, location)) = incompatible_identity(base, &candidate) {
            return ChangeResponse::Rejected(self.failure(
                &request,
                "schema",
                "AIL.SCHEMA.IDENTITY_INCOMPATIBLE",
                &location,
                [("identity", identity.as_str())],
                [("schema", "incompatible shape")],
                "compare-persistent-schema",
            ));
        }

        let base_effects = declared_effects(base);
        let candidate_effects = declared_effects(&candidate);
        let base_capabilities = declared_capabilities(base);
        let candidate_capabilities = declared_capabilities(&candidate);
        if !candidate_effects.is_subset(&base_effects)
            || !candidate_capabilities.is_subset(&base_capabilities)
        {
            let added_effect = candidate_effects
                .difference(&base_effects)
                .next()
                .cloned()
                .unwrap_or_else(|| "capability authority".to_owned());
            let location = added_effect_location(&candidate, &added_effect);
            return ChangeResponse::Rejected(self.failure(
                &request,
                "capabilities",
                "AIL.CAPABILITY.EFFECT_GROWTH",
                &location,
                [("effects", join_set(&base_effects).as_str())],
                [("added_effect", added_effect.as_str())],
                "compare-effects-and-authority",
            ));
        }

        let behavior = match validate_public_behavior(&CandidateRevision { stored: &candidate }) {
            Ok(summary) => summary,
            Err(failure) => {
                return ChangeResponse::Rejected(self.failure(
                    &request,
                    "public-behavior",
                    "AIL.VALIDATION.BEHAVIOR_MISMATCH",
                    &failure.location,
                    [("public_behavior", "accepted fixture corpus")],
                    [("failing_case", failure.failing_case.as_str())],
                    "validate-public-behavior",
                ));
            }
        };

        let edits = canonical_edits(base, &candidate);
        let identity_map = build_identity_map(base, &candidate);
        let persistent_identities = persistent_identity_changes(base, &candidate);
        let semantic_diff = semantic_diff(base, &candidate, &impact_report.change);
        let revision = candidate.revision.clone();
        let completion = CompletionEvidence {
            base_revision_id: request.base_revision_id.clone(),
            revision_id: child_revision_id.clone(),
            impact_report: recomputed.clone(),
            edits: edits.clone(),
            identity_map,
            persistent_identities,
            semantic_diff,
            validation: ValidationSummary {
                revision_id: child_revision_id.clone(),
                parse: "ok",
                types: "ok",
                capabilities: "ok",
                impact: "complete",
                public_behavior: behavior,
            },
            analyzed_paths: recomputed.analyzed_paths.clone(),
            unchecked: recomputed.unchecked.clone(),
        };
        self.revisions.insert(child_revision_id.clone(), candidate);
        self.current_revision_id = child_revision_id;
        ChangeResponse::Committed(Box::new(ChangeSuccess {
            status: "committed",
            base_revision_id: request.base_revision_id,
            revision,
            completion,
        }))
    }

    fn next_revision_id(&self, base_revision_id: &str) -> String {
        let split = base_revision_id
            .char_indices()
            .rev()
            .find(|(_, ch)| !ch.is_ascii_digit())
            .map_or(0, |(index, ch)| index + ch.len_utf8());
        let prefix = &base_revision_id[..split];
        let number = base_revision_id[split..].parse::<u64>().unwrap_or(0);
        let mut next = number.saturating_add(1);
        loop {
            let revision_id = format!("{prefix}{next}");
            if !self.revisions.contains_key(&revision_id) {
                return revision_id;
            }
            next = next.saturating_add(1);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn failure<const E: usize, const A: usize>(
        &self,
        request: &CandidateChangeRequest,
        phase: &'static str,
        code: &'static str,
        location: &str,
        expected: [(&str, &str); E],
        actual: [(&str, &str); A],
        step: &str,
    ) -> Box<ChangeFailure> {
        let primary_handle = SemanticHandle {
            revision_id: request.base_revision_id.clone(),
            kind: HandleKind::Symbol,
            local_id: location.to_owned(),
        };
        Box::new(ChangeFailure {
            status: "rejected",
            base_revision_id: request.base_revision_id.clone(),
            current_revision_id: self.current_revision_id.clone(),
            phase,
            diagnostic: StructuredDiagnostic {
                code,
                revision_id: request.base_revision_id.clone(),
                category: phase,
                primary_handle: primary_handle.clone(),
                primary_span: Span::empty(0),
                expected: diagnostic_fields(expected),
                actual: diagnostic_fields(actual),
                related_handles: Vec::new(),
                causal_chain: vec![CausalStep {
                    step: step.to_owned(),
                    handle: primary_handle,
                }],
            },
            edits: Vec::new(),
        })
    }
}

fn diagnostic_fields<const N: usize>(
    fields: [(&str, &str); N],
) -> BTreeMap<String, DiagnosticValue> {
    fields
        .into_iter()
        .map(|(key, value)| (key.to_owned(), DiagnosticValue::Text(value.to_owned())))
        .collect()
}

fn impact_id(entry: &ImpactEntry) -> String {
    match entry.role.as_str() {
        "request-schema" => "request-contract",
        "stored-schema" => "stored-contract",
        "closed-priority-schema" => "priority-contract",
        "v1-request-adapter" => "request-adapter",
        "v1-stored-adapter" => "stored-adapter",
        "handler" => "handler-construction",
        "store-capability" => "store-contract",
        "persisted-encoder" => "stored-encoder",
        "v1-response-projection" => "v1-projection",
        "v2-response-projection" => "v2-projection",
        "v2-request-fixture" => "v2-fixture",
        "completion-evidence" => "completion-artifact",
        role => role,
    }
    .to_owned()
}

fn canonical_edits(base: &StoredSourceSet, candidate: &StoredSourceSet) -> Vec<CanonicalEdit> {
    base.sources
        .iter()
        .zip(&candidate.sources)
        .filter(|(before, after)| before.source != after.source)
        .map(|(before, after)| CanonicalEdit {
            path: before.path.clone(),
            span: Span::new(0, before.source.len()),
            replacement: after.source.clone(),
        })
        .collect()
}

fn top_identities(stored: &StoredSourceSet) -> BTreeSet<String> {
    stored
        .identities
        .iter()
        .filter(|identity| identity.parent_identity.is_none())
        .map(|identity| identity.identity.clone())
        .collect()
}

fn persistent_identity_changes(
    base: &StoredSourceSet,
    candidate: &StoredSourceSet,
) -> PersistentIdentityChanges {
    let before = top_identities(base);
    let after = top_identities(candidate);
    PersistentIdentityChanges {
        preserved: before.intersection(&after).cloned().collect(),
        added: after.difference(&before).cloned().collect(),
        retired: before.difference(&after).cloned().collect(),
    }
}

fn normalized_types(stored: &StoredSourceSet) -> BTreeMap<String, String> {
    stored
        .unit
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Record(record) => record
                .identity
                .as_ref()
                .map(|identity| (record.name.clone(), identity.clone())),
            Declaration::Variant(variant) => variant
                .identity
                .as_ref()
                .map(|identity| (variant.name.clone(), identity.clone())),
            Declaration::Function(_) => None,
        })
        .collect()
}

fn schema_shapes(stored: &StoredSourceSet) -> BTreeMap<String, (String, String)> {
    let mut shapes = BTreeMap::new();
    for declaration in &stored.unit.declarations {
        match declaration {
            Declaration::Record(record) => {
                let Some(identity) = &record.identity else {
                    continue;
                };
                let members = record
                    .fields
                    .iter()
                    .map(|field| field.identity.as_deref().unwrap_or("").to_owned())
                    .collect::<Vec<_>>()
                    .join("|");
                shapes.insert(
                    identity.clone(),
                    (format!("record:{members}"), record.name.clone()),
                );
            }
            Declaration::Variant(variant) => {
                let Some(identity) = &variant.identity else {
                    continue;
                };
                let members = variant
                    .cases
                    .iter()
                    .map(|case| {
                        format!(
                            "{}:{}",
                            case.identity.as_deref().unwrap_or(""),
                            if case.payload.is_some() {
                                "payload"
                            } else {
                                "unit"
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("|");
                shapes.insert(
                    identity.clone(),
                    (format!("variant:{members}"), variant.name.clone()),
                );
            }
            Declaration::Function(_) => {}
        }
    }
    shapes
}

fn incompatible_identity(
    base: &StoredSourceSet,
    candidate: &StoredSourceSet,
) -> Option<(String, String)> {
    let before = schema_shapes(base);
    let after = schema_shapes(candidate);
    before.iter().find_map(|(identity, (before_shape, _))| {
        let (after_shape, display_name) = after.get(identity)?;
        (before_shape != after_shape).then(|| {
            let persistent = candidate
                .identities
                .iter()
                .find(|candidate| candidate.identity == *identity)?;
            let path = persistent.handle.local_id.split('#').next().unwrap_or("");
            Some((identity.clone(), format!("{path}#{display_name}")))
        })?
    })
}

fn declared_effects(stored: &StoredSourceSet) -> BTreeSet<String> {
    stored
        .unit
        .declarations
        .iter()
        .filter_map(|declaration| {
            let Declaration::Function(function) = declaration else {
                return None;
            };
            Some(
                function
                    .effects
                    .iter()
                    .map(|effect| format!("{}.{}", effect.receiver, effect.operation)),
            )
        })
        .flatten()
        .collect()
}

fn declared_capabilities(stored: &StoredSourceSet) -> BTreeSet<String> {
    stored
        .unit
        .declarations
        .iter()
        .filter_map(|declaration| {
            let Declaration::Function(function) = declaration else {
                return None;
            };
            Some(function.parameters.iter().filter_map(|parameter| {
                let ParameterType::Capability(interface) = &parameter.ty else {
                    return None;
                };
                Some(interface.clone())
            }))
        })
        .flatten()
        .collect()
}

fn added_effect_location(stored: &StoredSourceSet, effect: &str) -> String {
    stored
        .unit
        .declarations
        .iter()
        .find_map(|declaration| {
            let Declaration::Function(function) = declaration else {
                return None;
            };
            function
                .effects
                .iter()
                .any(|candidate| {
                    format!("{}.{}", candidate.receiver, candidate.operation) == effect
                })
                .then(|| {
                    format!(
                        "{}#{}",
                        source_path_for_function(&stored.sources, &function.name),
                        source_name(&function.name)
                    )
                })
        })
        .unwrap_or_else(|| "workspace:capabilities".to_owned())
}

fn join_set(values: &BTreeSet<String>) -> String {
    values.iter().cloned().collect::<Vec<_>>().join(",")
}

fn indexed_handles(stored: &StoredSourceSet) -> BTreeMap<SemanticHandle, String> {
    let mut indexed = BTreeMap::new();
    for identity in &stored.identities {
        indexed.insert(
            identity.handle.clone(),
            format!("identity:{}", identity.identity),
        );
    }
    let mut repeated = BTreeMap::<String, usize>::new();
    for edge in &stored.graph {
        let root = format!(
            "edge:{}:{}:{}:{}",
            edge.site.path,
            edge.kind,
            normalize_endpoint(&edge.source),
            normalize_endpoint(&edge.target)
        );
        let ordinal = repeated.entry(root.clone()).or_default();
        indexed.insert(edge.site.handle.clone(), format!("{root}:{}", *ordinal));
        *ordinal += 1;
    }
    indexed
}

fn normalize_endpoint(endpoint: &str) -> String {
    let Some(value) = endpoint.strip_prefix("handle:") else {
        return endpoint.to_owned();
    };
    let mut parts = value.rsplitn(3, ':');
    let end = parts.next();
    let start = parts.next();
    let prefix = parts.next();
    if end.is_some_and(|part| part.chars().all(|ch| ch.is_ascii_digit()))
        && start.is_some_and(|part| part.chars().all(|ch| ch.is_ascii_digit()))
    {
        format!("handle:{}", prefix.unwrap_or(value))
    } else {
        endpoint.to_owned()
    }
}

fn handle_source<'a>(stored: &'a StoredSourceSet, semantic: &SemanticHandle) -> Option<&'a str> {
    let path = semantic.local_id.split('#').next()?;
    stored
        .sources
        .iter()
        .find(|source| source.path == path)
        .map(|source| source.source.as_str())
}

fn handle_span(semantic: &SemanticHandle) -> Option<Span> {
    let mut parts = semantic.local_id.rsplitn(3, ':');
    let end = parts.next()?.parse().ok()?;
    let start = parts.next()?.parse().ok()?;
    Some(Span::new(start, end))
}

fn equivalent_bytes(
    base: &StoredSourceSet,
    old: &SemanticHandle,
    candidate: &StoredSourceSet,
    new: &SemanticHandle,
) -> bool {
    let Some(old_source) = handle_source(base, old) else {
        return false;
    };
    let Some(new_source) = handle_source(candidate, new) else {
        return false;
    };
    let Some(old_span) = handle_span(old) else {
        return false;
    };
    let Some(new_span) = handle_span(new) else {
        return false;
    };
    old_source.get(old_span.start..old_span.end) == new_source.get(new_span.start..new_span.end)
}

fn build_identity_map(base: &StoredSourceSet, candidate: &StoredSourceSet) -> IdentityMap {
    let before = indexed_handles(base);
    let after = indexed_handles(candidate);
    let after_by_key = after
        .iter()
        .map(|(handle, key)| (key.as_str(), handle))
        .collect::<BTreeMap<_, _>>();
    let entries = before
        .iter()
        .map(|(old_handle, key)| {
            let new_handle = after_by_key
                .get(key.as_str())
                .map(|handle| (*handle).clone());
            let classification = match &new_handle {
                Some(new_handle) if equivalent_bytes(base, old_handle, candidate, new_handle) => {
                    IdentityClassification::Surviving
                }
                Some(_) => IdentityClassification::Replaced,
                None => IdentityClassification::Removed,
            };
            IdentityMapEntry {
                old_handle: old_handle.clone(),
                classification,
                new_handle,
            }
        })
        .collect();
    let before_keys = before.values().cloned().collect::<BTreeSet<_>>();
    let new_handles = after
        .iter()
        .filter(|(_, key)| !before_keys.contains(*key))
        .map(|(handle, _)| handle.clone())
        .collect();
    IdentityMap {
        from_revision_id: base.revision.revision_id.clone(),
        to_revision_id: candidate.revision.revision_id.clone(),
        entries,
        new_handles,
    }
}

fn semantic_diff(
    base: &StoredSourceSet,
    candidate: &StoredSourceSet,
    change: &ProposedSchemaChange,
) -> SemanticDiff {
    let before_identities = top_identities(base);
    let after_identities = top_identities(candidate);
    let added = after_identities
        .difference(&before_identities)
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_type_identity =
        super::persistent_type_identity(&change.subject_identity, &change.member_type);
    let type_identity = candidate
        .identities
        .iter()
        .find(|identity| {
            identity.parent_identity.is_none() && identity.identity == expected_type_identity
        })
        .map(|identity| identity.identity.as_str())
        .or_else(|| identity_for_display(candidate, &change.member_type))
        .expect("validated schema candidate resolves its required member type");
    let mut changes = Vec::new();
    if added.contains(type_identity) {
        changes.push(SemanticChange {
            kind: "schema-added".to_owned(),
            identity: type_identity.to_owned(),
            before: None,
            after: schema_description(candidate, type_identity),
        });
    }
    if let Some(option_identity) = option_identity(candidate, type_identity, &added) {
        changes.push(SemanticChange {
            kind: "schema-added".to_owned(),
            identity: option_identity.to_owned(),
            before: None,
            after: schema_description(candidate, option_identity),
        });
    }
    changes.push(SemanticChange {
        kind: "schema-successor".to_owned(),
        identity: change.successor_identity.clone(),
        before: Some(change.subject_identity.clone()),
        after: Some(format!(
            "required {}:{type_identity}",
            change.member_identity
        )),
    });
    if let Some((before, after)) = stored_successor(base, candidate, &change.member_display_name) {
        changes.push(SemanticChange {
            kind: "schema-successor".to_owned(),
            identity: after,
            before: Some(before),
            after: Some(format!(
                "required {}:{type_identity}",
                change.member_identity
            )),
        });
    }
    changes.extend(operation_changes(
        base,
        candidate,
        &change.member_display_name,
    ));
    let before_effects = declared_effects(base).into_iter().collect::<Vec<_>>();
    let after_effects = declared_effects(candidate).into_iter().collect::<Vec<_>>();
    let before_capabilities = declared_capabilities(base).into_iter().collect::<Vec<_>>();
    let after_capabilities = declared_capabilities(candidate)
        .into_iter()
        .collect::<Vec<_>>();
    SemanticDiff {
        from_revision_id: base.revision.revision_id.clone(),
        to_revision_id: candidate.revision.revision_id.clone(),
        changes,
        effect_summary: ChangeEffectSummary {
            ordering: if before_effects == after_effects {
                "unchanged"
            } else {
                "changed"
            },
            before: before_effects,
            after: after_effects,
        },
        capability_summary: ChangeCapabilitySummary {
            authority: if before_capabilities == after_capabilities {
                "unchanged"
            } else {
                "changed"
            },
            before: before_capabilities,
            after: after_capabilities,
        },
    }
}

fn operation_changes(
    base: &StoredSourceSet,
    candidate: &StoredSourceSet,
    member: &str,
) -> Vec<SemanticChange> {
    let base_functions = function_names(base);
    let mut changes = function_names(candidate)
        .difference(&base_functions)
        .filter(|function| source_name(function).starts_with("adapt_"))
        .map(|function| SemanticChange {
            kind: "adapter-added".to_owned(),
            identity: function.clone(),
            before: None,
            after: Some(format!("{member}=normal")),
        })
        .collect::<Vec<_>>();
    let projection_identity = candidate.unit.declarations.iter().find_map(|declaration| {
        let Declaration::Function(function) = declaration else {
            return None;
        };
        (!base_functions.contains(&function.name)
            && source_name(&function.name).starts_with("project_"))
        .then(|| {
            function
                .result_type
                .as_named()
                .and_then(|name| identity_for_display(candidate, name))
        })
        .flatten()
    });
    if let Some(identity) = projection_identity {
        changes.push(SemanticChange {
            kind: "projection-added".to_owned(),
            identity: identity.to_owned(),
            before: None,
            after: Some(format!("{member} preserved")),
        });
    }
    changes
}

fn identity_for_display<'a>(stored: &'a StoredSourceSet, display: &str) -> Option<&'a str> {
    let exact = stored.unit.declarations.iter().find_map(|declaration| {
        let (name, identity) = match declaration {
            Declaration::Record(record) => (record.name.as_str(), record.identity.as_deref()),
            Declaration::Variant(variant) => (variant.name.as_str(), variant.identity.as_deref()),
            Declaration::Function(_) => return None,
        };
        (name == display).then_some(identity).flatten()
    });
    if exact.is_some() {
        return exact;
    }
    let mut matches = stored.unit.declarations.iter().filter_map(|declaration| {
        let (name, identity) = match declaration {
            Declaration::Record(record) => (record.name.as_str(), record.identity.as_deref()),
            Declaration::Variant(variant) => (variant.name.as_str(), variant.identity.as_deref()),
            Declaration::Function(_) => return None,
        };
        (source_name(name) == display).then_some(identity).flatten()
    });
    let identity = matches.next()?;
    matches.next().is_none().then_some(identity)
}

fn schema_description(stored: &StoredSourceSet, identity: &str) -> Option<String> {
    let types = normalized_types(stored);
    stored.unit.declarations.iter().find_map(|declaration| {
        let Declaration::Variant(variant) = declaration else {
            return None;
        };
        (variant.identity.as_deref() == Some(identity)).then(|| {
            variant
                .cases
                .iter()
                .map(|case| {
                    case.payload.as_ref().map_or_else(
                        || case.name.clone(),
                        |payload| {
                            let rendered = payload.as_named().map_or_else(
                                || payload.to_string(),
                                |name| {
                                    types
                                        .get(name)
                                        .map_or_else(|| name.to_owned(), Clone::clone)
                                },
                            );
                            format!("{}({})", case.name, rendered)
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join("|")
        })
    })
}

fn option_identity<'a>(
    candidate: &'a StoredSourceSet,
    payload_identity: &str,
    added: &BTreeSet<String>,
) -> Option<&'a str> {
    let types = normalized_types(candidate);
    candidate.unit.declarations.iter().find_map(|declaration| {
        let Declaration::Variant(variant) = declaration else {
            return None;
        };
        let identity = variant.identity.as_deref()?;
        let has_payload = variant.cases.iter().any(|case| {
            case.payload
                .as_ref()
                .and_then(crate::TypeRef::as_named)
                .is_some_and(|payload| {
                    types
                        .get(payload)
                        .is_some_and(|identity| identity == payload_identity)
                })
        });
        (added.contains(identity)
            && has_payload
            && variant.cases.iter().any(|case| case.payload.is_none()))
        .then_some(identity)
    })
}

fn function_names(stored: &StoredSourceSet) -> BTreeSet<String> {
    stored
        .unit
        .declarations
        .iter()
        .filter_map(|declaration| {
            let Declaration::Function(function) = declaration else {
                return None;
            };
            Some(function.name.clone())
        })
        .collect()
}

fn stored_successor(
    base: &StoredSourceSet,
    candidate: &StoredSourceSet,
    member: &str,
) -> Option<(String, String)> {
    let before = capability_record_identity(base)?;
    let after = capability_record_identity(candidate)?;
    let has_member = candidate.identities.iter().any(|identity| {
        identity.parent_identity.as_deref() == Some(after.as_str())
            && identity.display_name == member
    });
    (before != after && has_member).then_some((before, after))
}

fn capability_record_identity(stored: &StoredSourceSet) -> Option<String> {
    stored.unit.declarations.iter().find_map(|declaration| {
        let Declaration::Function(function) = declaration else {
            return None;
        };
        if !function
            .parameters
            .iter()
            .any(|parameter| matches!(parameter.ty, ParameterType::Capability(_)))
        {
            return None;
        }
        let name = super::capability_argument_record(function)?;
        identity_for_display(stored, &name).map(str::to_owned)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityEnvironment, EvolutionCoverage};

    fn stored(revision: &str, domain: &str) -> StoredSourceSet {
        StoredSourceSet::build(
            "transaction-qualification",
            revision,
            None,
            vec![
                EvolutionSource::new("domain.ail", domain),
                EvolutionSource::new(
                    "other.ail",
                    concat!(
                        "module other;\n",
                        "record Priority identity \"other.priority.v1\" {\n",
                        "  label identity \"label\": Text;\n",
                        "}\n",
                    ),
                ),
            ],
            &CapabilityEnvironment::new(),
            EvolutionCoverage {
                declared_complete: true,
                ..EvolutionCoverage::default()
            },
        )
        .expect("test source set is valid")
    }

    #[test]
    fn semantic_diff_uses_persistent_identity_when_display_names_collide() {
        let base = stored(
            "r1",
            concat!(
                "module domain;\n",
                "record Request identity \"job.request.v1\" {\n",
                "  name identity \"name\": Text;\n",
                "}\n",
            ),
        );
        let candidate = stored(
            "r2",
            concat!(
                "module domain;\n",
                "record Request identity \"job.request.v1\" {\n",
                "  name identity \"name\": Text;\n",
                "}\n\n",
                "record Priority identity \"job.priority.v1\" {\n",
                "  label identity \"label\": Text;\n",
                "}\n\n",
                "record RequestV2 identity \"job.request.v2\" {\n",
                "  name identity \"name\": Text;\n",
                "  priority identity \"priority\": Priority;\n",
                "}\n",
            ),
        );
        let diff = semantic_diff(
            &base,
            &candidate,
            &ProposedSchemaChange {
                kind: "add-required-field-with-version-successor".to_owned(),
                subject_identity: "job.request.v1".to_owned(),
                successor_identity: "job.request.v2".to_owned(),
                member_display_name: "priority".to_owned(),
                member_identity: "priority".to_owned(),
                member_type: "Priority".to_owned(),
            },
        );

        assert!(diff.changes.iter().any(|change| {
            change.kind == "schema-added" && change.identity == "job.priority.v1"
        }));
        assert!(diff.changes.iter().any(|change| {
            change.identity == "job.request.v2"
                && change.after.as_deref() == Some("required priority:job.priority.v1")
        }));
    }
}
