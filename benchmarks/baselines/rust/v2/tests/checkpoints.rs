use std::{fmt::Write as _, fs, path::PathBuf};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
struct CheckpointManifest {
    checkpoints: Vec<Checkpoint>,
}

#[derive(Debug, Deserialize)]
struct Checkpoint {
    id: String,
    source_tree_sha256: String,
    files: Vec<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("v2 crate is nested under benchmarks/baselines/rust")
        .to_path_buf()
}

#[test]
fn checkpoint_source_trees_match_their_frozen_digests() {
    let root = repo_root();
    let manifest: CheckpointManifest = serde_json::from_slice(
        &fs::read(root.join("benchmarks/baselines/rust/checkpoints.json"))
            .expect("read checkpoint manifest"),
    )
    .expect("parse checkpoint manifest");
    assert_eq!(manifest.checkpoints.len(), 2);

    for checkpoint in manifest.checkpoints {
        let mut records = String::new();
        for path in &checkpoint.files {
            let digest = format!(
                "{:x}",
                Sha256::digest(fs::read(root.join(path)).expect("read checkpoint source"))
            );
            writeln!(records, "{digest}  {path}").expect("write digest record");
        }
        let actual = format!("{:x}", Sha256::digest(records));
        assert_eq!(
            actual, checkpoint.source_tree_sha256,
            "{} checkpoint changed without updating its digest",
            checkpoint.id
        );
    }
}

#[test]
fn rust_seed_locations_cover_every_frozen_seed_category_once() {
    let root = repo_root();
    let hidden: Value = serde_json::from_slice(
        &fs::read(root.join("benchmarks/contracts/hidden-contract.json"))
            .expect("read hidden contract"),
    )
    .expect("parse hidden contract");
    let locations: Value = serde_json::from_slice(
        &fs::read(root.join("benchmarks/baselines/rust/seed-locations.json"))
            .expect("read Rust seed locations"),
    )
    .expect("parse Rust seed locations");

    let expected = hidden["seed_categories"]
        .as_array()
        .expect("hidden seed categories")
        .iter()
        .map(|entry| entry["id"].as_str().expect("hidden seed id"))
        .collect::<Vec<_>>();
    let actual_entries = locations["locations"]
        .as_array()
        .expect("Rust seed locations");
    let actual = actual_entries
        .iter()
        .map(|entry| entry["seed_id"].as_str().expect("Rust seed id"))
        .collect::<Vec<_>>();

    assert_eq!(locations["language"], "rust");
    assert_eq!(actual, expected);
    assert!(actual_entries.iter().all(|entry| {
        entry["semantic_locations"]
            .as_array()
            .is_some_and(|values| !values.is_empty())
    }));
}
