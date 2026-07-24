//! Revision-bound architectural snapshot derivation (M25).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::source_digest;

const LIMITS: [(&str, usize); 5] = [
    ("semantic_nodes", 512),
    ("typed_edges", 2_048),
    ("structured_bytes", 65_536),
    ("compact_bytes", 2_048),
    ("compact_lines", 12),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureSnapshotRequest {
    pub revision_id: String,
    pub analysis_scope: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureSnapshotInput {
    pub request: ArchitectureSnapshotRequest,
    pub active_exceptions: Vec<ArchitectureException>,
    pub active_policy_revision: String,
    pub active_baseline_revision: String,
}
const REQUIRED_GROUPS: [&str; 5] = [
    "contract",
    "transport",
    "domain",
    "persistence-adapter",
    "verification",
];
const EDGE_KINDS: [&str; 7] = [
    "calls",
    "type-use",
    "verifies",
    "capability-use",
    "state-read",
    "state-write",
    "delegates",
];
const EXCEPTION_RULES: [&str; 6] = [
    "M23-POL-GROUP-DEPENDENCY",
    "M23-POL-TRANSPORT-CAPABILITY",
    "M23-POL-TRANSPORT-STATE",
    "M23-POL-DISPATCH-NO-GROWTH",
    "M23-POL-NEW-UNIT",
    "M23-POL-NO-NEW-CYCLE",
];
const JSON_FIELD_ORDER: &[&str] = &[
    "id",
    "code",
    "classification",
    "disposition",
    "rule",
    "scope",
    "contributors",
    "facts",
    "base_cfc",
    "candidate_cfc",
    "base_context",
    "candidate_context",
    "actual",
    "reads",
    "writes",
    "allowed",
    "forbidden_group_edges",
    "source_group",
    "target_group",
    "source",
    "target",
    "kind",
    "cfc",
    "cfc_max",
    "context",
    "context_max",
    "components",
    "members",
    "required_groups",
    "analyzed_groups",
    "unchecked_boundaries",
    "required",
    "provided",
    "requested_changes",
    "authorization_id",
    "trusted_authorization_matched",
    "exhausted",
    "used",
    "limit",
    "exception_ids",
    "policy_revision",
    "expires_after_review_boundary",
    "status",
    "cases_passed",
    "cases_total",
    "exception",
];
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlFlowGraph {
    pub nodes: usize,
    pub edges: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureUnit {
    pub id: String,
    pub module: String,
    pub group: String,
    pub cfg: ControlFlowGraph,
    pub capabilities: Vec<String>,
    pub state_reads: Vec<String>,
    pub state_writes: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicySelector {
    pub scope_kind: String,
    pub scope_identity: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyValue {
    GroupDependencies(GroupDependencies),
    Governance(PolicyGovernance),
    Dispatch(DispatchBudget),
    NewUnit(NewUnitBudget),
    Strings(Vec<String>),
    Boolean(bool),
    Integer(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupDependencies {
    pub contract: Vec<String>,
    pub transport: Vec<String>,
    pub domain: Vec<String>,
    pub persistence_adapter: Vec<String>,
    pub verification: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyGovernance {
    pub policy_revision: String,
    pub baseline_revision: String,
    pub exceptions: Vec<ArchitectureException>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchBudget {
    pub control_flow_complexity: usize,
    pub minimal_context_node_count: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewUnitBudget {
    pub control_flow_complexity_max: usize,
    pub minimal_context_node_count_max: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitecturePolicy {
    pub id: String,
    pub selector: PolicySelector,
    pub classification: String,
    pub disposition: String,
    pub comparison: String,
    pub value: PolicyValue,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedDebt {
    pub rule: String,
    pub scope: String,
    pub baseline_revision: String,
    pub metrics: DispatchBudget,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitecturePolicyContext {
    pub revision: String,
    pub allowed_group_dependencies: GroupDependencies,
    pub transport_capabilities: Vec<String>,
    pub transport_state: Vec<String>,
    pub dispatch_no_growth: DispatchBudget,
    pub new_unit: NewUnitBudget,
    pub new_cycles: bool,
    pub coverage_required: bool,
    pub baseline_match: BaselineMatch,
}

/// Caller-supplied evidence from the fixed M26 six-case behavior oracle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorValidation {
    pub status: String,
    pub cases_passed: usize,
    pub cases_total: usize,
}

impl ArchitecturePolicyContext {
    fn policies(&self) -> Vec<ArchitecturePolicy> {
        vec![
            policy(
                "M23-POL-GROUP-DEPENDENCY",
                "architecture-group",
                "*",
                "violation",
                "subset-by-source-group",
                PolicyValue::GroupDependencies(self.allowed_group_dependencies.clone()),
            ),
            policy(
                "M23-POL-TRANSPORT-CAPABILITY",
                "architecture-group",
                "group:transport",
                "violation",
                "set-subset",
                PolicyValue::Strings(self.transport_capabilities.clone()),
            ),
            policy(
                "M23-POL-TRANSPORT-STATE",
                "architecture-group",
                "group:transport",
                "violation",
                "read-and-write-set-subset",
                PolicyValue::Strings(self.transport_state.clone()),
            ),
            policy(
                "M23-POL-DISPATCH-NO-GROWTH",
                "executable-unit",
                "transport:dispatch",
                "regression",
                "both-less-than-or-equal-baseline",
                PolicyValue::Dispatch(self.dispatch_no_growth.clone()),
            ),
            policy(
                "M23-POL-NEW-UNIT",
                "executable-unit",
                "new-non-contract-unit",
                "violation",
                "both-less-than-or-equal",
                PolicyValue::NewUnit(self.new_unit.clone()),
            ),
            policy(
                "M23-POL-NO-NEW-CYCLE",
                "dependency-component",
                "new-component",
                "violation",
                "new-component-member-count-less-than-or-equal",
                PolicyValue::Integer(usize::from(!self.new_cycles)),
            ),
            policy(
                "M23-POL-COVERAGE",
                "architecture-group",
                "*",
                "incomplete",
                "complete-for-policy-equals",
                PolicyValue::Boolean(self.coverage_required),
            ),
            policy(
                "M23-POL-GOVERNANCE",
                "governance",
                "trusted-input",
                "violation",
                "trusted-revisions-and-exact-exceptions-equal",
                PolicyValue::Governance(PolicyGovernance {
                    policy_revision: self.revision.clone(),
                    baseline_revision: self.baseline_match.baseline_revision.clone(),
                    exceptions: Vec::new(),
                }),
            ),
        ]
    }

    fn accepted_debt(&self) -> Vec<AcceptedDebt> {
        vec![AcceptedDebt {
            rule: "M23-POL-DISPATCH-NO-GROWTH".into(),
            scope: self.baseline_match.scope.clone(),
            baseline_revision: self.baseline_match.baseline_revision.clone(),
            metrics: self.baseline_match.metrics.clone(),
        }]
    }
}

fn policy(
    id: &str,
    scope_kind: &str,
    scope_identity: &str,
    classification: &str,
    comparison: &str,
    value: PolicyValue,
) -> ArchitecturePolicy {
    ArchitecturePolicy {
        id: id.into(),
        selector: PolicySelector {
            scope_kind: scope_kind.into(),
            scope_identity: scope_identity.into(),
        },
        classification: classification.into(),
        disposition: "deny".into(),
        comparison: comparison.into(),
        value,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureRevision {
    pub workspace_id: String,
    pub revision_id: String,
    pub semantic_model_version: String,
    pub units: Vec<ArchitectureUnit>,
    pub edges: Vec<ArchitectureEdge>,
    pub endpoint_groups: BTreeMap<String, String>,
    pub coverage: ArchitectureCoverage,
    pub policy: ArchitecturePolicyContext,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureRevisionError(pub String);

impl ArchitectureRevision {
    /// Establish the immutable, trusted boundary used by architecture queries.
    ///
    /// # Errors
    /// Returns an error when graph, unit, governance, or representation facts
    /// violate the M25 revision invariants.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: String,
        revision_id: String,
        semantic_model_version: String,
        units: Vec<ArchitectureUnit>,
        edges: Vec<ArchitectureEdge>,
        endpoint_groups: BTreeMap<String, String>,
        coverage: ArchitectureCoverage,
        policy: ArchitecturePolicyContext,
    ) -> Result<Self, ArchitectureRevisionError> {
        let revision = Self {
            workspace_id,
            revision_id,
            semantic_model_version,
            units,
            edges,
            endpoint_groups,
            coverage,
            policy,
        };
        revision.validate()?;
        Ok(revision)
    }

    #[allow(clippy::too_many_lines)]
    fn validate(&self) -> Result<(), ArchitectureRevisionError> {
        let mut ids = BTreeSet::new();
        let mut total_cfc = 0_usize;
        for unit in &self.units {
            if unit.id.is_empty()
                || unit.id.contains(['\n', '\r'])
                || unit.module.is_empty()
                || unit.group.is_empty()
            {
                return Err(ArchitectureRevisionError(
                    "invalid unit identity, module, or group".into(),
                ));
            }
            if !REQUIRED_GROUPS.contains(&unit.group.as_str()) {
                return Err(ArchitectureRevisionError(format!(
                    "unknown unit group {}",
                    unit.group
                )));
            }
            if !ids.insert(unit.id.as_str()) {
                return Err(ArchitectureRevisionError(format!(
                    "duplicate unit {}",
                    unit.id
                )));
            }
            let cfc = unit
                .cfg
                .edges
                .checked_add(2)
                .and_then(|edges| edges.checked_sub(unit.cfg.nodes));
            if unit.cfg.nodes == 0 || cfc.is_none_or(|value| value < 1) {
                return Err(ArchitectureRevisionError(format!(
                    "invalid CFG for {}",
                    unit.id
                )));
            }
            total_cfc = total_cfc
                .checked_add(cfc.expect("positive CFC was checked"))
                .ok_or_else(|| {
                    ArchitectureRevisionError("aggregate CFC is not representable".into())
                })?;
            for facts in [&unit.capabilities, &unit.state_reads, &unit.state_writes] {
                if !facts.windows(2).all(|w| w[0] < w[1]) {
                    return Err(ArchitectureRevisionError(format!(
                        "unsorted or duplicate facts for {}",
                        unit.id
                    )));
                }
            }
        }
        if !ids.contains("transport:dispatch") {
            return Err(ArchitectureRevisionError(
                "transport:dispatch unit is required".into(),
            ));
        }
        for (endpoint, group) in &self.endpoint_groups {
            if endpoint.is_empty()
                || endpoint.contains(['\n', '\r'])
                || self
                    .units
                    .iter()
                    .find(|unit| unit.id == *endpoint)
                    .is_some_and(|unit| unit.group != *group)
                || !REQUIRED_GROUPS.contains(&group.as_str())
            {
                return Err(ArchitectureRevisionError(format!(
                    "invalid endpoint group entry {endpoint}"
                )));
            }
        }
        if !self
            .coverage
            .analyzed_groups
            .iter()
            .all(|group| REQUIRED_GROUPS.contains(&group.as_str()))
            || self
                .coverage
                .analyzed_groups
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != self.coverage.analyzed_groups.len()
        {
            return Err(ArchitectureRevisionError(
                "analyzed coverage contains unknown or duplicate groups".into(),
            ));
        }
        let mut boundary_ids = BTreeSet::new();
        for value in &self.coverage.unchecked_boundaries {
            let Value::Object(boundary) = value else {
                return Err(ArchitectureRevisionError(
                    "unchecked boundaries must be objects".into(),
                ));
            };
            if boundary.len() != 2
                || !boundary.contains_key("id")
                || !boundary.contains_key("reason")
            {
                return Err(ArchitectureRevisionError(
                    "unchecked boundaries require exactly id and reason".into(),
                ));
            }
            let valid_string = |key: &str| {
                boundary
                    .get(key)
                    .and_then(Value::as_str)
                    .is_some_and(|text| !text.is_empty() && !text.contains(['\n', '\r']))
            };
            if !valid_string("id") || !valid_string("reason") {
                return Err(ArchitectureRevisionError(
                    "unchecked boundary id and reason must be nonempty single-line strings".into(),
                ));
            }
            let id = boundary["id"].as_str().expect("validated string");
            if !boundary_ids.insert(id) {
                return Err(ArchitectureRevisionError(format!(
                    "duplicate unchecked boundary {id}"
                )));
            }
        }
        let mut triples = BTreeSet::new();
        for edge in &self.edges {
            if !EDGE_KINDS.contains(&edge.kind.as_str()) {
                return Err(ArchitectureRevisionError(format!(
                    "unknown edge kind {}",
                    edge.kind
                )));
            }
            let target_known = ids.contains(edge.target.as_str())
                || self.endpoint_groups.contains_key(&edge.target);
            if !ids.contains(edge.source.as_str())
                || !target_known
                || edge.target.is_empty()
                || edge.target.contains(['\n', '\r'])
            {
                return Err(ArchitectureRevisionError(format!(
                    "invalid edge endpoint {} -> {}",
                    edge.source, edge.target
                )));
            }
            if !triples.insert((&edge.source, &edge.target, &edge.kind)) {
                return Err(ArchitectureRevisionError("duplicate edge triple".into()));
            }
            if (edge.kind == "capability-use" && !edge.target.starts_with("capability:"))
                || (matches!(edge.kind.as_str(), "state-read" | "state-write")
                    && !edge.target.starts_with("state:"))
            {
                return Err(ArchitectureRevisionError(format!(
                    "invalid {} target {}",
                    edge.kind, edge.target
                )));
            }
        }
        for unit in &self.units {
            let edge_set = |kind: &str, prefix: &str| -> Vec<String> {
                self.edges
                    .iter()
                    .filter(|e| e.source == unit.id && e.kind == kind)
                    .filter_map(|e| e.target.strip_prefix(prefix).map(str::to_owned))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            };
            let capabilities: Vec<_> = edge_set("capability-use", "capability:")
                .into_iter()
                .filter_map(|x| x.split('.').next().map(str::to_owned))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            if capabilities != unit.capabilities
                || edge_set("state-read", "state:") != unit.state_reads
                || edge_set("state-write", "state:") != unit.state_writes
            {
                return Err(ArchitectureRevisionError(format!(
                    "unit facts disagree with graph edges for {}",
                    unit.id
                )));
            }
        }
        if !self.policy.baseline_match.accepted_debt
            || self.policy.baseline_match.scope != "transport:dispatch"
            || self.policy.dispatch_no_growth != self.policy.baseline_match.metrics
        {
            return Err(ArchitectureRevisionError(
                "policy and accepted baseline disagree".into(),
            ));
        }
        if self.policy.new_cycles {
            return Err(ArchitectureRevisionError(
                "new dependency cycles cannot be enabled".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureCoverage {
    pub analyzed_groups: Vec<String>,
    pub unchecked_boundaries: Vec<Value>,
    pub complete_for_policy: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureException {
    pub id: String,
    pub rule: String,
    pub scope: String,
    pub contributors: Vec<String>,
    pub policy_revision: String,
    pub expires_after_review_boundary: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineMatch {
    pub baseline_revision: String,
    pub scope: String,
    pub metrics: DispatchBudget,
    pub accepted_debt: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisIdentity {
    pub workspace_id: String,
    pub revision_id: String,
    pub semantic_model_version: String,
    pub policy_revision: String,
    pub baseline_revision: String,
    pub analysis_scope: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CfcContributor {
    pub unit_id: String,
    pub value: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlFlowComplexity {
    pub maximum: usize,
    pub sum: usize,
    pub contributors: Vec<CfcContributor>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeMetrics {
    pub scope_kind: String,
    pub identity: String,
    pub unit_ids: Vec<String>,
    pub control_flow_complexity: ControlFlowComplexity,
    pub direct_dependency_set: Vec<String>,
    pub declared_capability_set: Vec<String>,
    pub state_read_set: Vec<String>,
    pub state_write_set: Vec<String>,
    pub dependency_component_size: usize,
    pub minimal_context_node_count: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetUse {
    pub semantic_nodes: usize,
    pub typed_edges: usize,
    pub structured_bytes: usize,
    pub compact_bytes: usize,
    pub compact_lines: usize,
    pub exhausted: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureSnapshot {
    pub format: String,
    pub analysis: AnalysisIdentity,
    pub scopes: Vec<ScopeMetrics>,
    pub coverage: ArchitectureCoverage,
    pub budgets: BudgetUse,
    pub active_policies: Vec<ArchitecturePolicy>,
    pub baseline_match: Option<BaselineMatch>,
    pub accepted_debt: Vec<AcceptedDebt>,
    pub exceptions: Vec<ArchitectureException>,
    pub findings: Vec<Value>,
    pub classification: String,
    pub compact: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureSnapshotResponse {
    pub status: String,
    pub snapshot: ArchitectureSnapshot,
    pub snapshot_digest: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureIncompleteFailure {
    pub status: String,
    pub analysis: AnalysisIdentity,
    pub coverage: ArchitectureCoverage,
    pub budgets: BudgetUse,
    pub diagnostics: Vec<Value>,
    pub edits: Vec<Value>,
    pub current_revision_id: String,
    pub published_child_revision_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ArchitectureSnapshotResult {
    Success(ArchitectureSnapshotResponse),
    Incomplete(ArchitectureIncompleteFailure),
}

/// A candidate-owned governance mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovernanceChange {
    pub kind: String,
    pub rule: String,
    pub scope: String,
    pub exception_ids: Vec<String>,
}
/// Trusted evidence authorizing exactly one governance mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovernanceAuthorization {
    pub authorization_id: String,
    pub candidate_revision_id: String,
    pub candidate_graph_digest: String,
    pub policy_revision: String,
    pub baseline_revision: String,
    pub kind: String,
    pub exception_ids: Vec<String>,
    pub rule: String,
    pub scope: String,
    pub review_boundary: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureRequest {
    pub base_revision_id: String,
    pub candidate_revision_id: String,
    pub analysis_scope: String,
    pub policy_revision: String,
    pub baseline_revision: String,
    pub review_boundary: String,
    pub requested_governance_changes: Vec<GovernanceChange>,
    pub authorization_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureEvaluationInput {
    pub request: ArchitectureRequest,
    pub governance_authorizations: Vec<GovernanceAuthorization>,
    pub active_exceptions: Vec<ArchitectureException>,
    pub active_policy_revision: String,
    pub active_baseline_revision: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureDelta {
    pub format: String,
    pub base_snapshot_digest: String,
    pub candidate_snapshot_digest: String,
    pub base_revision_id: String,
    pub candidate_revision_id: String,
    pub scope_changes: Vec<ScopeChange>,
    pub findings: Vec<Value>,
    pub classification: String,
    pub publication: String,
    pub commit: String,
    pub compact: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeChange {
    pub scope_kind: String,
    pub identity: String,
    pub base: Option<String>,
    pub candidate: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureCompletionEvidence {
    pub base_revision_id: String,
    pub revision_id: String,
    pub base_snapshot_digest: String,
    pub snapshot_digest: String,
    pub delta_digest: String,
    pub policy_revision: String,
    pub baseline_revision: String,
    pub coverage: ArchitectureCoverage,
    pub budgets: BudgetUse,
    pub behavior_validation: BehaviorValidation,
    pub commit: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureSuccess {
    pub status: String,
    pub snapshot: ArchitectureSnapshot,
    pub delta: ArchitectureDelta,
    pub completion: ArchitectureCompletionEvidence,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureFailure {
    pub status: String,
    pub base_revision_id: String,
    pub current_revision_id: String,
    pub snapshot: ArchitectureSnapshot,
    pub delta: ArchitectureDelta,
    pub diagnostics: Vec<Value>,
    pub edits: Vec<Value>,
    pub published_child_revision_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ArchitectureChangeResult {
    Success(Box<ArchitectureSuccess>),
    Failure(Box<ArchitectureFailure>),
    Incomplete(Box<ArchitectureIncompleteFailure>),
}

/// Atomic owner for immutable architecture revisions.
#[derive(Clone, Debug)]
pub struct ArchitectureWorkspace {
    current_revision_id: String,
    revisions: BTreeMap<String, ArchitectureRevision>,
}
impl ArchitectureWorkspace {
    #[must_use]
    pub fn new(base: ArchitectureRevision) -> Self {
        Self {
            current_revision_id: base.revision_id.clone(),
            revisions: BTreeMap::from([(base.revision_id.clone(), base)]),
        }
    }
    #[must_use]
    pub fn current_revision_id(&self) -> &str {
        &self.current_revision_id
    }
    #[must_use]
    pub fn retained_revision_ids(&self) -> Vec<&str> {
        self.revisions.keys().map(String::as_str).collect()
    }
    #[must_use]
    pub fn revision(&self, id: &str) -> Option<&ArchitectureRevision> {
        self.revisions.get(id)
    }
    /// Validates and atomically publishes an architecture candidate when accepted.
    ///
    /// # Errors
    /// Returns a request error when the base or candidate revision is invalid or stale,
    /// governance does not match trusted inputs, or analysis cannot evaluate the scope.
    pub fn validate_architecture_change<F>(
        &mut self,
        candidate: ArchitectureRevision,
        input: &ArchitectureEvaluationInput,
        validate_behavior: F,
    ) -> Result<ArchitectureChangeResult, ArchitectureRequestError>
    where
        F: FnOnce(&ArchitectureRevision) -> Result<BehaviorValidation, ArchitectureRequestError>,
    {
        let base = self
            .revisions
            .get(&input.request.base_revision_id)
            .ok_or_else(|| ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::StaleRevision,
                message: "base revision is not retained".into(),
            })?
            .clone();
        if self.current_revision_id != input.request.base_revision_id {
            return Err(ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::StaleRevision,
                message: "base revision is not current".into(),
            });
        }
        if self.revisions.contains_key(&candidate.revision_id) {
            return Err(ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::InvalidRevision,
                message: "candidate revision identity is already retained".into(),
            });
        }
        let result = evaluate_change(&base, &candidate, input, validate_behavior)?;
        if matches!(result, ArchitectureChangeResult::Success(_)) {
            self.current_revision_id.clone_from(&candidate.revision_id);
            self.revisions
                .insert(candidate.revision_id.clone(), candidate);
        }
        Ok(result)
    }
}

/// Validates an architecture candidate through the supplied workspace.
///
/// # Errors
/// Returns a request error when the base or candidate revision is invalid or stale,
/// governance does not match trusted inputs, or analysis cannot evaluate the scope.
pub fn validate_architecture_change<F>(
    workspace: &mut ArchitectureWorkspace,
    candidate: ArchitectureRevision,
    input: &ArchitectureEvaluationInput,
    validate_behavior: F,
) -> Result<ArchitectureChangeResult, ArchitectureRequestError>
where
    F: FnOnce(&ArchitectureRevision) -> Result<BehaviorValidation, ArchitectureRequestError>,
{
    workspace.validate_architecture_change(candidate, input, validate_behavior)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureRequestError {
    pub kind: ArchitectureRequestErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchitectureRequestErrorKind {
    InvalidRevision,
    StaleRevision,
    StaleGovernance,
    UnknownScope,
}

/// Derive a snapshot exclusively from the caller-supplied immutable revision facts.
///
/// # Errors
/// Returns a request error for an invalid revision, stale governance or
/// revision identity, or an unknown analysis scope.
#[allow(clippy::too_many_lines)]
pub fn architecture_snapshot(
    revision: &ArchitectureRevision,
    input: &ArchitectureSnapshotInput,
) -> Result<ArchitectureSnapshotResult, ArchitectureRequestError> {
    architecture_snapshot_impl(revision, input, true)
}

#[allow(clippy::too_many_lines)]
fn architecture_snapshot_impl(
    revision: &ArchitectureRevision,
    input: &ArchitectureSnapshotInput,
    enforce_budgets: bool,
) -> Result<ArchitectureSnapshotResult, ArchitectureRequestError> {
    revision
        .validate()
        .map_err(|error| ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: error.0,
        })?;
    if input.request.revision_id != revision.revision_id {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::StaleRevision,
            message: "request revision does not match stored revision".into(),
        });
    }
    if input.active_policy_revision != revision.policy.revision
        || input.active_baseline_revision != revision.policy.baseline_match.baseline_revision
    {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::StaleGovernance,
            message: "request governance does not match stored revision".into(),
        });
    }
    let analysis = AnalysisIdentity {
        workspace_id: revision.workspace_id.clone(),
        revision_id: input.request.revision_id.clone(),
        semantic_model_version: revision.semantic_model_version.clone(),
        policy_revision: input.active_policy_revision.clone(),
        baseline_revision: input.active_baseline_revision.clone(),
        analysis_scope: input.request.analysis_scope.clone(),
    };
    let components = components(&revision.units, &revision.edges);
    let Some(owner) = revision
        .units
        .iter()
        .find(|u| u.id == input.request.analysis_scope)
    else {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::UnknownScope,
            message: "analysis scope does not exist".into(),
        });
    };
    let Some(component) = components.iter().find(|c| c.contains(&owner.id)) else {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: "analysis scope has no dependency component".into(),
        });
    };
    let specs = [
        (
            "executable-unit",
            owner.id.clone(),
            vec![owner.id.clone()],
            true,
        ),
        (
            "module",
            format!("module:{}", owner.module),
            revision
                .units
                .iter()
                .filter(|u| u.module == owner.module)
                .map(|u| u.id.clone())
                .collect(),
            false,
        ),
        (
            "dependency-component",
            component_id(component),
            component.clone(),
            false,
        ),
        (
            "architecture-group",
            format!("group:{}", owner.group),
            revision
                .units
                .iter()
                .filter(|u| u.group == owner.group)
                .map(|u| u.id.clone())
                .collect(),
            false,
        ),
    ];
    let scopes = specs
        .into_iter()
        .map(|(k, id, m, unit)| metric(k, id, m, unit, revision, &components))
        .collect();
    let endpoints: BTreeSet<_> = revision
        .edges
        .iter()
        .flat_map(|e| [&e.source, &e.target])
        .chain(revision.units.iter().map(|u| &u.id))
        .collect();
    let mut budgets = BudgetUse {
        semantic_nodes: endpoints.len(),
        typed_edges: revision.edges.len(),
        structured_bytes: 0,
        compact_bytes: 0,
        compact_lines: 0,
        exhausted: None,
    };
    let finding_prefix = finding_prefix(&revision.revision_id);
    let coverage_diagnostic = coverage_diagnostic(&revision.revision_id, &revision.coverage);
    let coverage_complete = revision.coverage.complete_for_policy
        && revision
            .coverage
            .analyzed_groups
            .iter()
            .map(String::as_str)
            .eq(REQUIRED_GROUPS);
    let coverage = ArchitectureCoverage {
        complete_for_policy: coverage_complete,
        ..revision.coverage.clone()
    };
    let findings = if coverage_complete {
        vec![]
    } else {
        vec![coverage_diagnostic.clone()]
    };
    let mut snapshot = ArchitectureSnapshot {
        format: "ail.architecture.snapshot.v1".into(),
        analysis: analysis.clone(),
        scopes,
        coverage: coverage.clone(),
        budgets: budgets.clone(),
        active_policies: revision.policy.policies(),
        baseline_match: Some(revision.policy.baseline_match.clone()),
        accepted_debt: revision.policy.accepted_debt(),
        exceptions: input.active_exceptions.clone(),
        findings,
        classification: if coverage_complete {
            "accepted"
        } else {
            "incomplete"
        }
        .into(),
        compact: String::new(),
    };
    snapshot.compact = render_compact(&snapshot);
    budgets.compact_bytes = snapshot.compact.len();
    budgets.compact_lines = snapshot.compact.matches('\n').count();
    measure(&mut snapshot, &mut budgets);
    snapshot.budgets = budgets.clone();
    if !coverage_complete || (enforce_budgets && budgets.exhausted.is_some()) {
        let diagnostic = if let Some(field) = &budgets.exhausted {
            let used = budget_value(&budgets, field);
            let limit = LIMITS
                .iter()
                .find(|candidate| candidate.0 == field)
                .map_or(0, |candidate| candidate.1);
            serde_json::json!({"id":format!("{finding_prefix}:analysis-budget"),"code":"AIL.ARCH.ANALYSIS_INCOMPLETE","classification":"incomplete","disposition":"deny","rule":"M24-ANALYSIS-BUDGET","scope":"workspace","contributors":[],"facts":{"exhausted":field,"used":used,"limit":limit},"exception":null})
        } else {
            coverage_diagnostic
        };
        return bounded_incomplete(
            analysis,
            coverage,
            budgets,
            diagnostic,
            &revision.revision_id,
        );
    }
    let bytes = canonical(&snapshot);
    let canonical_snapshot =
        String::from_utf8(bytes).map_err(|error| ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: error.to_string(),
        })?;
    let response = ArchitectureSnapshotResponse {
        status: "success".into(),
        snapshot,
        snapshot_digest: source_digest(&canonical_snapshot),
    };
    if enforce_budgets && canonical(&response).len() > LIMITS[2].1 {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: "success response exceeds its fixed bound".into(),
        });
    }
    Ok(ArchitectureSnapshotResult::Success(response))
}

#[allow(clippy::too_many_lines)]
fn evaluate_change<F>(
    base: &ArchitectureRevision,
    candidate: &ArchitectureRevision,
    input: &ArchitectureEvaluationInput,
    validate_behavior: F,
) -> Result<ArchitectureChangeResult, ArchitectureRequestError>
where
    F: FnOnce(&ArchitectureRevision) -> Result<BehaviorValidation, ArchitectureRequestError>,
{
    base.validate().map_err(|e| ArchitectureRequestError {
        kind: ArchitectureRequestErrorKind::InvalidRevision,
        message: e.0,
    })?;
    candidate.validate().map_err(|e| ArchitectureRequestError {
        kind: ArchitectureRequestErrorKind::InvalidRevision,
        message: e.0,
    })?;
    let behavior_validation = validate_behavior(candidate)?;
    if behavior_validation.status != "passed"
        || behavior_validation.cases_passed != 6
        || behavior_validation.cases_total != 6
    {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: "M26 behavior validation must report passed 6/6".into(),
        });
    }
    if base.workspace_id != candidate.workspace_id
        || base.semantic_model_version != candidate.semantic_model_version
        || input.request.candidate_revision_id != candidate.revision_id
        || candidate.policy != base.policy
    {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::StaleRevision,
            message: "candidate identity is incompatible".into(),
        });
    }
    if input.active_policy_revision != base.policy.revision
        || input.active_baseline_revision != base.policy.baseline_match.baseline_revision
        || input.request.policy_revision != input.active_policy_revision
    {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::StaleGovernance,
            message: "trusted governance context mismatch".into(),
        });
    }
    let base_input = ArchitectureSnapshotInput {
        request: ArchitectureSnapshotRequest {
            revision_id: base.revision_id.clone(),
            analysis_scope: "transport:dispatch".into(),
        },
        active_exceptions: vec![],
        active_policy_revision: input.active_policy_revision.clone(),
        active_baseline_revision: input.active_baseline_revision.clone(),
    };
    let ArchitectureSnapshotResult::Success(base_result) =
        architecture_snapshot(base, &base_input)?
    else {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: "base architecture is incomplete".into(),
        });
    };
    let mut safe = candidate.clone();
    safe.coverage.complete_for_policy = true;
    safe.coverage.analyzed_groups = REQUIRED_GROUPS.iter().map(|x| (*x).into()).collect();
    let candidate_input = ArchitectureSnapshotInput {
        request: ArchitectureSnapshotRequest {
            revision_id: candidate.revision_id.clone(),
            analysis_scope: input.request.analysis_scope.clone(),
        },
        active_exceptions: input.active_exceptions.clone(),
        active_policy_revision: input.active_policy_revision.clone(),
        active_baseline_revision: input.active_baseline_revision.clone(),
    };
    let initial = architecture_snapshot_impl(&safe, &candidate_input, false)?;
    let ArchitectureSnapshotResult::Success(success) = initial else {
        let ArchitectureSnapshotResult::Incomplete(mut failure) = initial else {
            unreachable!()
        };
        failure.current_revision_id.clone_from(&base.revision_id);
        return Ok(ArchitectureChangeResult::Incomplete(Box::new(failure)));
    };
    let mut snapshot = success.snapshot;
    snapshot
        .analysis
        .baseline_revision
        .clone_from(&input.request.baseline_revision);
    snapshot.coverage = candidate.coverage.clone();
    let mut findings = architecture_findings(base, candidate, input);
    for finding in &mut findings {
        let contributors = finding["contributors"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        if let Some(exception) = input.active_exceptions.iter().find(|e| {
            finding["rule"]
                .as_str()
                .is_some_and(|rule| EXCEPTION_RULES.contains(&rule))
                && e.rule == finding["rule"]
                && e.scope == finding["scope"]
                && e.contributors == contributors
                && e.policy_revision == input.active_policy_revision
                && e.expires_after_review_boundary >= input.request.review_boundary
        }) {
            finding["disposition"] = Value::String("record".into());
            finding["exception"] = ordered_value(&exception.ordered_json());
        }
    }
    findings.sort_by_key(finding_key);
    let extra = findings
        .iter()
        .filter_map(|f| f["scope"].as_str())
        .collect::<BTreeSet<_>>();
    let comps = components(&candidate.units, &candidate.edges);
    for scope in extra {
        if scope.starts_with("group:") && !snapshot.scopes.iter().any(|s| s.identity == scope) {
            let group = scope.trim_start_matches("group:");
            let members = candidate
                .units
                .iter()
                .filter(|u| u.group == group)
                .map(|u| u.id.clone())
                .collect();
            snapshot.scopes.push(metric(
                "architecture-group",
                scope.into(),
                members,
                false,
                candidate,
                &comps,
            ));
        } else if candidate.units.iter().any(|u| u.id == scope)
            && !snapshot.scopes.iter().any(|s| s.identity == scope)
        {
            snapshot.scopes.push(metric(
                "executable-unit",
                scope.into(),
                vec![scope.into()],
                true,
                candidate,
                &comps,
            ));
        }
    }
    snapshot
        .scopes
        .sort_by_key(|s| (scope_rank(&s.scope_kind), s.identity.clone()));
    snapshot.findings = findings;
    let coverage_complete = candidate.coverage.complete_for_policy
        && candidate
            .coverage
            .analyzed_groups
            .iter()
            .map(String::as_str)
            .eq(REQUIRED_GROUPS);
    snapshot.coverage.complete_for_policy = coverage_complete;
    snapshot.baseline_match = (input.request.baseline_revision == input.active_baseline_revision)
        .then(|| base.policy.baseline_match.clone());
    let denied = snapshot.findings.iter().any(|f| f["disposition"] == "deny");
    snapshot.classification = if !coverage_complete
        || snapshot.findings.iter().any(|finding| {
            finding["classification"] == "incomplete" && finding["disposition"] == "deny"
        }) {
        "incomplete"
    } else if denied {
        "rejected"
    } else {
        "accepted"
    }
    .into();
    snapshot.compact = render_compact(&snapshot);
    let mut budgets = snapshot.budgets.clone();
    budgets.compact_bytes = snapshot.compact.len();
    budgets.compact_lines = snapshot.compact.matches('\n').count();
    measure(&mut snapshot, &mut budgets);
    snapshot.budgets = budgets.clone();
    if !coverage_complete || budgets.exhausted.is_some() {
        let diagnostic = if let Some(field) = &budgets.exhausted {
            serde_json::json!({"id":format!("{}:analysis-budget",finding_prefix(&candidate.revision_id)),"code":"AIL.ARCH.ANALYSIS_INCOMPLETE","classification":"incomplete","disposition":"deny","rule":"M24-ANALYSIS-BUDGET","scope":"workspace","contributors":[],"facts":{"exhausted":field,"used":budget_value(&budgets,field),"limit":LIMITS.iter().find(|x|x.0==field).unwrap().1},"exception":null})
        } else {
            snapshot
                .findings
                .iter()
                .find(|f| f["code"] == "AIL.ARCH.COVERAGE_INCOMPLETE")
                .cloned()
                .unwrap()
        };
        let result =
            ArchitectureChangeResult::Incomplete(Box::new(ArchitectureIncompleteFailure {
                status: "incomplete".into(),
                analysis: snapshot.analysis,
                coverage: snapshot.coverage,
                budgets,
                diagnostics: vec![diagnostic],
                edits: vec![],
                current_revision_id: base.revision_id.clone(),
                published_child_revision_id: None,
            }));
        return bounded_change_result(result);
    }
    let delta = architecture_delta(&base_result.snapshot, &snapshot);
    if denied {
        let result = ArchitectureChangeResult::Failure(Box::new(ArchitectureFailure {
            status: "failure".into(),
            base_revision_id: base.revision_id.clone(),
            current_revision_id: base.revision_id.clone(),
            diagnostics: snapshot.findings.clone(),
            snapshot,
            delta,
            edits: vec![],
            published_child_revision_id: None,
        }));
        return bounded_change_result(result);
    }
    let base_digest = digest(&base_result.snapshot);
    let snapshot_digest = digest(&snapshot);
    let delta_digest = digest(&delta);
    bounded_change_result(ArchitectureChangeResult::Success(Box::new(
        ArchitectureSuccess {
            status: "success".into(),
            completion: ArchitectureCompletionEvidence {
                base_revision_id: base.revision_id.clone(),
                revision_id: candidate.revision_id.clone(),
                base_snapshot_digest: base_digest,
                snapshot_digest,
                delta_digest,
                policy_revision: input.active_policy_revision.clone(),
                baseline_revision: input.active_baseline_revision.clone(),
                coverage: snapshot.coverage.clone(),
                budgets: snapshot.budgets.clone(),
                behavior_validation,
                commit: "committed".into(),
            },
            snapshot,
            delta,
        },
    )))
}

fn bounded_change_result(
    result: ArchitectureChangeResult,
) -> Result<ArchitectureChangeResult, ArchitectureRequestError> {
    if canonical(&result).len() > LIMITS[2].1 {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: "operation response exceeds its fixed bound".into(),
        });
    }
    Ok(result)
}

fn finding(
    id: &str,
    code: &str,
    class: &str,
    rule: &str,
    scope: &str,
    contributors: Vec<String>,
    facts: Value,
) -> Value {
    let mut value = serde_json::Map::new();
    value.insert("id".into(), Value::String(id.into()));
    value.insert("code".into(), Value::String(code.into()));
    value.insert("classification".into(), Value::String(class.into()));
    value.insert("disposition".into(), Value::String("deny".into()));
    value.insert("rule".into(), Value::String(rule.into()));
    value.insert("scope".into(), Value::String(scope.into()));
    value.insert(
        "contributors".into(),
        Value::Array(contributors.into_iter().map(Value::String).collect()),
    );
    value.insert("facts".into(), facts);
    value.insert("exception".into(), Value::Null);
    Value::Object(value)
}
fn architecture_findings(
    base: &ArchitectureRevision,
    candidate: &ArchitectureRevision,
    input: &ArchitectureEvaluationInput,
) -> Vec<Value> {
    let prefix = finding_prefix(&candidate.revision_id);
    let mut findings = Vec::new();
    findings.extend(hotspot_finding(base, candidate, prefix));
    findings.extend(authority_finding(base, candidate, prefix));
    findings.extend(state_finding(base, candidate, prefix));
    findings.extend(boundary_finding(base, candidate, prefix));
    findings.extend(new_unit_findings(base, candidate, prefix));
    findings.extend(cycle_finding(base, candidate, prefix));
    findings.extend(coverage_finding(candidate));
    findings.extend(stale_baseline_finding(input, prefix));
    findings.extend(governance_finding(candidate, input, prefix));
    findings
}

fn hotspot_finding(
    base: &ArchitectureRevision,
    c: &ArchitectureRevision,
    prefix: &str,
) -> Option<Value> {
    let map: BTreeMap<_, _> = c.units.iter().map(|u| (u.id.as_str(), u)).collect();
    let dispatch = &map["transport:dispatch"];
    let dc = dispatch.cfg.edges + 2 - dispatch.cfg.nodes;
    let context = metric(
        "executable-unit",
        "transport:dispatch".into(),
        vec!["transport:dispatch".into()],
        true,
        c,
        &components(&c.units, &c.edges),
    )
    .minimal_context_node_count;
    if dc > base.policy.dispatch_no_growth.control_flow_complexity
        || context > base.policy.dispatch_no_growth.minimal_context_node_count
    {
        return Some(finding(
            &format!("{prefix}:dispatch-growth"),
            "AIL.ARCH.HOTSPOT_GROWTH",
            "regression",
            "M23-POL-DISPATCH-NO-GROWTH",
            "transport:dispatch",
            vec!["transport:dispatch".into()],
            serde_json::json!({"base_cfc":base.policy.dispatch_no_growth.control_flow_complexity,"candidate_cfc":dc,"base_context":base.policy.dispatch_no_growth.minimal_context_node_count,"candidate_context":context}),
        ));
    }
    None
}

fn authority_finding(
    base: &ArchitectureRevision,
    c: &ArchitectureRevision,
    prefix: &str,
) -> Option<Value> {
    let transport: BTreeSet<_> = c
        .units
        .iter()
        .filter(|u| u.group == "transport")
        .map(|u| u.id.as_str())
        .collect();
    let ce = c
        .edges
        .iter()
        .filter(|e| transport.contains(e.source.as_str()) && e.kind == "capability-use")
        .collect::<Vec<_>>();
    let allowed_capabilities: BTreeSet<_> = base
        .policy
        .transport_capabilities
        .iter()
        .map(String::as_str)
        .collect();
    let ce = ce
        .into_iter()
        .filter(|edge| {
            edge.target
                .strip_prefix("capability:")
                .and_then(|value| value.split('.').next())
                .is_none_or(|capability| !allowed_capabilities.contains(capability))
        })
        .collect::<Vec<_>>();
    if !ce.is_empty() {
        let contributors = ce
            .iter()
            .flat_map(|e| [e.source.clone(), e.target.clone()])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let actual = ce
            .iter()
            .filter_map(|e| {
                e.target
                    .strip_prefix("capability:")
                    .and_then(|x| x.split('.').next())
            })
            .collect::<BTreeSet<_>>();
        return Some(finding(
            &format!("{prefix}:transport-capability"),
            "AIL.ARCH.AUTHORITY",
            "violation",
            "M23-POL-TRANSPORT-CAPABILITY",
            "group:transport",
            contributors,
            serde_json::json!({"actual":actual,"allowed":base.policy.transport_capabilities}),
        ));
    }
    None
}

fn state_finding(
    base: &ArchitectureRevision,
    c: &ArchitectureRevision,
    prefix: &str,
) -> Option<Value> {
    let transport: BTreeSet<_> = c
        .units
        .iter()
        .filter(|u| u.group == "transport")
        .map(|u| u.id.as_str())
        .collect();
    let se = c
        .edges
        .iter()
        .filter(|e| {
            transport.contains(e.source.as_str())
                && matches!(e.kind.as_str(), "state-read" | "state-write")
        })
        .collect::<Vec<_>>();
    let allowed_state: BTreeSet<_> = base
        .policy
        .transport_state
        .iter()
        .map(String::as_str)
        .collect();
    let se = se
        .into_iter()
        .filter(|edge| {
            edge.target
                .strip_prefix("state:")
                .is_none_or(|state| !allowed_state.contains(state))
        })
        .collect::<Vec<_>>();
    if !se.is_empty() {
        let contributors = se
            .iter()
            .flat_map(|e| [e.source.clone(), e.target.clone()])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let reads = se
            .iter()
            .filter(|e| e.kind == "state-read")
            .filter_map(|e| e.target.strip_prefix("state:"))
            .collect::<BTreeSet<_>>();
        let writes = se
            .iter()
            .filter(|e| e.kind == "state-write")
            .filter_map(|e| e.target.strip_prefix("state:"))
            .collect::<BTreeSet<_>>();
        return Some(finding(
            &format!("{prefix}:transport-state"),
            "AIL.ARCH.STATE",
            "violation",
            "M23-POL-TRANSPORT-STATE",
            "group:transport",
            contributors,
            serde_json::json!({"reads":reads,"writes":writes,"allowed":base.policy.transport_state}),
        ));
    }
    None
}

fn boundary_finding(
    base: &ArchitectureRevision,
    c: &ArchitectureRevision,
    prefix: &str,
) -> Option<Value> {
    let groups = c
        .units
        .iter()
        .map(|u| (u.id.as_str(), u.group.as_str()))
        .chain(
            c.endpoint_groups
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str())),
        )
        .collect::<BTreeMap<_, _>>();
    let allowed = |a: &str, b: &str| match a {
        "contract" => base
            .policy
            .allowed_group_dependencies
            .contract
            .iter()
            .any(|x| x == b),
        "transport" => base
            .policy
            .allowed_group_dependencies
            .transport
            .iter()
            .any(|x| x == b),
        "domain" => base
            .policy
            .allowed_group_dependencies
            .domain
            .iter()
            .any(|x| x == b),
        "persistence-adapter" => base
            .policy
            .allowed_group_dependencies
            .persistence_adapter
            .iter()
            .any(|x| x == b),
        _ => base
            .policy
            .allowed_group_dependencies
            .verification
            .iter()
            .any(|x| x == b),
    };
    let mut forbidden = vec![];
    for e in &c.edges {
        if e.kind != "verifies" {
            if let (Some(a), Some(b)) =
                (groups.get(e.source.as_str()), groups.get(e.target.as_str()))
            {
                if a != b && !allowed(a, b) {
                    forbidden.push((a, b, e));
                }
            }
        }
    }
    if !forbidden.is_empty() {
        forbidden.sort_by_key(|x| (x.0, x.1, &x.2.source, &x.2.target, &x.2.kind));
        let contributors = forbidden
            .iter()
            .flat_map(|x| [x.2.source.clone(), x.2.target.clone()])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let facts=forbidden.iter().map(|x|serde_json::json!({"source_group":x.0,"target_group":x.1,"source":x.2.source,"target":x.2.target,"kind":x.2.kind})).collect::<Vec<_>>();
        return Some(finding(
            &format!("{prefix}:group-dependency"),
            "AIL.ARCH.BOUNDARY",
            "violation",
            "M23-POL-GROUP-DEPENDENCY",
            &format!("group:{}", forbidden[0].0),
            contributors,
            serde_json::json!({"forbidden_group_edges":facts}),
        ));
    }
    None
}

fn new_unit_findings(
    base: &ArchitectureRevision,
    c: &ArchitectureRevision,
    prefix: &str,
) -> Vec<Value> {
    let mut out = Vec::new();
    let base_ids: BTreeSet<_> = base.units.iter().map(|u| u.id.as_str()).collect();
    let comps = components(&c.units, &c.edges);
    for u in c
        .units
        .iter()
        .filter(|u| !base_ids.contains(u.id.as_str()) && u.group != "contract")
    {
        let cf = u.cfg.edges + 2 - u.cfg.nodes;
        let cx = metric(
            "executable-unit",
            u.id.clone(),
            vec![u.id.clone()],
            true,
            c,
            &comps,
        )
        .minimal_context_node_count;
        if cf > base.policy.new_unit.control_flow_complexity_max
            || cx > base.policy.new_unit.minimal_context_node_count_max
        {
            out.push(finding(
                &format!("{prefix}:new-unit:{}", u.id),
                "AIL.ARCH.NEW_UNIT",
                "violation",
                "M23-POL-NEW-UNIT",
                &u.id,
                vec![u.id.clone()],
                serde_json::json!({"cfc":cf,"cfc_max":base.policy.new_unit.control_flow_complexity_max,"context":cx,"context_max":base.policy.new_unit.minimal_context_node_count_max}),
            ));
        }
    }
    out
}

fn cycle_finding(
    base: &ArchitectureRevision,
    c: &ArchitectureRevision,
    prefix: &str,
) -> Option<Value> {
    let comps = components(&c.units, &c.edges);
    let bc = components(&base.units, &base.edges);
    let cycles = comps
        .iter()
        .filter(|component| component.len() > 1 && !bc.contains(component))
        .collect::<Vec<_>>();
    if !base.policy.new_cycles && !cycles.is_empty() {
        let contributors = cycles
            .iter()
            .flat_map(|component| component.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let component_values = cycles
            .iter()
            .map(|component| serde_json::json!({"id":component_id(component),"members":component}))
            .collect::<Vec<_>>();
        return Some(finding(
            &format!("{prefix}:new-cycle"),
            "AIL.ARCH.CYCLE",
            "violation",
            "M23-POL-NO-NEW-CYCLE",
            &component_id(cycles[0]),
            contributors,
            serde_json::json!({"components":component_values}),
        ));
    }
    None
}

fn coverage_finding(c: &ArchitectureRevision) -> Option<Value> {
    if !c.coverage.complete_for_policy
        || !c
            .coverage
            .analyzed_groups
            .iter()
            .map(String::as_str)
            .eq(REQUIRED_GROUPS)
    {
        return Some(coverage_diagnostic(&c.revision_id, &c.coverage));
    }
    None
}

fn stale_baseline_finding(input: &ArchitectureEvaluationInput, prefix: &str) -> Option<Value> {
    if input.request.baseline_revision != input.active_baseline_revision {
        return Some(finding(
            &format!("{prefix}:stale-baseline"),
            "AIL.ARCH.STALE_BASELINE",
            "incomplete",
            "M24-BASELINE-COMPATIBILITY",
            "transport:dispatch",
            vec![],
            serde_json::json!({"required":input.active_baseline_revision,"provided":input.request.baseline_revision}),
        ));
    }
    None
}

fn governance_finding(
    c: &ArchitectureRevision,
    input: &ArchitectureEvaluationInput,
    prefix: &str,
) -> Option<Value> {
    let governance_shape_valid = match input.request.requested_governance_changes.len() {
        0 => input.request.authorization_id.is_none(),
        1 => input.request.authorization_id.is_some(),
        _ => false,
    };
    if !governance_shape_valid || !input.request.requested_governance_changes.is_empty() {
        let change = input.request.requested_governance_changes.first();
        let digest = graph_digest(c);
        let matched = governance_shape_valid
            && input.request.policy_revision == input.active_policy_revision
            && input.request.baseline_revision == input.active_baseline_revision
            && input
                .request
                .authorization_id
                .as_ref()
                .and_then(|id| {
                    input
                        .governance_authorizations
                        .iter()
                        .find(|a| &a.authorization_id == id)
                })
                .is_some_and(|a| {
                    a.candidate_revision_id == c.revision_id
                        && a.candidate_graph_digest == digest
                        && a.policy_revision == input.active_policy_revision
                        && a.baseline_revision == input.active_baseline_revision
                        && change.is_some_and(|change| {
                            a.kind == change.kind
                                && sorted(&a.exception_ids) == sorted(&change.exception_ids)
                                && a.rule == change.rule
                                && a.scope == change.scope
                        })
                        && a.review_boundary == input.request.review_boundary
                });
        if !matched {
            return Some(finding(
                &format!("{prefix}:governance"),
                "AIL.ARCH.GOVERNANCE_UNAUTHORIZED",
                "violation",
                "M23-POL-GOVERNANCE",
                "governance",
                vec![],
                serde_json::json!({"requested_changes":input.request.requested_governance_changes.iter().map(change_value).collect::<Vec<_>>(),"authorization_id":input.request.authorization_id,"trusted_authorization_matched":false}),
            ));
        }
    }
    None
}

fn sorted(v: &[String]) -> Vec<String> {
    let mut x = v.to_vec();
    x.sort();
    x.dedup();
    x
}
fn change_value(c: &GovernanceChange) -> Value {
    serde_json::json!({"kind":c.kind,"rule":c.rule,"scope":c.scope,"exception_ids":c.exception_ids})
}
fn graph_digest(r: &ArchitectureRevision) -> String {
    let mut units = r.units.clone();
    units.sort_by(|a, b| a.id.cmp(&b.id));
    let us = units
        .iter()
        .map(|u| {
            OrderedJson::Object(vec![
                ("id".into(), OrderedJson::String(u.id.clone())),
                ("module".into(), OrderedJson::String(u.module.clone())),
                ("group".into(), OrderedJson::String(u.group.clone())),
                (
                    "cfg".into(),
                    OrderedJson::Object(vec![
                        ("nodes".into(), OrderedJson::Number(u.cfg.nodes)),
                        ("edges".into(), OrderedJson::Number(u.cfg.edges)),
                    ]),
                ),
                ("capabilities".into(), strings(&sorted(&u.capabilities))),
                ("state_reads".into(), strings(&sorted(&u.state_reads))),
                ("state_writes".into(), strings(&sorted(&u.state_writes))),
            ])
        })
        .collect::<Vec<_>>();
    let mut edges = r.edges.clone();
    edges.sort_by_key(|e| {
        (
            e.source.clone(),
            e.target.clone(),
            EDGE_KINDS.iter().position(|x| *x == e.kind).unwrap(),
        )
    });
    let es = edges
        .iter()
        .map(|e| {
            OrderedJson::Array(vec![
                OrderedJson::String(e.source.clone()),
                OrderedJson::String(e.target.clone()),
                OrderedJson::String(e.kind.clone()),
            ])
        })
        .collect();
    let graph = OrderedJson::Object(vec![
        ("units".into(), OrderedJson::Array(us)),
        ("edges".into(), OrderedJson::Array(es)),
    ]);
    let mut canonical = String::new();
    render_json(&graph, 0, &mut canonical);
    canonical.push('\n');
    source_digest(&canonical)
}
fn scope_rank(k: &str) -> usize {
    [
        "executable-unit",
        "module",
        "dependency-component",
        "architecture-group",
    ]
    .iter()
    .position(|x| *x == k)
    .unwrap_or(9)
}
fn finding_key(v: &Value) -> (usize, String, String, String) {
    let codes = [
        "AIL.ARCH.HOTSPOT_GROWTH",
        "AIL.ARCH.AUTHORITY",
        "AIL.ARCH.STATE",
        "AIL.ARCH.BOUNDARY",
        "AIL.ARCH.NEW_UNIT",
        "AIL.ARCH.CYCLE",
        "AIL.ARCH.COVERAGE_INCOMPLETE",
        "AIL.ARCH.STALE_BASELINE",
        "AIL.ARCH.GOVERNANCE_UNAUTHORIZED",
        "AIL.ARCH.ANALYSIS_INCOMPLETE",
    ];
    (
        codes.iter().position(|x| v["code"] == *x).unwrap(),
        v["scope"].as_str().unwrap().into(),
        serde_json::to_string(&v["contributors"]).unwrap(),
        v["id"].as_str().unwrap().into(),
    )
}

fn finding_prefix(revision_id: &str) -> &str {
    if let Some(rest) = revision_id.strip_prefix("arch-r") {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && rest.as_bytes().get(digits) == Some(&b'-') {
            return &rest[digits + 1..];
        }
    }
    revision_id.strip_prefix("arch-").unwrap_or(revision_id)
}

fn coverage_diagnostic(revision_id: &str, coverage: &ArchitectureCoverage) -> Value {
    serde_json::json!({"id":format!("{}:coverage", finding_prefix(revision_id)),"code":"AIL.ARCH.COVERAGE_INCOMPLETE","classification":"incomplete","disposition":"deny","rule":"M23-POL-COVERAGE","scope":"workspace","contributors":[],"facts":{"required_groups":["contract","transport","domain","persistence-adapter","verification"],"analyzed_groups":coverage.analyzed_groups,"unchecked_boundaries":coverage.unchecked_boundaries},"exception":null})
}

fn bounded_incomplete(
    analysis: AnalysisIdentity,
    coverage: ArchitectureCoverage,
    budgets: BudgetUse,
    diagnostic: Value,
    current_revision_id: &str,
) -> Result<ArchitectureSnapshotResult, ArchitectureRequestError> {
    let failure = ArchitectureIncompleteFailure {
        status: "incomplete".into(),
        analysis,
        coverage,
        budgets,
        diagnostics: vec![diagnostic],
        edits: Vec::new(),
        current_revision_id: current_revision_id.into(),
        published_child_revision_id: None,
    };
    if canonical(&failure).len() > LIMITS[2].1 {
        return Err(ArchitectureRequestError {
            kind: ArchitectureRequestErrorKind::InvalidRevision,
            message: "incomplete response exceeds its fixed bound".into(),
        });
    }
    Ok(ArchitectureSnapshotResult::Incomplete(failure))
}

fn render_compact(snapshot: &ArchitectureSnapshot) -> String {
    let mut lines = vec![
        format!(
            "{} {} scope={}",
            snapshot.classification,
            snapshot.analysis.revision_id,
            snapshot.analysis.analysis_scope
        ),
        format!(
            "policy={} baseline={}",
            snapshot.analysis.policy_revision, snapshot.analysis.baseline_revision
        ),
    ];
    lines.extend(snapshot.findings.iter().map(|finding| {
        let contributors = finding["contributors"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "-".into());
        format!(
            "{} {} contributors={contributors}",
            finding["code"].as_str().unwrap_or_default(),
            finding["scope"].as_str().unwrap_or_default()
        )
    }));
    lines.push(format!(
        "coverage={} unchecked={}",
        snapshot.coverage.analyzed_groups.len(),
        snapshot.coverage.unchecked_boundaries.len()
    ));
    lines.join("\n") + "\n"
}

#[allow(clippy::too_many_lines)]
fn metric(
    kind: &str,
    identity: String,
    mut members: Vec<String>,
    _unit_scope: bool,
    r: &ArchitectureRevision,
    comps: &[Vec<String>],
) -> ScopeMetrics {
    members.sort();
    let selected: BTreeSet<_> = members.iter().cloned().collect();
    let map: BTreeMap<_, _> = r.units.iter().map(|u| (u.id.clone(), u)).collect();
    let contributors: Vec<_> = members
        .iter()
        .map(|id| {
            let u = map[id];
            CfcContributor {
                unit_id: id.clone(),
                value: u.cfg.edges + 2 - u.cfg.nodes,
            }
        })
        .collect();
    let set = |kind: &str| {
        r.edges
            .iter()
            .filter(|e| selected.contains(&e.source) && e.kind == kind)
            .map(|e| e.target.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    };
    let deps = r
        .edges
        .iter()
        .filter(|e| {
            selected.contains(&e.source)
                && matches!(
                    e.kind.as_str(),
                    "calls"
                        | "type-use"
                        | "delegates"
                        | "capability-use"
                        | "state-read"
                        | "state-write"
                )
                && !selected.contains(&e.target)
        })
        .map(|e| e.target.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let caps = r
        .edges
        .iter()
        .filter(|e| selected.contains(&e.source) && e.kind == "capability-use")
        .filter_map(|e| {
            e.target
                .strip_prefix("capability:")
                .and_then(|x| x.split('.').next())
        })
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let context = selected
        .iter()
        .cloned()
        .chain(
            r.edges
                .iter()
                .filter(|e| selected.contains(&e.source) || selected.contains(&e.target))
                .flat_map(|e| [e.source.clone(), e.target.clone()]),
        )
        .chain(
            selected
                .iter()
                .filter_map(|id| map.get(id).map(|u| format!("policy:group:{}", u.group))),
        )
        .chain(
            (identity == "transport:dispatch")
                .then(|| format!("baseline:{}", r.policy.baseline_match.baseline_revision)),
        )
        .collect::<BTreeSet<_>>()
        .len();
    let size = if kind == "dependency-component" {
        members.len()
    } else {
        members
            .iter()
            .map(|id| comps.iter().find(|c| c.contains(id)).unwrap().len())
            .max()
            .unwrap()
    };
    ScopeMetrics {
        scope_kind: kind.into(),
        identity,
        unit_ids: members,
        control_flow_complexity: ControlFlowComplexity {
            maximum: contributors.iter().map(|x| x.value).max().unwrap(),
            sum: contributors.iter().map(|x| x.value).sum(),
            contributors,
        },
        direct_dependency_set: deps,
        declared_capability_set: caps,
        state_read_set: set("state-read")
            .into_iter()
            .filter_map(|x: String| x.strip_prefix("state:").map(str::to_owned))
            .collect(),
        state_write_set: set("state-write")
            .into_iter()
            .filter_map(|x: String| x.strip_prefix("state:").map(str::to_owned))
            .collect(),
        dependency_component_size: size,
        minimal_context_node_count: context,
    }
}

fn components(units: &[ArchitectureUnit], edges: &[ArchitectureEdge]) -> Vec<Vec<String>> {
    // Deterministic reachability intersection is equivalent to SCC partitioning and is bounded by M25 limits.
    let ids: BTreeSet<_> = units.iter().map(|u| u.id.clone()).collect();
    let mut remaining = ids.clone();
    let mut out = vec![];
    while let Some(root) = remaining.first().cloned() {
        let f = reachable(&root, edges, &ids, false);
        let b = reachable(&root, edges, &ids, true);
        let c: Vec<_> = f.intersection(&b).cloned().collect();
        for x in &c {
            remaining.remove(x);
        }
        out.push(c);
    }
    out
}
fn reachable(
    root: &str,
    edges: &[ArchitectureEdge],
    ids: &BTreeSet<String>,
    reverse: bool,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::from([root.to_owned()]);
    loop {
        let old = seen.len();
        for e in edges.iter().filter(|e| {
            matches!(
                e.kind.as_str(),
                "calls"
                    | "type-use"
                    | "delegates"
                    | "capability-use"
                    | "state-read"
                    | "state-write"
            )
        }) {
            let (a, b) = if reverse {
                (&e.target, &e.source)
            } else {
                (&e.source, &e.target)
            };
            if seen.contains(a) && ids.contains(b) {
                seen.insert(b.clone());
            }
        }
        if seen.len() == old {
            return seen;
        }
    }
}
fn component_id(m: &[String]) -> String {
    source_digest(&m.join("\0")).replace("sha256:", "component:sha256:")
}

#[derive(Clone)]
enum OrderedJson {
    Null,
    Bool(bool),
    Number(usize),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

trait ToOrderedJson {
    fn ordered_json(&self) -> OrderedJson;
}
impl<T: ToOrderedJson> ToOrderedJson for Option<T> {
    fn ordered_json(&self) -> OrderedJson {
        self.as_ref()
            .map_or(OrderedJson::Null, ToOrderedJson::ordered_json)
    }
}

fn strings(values: &[String]) -> OrderedJson {
    OrderedJson::Array(values.iter().cloned().map(OrderedJson::String).collect())
}

fn values(values: &[Value]) -> OrderedJson {
    OrderedJson::Array(values.iter().map(value_json).collect())
}

fn value_json(value: &Value) -> OrderedJson {
    match value {
        Value::Null => OrderedJson::Null,
        Value::Bool(value) => OrderedJson::Bool(*value),
        Value::Number(value) => OrderedJson::Number(
            value
                .as_u64()
                .and_then(|number| number.try_into().ok())
                .expect("architecture contract numbers are non-negative integers"),
        ),
        Value::String(value) => OrderedJson::String(value.clone()),
        Value::Array(value) => OrderedJson::Array(value.iter().map(value_json).collect()),
        Value::Object(value) => {
            if value.contains_key("kind") && value.contains_key("exception_ids") {
                return OrderedJson::Object(
                    ["kind", "rule", "scope", "exception_ids"]
                        .into_iter()
                        .map(|key| (key.into(), value_json(&value[key])))
                        .collect(),
                );
            }
            if value.contains_key("source_group") {
                return OrderedJson::Object(
                    ["source_group", "target_group", "source", "target", "kind"]
                        .into_iter()
                        .map(|key| (key.into(), value_json(&value[key])))
                        .collect(),
                );
            }
            let mut fields = Vec::new();
            for key in JSON_FIELD_ORDER {
                if let Some(item) = value.get(*key) {
                    fields.push(((*key).into(), value_json(item)));
                }
            }
            for (key, item) in value {
                if !JSON_FIELD_ORDER.contains(&key.as_str()) {
                    fields.push((key.clone(), value_json(item)));
                }
            }
            OrderedJson::Object(fields)
        }
    }
}

macro_rules! object {
    ($($name:literal: $value:expr),* $(,)?) => {
        OrderedJson::Object(vec![$(($name.into(), $value)),*])
    };
}

impl ToOrderedJson for AnalysisIdentity {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "workspace_id": OrderedJson::String(self.workspace_id.clone()),
        "revision_id": OrderedJson::String(self.revision_id.clone()),
        "semantic_model_version": OrderedJson::String(self.semantic_model_version.clone()),
        "policy_revision": OrderedJson::String(self.policy_revision.clone()),
        "baseline_revision": OrderedJson::String(self.baseline_revision.clone()),
        "analysis_scope": OrderedJson::String(self.analysis_scope.clone()))
    }
}
impl ToOrderedJson for ArchitectureCoverage {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "analyzed_groups": strings(&self.analyzed_groups),
        "unchecked_boundaries": values(&self.unchecked_boundaries),
        "complete_for_policy": OrderedJson::Bool(self.complete_for_policy))
    }
}
impl ToOrderedJson for BudgetUse {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "semantic_nodes": OrderedJson::Number(self.semantic_nodes),
        "typed_edges": OrderedJson::Number(self.typed_edges),
        "structured_bytes": OrderedJson::Number(self.structured_bytes),
        "compact_bytes": OrderedJson::Number(self.compact_bytes),
        "compact_lines": OrderedJson::Number(self.compact_lines),
        "exhausted": self.exhausted.clone().map_or(OrderedJson::Null, OrderedJson::String))
    }
}
impl ToOrderedJson for DispatchBudget {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "control_flow_complexity": OrderedJson::Number(self.control_flow_complexity),
        "minimal_context_node_count": OrderedJson::Number(self.minimal_context_node_count))
    }
}
impl ToOrderedJson for ArchitectureException {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "id": OrderedJson::String(self.id.clone()), "rule": OrderedJson::String(self.rule.clone()),
        "scope": OrderedJson::String(self.scope.clone()), "contributors": strings(&self.contributors),
        "policy_revision": OrderedJson::String(self.policy_revision.clone()),
        "expires_after_review_boundary": OrderedJson::String(self.expires_after_review_boundary.clone()))
    }
}
impl ToOrderedJson for BaselineMatch {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "baseline_revision": OrderedJson::String(self.baseline_revision.clone()),
        "scope": OrderedJson::String(self.scope.clone()), "metrics": self.metrics.ordered_json(),
        "accepted_debt": OrderedJson::Bool(self.accepted_debt))
    }
}
impl ToOrderedJson for AcceptedDebt {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "rule": OrderedJson::String(self.rule.clone()), "scope": OrderedJson::String(self.scope.clone()),
        "baseline_revision": OrderedJson::String(self.baseline_revision.clone()),
        "metrics": self.metrics.ordered_json())
    }
}
impl ToOrderedJson for CfcContributor {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "unit_id": OrderedJson::String(self.unit_id.clone()), "value": OrderedJson::Number(self.value))
    }
}
impl ToOrderedJson for ControlFlowComplexity {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "maximum": OrderedJson::Number(self.maximum), "sum": OrderedJson::Number(self.sum),
        "contributors": OrderedJson::Array(self.contributors.iter().map(ToOrderedJson::ordered_json).collect()))
    }
}
impl ToOrderedJson for ScopeMetrics {
    fn ordered_json(&self) -> OrderedJson {
        object!(
        "scope_kind": OrderedJson::String(self.scope_kind.clone()), "identity": OrderedJson::String(self.identity.clone()),
        "unit_ids": strings(&self.unit_ids), "control_flow_complexity": self.control_flow_complexity.ordered_json(),
        "direct_dependency_set": strings(&self.direct_dependency_set), "declared_capability_set": strings(&self.declared_capability_set),
        "state_read_set": strings(&self.state_read_set), "state_write_set": strings(&self.state_write_set),
        "dependency_component_size": OrderedJson::Number(self.dependency_component_size),
        "minimal_context_node_count": OrderedJson::Number(self.minimal_context_node_count))
    }
}
impl ToOrderedJson for PolicySelector {
    fn ordered_json(&self) -> OrderedJson {
        object!("scope_kind": OrderedJson::String(self.scope_kind.clone()), "scope_identity": OrderedJson::String(self.scope_identity.clone()))
    }
}
impl ToOrderedJson for GroupDependencies {
    fn ordered_json(&self) -> OrderedJson {
        object!("contract": strings(&self.contract), "transport": strings(&self.transport),
        "domain": strings(&self.domain), "persistence-adapter": strings(&self.persistence_adapter), "verification": strings(&self.verification))
    }
}
impl ToOrderedJson for PolicyGovernance {
    fn ordered_json(&self) -> OrderedJson {
        object!("policy_revision": OrderedJson::String(self.policy_revision.clone()),
        "baseline_revision": OrderedJson::String(self.baseline_revision.clone()),
        "exceptions": OrderedJson::Array(self.exceptions.iter().map(ToOrderedJson::ordered_json).collect()))
    }
}
impl ToOrderedJson for NewUnitBudget {
    fn ordered_json(&self) -> OrderedJson {
        object!("control_flow_complexity_max": OrderedJson::Number(self.control_flow_complexity_max),
        "minimal_context_node_count_max": OrderedJson::Number(self.minimal_context_node_count_max))
    }
}
impl ToOrderedJson for PolicyValue {
    fn ordered_json(&self) -> OrderedJson {
        match self {
            Self::GroupDependencies(v) => v.ordered_json(),
            Self::Governance(v) => v.ordered_json(),
            Self::Dispatch(v) => v.ordered_json(),
            Self::NewUnit(v) => v.ordered_json(),
            Self::Strings(v) => strings(v),
            Self::Boolean(v) => OrderedJson::Bool(*v),
            Self::Integer(v) => OrderedJson::Number(*v),
        }
    }
}
impl ToOrderedJson for ArchitecturePolicy {
    fn ordered_json(&self) -> OrderedJson {
        object!("id": OrderedJson::String(self.id.clone()), "selector": self.selector.ordered_json(),
        "classification": OrderedJson::String(self.classification.clone()), "disposition": OrderedJson::String(self.disposition.clone()),
        "comparison": OrderedJson::String(self.comparison.clone()), "value": self.value.ordered_json())
    }
}
impl ToOrderedJson for ArchitectureSnapshot {
    fn ordered_json(&self) -> OrderedJson {
        object!("format": OrderedJson::String(self.format.clone()), "analysis": self.analysis.ordered_json(),
        "scopes": OrderedJson::Array(self.scopes.iter().map(ToOrderedJson::ordered_json).collect()), "coverage": self.coverage.ordered_json(),
        "budgets": self.budgets.ordered_json(), "active_policies": OrderedJson::Array(self.active_policies.iter().map(ToOrderedJson::ordered_json).collect()),
        "baseline_match": self.baseline_match.ordered_json(), "accepted_debt": OrderedJson::Array(self.accepted_debt.iter().map(ToOrderedJson::ordered_json).collect()),
        "exceptions": OrderedJson::Array(self.exceptions.iter().map(ToOrderedJson::ordered_json).collect()), "findings": values(&self.findings),
        "classification": OrderedJson::String(self.classification.clone()), "compact": OrderedJson::String(self.compact.clone()))
    }
}
impl ToOrderedJson for ArchitectureSnapshotResponse {
    fn ordered_json(&self) -> OrderedJson {
        object!("status": OrderedJson::String(self.status.clone()), "snapshot": self.snapshot.ordered_json(),
        "snapshot_digest": OrderedJson::String(self.snapshot_digest.clone()))
    }
}
impl ToOrderedJson for ArchitectureIncompleteFailure {
    fn ordered_json(&self) -> OrderedJson {
        object!("status": OrderedJson::String(self.status.clone()), "analysis": self.analysis.ordered_json(),
        "coverage": self.coverage.ordered_json(), "budgets": self.budgets.ordered_json(), "diagnostics": values(&self.diagnostics),
        "edits": values(&self.edits), "current_revision_id": OrderedJson::String(self.current_revision_id.clone()),
        "published_child_revision_id": self.published_child_revision_id.clone().map_or(OrderedJson::Null, OrderedJson::String))
    }
}
impl ToOrderedJson for ScopeChange {
    fn ordered_json(&self) -> OrderedJson {
        object!("scope_kind":OrderedJson::String(self.scope_kind.clone()),"identity":OrderedJson::String(self.identity.clone()),"base":self.base.clone().map_or(OrderedJson::Null,OrderedJson::String),"candidate":self.candidate.clone().map_or(OrderedJson::Null,OrderedJson::String))
    }
}
impl ToOrderedJson for ArchitectureDelta {
    fn ordered_json(&self) -> OrderedJson {
        object!("format":OrderedJson::String(self.format.clone()),"base_snapshot_digest":OrderedJson::String(self.base_snapshot_digest.clone()),"candidate_snapshot_digest":OrderedJson::String(self.candidate_snapshot_digest.clone()),"base_revision_id":OrderedJson::String(self.base_revision_id.clone()),"candidate_revision_id":OrderedJson::String(self.candidate_revision_id.clone()),"scope_changes":OrderedJson::Array(self.scope_changes.iter().map(ToOrderedJson::ordered_json).collect()),"findings":values(&self.findings),"classification":OrderedJson::String(self.classification.clone()),"publication":OrderedJson::String(self.publication.clone()),"commit":OrderedJson::String(self.commit.clone()),"compact":OrderedJson::String(self.compact.clone()))
    }
}
impl ToOrderedJson for BehaviorValidation {
    fn ordered_json(&self) -> OrderedJson {
        object!("status":OrderedJson::String(self.status.clone()),"cases_passed":OrderedJson::Number(self.cases_passed),"cases_total":OrderedJson::Number(self.cases_total))
    }
}
impl ToOrderedJson for ArchitectureCompletionEvidence {
    fn ordered_json(&self) -> OrderedJson {
        object!("base_revision_id":OrderedJson::String(self.base_revision_id.clone()),"revision_id":OrderedJson::String(self.revision_id.clone()),"base_snapshot_digest":OrderedJson::String(self.base_snapshot_digest.clone()),"snapshot_digest":OrderedJson::String(self.snapshot_digest.clone()),"delta_digest":OrderedJson::String(self.delta_digest.clone()),"policy_revision":OrderedJson::String(self.policy_revision.clone()),"baseline_revision":OrderedJson::String(self.baseline_revision.clone()),"coverage":self.coverage.ordered_json(),"budgets":self.budgets.ordered_json(),"behavior_validation":self.behavior_validation.ordered_json(),"commit":OrderedJson::String(self.commit.clone()))
    }
}
impl ToOrderedJson for ArchitectureSuccess {
    fn ordered_json(&self) -> OrderedJson {
        object!("status":OrderedJson::String(self.status.clone()),"snapshot":self.snapshot.ordered_json(),"delta":self.delta.ordered_json(),"completion":self.completion.ordered_json())
    }
}
impl ToOrderedJson for ArchitectureFailure {
    fn ordered_json(&self) -> OrderedJson {
        object!("status":OrderedJson::String(self.status.clone()),"base_revision_id":OrderedJson::String(self.base_revision_id.clone()),"current_revision_id":OrderedJson::String(self.current_revision_id.clone()),"snapshot":self.snapshot.ordered_json(),"delta":self.delta.ordered_json(),"diagnostics":values(&self.diagnostics),"edits":values(&self.edits),"published_child_revision_id":self.published_child_revision_id.clone().map_or(OrderedJson::Null,OrderedJson::String))
    }
}
impl ToOrderedJson for ArchitectureChangeResult {
    fn ordered_json(&self) -> OrderedJson {
        match self {
            Self::Success(v) => v.ordered_json(),
            Self::Failure(v) => v.ordered_json(),
            Self::Incomplete(v) => v.ordered_json(),
        }
    }
}
impl ToOrderedJson for ArchitectureSnapshotResult {
    fn ordered_json(&self) -> OrderedJson {
        match self {
            Self::Success(v) => v.ordered_json(),
            Self::Incomplete(v) => v.ordered_json(),
        }
    }
}

impl ArchitectureSnapshotResult {
    /// Converts this typed result to a transport-neutral JSON value.
    #[must_use]
    pub fn to_json_value(&self) -> Value {
        ordered_value(&self.ordered_json())
    }
}
impl ArchitectureChangeResult {
    #[must_use]
    pub fn to_json_value(&self) -> Value {
        ordered_value(&self.ordered_json())
    }
}

fn digest<T: ToOrderedJson>(v: &T) -> String {
    source_digest(&String::from_utf8(canonical(v)).expect("canonical JSON is UTF-8"))
}
fn architecture_delta(
    base: &ArchitectureSnapshot,
    candidate: &ArchitectureSnapshot,
) -> ArchitectureDelta {
    let bm = base
        .scopes
        .iter()
        .map(|s| ((scope_rank(&s.scope_kind), s.identity.clone()), s))
        .collect::<BTreeMap<_, _>>();
    let cm = candidate
        .scopes
        .iter()
        .map(|s| ((scope_rank(&s.scope_kind), s.identity.clone()), s))
        .collect::<BTreeMap<_, _>>();
    let keys = bm.keys().chain(cm.keys()).cloned().collect::<BTreeSet<_>>();
    let changes = keys
        .iter()
        .filter_map(|k| {
            let b = bm.get(k);
            let c = cm.get(k);
            (b != c).then(|| ScopeChange {
                scope_kind: b.or(c).unwrap().scope_kind.clone(),
                identity: k.1.clone(),
                base: b.map(|x| digest(*x)),
                candidate: c.map(|x| digest(*x)),
            })
        })
        .collect::<Vec<_>>();
    let denied = candidate
        .findings
        .iter()
        .any(|f| f["disposition"] == "deny");
    let change_count = changes.len();
    ArchitectureDelta {
        format: "ail.architecture.delta.v1".into(),
        base_snapshot_digest: digest(base),
        candidate_snapshot_digest: digest(candidate),
        base_revision_id: base.analysis.revision_id.clone(),
        candidate_revision_id: candidate.analysis.revision_id.clone(),
        scope_changes: changes,
        findings: candidate.findings.clone(),
        classification: candidate.classification.clone(),
        publication: if denied { "not-published" } else { "published" }.into(),
        commit: if denied { "rolled-back" } else { "committed" }.into(),
        compact: format!(
            "{} {}->{} changes={} findings={} publication={}\n",
            candidate.classification,
            base.analysis.revision_id,
            candidate.analysis.revision_id,
            change_count,
            candidate.findings.len(),
            if denied {
                "none"
            } else {
                &candidate.analysis.revision_id
            }
        ),
    }
}

fn ordered_value(value: &OrderedJson) -> Value {
    match value {
        OrderedJson::Null => Value::Null,
        OrderedJson::Bool(v) => Value::Bool(*v),
        OrderedJson::Number(v) => Value::from(*v),
        OrderedJson::String(v) => Value::String(v.clone()),
        OrderedJson::Array(v) => Value::Array(v.iter().map(ordered_value).collect()),
        OrderedJson::Object(v) => Value::Object(
            v.iter()
                .map(|(k, v)| (k.clone(), ordered_value(v)))
                .collect(),
        ),
    }
}

fn render_json(value: &OrderedJson, depth: usize, output: &mut String) {
    match value {
        OrderedJson::Null => output.push_str("null"),
        OrderedJson::Bool(v) => output.push_str(if *v { "true" } else { "false" }),
        OrderedJson::Number(v) => output.push_str(&v.to_string()),
        OrderedJson::String(v) => {
            output.push_str(&serde_json::to_string(v).expect("string serialization cannot fail"));
        }
        OrderedJson::Array(items) => {
            render_collection(items.iter().map(|v| (None, v)), depth, '[', ']', output);
        }
        OrderedJson::Object(fields) => render_collection(
            fields.iter().map(|(k, v)| (Some(k.as_str()), v)),
            depth,
            '{',
            '}',
            output,
        ),
    }
}

fn render_collection<'a>(
    items: impl Iterator<Item = (Option<&'a str>, &'a OrderedJson)>,
    depth: usize,
    open: char,
    close: char,
    output: &mut String,
) {
    let items: Vec<_> = items.collect();
    output.push(open);
    if !items.is_empty() {
        output.push('\n');
        for (index, (key, value)) in items.iter().enumerate() {
            output.push_str(&"  ".repeat(depth + 1));
            if let Some(key) = key {
                output
                    .push_str(&serde_json::to_string(key).expect("key serialization cannot fail"));
                output.push_str(": ");
            }
            render_json(value, depth + 1, output);
            if index + 1 != items.len() {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str(&"  ".repeat(depth));
    }
    output.push(close);
}

fn canonical<T: ToOrderedJson>(value: &T) -> Vec<u8> {
    let mut output = String::new();
    render_json(&value.ordered_json(), 0, &mut output);
    output.push('\n');
    output.into_bytes()
}
fn budget_value(b: &BudgetUse, f: &str) -> usize {
    match f {
        "semantic_nodes" => b.semantic_nodes,
        "typed_edges" => b.typed_edges,
        "structured_bytes" => b.structured_bytes,
        "compact_bytes" => b.compact_bytes,
        _ => b.compact_lines,
    }
}

fn first_exhausted(budget: &BudgetUse, structured_bytes: usize) -> Option<String> {
    [
        ("semantic_nodes", budget.semantic_nodes, LIMITS[0].1),
        ("typed_edges", budget.typed_edges, LIMITS[1].1),
        ("structured_bytes", structured_bytes, LIMITS[2].1),
        ("compact_bytes", budget.compact_bytes, LIMITS[3].1),
        ("compact_lines", budget.compact_lines, LIMITS[4].1),
    ]
    .into_iter()
    .find(|(_, used, limit)| used > limit)
    .map(|(name, _, _)| name.to_owned())
}

fn measure(s: &mut ArchitectureSnapshot, b: &mut BudgetUse) {
    b.exhausted = None;
    b.structured_bytes = 0;
    s.budgets = b.clone();
    let structured = canonical(s).len();
    let mut exhausted = first_exhausted(b, structured);
    if matches!(
        exhausted.as_deref(),
        Some("compact_bytes" | "compact_lines")
    ) {
        b.exhausted.clone_from(&exhausted);
        b.structured_bytes = 0;
        s.budgets = b.clone();
        if canonical(s).len() > LIMITS[2].1 {
            exhausted = Some("structured_bytes".into());
        }
    }
    b.exhausted = exhausted;
    b.structured_bytes = 0;
    s.budgets = b.clone();
    b.structured_bytes = canonical(s).len();
    s.budgets = b.clone();
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        AnalysisIdentity, ArchitectureCoverage, ArchitectureSnapshot, ArchitectureSnapshotResult,
        BaselineMatch, BudgetUse, DispatchBudget, bounded_incomplete, canonical, finding_prefix,
        measure, render_compact,
    };

    #[test]
    fn finding_prefix_accepts_any_numeric_architecture_revision() {
        assert_eq!(finding_prefix("arch-r2-valid"), "valid");
        assert_eq!(finding_prefix("arch-r27-logical-id"), "logical-id");
        assert_eq!(finding_prefix("arch-rx-logical-id"), "rx-logical-id");
    }

    #[test]
    fn compact_line_budget_returns_a_bounded_incomplete_result() {
        let analysis = AnalysisIdentity {
            workspace_id: "workspace".into(),
            revision_id: "arch-r1".into(),
            semantic_model_version: "m24-v1".into(),
            policy_revision: "policy-r1".into(),
            baseline_revision: "baseline-r1".into(),
            analysis_scope: "unit:owner".into(),
        };
        let coverage = ArchitectureCoverage {
            analyzed_groups: vec!["contract".into()],
            unchecked_boundaries: Vec::new(),
            complete_for_policy: true,
        };
        let mut budget = BudgetUse {
            semantic_nodes: 1,
            typed_edges: 0,
            structured_bytes: 0,
            compact_bytes: 0,
            compact_lines: 0,
            exhausted: None,
        };
        let mut snapshot = ArchitectureSnapshot {
            format: "ail.architecture.snapshot.v1".into(),
            analysis: analysis.clone(),
            scopes: Vec::new(),
            coverage: coverage.clone(),
            budgets: budget.clone(),
            active_policies: Vec::new(),
            baseline_match: Some(BaselineMatch {
                baseline_revision: "baseline-r1".into(),
                scope: "unit:owner".into(),
                metrics: DispatchBudget {
                    control_flow_complexity: 1,
                    minimal_context_node_count: 1,
                },
                accepted_debt: true,
            }),
            accepted_debt: Vec::new(),
            exceptions: Vec::new(),
            findings: (0..10)
                .map(|index| {
                    json!({"id":format!("finding:{index}"),"code":"AIL.ARCH.NEW_UNIT","scope":format!("unit:{index}"),"contributors":[]})
                })
                .collect(),
            classification: "rejected".into(),
            compact: String::new(),
        };
        snapshot.compact = render_compact(&snapshot);
        budget.compact_bytes = snapshot.compact.len();
        budget.compact_lines = snapshot.compact.matches('\n').count();
        measure(&mut snapshot, &mut budget);
        assert_eq!(budget.compact_lines, 13);
        assert_eq!(budget.exhausted.as_deref(), Some("compact_lines"));

        let diagnostic = json!({"id":"r1:analysis-budget","code":"AIL.ARCH.ANALYSIS_INCOMPLETE","classification":"incomplete","disposition":"deny","rule":"M24-ANALYSIS-BUDGET","scope":"workspace","contributors":[],"facts":{"exhausted":"compact_lines","used":13,"limit":12},"exception":null});
        let result = bounded_incomplete(analysis, coverage, budget, diagnostic, "arch-r1")
            .expect("compact-line failure is bounded");
        assert!(matches!(result, ArchitectureSnapshotResult::Incomplete(_)));
        assert!(canonical(&result).len() <= 65_536);
    }
}
