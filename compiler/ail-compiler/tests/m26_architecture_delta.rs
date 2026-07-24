use ail_compiler::{
    ArchitectureCoverage, ArchitectureEdge, ArchitectureEvaluationInput, ArchitectureException,
    ArchitecturePolicyContext, ArchitectureRequest, ArchitectureRequestErrorKind,
    ArchitectureRevision, ArchitectureUnit, ArchitectureWorkspace, BaselineMatch,
    BehaviorValidation, ControlFlowGraph, DispatchBudget, GovernanceAuthorization,
    GovernanceChange, GroupDependencies, NewUnitBudget, validate_architecture_change,
};
use serde_json::{Value, json};

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().into())
        .collect()
}

fn usize_value(value: &Value) -> usize {
    usize::try_from(value.as_u64().unwrap()).unwrap()
}

fn unit(value: &Value, module: String) -> ArchitectureUnit {
    ArchitectureUnit {
        id: value["id"].as_str().unwrap().into(),
        module,
        group: value["group"].as_str().unwrap().into(),
        cfg: ControlFlowGraph {
            nodes: usize_value(&value["cfg"]["nodes"]),
            edges: usize_value(&value["cfg"]["edges"]),
        },
        capabilities: strings(&value["capabilities"]),
        state_reads: strings(&value["state_reads"]),
        state_writes: strings(&value["state_writes"]),
    }
}

fn module_for(value: &Value, workspace: &Value) -> String {
    if let Some(found) = workspace["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == value["id"])
    {
        return found["module"].as_str().unwrap().into();
    }
    match value["group"].as_str().unwrap() {
        "contract" => "contracts",
        "transport" => "transport",
        "domain" => "domain",
        "persistence-adapter" => "adapters",
        "verification" => "tests",
        group => panic!("unknown group {group}"),
    }
    .into()
}

fn edge(value: &Value) -> ArchitectureEdge {
    ArchitectureEdge {
        source: value[0].as_str().unwrap().into(),
        target: value[1].as_str().unwrap().into(),
        kind: value[2].as_str().unwrap().into(),
    }
}

fn base_revision(workspace: &Value, package: &Value) -> ArchitectureRevision {
    let mut units = workspace["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| unit(u, u["module"].as_str().unwrap().into()))
        .collect::<Vec<_>>();
    let expansion = &workspace["operation_expansion"];
    for operation in workspace["operations"].as_array().unwrap() {
        let id = operation["id"].as_str().unwrap();
        for (index, template) in expansion["units"].as_array().unwrap().iter().enumerate() {
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
            let value = json!({"id":template.as_str().unwrap().replace("{id}",id),"group":expansion["groups"][index],"cfg":cfg,"capabilities":[],"state_reads":[],"state_writes":[]});
            units.push(unit(&value, module_for(&value, workspace)));
        }
    }
    let mut edges = Vec::new();
    for operation in workspace["operations"].as_array().unwrap() {
        let id = operation["id"].as_str().unwrap();
        for value in expansion["edges"].as_array().unwrap() {
            edges.push(ArchitectureEdge {
                source: value[0].as_str().unwrap().replace("{id}", id),
                target: value[1].as_str().unwrap().replace("{id}", id),
                kind: value[2].as_str().unwrap().into(),
            });
        }
    }
    let p = &package["policy"];
    let b = &workspace["baseline"];
    let dispatch = DispatchBudget {
        control_flow_complexity: usize_value(&b["control_flow_complexity"]),
        minimal_context_node_count: usize_value(&b["minimal_context_node_count"]),
    };
    ArchitectureRevision::new(
        "m23-job-service".into(),
        workspace["revision"].as_str().unwrap().into(),
        "m24-v1".into(),
        units,
        edges,
        workspace["endpoint_groups"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().into()))
            .collect(),
        ArchitectureCoverage {
            analyzed_groups: strings(&workspace["coverage"]["analyzed"]),
            unchecked_boundaries: workspace["coverage"]["unchecked"]
                .as_array()
                .unwrap()
                .clone(),
            complete_for_policy: true,
        },
        ArchitecturePolicyContext {
            revision: workspace["policy_revision"].as_str().unwrap().into(),
            allowed_group_dependencies: GroupDependencies {
                contract: strings(&p["allowed_group_dependencies"]["contract"]),
                transport: strings(&p["allowed_group_dependencies"]["transport"]),
                domain: strings(&p["allowed_group_dependencies"]["domain"]),
                persistence_adapter: strings(
                    &p["allowed_group_dependencies"]["persistence-adapter"],
                ),
                verification: strings(&p["allowed_group_dependencies"]["verification"]),
            },
            transport_capabilities: strings(&p["transport_capabilities"]),
            transport_state: strings(&p["transport_state"]),
            dispatch_no_growth: dispatch.clone(),
            new_unit: NewUnitBudget {
                control_flow_complexity_max: usize_value(
                    &p["new_unit"]["control_flow_complexity_max"],
                ),
                minimal_context_node_count_max: usize_value(
                    &p["new_unit"]["minimal_context_node_count_max"],
                ),
            },
            new_cycles: false,
            coverage_required: true,
            baseline_match: BaselineMatch {
                baseline_revision: workspace["baseline_revision"].as_str().unwrap().into(),
                scope: b["scope"].as_str().unwrap().into(),
                metrics: dispatch,
                accepted_debt: true,
            },
        },
    )
    .unwrap()
}

fn candidate(
    base: &ArchitectureRevision,
    candidates: &Value,
    id: &str,
    mutation: &Value,
    workspace: &Value,
) -> ArchitectureRevision {
    let fixture = candidates["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap();
    let mut result = base.clone();
    result.revision_id = fixture["revision"].as_str().unwrap().into();
    for changed in fixture
        .get("changed_units")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let pos = result
            .units
            .iter()
            .position(|u| u.id == changed["id"])
            .unwrap();
        result.units[pos] = unit(changed, module_for(changed, workspace));
    }
    for added in fixture["added_units"].as_array().unwrap() {
        result.units.push(unit(added, module_for(added, workspace)));
    }
    for added in fixture["added_edges"].as_array().unwrap() {
        result.edges.push(edge(added));
    }
    let additions = mutation
        .get("add_units")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(mutation.get("add_unit"));
    for added in additions {
        result.units.push(unit(added, module_for(added, workspace)));
    }
    if let Some(change) = mutation.get("change_cfg") {
        let u = result
            .units
            .iter_mut()
            .find(|u| u.id == change["unit_id"])
            .unwrap();
        u.cfg = ControlFlowGraph {
            nodes: usize_value(&change["nodes"]),
            edges: usize_value(&change["edges"]),
        };
    }
    if let Some(change) = mutation.get("change_unit_facts") {
        let u = result
            .units
            .iter_mut()
            .find(|u| u.id == change["unit_id"])
            .unwrap();
        u.capabilities = strings(&change["capabilities"]);
        u.state_reads = strings(&change["state_reads"]);
        u.state_writes = strings(&change["state_writes"]);
    }
    for added in mutation
        .get("add_edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        result.edges.push(edge(added));
    }
    if let Some(c) = mutation.get("coverage") {
        if let Some(v) = c.get("analyzed") {
            result.coverage.analyzed_groups = strings(v);
        }
        if let Some(v) = c.get("complete_for_policy") {
            result.coverage.complete_for_policy = v.as_bool().unwrap();
        }
    }
    if let Some(groups) = mutation.get("endpoint_groups").and_then(Value::as_object) {
        result.endpoint_groups.extend(
            groups
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().into())),
        );
    }
    ArchitectureRevision::new(
        result.workspace_id,
        result.revision_id,
        result.semantic_model_version,
        result.units,
        result.edges,
        result.endpoint_groups,
        result.coverage,
        result.policy,
    )
    .unwrap()
}

fn exceptions(v: &Value) -> Vec<ArchitectureException> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| ArchitectureException {
            id: x["id"].as_str().unwrap().into(),
            rule: x["rule"].as_str().unwrap().into(),
            scope: x["scope"].as_str().unwrap().into(),
            contributors: strings(&x["contributors"]),
            policy_revision: x["policy_revision"].as_str().unwrap().into(),
            expires_after_review_boundary: x["expires_after_review_boundary"]
                .as_str()
                .unwrap()
                .into(),
        })
        .collect()
}
fn input(v: &Value) -> ArchitectureEvaluationInput {
    let r = &v["request"];
    ArchitectureEvaluationInput {
        request: ArchitectureRequest {
            base_revision_id: r["base_revision_id"].as_str().unwrap().into(),
            candidate_revision_id: r["candidate_revision_id"].as_str().unwrap().into(),
            analysis_scope: r["analysis_scope"].as_str().unwrap().into(),
            policy_revision: r["policy_revision"].as_str().unwrap().into(),
            baseline_revision: r["baseline_revision"].as_str().unwrap().into(),
            review_boundary: r["review_boundary"].as_str().unwrap().into(),
            requested_governance_changes: r["requested_governance_changes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| GovernanceChange {
                    kind: x["kind"].as_str().unwrap().into(),
                    rule: x["rule"].as_str().unwrap().into(),
                    scope: x["scope"].as_str().unwrap().into(),
                    exception_ids: strings(&x["exception_ids"]),
                })
                .collect(),
            authorization_id: r["authorization_id"].as_str().map(Into::into),
        },
        governance_authorizations: v["governance_authorizations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| GovernanceAuthorization {
                authorization_id: x["authorization_id"].as_str().unwrap().into(),
                candidate_revision_id: x["candidate_revision_id"].as_str().unwrap().into(),
                candidate_graph_digest: x["candidate_graph_digest"].as_str().unwrap().into(),
                policy_revision: x["policy_revision"].as_str().unwrap().into(),
                baseline_revision: x["baseline_revision"].as_str().unwrap().into(),
                kind: x["kind"].as_str().unwrap().into(),
                exception_ids: strings(&x["exception_ids"]),
                rule: x["rule"].as_str().unwrap().into(),
                scope: x["scope"].as_str().unwrap().into(),
                review_boundary: x["review_boundary"].as_str().unwrap().into(),
            })
            .collect(),
        active_exceptions: exceptions(&v["active_exceptions"]),
        active_policy_revision: v["active_policy_revision"].as_str().unwrap().into(),
        active_baseline_revision: v["active_baseline_revision"].as_str().unwrap().into(),
    }
}

fn differences(path: &str, actual: &Value, expected: &Value, out: &mut Vec<String>) {
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => {
            for key in a
                .keys()
                .chain(e.keys())
                .collect::<std::collections::BTreeSet<_>>()
            {
                differences(
                    &format!("{path}/{key}"),
                    a.get(key).unwrap_or(&Value::Null),
                    e.get(key).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        (Value::Array(a), Value::Array(e)) => {
            for index in 0..a.len().max(e.len()) {
                differences(
                    &format!("{path}/{index}"),
                    a.get(index).unwrap_or(&Value::Null),
                    e.get(index).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        _ if actual != expected => out.push(format!("{path}: {actual} != {expected}")),
        _ => {}
    }
}

fn passing_validator(
    expected: ArchitectureRevision,
) -> impl FnOnce(
    &ArchitectureRevision,
) -> Result<BehaviorValidation, ail_compiler::ArchitectureRequestError> {
    move |observed| {
        assert_eq!(observed, &expected);
        Ok(BehaviorValidation {
            status: "passed".into(),
            cases_passed: 6,
            cases_total: 6,
        })
    }
}

fn reconstruct(
    revision: ArchitectureRevision,
) -> Result<ArchitectureRevision, ail_compiler::ArchitectureRevisionError> {
    ArchitectureRevision::new(
        revision.workspace_id,
        revision.revision_id,
        revision.semantic_model_version,
        revision.units,
        revision.edges,
        revision.endpoint_groups,
        revision.coverage,
        revision.policy,
    )
}

#[test]
fn revision_rejects_malformed_unchecked_boundaries_and_enabled_cycles() {
    let workspace: Value = serde_json::from_str(include_str!(
        "../../../specs/architecture-acceptance-fixtures/workspace.json"
    ))
    .unwrap();
    let package: Value =
        serde_json::from_str(include_str!("../../../specs/architecture-acceptance.json")).unwrap();
    let base = base_revision(&workspace, &package);
    for boundaries in [
        vec![Value::Null],
        vec![json!([])],
        vec![json!({"id":"x"})],
        vec![json!({"id":"x","reason":"why","extra":true})],
        vec![json!({"id":"","reason":"why"})],
        vec![json!({"id":"x","reason":"why\nnot"})],
        vec![
            json!({"id":"x","reason":"one"}),
            json!({"id":"x","reason":"two"}),
        ],
    ] {
        let mut revision = base.clone();
        revision.coverage.unchecked_boundaries = boundaries;
        assert!(reconstruct(revision).is_err());
    }
    let mut revision = base;
    revision.policy.new_cycles = true;
    assert!(reconstruct(revision).is_err());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end fixture test keeps shared workspace state explicit"
)]
fn all_m26_fixtures_match_and_transactions_are_atomic() {
    let workspace: Value = serde_json::from_str(include_str!(
        "../../../specs/architecture-acceptance-fixtures/workspace.json"
    ))
    .unwrap();
    let candidates: Value = serde_json::from_str(include_str!(
        "../../../specs/architecture-acceptance-fixtures/candidates.json"
    ))
    .unwrap();
    let package: Value =
        serde_json::from_str(include_str!("../../../specs/architecture-acceptance.json")).unwrap();
    let results: Value = serde_json::from_str(include_str!(
        "../../../specs/architecture-fixtures/results.json"
    ))
    .unwrap();
    let rejections: Value = serde_json::from_str(include_str!(
        "../../../specs/architecture-fixtures/rejections.json"
    ))
    .unwrap();
    let base = base_revision(&workspace, &package);
    let mut parity = 0;
    let mut commits = 0;
    let operations = results["operations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["operation"] == "validate_architecture_change")
        .map(|x| (x, None));
    let scenarios = rejections["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (x, x.get("semantic_mutation")));
    for (item, mutation) in operations.chain(scenarios) {
        let id = item["candidate"].as_str().unwrap();
        let candidate = candidate(
            &base,
            &candidates,
            id,
            mutation.unwrap_or(&Value::Null),
            &workspace,
        );
        let input = input(&item["input"]);
        let mut ws = ArchitectureWorkspace::new(base.clone());
        let first = validate_architecture_change(
            &mut ws,
            candidate.clone(),
            &input,
            passing_validator(candidate.clone()),
        )
        .unwrap();
        let actual = first.to_json_value();
        let mut diff = Vec::new();
        differences("", &actual, &item["expected_response"], &mut diff);
        assert!(
            diff.is_empty(),
            "fixture {}\n{}",
            item.get("id").and_then(Value::as_str).unwrap_or(id),
            diff.join("\n")
        );
        parity += 1;
        let success = first.to_json_value()["status"] == "success";
        commits += usize::from(success && item.get("id").is_none());
        assert_eq!(
            ws.current_revision_id(),
            if success {
                candidate.revision_id.as_str()
            } else {
                base.revision_id.as_str()
            }
        );
        assert_eq!(
            ws.retained_revision_ids().len(),
            if success { 2 } else { 1 }
        );
        let mut repeated = ArchitectureWorkspace::new(base.clone());
        let again = validate_architecture_change(
            &mut repeated,
            candidate.clone(),
            &input,
            passing_validator(candidate),
        )
        .unwrap();
        assert_eq!(first, again);
    }
    assert_eq!(parity, 26);
    assert_eq!(commits, 1);

    let valid_operation = &results["operations"][1];
    let valid_candidate = candidate(&base, &candidates, "valid", &Value::Null, &workspace);
    let valid_input = input(&valid_operation["input"]);

    let observed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed_by_validator = observed.clone();
    let expected = valid_candidate.clone();
    let mut ws = ArchitectureWorkspace::new(base.clone());
    let validator_error = validate_architecture_change(
        &mut ws,
        valid_candidate.clone(),
        &valid_input,
        move |candidate_revision| {
            assert_eq!(candidate_revision, &expected);
            observed_by_validator
                .lock()
                .unwrap()
                .push(candidate_revision.revision_id.clone());
            Err(ail_compiler::ArchitectureRequestError {
                kind: ArchitectureRequestErrorKind::InvalidRevision,
                message: "behavior validator failed before producing a summary".into(),
            })
        },
    )
    .unwrap_err();
    assert_eq!(
        validator_error.kind,
        ArchitectureRequestErrorKind::InvalidRevision
    );
    assert_eq!(
        *observed.lock().unwrap(),
        [valid_candidate.revision_id.clone()]
    );
    assert_eq!(ws.current_revision_id(), "arch-r1");
    assert_eq!(ws.retained_revision_ids(), ["arch-r1"]);

    let mut multi_cycle_candidate = valid_candidate.clone();
    for operation_id in workspace["operations"]
        .as_array()
        .unwrap()
        .iter()
        .take(2)
        .map(|operation| operation["id"].as_str().unwrap())
    {
        multi_cycle_candidate.edges.push(ArchitectureEdge {
            source: format!("domain:handle:{operation_id}"),
            target: format!("transport:adapt:{operation_id}"),
            kind: "calls".into(),
        });
    }
    let multi_cycle_candidate = reconstruct(multi_cycle_candidate).unwrap();
    let mut ws = ArchitectureWorkspace::new(base.clone());
    let cycle_result = validate_architecture_change(
        &mut ws,
        multi_cycle_candidate.clone(),
        &valid_input,
        passing_validator(multi_cycle_candidate),
    )
    .unwrap()
    .to_json_value();
    let cycle = cycle_result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["code"] == "AIL.ARCH.CYCLE")
        .unwrap();
    assert!(cycle["facts"]["components"].as_array().unwrap().len() > 1);
    assert_eq!(cycle["scope"], cycle["facts"]["components"][0]["id"]);
    assert_eq!(ws.retained_revision_ids(), ["arch-r1"]);

    for invalid_behavior in [
        BehaviorValidation {
            status: "failed".into(),
            cases_passed: 6,
            cases_total: 6,
        },
        BehaviorValidation {
            status: "passed".into(),
            cases_passed: 5,
            cases_total: 6,
        },
    ] {
        let mut ws = ArchitectureWorkspace::new(base.clone());
        let error = validate_architecture_change(&mut ws, valid_candidate.clone(), &valid_input, {
            let expected = valid_candidate.clone();
            move |observed| {
                assert_eq!(observed, &expected);
                Ok(invalid_behavior)
            }
        })
        .unwrap_err();
        assert_eq!(error.kind, ArchitectureRequestErrorKind::InvalidRevision);
        assert_eq!(ws.current_revision_id(), "arch-r1");
        assert_eq!(ws.retained_revision_ids(), ["arch-r1"]);
    }

    let mut policy_mutation = valid_candidate.clone();
    policy_mutation
        .policy
        .transport_capabilities
        .push("jobs_store".into());
    let mut ws = ArchitectureWorkspace::new(base.clone());
    assert!(
        validate_architecture_change(
            &mut ws,
            policy_mutation.clone(),
            &valid_input,
            passing_validator(policy_mutation),
        )
        .is_err()
    );
    assert_eq!(ws.retained_revision_ids(), ["arch-r1"]);

    let mut reused = valid_candidate.clone();
    reused.revision_id = "arch-r1".into();
    let mut reused_input = valid_input.clone();
    reused_input.request.candidate_revision_id = "arch-r1".into();
    let error = validate_architecture_change(
        &mut ws,
        reused.clone(),
        &reused_input,
        passing_validator(reused),
    )
    .unwrap_err();
    assert_eq!(error.kind, ArchitectureRequestErrorKind::InvalidRevision);
    assert_eq!(ws.current_revision_id(), "arch-r1");
    assert_eq!(ws.retained_revision_ids(), ["arch-r1"]);

    for scenario_id in ["stale-baseline", "unknown-authorization-id"] {
        let scenario = rejections["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scenario| scenario["id"] == scenario_id)
            .unwrap();
        let scenario_candidate = candidate(
            &base,
            &candidates,
            scenario["candidate"].as_str().unwrap(),
            &scenario["semantic_mutation"],
            &workspace,
        );
        let mut scenario_input = input(&scenario["input"]);
        let diagnostic = &scenario["expected_response"]["diagnostics"][0];
        scenario_input
            .active_exceptions
            .push(ArchitectureException {
                id: format!("attempted-{scenario_id}-exception"),
                rule: diagnostic["rule"].as_str().unwrap().into(),
                scope: diagnostic["scope"].as_str().unwrap().into(),
                contributors: strings(&diagnostic["contributors"]),
                policy_revision: "m23-policy-r1".into(),
                expires_after_review_boundary: "review-2026-07-22".into(),
            });
        let mut ws = ArchitectureWorkspace::new(base.clone());
        let result = validate_architecture_change(
            &mut ws,
            scenario_candidate.clone(),
            &scenario_input,
            passing_validator(scenario_candidate),
        )
        .unwrap();
        assert_eq!(result.to_json_value()["status"], "failure");
        assert_eq!(ws.current_revision_id(), "arch-r1");
        assert_eq!(ws.retained_revision_ids(), ["arch-r1"]);
    }
}
