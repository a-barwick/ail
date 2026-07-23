use std::collections::{BTreeMap, BTreeSet};

use ail_compiler::{
    ArchitectureCoverage, ArchitectureEdge, ArchitecturePolicyContext,
    ArchitectureRequestErrorKind, ArchitectureRevision, ArchitectureSnapshotInput,
    ArchitectureSnapshotRequest, ArchitectureSnapshotResult, ArchitectureUnit, BaselineMatch,
    ControlFlowGraph, DispatchBudget, GroupDependencies, NewUnitBudget, architecture_snapshot,
};
use serde_json::{Value, json};

#[allow(clippy::too_many_lines)]
fn fixtures() -> (
    ArchitectureRevision,
    ArchitectureSnapshotInput,
    Value,
    Value,
) {
    let workspace: Value = serde_json::from_str(include_str!(
        "../../../specs/architecture-acceptance-fixtures/workspace.json"
    ))
    .unwrap();
    let results: Value = serde_json::from_str(include_str!(
        "../../../specs/architecture-fixtures/results.json"
    ))
    .unwrap();
    let package: Value =
        serde_json::from_str(include_str!("../../../specs/architecture-acceptance.json")).unwrap();
    let expected = results["operations"][0]["expected_response"].clone();
    let incomplete = results["operations"][4]["expected_response"].clone();
    let mut units = workspace["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| ArchitectureUnit {
            id: unit["id"].as_str().unwrap().into(),
            module: unit["module"].as_str().unwrap().into(),
            group: unit["group"].as_str().unwrap().into(),
            cfg: ControlFlowGraph {
                nodes: unit["cfg"]["nodes"].as_u64().unwrap().try_into().unwrap(),
                edges: unit["cfg"]["edges"].as_u64().unwrap().try_into().unwrap(),
            },
            capabilities: vec![],
            state_reads: vec![],
            state_writes: vec![],
        })
        .collect::<Vec<_>>();
    let expansion = &workspace["operation_expansion"];
    let modules = ["contracts", "transport", "transport", "domain", "tests"];
    for operation in workspace["operations"].as_array().unwrap() {
        let id = operation["id"].as_str().unwrap();
        for index in 0..5 {
            let cfg = if index == 0 {
                json!({"nodes":1,"edges":0})
            } else {
                expansion[[
                    "",
                    "registration_cfg",
                    "adapter_cfg",
                    "handler_cfg",
                    "test_cfg",
                ][index]]
                    .clone()
            };
            units.push(ArchitectureUnit {
                id: expansion["units"][index]
                    .as_str()
                    .unwrap()
                    .replace("{id}", id),
                module: modules[index].into(),
                group: expansion["groups"][index].as_str().unwrap().into(),
                cfg: ControlFlowGraph {
                    nodes: cfg["nodes"].as_u64().unwrap().try_into().unwrap(),
                    edges: cfg["edges"].as_u64().unwrap().try_into().unwrap(),
                },
                capabilities: vec![],
                state_reads: vec![],
                state_writes: vec![],
            });
        }
    }
    let mut edges = vec![];
    for operation in workspace["operations"].as_array().unwrap() {
        let id = operation["id"].as_str().unwrap();
        for edge in expansion["edges"].as_array().unwrap() {
            edges.push(ArchitectureEdge {
                source: edge[0].as_str().unwrap().replace("{id}", id),
                target: edge[1].as_str().unwrap().replace("{id}", id),
                kind: edge[2].as_str().unwrap().into(),
            });
        }
    }
    let policy = &package["policy"];
    let baseline = &workspace["baseline"];
    let baseline_match = BaselineMatch {
        baseline_revision: workspace["baseline_revision"].as_str().unwrap().into(),
        scope: baseline["scope"].as_str().unwrap().into(),
        metrics: DispatchBudget {
            control_flow_complexity: baseline["control_flow_complexity"]
                .as_u64()
                .unwrap()
                .try_into()
                .unwrap(),
            minimal_context_node_count: baseline["minimal_context_node_count"]
                .as_u64()
                .unwrap()
                .try_into()
                .unwrap(),
        },
        accepted_debt: true,
    };
    let revision = ArchitectureRevision::new(
        "m23-job-service".into(),
        "arch-r1".into(),
        "m24-v1".into(),
        units,
        edges,
        workspace["endpoint_groups"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.clone(), value.as_str().unwrap().into()))
            .collect::<BTreeMap<_, _>>(),
        ArchitectureCoverage {
            analyzed_groups: workspace["coverage"]["analyzed"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().into())
                .collect(),
            unchecked_boundaries: workspace["coverage"]["unchecked"]
                .as_array()
                .unwrap()
                .clone(),
            complete_for_policy: true,
        },
        ArchitecturePolicyContext {
            revision: "m23-policy-r1".into(),
            allowed_group_dependencies: GroupDependencies {
                contract: strings(&policy["allowed_group_dependencies"]["contract"]),
                transport: strings(&policy["allowed_group_dependencies"]["transport"]),
                domain: strings(&policy["allowed_group_dependencies"]["domain"]),
                persistence_adapter: strings(
                    &policy["allowed_group_dependencies"]["persistence-adapter"],
                ),
                verification: strings(&policy["allowed_group_dependencies"]["verification"]),
            },
            transport_capabilities: strings(&policy["transport_capabilities"]),
            transport_state: strings(&policy["transport_state"]),
            dispatch_no_growth: DispatchBudget {
                control_flow_complexity: policy["dispatch_no_growth"]["control_flow_complexity"]
                    .as_u64()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                minimal_context_node_count:
                    policy["dispatch_no_growth"]["minimal_context_node_count"]
                        .as_u64()
                        .unwrap()
                        .try_into()
                        .unwrap(),
            },
            new_unit: NewUnitBudget {
                control_flow_complexity_max: policy["new_unit"]["control_flow_complexity_max"]
                    .as_u64()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                minimal_context_node_count_max:
                    policy["new_unit"]["minimal_context_node_count_max"]
                        .as_u64()
                        .unwrap()
                        .try_into()
                        .unwrap(),
            },
            new_cycles: policy["new_cycles"].as_bool().unwrap(),
            coverage_required: policy["coverage_required"].as_bool().unwrap(),
            baseline_match,
        },
    )
    .unwrap();
    let operation_input = &results["operations"][0]["input"];
    let input = ArchitectureSnapshotInput {
        request: ArchitectureSnapshotRequest {
            revision_id: operation_input["request"]["revision_id"]
                .as_str()
                .unwrap()
                .into(),
            analysis_scope: operation_input["request"]["analysis_scope"]
                .as_str()
                .unwrap()
                .into(),
        },
        active_exceptions: Vec::new(),
        active_policy_revision: operation_input["active_policy_revision"]
            .as_str()
            .unwrap()
            .into(),
        active_baseline_revision: operation_input["active_baseline_revision"]
            .as_str()
            .unwrap()
            .into(),
    };
    (revision, input, expected, incomplete)
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().into())
        .collect()
}

#[test]
fn exact_r1_snapshot_is_complete_ordered_and_repeatable() {
    let (revision, input, expected, _) = fixtures();
    let first = architecture_snapshot(&revision, &input).unwrap();
    assert_eq!(first.to_json_value(), expected);
    assert_eq!(first, architecture_snapshot(&revision, &input).unwrap());
    let ArchitectureSnapshotResult::Success(success) = first else {
        panic!("expected success")
    };
    assert_eq!(
        success
            .snapshot
            .scopes
            .iter()
            .map(|s| s.scope_kind.as_str())
            .collect::<Vec<_>>(),
        [
            "executable-unit",
            "module",
            "dependency-component",
            "architecture-group"
        ]
    );
    for scope in success.snapshot.scopes {
        assert!(!scope.control_flow_complexity.contributors.is_empty());
        let _ = (
            &scope.direct_dependency_set,
            &scope.declared_capability_set,
            &scope.state_read_set,
            &scope.state_write_set,
            scope.dependency_component_size,
            scope.minimal_context_node_count,
        );
    }
}

#[test]
fn exact_r1_coverage_failure_has_no_snapshot() {
    let (mut revision, input, _, expected) = fixtures();
    revision.coverage.complete_for_policy = false;
    revision.coverage.analyzed_groups.pop();
    let actual = architecture_snapshot(&revision, &input)
        .unwrap()
        .to_json_value();
    assert_eq!(actual, expected);
    assert!(actual.get("snapshot").is_none());

    let (mut forged, input, _, _) = fixtures();
    forged.coverage.analyzed_groups.pop();
    assert!(forged.coverage.complete_for_policy);
    let forged = architecture_snapshot(&forged, &input)
        .unwrap()
        .to_json_value();
    assert_eq!(forged["status"], "incomplete");
    assert_eq!(forged["coverage"]["complete_for_policy"], false);
}

#[test]
fn every_fixed_budget_returns_bounded_incomplete() {
    for dimension in [
        "semantic_nodes",
        "typed_edges",
        "structured_bytes",
        "compact_bytes",
    ] {
        let (mut revision, mut input, _, _) = fixtures();
        match dimension {
            "semantic_nodes" => {
                for i in 0..391 {
                    revision.units.push(ArchitectureUnit {
                        id: format!("node:{i}"),
                        module: "contracts".into(),
                        group: "contract".into(),
                        cfg: ControlFlowGraph { nodes: 1, edges: 0 },
                        capabilities: vec![],
                        state_reads: vec![],
                        state_writes: vec![],
                    });
                }
            }
            "typed_edges" => {
                let mut triples = revision
                    .edges
                    .iter()
                    .map(|edge| (edge.source.clone(), edge.target.clone(), edge.kind.clone()))
                    .collect::<BTreeSet<_>>();
                let ids = revision
                    .units
                    .iter()
                    .map(|unit| unit.id.clone())
                    .filter(|id| !id.starts_with("transport:"))
                    .collect::<Vec<_>>();
                'outer: for source in &ids {
                    for target in &ids {
                        let triple = (source.clone(), target.clone(), "verifies".into());
                        if triples.insert(triple.clone()) {
                            revision.edges.push(ArchitectureEdge {
                                source: triple.0,
                                target: triple.1,
                                kind: triple.2,
                            });
                        }
                        if revision.edges.len() == 2049 {
                            break 'outer;
                        }
                    }
                }
            }
            "structured_bytes" => {
                for index in 0..20 {
                    revision.units.push(ArchitectureUnit {
                        id: format!("transport:budget:{index}:{}", "x".repeat(1_500)),
                        module: "transport".into(),
                        group: "transport".into(),
                        cfg: ControlFlowGraph { nodes: 1, edges: 0 },
                        capabilities: vec![],
                        state_reads: vec![],
                        state_writes: vec![],
                    });
                }
            }
            "compact_bytes" => {
                input.request.analysis_scope = format!("domain:{}", "x".repeat(3_000));
                revision.units.push(ArchitectureUnit {
                    id: input.request.analysis_scope.clone(),
                    module: "budget".into(),
                    group: "domain".into(),
                    cfg: ControlFlowGraph { nodes: 1, edges: 0 },
                    capabilities: vec![],
                    state_reads: vec![],
                    state_writes: vec![],
                });
            }
            _ => unreachable!(),
        }
        let result = architecture_snapshot(&revision, &input).unwrap();
        let value = result.to_json_value();
        assert_eq!(value["status"], "incomplete", "{dimension}");
        assert_eq!(value["budgets"]["exhausted"], dimension, "{dimension}");
        assert!(value["budgets"]["structured_bytes"].as_u64().unwrap() > 0);
        assert_eq!(value["diagnostics"][0]["facts"]["exhausted"], dimension);
        assert_eq!(
            value["diagnostics"][0]["facts"]["used"],
            value["budgets"][dimension]
        );
        assert!(value.get("snapshot").is_none());
        assert!(
            serde_json::to_string_pretty(&value).unwrap().len() < 65_536,
            "{dimension}"
        );
    }
}

#[test]
fn malformed_and_stale_requests_do_not_panic_or_look_incomplete() {
    let (revision, mut input, _, _) = fixtures();
    input.request.revision_id = "arch-stale".into();
    assert_eq!(
        architecture_snapshot(&revision, &input).unwrap_err().kind,
        ArchitectureRequestErrorKind::StaleRevision
    );

    let (mut malformed, input, _, _) = fixtures();
    malformed.units.push(malformed.units[0].clone());
    assert_eq!(
        architecture_snapshot(&malformed, &input).unwrap_err().kind,
        ArchitectureRequestErrorKind::InvalidRevision
    );

    let (mut duplicate_edge, input, _, _) = fixtures();
    duplicate_edge.edges.push(duplicate_edge.edges[0].clone());
    assert_eq!(
        architecture_snapshot(&duplicate_edge, &input)
            .unwrap_err()
            .kind,
        ArchitectureRequestErrorKind::InvalidRevision
    );

    let (mut unknown_endpoint, input, _, _) = fixtures();
    unknown_endpoint.edges.push(ArchitectureEdge {
        source: "transport:dispatch".into(),
        target: "unknown:endpoint".into(),
        kind: "calls".into(),
    });
    assert_eq!(
        architecture_snapshot(&unknown_endpoint, &input)
            .unwrap_err()
            .kind,
        ArchitectureRequestErrorKind::InvalidRevision
    );

    let (mut malformed_capability, input, _, _) = fixtures();
    malformed_capability.edges.push(ArchitectureEdge {
        source: "transport:dispatch".into(),
        target: "state:jobs".into(),
        kind: "capability-use".into(),
    });
    assert_eq!(
        architecture_snapshot(&malformed_capability, &input)
            .unwrap_err()
            .kind,
        ArchitectureRequestErrorKind::InvalidRevision
    );

    let (mut invalid_cfg, input, _, _) = fixtures();
    invalid_cfg.units[0].cfg = ControlFlowGraph {
        nodes: usize::MAX,
        edges: usize::MAX,
    };
    assert_eq!(
        architecture_snapshot(&invalid_cfg, &input)
            .unwrap_err()
            .kind,
        ArchitectureRequestErrorKind::InvalidRevision
    );

    let (mut stale_facts, input, _, _) = fixtures();
    stale_facts.units[0].capabilities = vec!["jobs_store".into()];
    assert_eq!(
        architecture_snapshot(&stale_facts, &input)
            .unwrap_err()
            .kind,
        ArchitectureRequestErrorKind::InvalidRevision
    );

    let (mut overflowing_aggregate, input, _, _) = fixtures();
    overflowing_aggregate.units.truncate(2);
    overflowing_aggregate.edges.clear();
    overflowing_aggregate.units[0].cfg = ControlFlowGraph {
        nodes: 1,
        edges: usize::MAX - 2,
    };
    overflowing_aggregate.units[1].cfg = ControlFlowGraph { nodes: 1, edges: 1 };
    assert_eq!(
        architecture_snapshot(&overflowing_aggregate, &input)
            .unwrap_err()
            .kind,
        ArchitectureRequestErrorKind::InvalidRevision
    );
}
