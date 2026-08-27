//! Run the frozen bytes of a published revision, never the live workspace files.
//!
//! [`load_published_program`] reads only
//! `<dir>/.ail/revisions/<current>/sources/`. It never opens the `.ail` files
//! that sit next to the store, so a live edit that was never published cannot
//! change what runs. The loader refuses when no published revision exists, when
//! the frozen bytes disagree with the digests `ailc publish` recorded, or when
//! the frozen source set no longer passes the compiler checks.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{
    CapabilityEnvironment, CapabilityProvider, EvolutionBuildFailure, EvolutionCoverage,
    EvolutionSource, EvolutionWorkspace, ExecutionResponse, RuntimeValue, source_digest,
    valid_source_path,
};

const REVISION_STORE: &str = ".ail";
const CURRENT_POINTER: &str = "current";
const REVISIONS_DIRECTORY: &str = "revisions";
const SOURCES_DIRECTORY: &str = "sources";
const REVISION_DOCUMENT: &str = "revision.json";

/// Why the runner refused to run a published revision.
///
/// Every variant carries the fact the runner measured. The runner never falls
/// back to the live workspace files and never runs a partially verified
/// revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunRefusal {
    /// The directory holds no published revision to run.
    NoPublishedRevision {
        /// Store path the runner looked for.
        store: PathBuf,
        /// The exact missing artifact.
        reason: String,
    },
    /// The store exists but cannot be read as one complete revision.
    UnreadableStore {
        /// Path that could not be read or understood.
        path: PathBuf,
        /// The exact read or structure failure.
        reason: String,
    },
    /// One frozen source file's bytes disagree with the recorded digest.
    FrozenSourceDigest {
        /// Source path inside the revision.
        path: String,
        /// Digest `ailc publish` recorded for those bytes.
        expected_sha256: String,
        /// Digest of the bytes now on disk.
        actual_sha256: String,
    },
    /// The frozen source set disagrees with the recorded source-set digest.
    FrozenSourceSetDigest {
        /// Digest `ailc publish` recorded for the ordered source set.
        expected: String,
        /// Digest the compiler computed from the loaded frozen bytes.
        actual: String,
    },
    /// The revision was checked under a different capability environment.
    CapabilityEnvironmentDigest {
        /// Digest recorded when the revision was published.
        expected: String,
        /// Digest of the empty capability environment the runner supplies.
        actual: String,
    },
    /// The frozen bytes no longer pass the compiler checks.
    FrozenSourceRejected(EvolutionBuildFailure),
}

impl RunRefusal {
    /// Stable refusal code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoPublishedRevision { .. } => "AIL.RUN.NO_PUBLISHED_REVISION",
            Self::UnreadableStore { .. } => "AIL.RUN.UNREADABLE_STORE",
            Self::FrozenSourceDigest { .. } => "AIL.RUN.FROZEN_SOURCE_DIGEST",
            Self::FrozenSourceSetDigest { .. } => "AIL.RUN.FROZEN_SOURCE_SET_DIGEST",
            Self::CapabilityEnvironmentDigest { .. } => "AIL.RUN.CAPABILITY_ENVIRONMENT_DIGEST",
            Self::FrozenSourceRejected(_) => "AIL.RUN.FROZEN_SOURCE_REJECTED",
        }
    }

    /// Render the code and the facts the runner measured, one line per fact.
    #[must_use]
    pub fn render(&self) -> String {
        let code = self.code();
        match self {
            Self::NoPublishedRevision { store, reason } => {
                format!("{code} store={} reason={reason}", store.display())
            }
            Self::UnreadableStore { path, reason } => {
                format!("{code} path={} reason={reason}", path.display())
            }
            Self::FrozenSourceDigest {
                path,
                expected_sha256,
                actual_sha256,
            } => format!(
                "{code} path={path} expected.sha256={expected_sha256} actual.sha256={actual_sha256}"
            ),
            Self::FrozenSourceSetDigest { expected, actual } => format!(
                "{code} expected.source_set_digest={expected} actual.source_set_digest={actual}"
            ),
            Self::CapabilityEnvironmentDigest { expected, actual } => format!(
                "{code} expected.capability_environment_digest={expected} \
                 actual.capability_environment_digest={actual}"
            ),
            Self::FrozenSourceRejected(failure) => {
                let mut lines = vec![code.to_owned()];
                for finding in &failure.findings {
                    lines.push(finding.render());
                }
                for cause in &failure.causes {
                    if !failure
                        .findings
                        .iter()
                        .any(|finding| finding.code == *cause)
                    {
                        lines.push(cause.clone());
                    }
                }
                lines.join("\n")
            }
        }
    }
}

/// One verified published revision, loaded from frozen bytes only.
#[derive(Debug, Clone)]
pub struct PublishedProgram {
    store: PathBuf,
    revision_id: String,
    source_set_digest: String,
    capability_environment_digest: String,
    sources: Vec<EvolutionSource>,
    workspace: EvolutionWorkspace,
}

impl PublishedProgram {
    /// Store the frozen bytes were read from.
    #[must_use]
    pub fn store(&self) -> &Path {
        &self.store
    }

    /// Identity of the revision that runs.
    #[must_use]
    pub fn revision_id(&self) -> &str {
        &self.revision_id
    }

    /// Digest binding this run to the exact ordered frozen source set.
    #[must_use]
    pub fn source_set_digest(&self) -> &str {
        &self.source_set_digest
    }

    /// Digest of the capability environment this revision was checked under.
    #[must_use]
    pub fn capability_environment_digest(&self) -> &str {
        &self.capability_environment_digest
    }

    /// The frozen source files that run, in revision order.
    #[must_use]
    pub fn frozen_sources(&self) -> &[EvolutionSource] {
        &self.sources
    }

    /// Execute one entry function from the frozen revision.
    ///
    /// The capability environment is empty, so a published function that
    /// declares a capability parameter fails with the existing
    /// `AIL.RUNTIME.MISSING_CAPABILITY` fault instead of gaining ambient
    /// authority.
    #[must_use]
    pub fn run(
        &self,
        function: &str,
        arguments: Vec<RuntimeValue>,
        capabilities: &mut dyn CapabilityProvider,
    ) -> ExecutionResponse {
        self.workspace
            .execute(&self.revision_id, function, arguments, capabilities)
    }
}

/// A capability provider that supplies nothing.
///
/// The runner has no host or project configuration that declares capability
/// instances, so it supplies none. Any capability a published function needs
/// fails honestly at the parameter that requires it.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoCapabilities;

impl CapabilityProvider for NoCapabilities {
    fn supports(&self, _receiver: &str, _interface: &str) -> bool {
        false
    }

    fn call(
        &mut self,
        receiver: &str,
        interface: &str,
        operation: &str,
        _arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, crate::RuntimeFault> {
        Err(crate::RuntimeFault::new(
            "AIL.RUNTIME.MISSING_CAPABILITY",
            crate::Span::empty(0),
            [
                ("receiver", receiver),
                ("interface", interface),
                ("operation", operation),
            ],
            std::iter::empty::<(&str, &str)>(),
        ))
    }
}

/// Load the published revision of a workspace directory from frozen bytes.
///
/// Reads `<dir>/.ail/current`, the revision document it names, and every source
/// file that document lists. The live `.ail` files in `dir` are never read.
///
/// # Errors
///
/// Returns [`RunRefusal::NoPublishedRevision`] when the directory holds no
/// published revision, [`RunRefusal::UnreadableStore`] when the store is not one
/// complete revision, a digest refusal when the frozen bytes disagree with the
/// recorded digests, or [`RunRefusal::FrozenSourceRejected`] when the frozen
/// source set no longer passes the compiler checks.
pub fn load_published_program(path: impl AsRef<Path>) -> Result<PublishedProgram, RunRefusal> {
    let directory = path.as_ref();
    let store = directory.join(REVISION_STORE);
    if !store.is_dir() {
        return Err(RunRefusal::NoPublishedRevision {
            store,
            reason: "revision store directory is absent".to_owned(),
        });
    }
    let pointer_path = store.join(CURRENT_POINTER);
    let revision_id = match fs::read_to_string(&pointer_path) {
        Ok(text) => text.trim().to_owned(),
        Err(error) => {
            return Err(RunRefusal::NoPublishedRevision {
                store,
                reason: format!("{CURRENT_POINTER}: {error}"),
            });
        }
    };
    if revision_id.is_empty() {
        return Err(RunRefusal::NoPublishedRevision {
            store,
            reason: format!("{CURRENT_POINTER} names no revision"),
        });
    }
    if !valid_source_path(&revision_id) || revision_id.contains('/') {
        return Err(RunRefusal::UnreadableStore {
            path: pointer_path,
            reason: format!("{CURRENT_POINTER} is not a revision identity"),
        });
    }
    let revision_directory = store.join(REVISIONS_DIRECTORY).join(&revision_id);
    if !revision_directory.is_dir() {
        return Err(RunRefusal::NoPublishedRevision {
            store,
            reason: format!("revision {revision_id} has no stored sources"),
        });
    }

    let document = read_revision_document(&revision_directory)?;
    if document.revision_id != revision_id {
        return Err(RunRefusal::UnreadableStore {
            path: revision_directory.join(REVISION_DOCUMENT),
            reason: format!(
                "revision_id {} does not match {CURRENT_POINTER} {revision_id}",
                document.revision_id
            ),
        });
    }

    let sources_directory = revision_directory.join(SOURCES_DIRECTORY);
    let sources = read_frozen_sources(&sources_directory, &document)?;

    let capabilities = CapabilityEnvironment::new();
    let capability_environment_digest = capabilities.stable_digest();
    if capability_environment_digest != document.capability_environment_digest {
        return Err(RunRefusal::CapabilityEnvironmentDigest {
            expected: document.capability_environment_digest,
            actual: capability_environment_digest,
        });
    }

    let workspace = EvolutionWorkspace::new(
        workspace_id(directory, &document),
        &revision_id,
        sources.clone(),
        &capabilities,
        EvolutionCoverage {
            declared_complete: true,
            ..EvolutionCoverage::default()
        },
    )
    .map_err(RunRefusal::FrozenSourceRejected)?;

    let built = workspace
        .revision(&revision_id)
        .ok_or_else(|| RunRefusal::UnreadableStore {
            path: revision_directory.clone(),
            reason: "loaded revision was not retained".to_owned(),
        })?;
    if built.source_set_digest != document.source_set_digest {
        return Err(RunRefusal::FrozenSourceSetDigest {
            expected: document.source_set_digest,
            actual: built.source_set_digest.clone(),
        });
    }

    Ok(PublishedProgram {
        store,
        revision_id,
        source_set_digest: document.source_set_digest,
        capability_environment_digest,
        sources,
        workspace,
    })
}

struct RevisionDocument {
    workspace_id: Option<String>,
    revision_id: String,
    source_set_digest: String,
    capability_environment_digest: String,
    sources: Vec<(String, String)>,
}

fn read_revision_document(revision_directory: &Path) -> Result<RevisionDocument, RunRefusal> {
    let document_path = revision_directory.join(REVISION_DOCUMENT);
    let unreadable = |reason: String| RunRefusal::UnreadableStore {
        path: document_path.clone(),
        reason,
    };
    let text = fs::read_to_string(&document_path).map_err(|error| unreadable(error.to_string()))?;
    let value =
        serde_json::from_str::<Value>(&text).map_err(|error| unreadable(error.to_string()))?;
    let string = |field: &str| -> Result<String, RunRefusal> {
        value
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| unreadable(format!("{field} must be a string")))
    };
    let entries = value
        .get("sources")
        .and_then(Value::as_array)
        .ok_or_else(|| unreadable("sources must be an array".to_owned()))?;
    if entries.is_empty() {
        return Err(unreadable("sources is empty".to_owned()));
    }
    let mut sources = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = entry
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| unreadable("sources[].path must be a string".to_owned()))?;
        if !valid_source_path(path) || path.contains('/') {
            return Err(unreadable(format!("{path}: invalid source path")));
        }
        let sha256 = entry
            .get("sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| unreadable("sources[].sha256 must be a string".to_owned()))?;
        sources.push((path.to_owned(), sha256.to_owned()));
    }
    Ok(RevisionDocument {
        workspace_id: value
            .get("workspace_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        revision_id: string("revision_id")?,
        source_set_digest: string("source_set_digest")?,
        capability_environment_digest: string("capability_environment_digest")?,
        sources,
    })
}

fn read_frozen_sources(
    sources_directory: &Path,
    document: &RevisionDocument,
) -> Result<Vec<EvolutionSource>, RunRefusal> {
    let mut sources = Vec::with_capacity(document.sources.len());
    let mut listed = BTreeSet::new();
    for (path, expected_sha256) in &document.sources {
        let file = sources_directory.join(path);
        let text = fs::read_to_string(&file).map_err(|error| RunRefusal::UnreadableStore {
            path: file.clone(),
            reason: error.to_string(),
        })?;
        let actual_sha256 = digest_hex(&text);
        if actual_sha256 != *expected_sha256 {
            return Err(RunRefusal::FrozenSourceDigest {
                path: path.clone(),
                expected_sha256: expected_sha256.clone(),
                actual_sha256,
            });
        }
        if !listed.insert(path.clone()) {
            return Err(RunRefusal::UnreadableStore {
                path: file,
                reason: format!("{path}: duplicate source entry"),
            });
        }
        sources.push(EvolutionSource::new(path.clone(), text));
    }

    // A file inside the frozen sources directory that the revision document
    // does not list means the runner cannot say which bytes the revision is.
    let entries = fs::read_dir(sources_directory).map_err(|error| RunRefusal::UnreadableStore {
        path: sources_directory.to_path_buf(),
        reason: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| RunRefusal::UnreadableStore {
            path: sources_directory.to_path_buf(),
            reason: error.to_string(),
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy().into_owned();
        if !listed.contains(&name) {
            return Err(RunRefusal::UnreadableStore {
                path: entry.path(),
                reason: format!("{name}: frozen source is not listed in {REVISION_DOCUMENT}"),
            });
        }
    }
    Ok(sources)
}

fn digest_hex(source: &str) -> String {
    source_digest(source)
        .strip_prefix("sha256:")
        .expect("source digest carries the sha256 prefix")
        .to_owned()
}

fn workspace_id(directory: &Path, document: &RevisionDocument) -> String {
    document.workspace_id.clone().unwrap_or_else(|| {
        directory
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("published")
            .to_owned()
    })
}
