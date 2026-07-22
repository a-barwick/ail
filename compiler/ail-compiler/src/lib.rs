//! Authoritative AIL compiler library.
//!
//! M15 adds static semantics and structured diagnostics over the M14 lossless
//! syntax tree. Revision operations remain a later milestone.

mod diagnostic;
mod evolution;
mod formatter;
mod interpreter;
mod lexer;
mod parser;
mod protocol;
mod semantics;
mod syntax;

pub use diagnostic::Diagnostic;
pub use evolution::{
    EffectSummary, EvolutionBuildFailure, EvolutionCoverage, EvolutionSource, EvolutionWorkspace,
    ImpactEntry, ImpactFailure, ImpactReport, ImpactRequest, PersistentIdentity,
    ProposedSchemaChange, RelationshipEdge, SemanticLocation, SourceArtifact, SourceFileMetadata,
    SourceSetRevision, UncheckedBoundary, relationship_kinds,
};
pub use interpreter::{CapabilityProvider, ObservedCapabilityCall, RuntimeFault, RuntimeValue};
pub use lexer::{Keyword, Span, Token, TokenKind, lex, reconstruct};
pub use parser::{ParseResult, parse};
pub use protocol::{
    CanonicalEdit, ExecutionFailure, ExecutionRequest, ExecutionResponse, ExecutionSuccess,
    IdentityClassification, IdentityMap, IdentityMapEntry, InspectionRequest, InspectionResult,
    RenameFailure, RenameRequest, RenameResponse, RenameSuccess, RenameValidation, Revision,
    RevisionBuildFailure, Workspace, source_digest,
};
pub use semantics::{
    CapabilityEnvironment, CapabilityInterface, CapabilityOperation, CausalStep, CheckResult,
    DiagnosticValue, HandleKind, SemanticHandle, StructuredDiagnostic, TypeCheckResult,
    TypeCheckStatus, TypeFact, check_source,
};
pub use syntax::{
    Block, Declaration, Effect, Expr, Field, FunctionDecl, LetBinding, MatchArm, Parameter,
    ParameterType, RecordDecl, RecordFieldValue, SourceUnit, VariantCase, VariantDecl,
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
