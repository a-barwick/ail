//! `ailc check` builds an [`EvolutionWorkspace`], not a one-file unit check.

use std::fs;
use std::path::Path;

use crate::{
    CapabilityEnvironment, EvolutionBuildFailure, EvolutionCoverage, EvolutionSource,
    EvolutionWorkspace, valid_source_path,
};

const CHECK_REVISION: &str = "check";

/// Failure from the `ailc check` driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCheckError {
    /// The path could not be read as a source set.
    Io(String),
    /// [`EvolutionWorkspace::new`] rejected the source set.
    Build(EvolutionBuildFailure),
}

/// Check a file or directory the same way `ailc check` does.
///
/// A directory becomes the `.ail` files whose names pass [`valid_source_path`].
/// A file becomes a one-file workspace named by its file name. The capability
/// environment is empty. Coverage is declared complete with no artifacts, the
/// same claim the composed-service example uses.
///
/// # Errors
///
/// Returns [`CliCheckError::Io`] when the path cannot be read, or
/// [`CliCheckError::Build`] when [`EvolutionWorkspace::new`] rejects the
/// source set.
pub fn check_cli_path(path: impl AsRef<Path>) -> Result<(), CliCheckError> {
    let path = path.as_ref();
    let sources = collect_sources(path)?;
    EvolutionWorkspace::new(
        workspace_id(path),
        CHECK_REVISION,
        sources,
        &CapabilityEnvironment::new(),
        EvolutionCoverage {
            declared_complete: true,
            ..EvolutionCoverage::default()
        },
    )
    .map(|_| ())
    .map_err(CliCheckError::Build)
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
    Ok(sources)
}
