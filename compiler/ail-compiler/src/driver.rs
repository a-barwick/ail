//! `ailc check` and `ailc publish` drive an [`EvolutionWorkspace`].

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::finding::{RelatedLocation, SourceFinding, flatten_json};
use crate::{
    ArchitectureChangeResult, BehaviorValidation, CapabilityEnvironment, CapabilityInterface,
    CapabilityOperation, EvolutionBuildFailure, EvolutionCoverage, EvolutionSource,
    EvolutionWorkspace, OutboundCapabilityMetadata, SourceArchitectureConfig, SourceSetRevision,
    valid_source_path,
};

const CHECK_REVISION: &str = "check";
const PUBLISH_REVISION: &str = "published";
const ARCHITECTURE_FILE: &str = "architecture.json";
const CAPABILITIES_FILE: &str = "capabilities.json";
const REVISION_STORE: &str = ".ail";

/// Failure from the `ailc check` driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCheckError {
    /// The path or project architecture file could not be read.
    Io(String),
    /// [`EvolutionWorkspace::new`] rejected the source set.
    Build(EvolutionBuildFailure),
    /// Architecture policy rejected or could not evaluate the workspace.
    Architecture(CliArchitectureFailure),
}

/// Architecture failure reported by `ailc check` or `ailc publish`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArchitectureFailure {
    pub diagnostics: Vec<String>,
    /// One located, fact-carrying finding per denied policy rule.
    pub findings: Vec<SourceFinding>,
}

/// Failure from the `ailc publish` driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliPublishError {
    /// The candidate failed the same checks `ailc check` runs.
    Check(CliCheckError),
    /// A passed candidate could not be written.
    Write(String),
}

/// Successful `ailc check` evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliCheckSuccess {
    /// Behavior status from architecture evaluation, when project policy exists.
    pub behavior_validation: Option<BehaviorValidation>,
    /// Source-set-relative path of the loaded capabilities file, when present.
    pub capability_environment_path: Option<String>,
    /// Digest of the loaded [`CapabilityEnvironment`], when `capabilities.json` was present.
    pub capability_environment_digest: Option<String>,
}

/// A revision written by `ailc publish`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedRevision {
    pub revision_id: String,
    pub source_set_digest: String,
    pub store: PathBuf,
    /// Behavior status from architecture evaluation, when project policy exists.
    pub behavior_validation: Option<BehaviorValidation>,
    /// Source-set-relative path of the loaded capabilities file, when present.
    pub capability_environment_path: Option<String>,
    /// Digest of the loaded [`CapabilityEnvironment`], when `capabilities.json` was present.
    pub capability_environment_digest: Option<String>,
}

struct BuiltCliWorkspace {
    workspace: EvolutionWorkspace,
    behavior_validation: Option<BehaviorValidation>,
    capability_environment_path: Option<String>,
    capability_environment_digest: Option<String>,
}

struct LoadedCapabilities {
    environment: CapabilityEnvironment,
    path: String,
    digest: String,
}

/// Check a file or directory the same way `ailc check` does.
///
/// A directory becomes the `.ail` files whose names pass [`valid_source_path`].
/// A file becomes a one-file workspace named by its file name. Coverage is
/// declared complete with no artifacts, the same claim the composed-service
/// example uses.
///
/// Capability interfaces load only from `capabilities.json` at the source-set
/// root, the same directory `architecture.json` uses. If the file is absent,
/// the environment is empty. The driver does not search another filename, the
/// repository root, or `.ail/`.
///
/// When `architecture.json` is present next to the named path, the workspace
/// is built with those project settings and evaluated through
/// [`EvolutionWorkspace::evaluate_current_architecture`]. Check does not write
/// a revision.
///
/// # Errors
///
/// Returns [`CliCheckError::Io`] when the path cannot be read,
/// [`CliCheckError::Build`] when the source set is rejected, or
/// [`CliCheckError::Architecture`] when project architecture policy fails.
pub fn check_cli_path(path: impl AsRef<Path>) -> Result<(), CliCheckError> {
    check_cli_path_with_evidence(path).map(drop)
}

/// Check a file or directory and return non-execution evidence for CLI output.
///
/// # Errors
///
/// Returns the same errors as [`check_cli_path`].
pub fn check_cli_path_with_evidence(
    path: impl AsRef<Path>,
) -> Result<CliCheckSuccess, CliCheckError> {
    build_cli_workspace(path.as_ref(), CHECK_REVISION).map(|built| CliCheckSuccess {
        behavior_validation: built.behavior_validation,
        capability_environment_path: built.capability_environment_path,
        capability_environment_digest: built.capability_environment_digest,
    })
}

/// Publish a directory workspace the same way `ailc publish` does.
///
/// Runs the same checks as [`check_cli_path`]. A passing candidate writes one
/// revision under `<dir>/.ail/revisions/published`. A failing candidate writes
/// nothing.
///
/// # Errors
///
/// Returns [`CliPublishError::Check`] when the candidate fails check or
/// architecture, or [`CliPublishError::Write`] when a passed candidate cannot
/// be written. Check failures leave any existing store unchanged and create no
/// store when none existed.
pub fn publish_cli_path(path: impl AsRef<Path>) -> Result<PublishedRevision, CliPublishError> {
    let path = path.as_ref();
    if !path.is_dir() {
        return Err(CliPublishError::Check(CliCheckError::Io(
            "publish requires a directory workspace".to_owned(),
        )));
    }
    let built = build_cli_workspace(path, PUBLISH_REVISION).map_err(CliPublishError::Check)?;
    write_published_revision(
        path,
        &built.workspace,
        built.behavior_validation,
        built.capability_environment_path,
        built.capability_environment_digest,
    )
    .map_err(CliPublishError::Write)
}

fn build_cli_workspace(path: &Path, revision_id: &str) -> Result<BuiltCliWorkspace, CliCheckError> {
    let sources = collect_sources(path)?;
    let architecture = load_architecture_policy(path)?;
    let loaded_capabilities = load_capability_environment(path)?;
    let capabilities = loaded_capabilities
        .as_ref()
        .map_or_else(CapabilityEnvironment::new, |loaded| {
            loaded.environment.clone()
        });
    let capability_path = loaded_capabilities
        .as_ref()
        .map(|loaded| loaded.path.clone());
    let capability_digest = loaded_capabilities
        .as_ref()
        .map(|loaded| loaded.digest.clone());
    let coverage = EvolutionCoverage {
        declared_complete: true,
        ..EvolutionCoverage::default()
    };
    let (workspace, behavior_validation) = match architecture {
        Some((config, analysis_scope)) => {
            let workspace = EvolutionWorkspace::new_with_architecture(
                workspace_id(path),
                revision_id,
                sources,
                &capabilities,
                coverage,
                config,
            )
            .map_err(|failure| {
                CliCheckError::Build(attach_capability_file_facts(
                    failure,
                    capability_path.as_deref(),
                    capability_digest.as_deref(),
                ))
            })?;
            match workspace.evaluate_current_architecture(&analysis_scope) {
                Ok(ArchitectureChangeResult::Success(success)) => {
                    let behavior_validation = success.completion.behavior_validation.clone();
                    (workspace, Some(behavior_validation))
                }
                Ok(result) => {
                    return Err(CliCheckError::Architecture(architecture_failure(
                        &result, &workspace,
                    )));
                }
                Err(error) => {
                    let mut finding =
                        SourceFinding::new("AIL.ARCH.ANALYSIS_INCOMPLETE", "architecture");
                    finding
                        .facts
                        .insert("reason".to_owned(), error.message.clone());
                    return Err(CliCheckError::Architecture(CliArchitectureFailure {
                        diagnostics: vec![format!(
                            "AIL.ARCH.ANALYSIS_INCOMPLETE {}",
                            error.message
                        )],
                        findings: vec![finding],
                    }));
                }
            }
        }
        None => (
            EvolutionWorkspace::new(
                workspace_id(path),
                revision_id,
                sources,
                &capabilities,
                coverage,
            )
            .map_err(|failure| {
                CliCheckError::Build(attach_capability_file_facts(
                    failure,
                    capability_path.as_deref(),
                    capability_digest.as_deref(),
                ))
            })?,
            None,
        ),
    };
    Ok(BuiltCliWorkspace {
        workspace,
        behavior_validation,
        capability_environment_path: capability_path,
        capability_environment_digest: capability_digest,
    })
}

fn attach_capability_file_facts(
    mut failure: EvolutionBuildFailure,
    path: Option<&str>,
    digest: Option<&str>,
) -> EvolutionBuildFailure {
    let (Some(path), Some(digest)) = (path, digest) else {
        return failure;
    };
    for finding in &mut failure.findings {
        if finding.code.starts_with("AIL.CAPABILITY.") {
            finding
                .facts
                .insert("capability_environment.path".to_owned(), path.to_owned());
            finding.facts.insert(
                "capability_environment.digest".to_owned(),
                digest.to_owned(),
            );
        }
    }
    failure
}

fn architecture_failure(
    result: &ArchitectureChangeResult,
    workspace: &EvolutionWorkspace,
) -> CliArchitectureFailure {
    let policy_findings = match result {
        ArchitectureChangeResult::Success(_) => &[] as &[Value],
        ArchitectureChangeResult::Failure(failure) => &failure.diagnostics,
        ArchitectureChangeResult::Incomplete(failure) => &failure.diagnostics,
    };
    let diagnostics = policy_findings
        .iter()
        .map(format_finding)
        .collect::<Vec<_>>();
    let findings = policy_findings
        .iter()
        .map(|finding| source_finding(finding, workspace))
        .collect::<Vec<_>>();
    if diagnostics.is_empty() {
        return CliArchitectureFailure {
            diagnostics: vec!["AIL.ARCH.ANALYSIS_INCOMPLETE".to_owned()],
            findings: vec![SourceFinding::new(
                "AIL.ARCH.ANALYSIS_INCOMPLETE",
                "architecture",
            )],
        };
    }
    CliArchitectureFailure {
        diagnostics,
        findings,
    }
}

fn format_finding(finding: &Value) -> String {
    let code = finding
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("AIL.ARCH.ANALYSIS_INCOMPLETE");
    let scope = finding
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("workspace");
    let rule = finding.get("rule").and_then(Value::as_str);
    let mut line = match rule {
        Some(rule) => format!("{code}:{scope}:{rule}:"),
        None => format!("{code}:{scope}:"),
    };
    let mut facts = Vec::new();
    if let Some(value) = finding.get("facts") {
        flatten_fact(String::new(), value, &mut facts);
    }
    for (key, value) in facts {
        line.push(' ');
        line.push_str(&key);
        line.push('=');
        line.push_str(&value);
    }
    line
}

/// Flatten one finding fact tree into deterministic `key=value` pairs.
///
/// Object keys are already ordered by `serde_json`'s map. Array elements use
/// their index. The pairs carry the numbers and identities the architecture
/// checker computed, so a reader never has to guess a threshold.
fn flatten_fact(prefix: String, value: &Value, facts: &mut Vec<(String, String)>) {
    let child = |key: &str| {
        if prefix.is_empty() {
            key.to_owned()
        } else {
            format!("{prefix}.{key}")
        }
    };
    match value {
        Value::Object(entries) => {
            for (key, entry) in entries {
                flatten_fact(child(key), entry, facts);
            }
        }
        Value::Array(entries) => {
            for (index, entry) in entries.iter().enumerate() {
                flatten_fact(child(&index.to_string()), entry, facts);
            }
        }
        Value::String(text) => facts.push((fact_key(prefix), text.clone())),
        Value::Null => facts.push((fact_key(prefix), "null".to_owned())),
        other => facts.push((fact_key(prefix), other.to_string())),
    }
}

fn fact_key(prefix: String) -> String {
    if prefix.is_empty() {
        "facts".to_owned()
    } else {
        prefix
    }
}

/// Turn one architecture policy finding into a located structured finding.
///
/// Policy identity is architectural: a rule, a scope, and ordered contributor
/// unit identifiers of the form `module:function`. Those identifiers resolve
/// back to a declared function, so a denied rule names the source that violated
/// it instead of leaving the caller to search for it.
fn source_finding(policy_finding: &Value, workspace: &EvolutionWorkspace) -> SourceFinding {
    let code = policy_finding
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("AIL.ARCH.ANALYSIS_INCOMPLETE");
    let mut finding = SourceFinding::new(code, "architecture");
    for key in ["rule", "scope", "classification"] {
        if let Some(value) = policy_finding.get(key).and_then(Value::as_str) {
            finding.facts.insert(key.to_owned(), value.to_owned());
        }
    }
    if let Some(facts) = policy_finding.get("facts") {
        flatten_json("facts", facts, &mut finding.facts);
    }
    let contributors = policy_finding
        .get("contributors")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let scope = policy_finding.get("scope").and_then(Value::as_str);
    let primary = finding
        .facts
        .get("facts.forbidden_group_edges.0.source")
        .cloned()
        .or_else(|| {
            scope
                .filter(|scope| contributors.iter().any(|unit| unit == scope))
                .map(str::to_owned)
        })
        .or_else(|| contributors.first().cloned());
    if let Some(unit) = &primary {
        finding.location = workspace.architecture_location(unit);
    }
    for unit in &contributors {
        if Some(unit) == primary.as_ref() {
            continue;
        }
        finding.related.push(RelatedLocation {
            role: "contributor".to_owned(),
            name: unit.clone(),
            location: workspace.architecture_location(unit),
        });
    }
    finding.with_derived_requirement()
}

fn source_set_directory(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }
}

fn load_architecture_policy(
    path: &Path,
) -> Result<Option<(SourceArchitectureConfig, String)>, CliCheckError> {
    let directory = source_set_directory(path);
    let policy_path = directory.join(ARCHITECTURE_FILE);
    if !policy_path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&policy_path)
        .map_err(|error| CliCheckError::Io(format!("{}: {error}", policy_path.display())))?;
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|error| CliCheckError::Io(format!("{}: {error}", policy_path.display())))?;
    let analysis_scope = value
        .get("analysis_scope")
        .and_then(Value::as_str)
        .unwrap_or("transport:dispatch")
        .to_owned();
    let config = SourceArchitectureConfig::from_json_value(&value)
        .map_err(|error| CliCheckError::Io(format!("{}: {error}", policy_path.display())))?;
    Ok(Some((config, analysis_scope)))
}

fn load_capability_environment(path: &Path) -> Result<Option<LoadedCapabilities>, CliCheckError> {
    let file_path = source_set_directory(path).join(CAPABILITIES_FILE);
    if !file_path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&file_path)
        .map_err(|error| CliCheckError::Io(format!("{}: {error}", file_path.display())))?;
    let environment = capability_environment_from_json(&text)
        .map_err(|error| CliCheckError::Io(format!("{}: {error}", file_path.display())))?;
    Ok(Some(LoadedCapabilities {
        digest: environment.stable_digest(),
        environment,
        path: CAPABILITIES_FILE.to_owned(),
    }))
}

fn capability_environment_from_json(text: &str) -> Result<CapabilityEnvironment, String> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| error.to_string())?;
    if let Some(name) = first_duplicate_object_key(text.as_bytes()) {
        return Err(format!("duplicate capability name {name}"));
    }
    let object = value
        .as_object()
        .ok_or_else(|| "capabilities must be a JSON object".to_owned())?;
    let mut environment = CapabilityEnvironment::new();
    for (interface_name, interface_value) in object {
        if interface_name.is_empty() {
            return Err("capability name must not be empty".to_owned());
        }
        let operations = interface_value.as_object().ok_or_else(|| {
            format!("{interface_name} must be an object of capability operations")
        })?;
        let mut interface = CapabilityInterface::new();
        for (operation_name, operation_value) in operations {
            if operation_name.is_empty() {
                return Err(format!("{interface_name} has an empty operation name"));
            }
            interface.insert_operation(
                operation_name,
                capability_operation_from_json(interface_name, operation_name, operation_value)?,
            );
        }
        environment.insert_interface(interface_name, interface);
    }
    Ok(environment)
}

fn capability_operation_from_json(
    interface_name: &str,
    operation_name: &str,
    value: &Value,
) -> Result<CapabilityOperation, String> {
    let object = value.as_object().ok_or_else(|| {
        format!("{interface_name}.{operation_name} must be a capability operation object")
    })?;
    let parameters = object
        .get("parameters")
        .ok_or_else(|| format!("{interface_name}.{operation_name}.parameters is required"))?;
    let parameters = parameters.as_array().ok_or_else(|| {
        format!("{interface_name}.{operation_name}.parameters must be an array of type names")
    })?;
    let parameters = parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            parameter.as_str().map(str::to_owned).ok_or_else(|| {
                format!("{interface_name}.{operation_name}.parameters.{index} must be a type name")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let result = object
        .get("result")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{interface_name}.{operation_name}.result is required"))?;
    match object
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("ordinary")
    {
        "ordinary" => Ok(CapabilityOperation::new(parameters, result)),
        "outbound" => Ok(CapabilityOperation::outbound(
            parameters,
            result,
            OutboundCapabilityMetadata {
                timeout_argument_index: json_usize(
                    object,
                    interface_name,
                    operation_name,
                    "timeout_argument_index",
                )?,
                cancellation_argument_index: json_usize(
                    object,
                    interface_name,
                    operation_name,
                    "cancellation_argument_index",
                )?,
                maximum_timeout_ms: json_u128(
                    object,
                    interface_name,
                    operation_name,
                    "maximum_timeout_ms",
                )?,
                timed_out_case_identity: json_string(
                    object,
                    interface_name,
                    operation_name,
                    "timed_out_case_identity",
                )?,
                cancelled_case_identity: json_string(
                    object,
                    interface_name,
                    operation_name,
                    "cancelled_case_identity",
                )?,
            },
        )),
        kind => Err(format!(
            "{interface_name}.{operation_name}.kind must be ordinary or outbound; it is {kind}"
        )),
    }
}

fn json_string(
    object: &serde_json::Map<String, Value>,
    interface_name: &str,
    operation_name: &str,
    field: &str,
) -> Result<String, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{interface_name}.{operation_name}.{field} is required"))
}

fn json_usize(
    object: &serde_json::Map<String, Value>,
    interface_name: &str,
    operation_name: &str,
    field: &str,
) -> Result<usize, String> {
    object
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| format!("{interface_name}.{operation_name}.{field} is required"))
}

fn json_u128(
    object: &serde_json::Map<String, Value>,
    interface_name: &str,
    operation_name: &str,
    field: &str,
) -> Result<u128, String> {
    object
        .get(field)
        .and_then(Value::as_u64)
        .map(u128::from)
        .ok_or_else(|| format!("{interface_name}.{operation_name}.{field} is required"))
}

fn first_duplicate_object_key(bytes: &[u8]) -> Option<String> {
    let mut index = 0;
    scan_json_value(bytes, &mut index, 0)
}

fn scan_json_value(bytes: &[u8], index: &mut usize, depth: usize) -> Option<String> {
    skip_json_whitespace(bytes, index);
    match bytes.get(*index).copied() {
        Some(b'{') => scan_json_object(bytes, index, depth),
        Some(b'[') => {
            *index += 1;
            loop {
                skip_json_whitespace(bytes, index);
                if bytes.get(*index) == Some(&b']') {
                    *index += 1;
                    return None;
                }
                if let Some(name) = scan_json_value(bytes, index, depth) {
                    return Some(name);
                }
                skip_json_whitespace(bytes, index);
                match bytes.get(*index) {
                    Some(b',') => *index += 1,
                    Some(b']') => {
                        *index += 1;
                        return None;
                    }
                    _ => return None,
                }
            }
        }
        Some(b'"') => {
            skip_json_string(bytes, index);
            None
        }
        Some(_) => {
            skip_json_primitive(bytes, index);
            None
        }
        None => None,
    }
}

fn scan_json_object(bytes: &[u8], index: &mut usize, depth: usize) -> Option<String> {
    *index += 1;
    let mut seen = BTreeSet::new();
    loop {
        skip_json_whitespace(bytes, index);
        if bytes.get(*index) == Some(&b'}') {
            *index += 1;
            return None;
        }
        let key = parse_json_string(bytes, index)?;
        if !seen.insert(key.clone()) {
            return Some(key);
        }
        skip_json_whitespace(bytes, index);
        if bytes.get(*index) != Some(&b':') {
            return None;
        }
        *index += 1;
        if let Some(name) = scan_json_value(bytes, index, depth.saturating_add(1)) {
            return Some(name);
        }
        skip_json_whitespace(bytes, index);
        match bytes.get(*index) {
            Some(b',') => *index += 1,
            Some(b'}') => {
                *index += 1;
                return None;
            }
            _ => return None,
        }
    }
}

fn skip_json_whitespace(bytes: &[u8], index: &mut usize) {
    while bytes.get(*index).is_some_and(u8::is_ascii_whitespace) {
        *index += 1;
    }
}

fn skip_json_string(bytes: &[u8], index: &mut usize) {
    let _ = parse_json_string(bytes, index);
}

fn parse_json_string(bytes: &[u8], index: &mut usize) -> Option<String> {
    if bytes.get(*index) != Some(&b'"') {
        return None;
    }
    *index += 1;
    let mut text = String::new();
    while *index < bytes.len() {
        match bytes[*index] {
            b'"' => {
                *index += 1;
                return Some(text);
            }
            b'\\' => {
                *index += 1;
                let escaped = *bytes.get(*index)?;
                *index += 1;
                match escaped {
                    b'"' | b'\\' | b'/' => text.push(char::from(escaped)),
                    b'b' => text.push('\u{0008}'),
                    b'f' => text.push('\u{000c}'),
                    b'n' => text.push('\n'),
                    b'r' => text.push('\r'),
                    b't' => text.push('\t'),
                    b'u' => {
                        let end = index.checked_add(4)?;
                        let hex = bytes.get(*index..end)?;
                        let code = std::str::from_utf8(hex).ok()?;
                        let scalar = u32::from_str_radix(code, 16).ok()?;
                        text.push(char::from_u32(scalar)?);
                        *index += 4;
                    }
                    _ => return None,
                }
            }
            byte => {
                text.push(char::from(byte));
                *index += 1;
            }
        }
    }
    None
}

fn skip_json_primitive(bytes: &[u8], index: &mut usize) {
    while bytes
        .get(*index)
        .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b',' | b'}' | b']'))
    {
        *index += 1;
    }
}

fn write_published_revision(
    path: &Path,
    workspace: &EvolutionWorkspace,
    behavior_validation: Option<BehaviorValidation>,
    capability_environment_path: Option<String>,
    capability_environment_digest: Option<String>,
) -> Result<PublishedRevision, String> {
    let revision = workspace
        .revision(PUBLISH_REVISION)
        .ok_or_else(|| "published revision was not retained".to_owned())?;
    let sources = workspace
        .sources(PUBLISH_REVISION)
        .ok_or_else(|| "published sources were not retained".to_owned())?;
    let store = path.join(REVISION_STORE);
    let staging = path.join(format!(
        "{REVISION_STORE}.staging.{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    if let Err(error) = write_revision_store(&staging, revision, sources) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let backup = path.join(format!("{REVISION_STORE}.backup.{}", std::process::id()));
    if store.exists() {
        fs::rename(&store, &backup).map_err(|error| {
            let _ = fs::remove_dir_all(&staging);
            format!("{}: {error}", store.display())
        })?;
    }
    if let Err(error) = fs::rename(&staging, &store) {
        if backup.exists() {
            let _ = fs::rename(&backup, &store);
        }
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("{}: {error}", store.display()));
    }
    if backup.exists() {
        let _ = fs::remove_dir_all(&backup);
    }
    Ok(PublishedRevision {
        revision_id: revision.revision_id.clone(),
        source_set_digest: revision.source_set_digest.clone(),
        store,
        behavior_validation,
        capability_environment_path,
        capability_environment_digest,
    })
}

fn write_revision_store(
    store: &Path,
    revision: &SourceSetRevision,
    sources: &[EvolutionSource],
) -> Result<(), String> {
    let revision_dir = store.join("revisions").join(&revision.revision_id);
    fs::create_dir_all(&revision_dir)
        .map_err(|error| format!("{}: {error}", revision_dir.display()))?;
    let sources_dir = revision_dir.join("sources");
    fs::create_dir_all(&sources_dir)
        .map_err(|error| format!("{}: {error}", sources_dir.display()))?;
    for source in sources {
        fs::write(sources_dir.join(&source.path), &source.source)
            .map_err(|error| format!("{}: {error}", source.path))?;
    }
    let document = serde_json::json!({
        "workspace_id": revision.workspace_id,
        "revision_id": revision.revision_id,
        "parent_revision_id": revision.parent_revision_id,
        "source_set_digest": revision.source_set_digest,
        "architecture_settings_digest": revision.architecture_settings_digest,
        "capability_environment_digest": revision.capability_environment_digest,
        "sources": revision.sources.iter().map(|source| {
            serde_json::json!({
                "path": source.path,
                "sha256": source.sha256,
            })
        }).collect::<Vec<_>>(),
    });
    fs::write(revision_dir.join("revision.json"), format!("{document}\n"))
        .map_err(|error| format!("revision.json: {error}"))?;
    fs::write(store.join("current"), format!("{}\n", revision.revision_id))
        .map_err(|error| format!("current: {error}"))?;
    Ok(())
}

fn workspace_id(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ailc")
        .to_owned()
}

fn collect_sources(path: &Path) -> Result<Vec<EvolutionSource>, CliCheckError> {
    if path.is_dir() {
        directory_sources(path)
    } else {
        Ok(vec![file_source(path)?])
    }
}

fn file_source(path: &Path) -> Result<EvolutionSource, CliCheckError> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            CliCheckError::Io(format!("{}: source path is not UTF-8", path.display()))
        })?;
    if !valid_source_path(name) {
        return Err(CliCheckError::Io(format!("{name}: invalid source path")));
    }
    let source = fs::read_to_string(path)
        .map_err(|error| CliCheckError::Io(format!("{}: {error}", path.display())))?;
    Ok(EvolutionSource::new(name, source))
}

fn directory_sources(path: &Path) -> Result<Vec<EvolutionSource>, CliCheckError> {
    let mut sources = Vec::new();
    let entries = fs::read_dir(path)
        .map_err(|error| CliCheckError::Io(format!("{}: {error}", path.display())))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| CliCheckError::Io(format!("{}: {error}", path.display())))?;
        let file_path = entry.path();
        if !file_path.is_file() {
            continue;
        }
        let Some(name) = file_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ail"))
            || !valid_source_path(name)
        {
            continue;
        }
        let source = fs::read_to_string(&file_path)
            .map_err(|error| CliCheckError::Io(format!("{}: {error}", file_path.display())))?;
        sources.push(EvolutionSource::new(name, source));
    }
    if sources.is_empty() {
        return Err(CliCheckError::Io(format!(
            "{}: no valid .ail source files",
            path.display()
        )));
    }
    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_json_loads_the_existing_environment_shape() {
        let environment = capability_environment_from_json(
            r#"{
  "JobsStore": {
    "insert": {
      "parameters": ["Job"],
      "result": "StoreOutcome"
    }
  }
}"#,
        )
        .expect("fixture shape loads");
        let store = environment
            .interface("JobsStore")
            .expect("JobsStore is declared");
        let insert = store.operation("insert").expect("insert is declared");
        assert_eq!(insert.parameters, ["Job"]);
        assert_eq!(insert.result, "StoreOutcome");
        assert!(matches!(
            insert.kind,
            crate::CapabilityOperationKind::Ordinary
        ));
    }

    #[test]
    fn capabilities_json_rejects_duplicate_capability_names() {
        let error = capability_environment_from_json(
            r#"{"JobsStore":{"insert":{"parameters":["Job"],"result":"StoreOutcome"}},"JobsStore":{"read":{"parameters":["Job"],"result":"StoreOutcome"}}}"#,
        )
        .expect_err("duplicate interface names are rejected");
        assert!(
            error.contains("duplicate capability name JobsStore"),
            "{error}"
        );
    }

    #[test]
    fn capabilities_json_rejects_a_shape_that_is_not_the_environment() {
        let error = capability_environment_from_json("[\"JobsStore\"]")
            .expect_err("an array is not CapabilityEnvironment");
        assert!(
            error.contains("capabilities must be a JSON object"),
            "{error}"
        );
    }

    #[test]
    fn architecture_failure_location_does_not_reparse_the_source_set() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/architecture-denied");
        let source_count = fs::read_dir(&path)
            .expect("example directory is readable")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "ail")
            })
            .count();

        crate::parser::reset_parse_calls();
        let result = check_cli_path(&path);

        assert!(matches!(result, Err(CliCheckError::Architecture(_))));
        assert_eq!(crate::parser::parse_calls(), source_count);
    }
}
