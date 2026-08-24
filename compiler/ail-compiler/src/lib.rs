//! Authoritative AIL compiler library.
//!
//! M15 adds static semantics and structured diagnostics over the M14 lossless
//! syntax tree. Revision operations remain a later milestone.

mod architecture;
mod diagnostic;
mod driver;
mod evolution;
mod finding;
mod formatter;
mod interpreter;
mod lexer;
mod parser;
mod protocol;
mod semantics;
mod syntax;

pub use architecture::{
    AcceptedDebt, AnalysisIdentity, ArchitectureChangeResult, ArchitectureCompletionEvidence,
    ArchitectureCoverage, ArchitectureDelta, ArchitectureEdge, ArchitectureEvaluationInput,
    ArchitectureException, ArchitectureFailure, ArchitectureIncompleteFailure, ArchitecturePolicy,
    ArchitecturePolicyContext, ArchitectureRequest, ArchitectureRequestError,
    ArchitectureRequestErrorKind, ArchitectureRevision, ArchitectureRevisionError,
    ArchitectureSnapshot, ArchitectureSnapshotInput, ArchitectureSnapshotRequest,
    ArchitectureSnapshotResponse, ArchitectureSnapshotResult, ArchitectureSuccess,
    ArchitectureUnit, ArchitectureWorkspace, BaselineMatch, BehaviorValidation, BudgetUse,
    ControlFlowGraph, DispatchBudget, GovernanceAuthorization, GovernanceChange, GroupDependencies,
    NewUnitBudget, PolicyGovernance, PolicySelector, PolicyValue, ScopeChange, ScopeMetrics,
    architecture_snapshot, validate_architecture_change,
};
pub use diagnostic::Diagnostic;
pub use driver::{
    CliArchitectureFailure, CliCheckError, CliPublishError, PublishedRevision, check_cli_path,
    publish_cli_path,
};
pub use evolution::{
    ArchitectureSourceChangeRequest, BoundedListInspection, BoundedParallelMapInspection,
    CandidateChangeRequest, CandidateRevision, ChangeCapabilitySummary, ChangeEffectSummary,
    ChangeFailure, ChangeResponse, ChangeSuccess, CompletionEvidence, EffectSummary,
    EvolutionBuildFailure, EvolutionCoverage, EvolutionSource, EvolutionWorkspace, ImpactEntry,
    ImpactFailure, ImpactReport, ImpactRequest, OutboundOperationInspection, PersistentIdentity,
    PersistentIdentityChanges, ProposedSchemaChange, PublicBehaviorFailure, RelationshipEdge,
    SemanticChange, SemanticDiff, SemanticLocation, SourceArchitectureConfig, SourceArtifact,
    SourceFileMetadata, SourceOperationArchitecture, SourceSetDiagnostic,
    SourceSetFunctionInspection, SourceSetInspectionFailure, SourceSetRevision, SourceStateAccess,
    UncheckedBoundary, ValidationSummary, ValueParameterInspection, relationship_kinds,
    valid_source_path,
};
pub use finding::{FindingLocation, RelatedLocation, SourceFinding, findings_document};
pub use interpreter::{
    CancellationToken, CapabilityProvider, ObservedCapabilityCall, ObservedOutboundCall,
    OutboundBatchCheck, OutboundCapabilityRequest, OutboundProviderOutcome, OutboundRequestHandle,
    RuntimeFault, RuntimeValue,
};
pub use lexer::{Keyword, Span, Token, TokenKind, lex, reconstruct};
pub use parser::{ParseResult, parse};
pub use protocol::{
    CanonicalEdit, ExecutionFailure, ExecutionRequest, ExecutionResponse, ExecutionSuccess,
    IdentityClassification, IdentityMap, IdentityMapEntry, InspectionRequest, InspectionResult,
    RenameFailure, RenameRequest, RenameResponse, RenameSuccess, RenameValidation, Revision,
    RevisionBuildFailure, Workspace, source_digest,
};
pub use semantics::{
    CapabilityEnvironment, CapabilityInterface, CapabilityOperation, CapabilityOperationKind,
    CausalStep, CheckResult, DiagnosticValue, HandleKind, OutboundCapabilityMetadata,
    SemanticHandle, StructuredDiagnostic, TypeCheckResult, TypeCheckStatus, TypeFact, check_source,
};
pub use syntax::{
    Block, Declaration, Effect, Expr, Field, FunctionDecl, ImportDecl, LetBinding, MAX_LIST_LENGTH,
    MatchArm, ModuleDecl, Parameter, ParameterType, RecordDecl, RecordFieldValue, SourceUnit,
    TypeRef, ValueType, VariantCase, VariantDecl,
};

/// Parse and canonically format one M11 source unit.
///
/// Formatting is unavailable when parsing produced a diagnostic because M11
/// forbids static or canonical processing after a parse error.
///
/// # Errors
///
/// Returns all parse diagnostics when the source is not a valid M11 source
/// unit.
pub fn format_source(source: &str) -> Result<String, Vec<Diagnostic>> {
    let parsed = parse(source);
    if parsed.diagnostics.is_empty() {
        Ok(formatter::format(&parsed.unit))
    } else {
        Err(parsed.diagnostics)
    }
}
