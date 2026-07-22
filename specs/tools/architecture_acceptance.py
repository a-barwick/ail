#!/usr/bin/env python3
"""Dependency-free verifier for the non-normative M23 acceptance package."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parents[2]
SPECS = ROOT / "specs"
EDGE_KINDS = {"calls", "type-use", "verifies", "capability-use", "state-read", "state-write", "delegates"}
DEPENDENCY_KINDS = {"calls", "type-use", "delegates", "capability-use", "state-read", "state-write"}


class CheckError(Exception):
    pass


def require(value: bool, message: str) -> None:
    if not value:
        raise CheckError(message)


def unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        require(key not in result, f"duplicate key: {key}")
        result[key] = value
    return result


def canonical(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode()


def load(path: Path) -> dict[str, Any]:
    raw = path.read_bytes()
    require(raw.endswith(b"\n"), f"missing newline: {path.name}")
    require(raw == raw.decode("utf-8").encode(), f"non-UTF-8: {path.name}")
    value = json.loads(raw, object_pairs_hook=unique)
    require(isinstance(value, dict), "root must be object")
    require(raw == canonical(value), f"noncanonical JSON: {path.name}")
    return value


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def review_subject_digest(package: dict[str, Any]) -> str:
    projection = {
        key: package[key]
        for key in ("use_case", "requirements", "metrics", "rules", "traceability", "evidence_traceability", "policy", "budgets")
    }
    projection["children"] = [{"path": child["path"], "sha256": child["sha256"]} for child in package["children"]]
    return hashlib.sha256(canonical(projection)).hexdigest()


def cfc(unit: dict[str, Any]) -> int:
    return unit["cfg"]["edges"] - unit["cfg"]["nodes"] + 2


def validate_graph(units: list[dict[str, Any]], edges: list[list[str]], endpoint_groups: dict[str, str]) -> None:
    ids = [unit["id"] for unit in units]
    known_endpoints = set(ids) | set(endpoint_groups)
    require(len(ids) == len(set(ids)), "duplicate unit identity")
    require(len(edges) == len({tuple(edge) for edge in edges}), "duplicate edge")
    for edge in edges:
        require(len(edge) == 3 and edge[2] in EDGE_KINDS, "invalid typed edge")
        require(edge[0] in known_endpoints and edge[1] in known_endpoints, "unknown graph endpoint")
        require(not (edge[2] in {"capability-use", "delegates"} and any(name in edge[1] for name in ("clock", "network", "telemetry"))), "forbidden semantic capability/effect edge")
    for unit in units:
        require(unit["cfg"]["nodes"] > 0 and unit["cfg"]["edges"] >= 0 and cfc(unit) >= 1, "invalid CFG facts")
        for key in ("capabilities", "state_reads", "state_writes"):
            require(unit[key] == sorted(set(unit[key])), f"invalid ordered set {unit['id']}:{key}")


def strongly_connected(nodes: set[str], edges: list[list[str]]) -> list[list[str]]:
    graph = {node: [] for node in nodes}
    for source, target, kind in edges:
        if source in nodes and target in nodes and kind in DEPENDENCY_KINDS:
            graph[source].append(target)
    for node in graph:
        graph[node] = sorted(set(graph[node]))
    index = 0
    stack: list[str] = []
    on_stack: set[str] = set()
    indices: dict[str, int] = {}
    low: dict[str, int] = {}
    result: list[list[str]] = []

    def visit(node: str) -> None:
        nonlocal index
        indices[node] = low[node] = index
        index += 1
        stack.append(node)
        on_stack.add(node)
        for target in graph[node]:
            if target not in indices:
                visit(target)
                low[node] = min(low[node], low[target])
            elif target in on_stack:
                low[node] = min(low[node], indices[target])
        if low[node] == indices[node]:
            component = []
            while True:
                member = stack.pop()
                on_stack.remove(member)
                component.append(member)
                if member == node:
                    break
            result.append(sorted(component))

    for node in sorted(nodes):
        if node not in indices:
            visit(node)
    return sorted(result)


def expand(workspace: dict[str, Any]) -> tuple[list[dict[str, Any]], list[list[str]]]:
    units = copy.deepcopy(workspace["units"])
    edges: list[list[str]] = []
    expansion = workspace["operation_expansion"]
    cfgs = [{"nodes": 1, "edges": 0}, expansion["registration_cfg"], expansion["adapter_cfg"], expansion["handler_cfg"], expansion["test_cfg"]]
    for operation in workspace["operations"]:
        for identity, group, cfg in zip(expansion["units"], expansion["groups"], cfgs):
            units.append({"id": identity.format(id=operation["id"]), "group": group, "cfg": copy.deepcopy(cfg), "capabilities": [], "state_reads": [], "state_writes": []})
        edges.extend([[source.format(id=operation["id"]), target.format(id=operation["id"]), kind] for source, target, kind in expansion["edges"]])
    return units, edges


def apply_candidate(base_units: list[dict[str, Any]], base_edges: list[list[str]], candidate: dict[str, Any]) -> tuple[list[dict[str, Any]], list[list[str]], set[str]]:
    by_id = {unit["id"]: copy.deepcopy(unit) for unit in base_units}
    changed_ids: set[str] = set()
    for unit in candidate.get("changed_units", []):
        require(unit["id"] in by_id, f"changed unit does not exist: {unit['id']}")
        require(unit["id"] not in changed_ids, f"duplicate changed unit: {unit['id']}")
        by_id[unit["id"]] = copy.deepcopy(unit)
        changed_ids.add(unit["id"])
    added_ids: set[str] = set()
    for unit in candidate.get("added_units", []):
        require(unit["id"] not in by_id and unit["id"] not in added_ids, f"added unit already exists: {unit['id']}")
        by_id[unit["id"]] = copy.deepcopy(unit)
        added_ids.add(unit["id"])
    edges = copy.deepcopy(base_edges) + copy.deepcopy(candidate["added_edges"])
    units = [by_id[identity] for identity in sorted(by_id)]
    validate_graph(units, edges, candidate["endpoint_groups"])
    return units, edges, added_ids


def context_closure(selected: set[str], edges: list[list[str]], workspace: dict[str, Any], baseline_match: bool) -> list[str]:
    closure = set(selected)
    for source, target, _kind in edges:
        if source in selected or target in selected:
            closure.update((source, target))
    prefixes = {"contract:": "contract", "transport:": "transport", "domain:": "domain", "test:": "verification"}
    groups = set()
    for identity in selected:
        if identity in workspace["endpoint_groups"]:
            groups.add(workspace["endpoint_groups"][identity])
        else:
            groups.update(group for prefix, group in prefixes.items() if identity.startswith(prefix))
    groups = sorted(groups)
    closure.update(f"policy:group:{group}" for group in groups)
    if baseline_match:
        closure.add(f"baseline:{workspace['baseline_revision']}")
    return sorted(closure)


def direct_sets(units: list[dict[str, Any]], edges: list[list[str]], workspace: dict[str, Any]) -> tuple[dict[str, set[str]], dict[str, set[str]], dict[str, set[str]], list[tuple[str, str, str, str, str]]]:
    capabilities: dict[str, set[str]] = {}
    state: dict[str, set[str]] = {}
    dependencies: dict[str, set[str]] = {}
    group_edges: list[tuple[str, str, str, str, str]] = []
    unit_group = {unit["id"]: unit["group"] for unit in units}
    endpoint_group = {**workspace["endpoint_groups"], **unit_group}
    for unit in units:
        capability_targets = sorted({target.split(":", 1)[1].split(".", 1)[0] for source, target, kind in edges if source == unit["id"] and kind == "capability-use"})
        read_targets = sorted({target.split(":", 1)[1] for source, target, kind in edges if source == unit["id"] and kind == "state-read"})
        write_targets = sorted({target.split(":", 1)[1] for source, target, kind in edges if source == unit["id"] and kind == "state-write"})
        require(unit["capabilities"] == capability_targets, f"capability facts disagree with edges: {unit['id']}")
        require(unit["state_reads"] == read_targets and unit["state_writes"] == write_targets, f"state facts disagree with edges: {unit['id']}")
        capabilities.setdefault(unit["group"], set()).update(capability_targets)
        state.setdefault(unit["group"], set()).update(read_targets + write_targets)
        dependencies[unit["id"]] = {target for source, target, kind in edges if source == unit["id"] and kind in DEPENDENCY_KINDS}
    for source, target, kind in edges:
        source_group, target_group = endpoint_group.get(source), endpoint_group.get(target)
        if source_group and target_group and source_group != target_group and kind in DEPENDENCY_KINDS:
            group_edges.append((source_group, target_group, source, target, kind))
    return capabilities, state, dependencies, sorted(set(group_edges))


def finding(identity: str, classification: str, rule: str, scope: str, contributors: list[str], facts: str) -> dict[str, Any]:
    return {"id": identity, "classification": classification, "rule": rule, "scope": scope, "contributors": contributors, "facts": facts, "contributors_truncated": False}


def compact(result: dict[str, Any], policy_revision: str, baseline_revision: str, baseline: dict[str, Any]) -> str:
    publication = "published" if result["published"] else "not-published"
    commit = "committed" if result["committed"] else "rolled-back"
    lines = [
        f"{result['classification']} {result['revision']} policy={policy_revision} baseline={baseline_revision}",
        f"behavior: {result['behavior'].replace(':', ' ')}; architecture: {result['architecture']}",
        f"publication: {publication}; commit: {commit}",
    ]
    for item in result["findings"]:
        lines.append(f"{item['classification']} {item['rule']} {item['scope']} {item['facts']} contributors={','.join(item['contributors'])}")
    debt_suffix = "unchanged" if result["classification"] == "accepted" else f"baseline={baseline_revision}"
    lines.append(f"debt: {','.join(result['accepted_debt'])} {debt_suffix} (CFC={baseline['control_flow_complexity']} context={baseline['minimal_context_node_count']})")
    lines.append(f"coverage: {result['coverage']['analyzed']} groups; unchecked {','.join(result['coverage']['unchecked'])}")
    lines.append(f"inspect: {','.join(result['next_inspection_ids'])}")
    return "\n".join(lines) + "\n"


def evaluate(package: dict[str, Any], workspace: dict[str, Any], candidates_doc: dict[str, Any]) -> dict[str, Any]:
    base_units, base_edges = expand(workspace)
    validate_graph(base_units, base_edges, workspace["endpoint_groups"])
    policy = package["policy"]
    baseline = workspace["baseline"]
    unit_by_id = {unit["id"]: unit for unit in base_units}
    baseline_unit = unit_by_id[baseline["scope"]]
    derived_baseline_context = len(context_closure({baseline["scope"]}, base_edges, workspace, True))
    require(baseline["control_flow_complexity"] == cfc(baseline_unit), "baseline CFC is not derived")
    require(baseline["minimal_context_node_count"] == derived_baseline_context, "baseline context is not derived")
    require(policy["dispatch_no_growth"] == {"control_flow_complexity": cfc(baseline_unit), "minimal_context_node_count": derived_baseline_context}, "dispatch baseline policy mismatch")
    coverage = {"analyzed": len(workspace["coverage"]["analyzed"]), "unchecked": [item["id"] for item in workspace["coverage"]["unchecked"]]}
    base_result = {
        "candidate": "r1", "revision": workspace["revision"], "classification": "accepted", "behavior": "not-applicable",
        "architecture": "accepted-baseline", "published": True, "committed": True, "findings": [],
        "accepted_debt": baseline["accepted_debt"], "coverage": coverage, "next_inspection_ids": [baseline["scope"]],
    }
    base_result["compact"] = compact(base_result, workspace["policy_revision"], workspace["baseline_revision"], baseline)
    results = [base_result]
    oracle = [{key: case[key] for key in ("id", "outcome", "calls", "state", "effects")} for case in candidates_doc["behavior"]["cases"]]
    case_ids = [case["id"] for case in oracle]
    base_components = strongly_connected({unit["id"] for unit in base_units}, base_edges)
    for candidate in candidates_doc["candidates"]:
        require(candidate["observed_results"] == oracle, f"candidate behavior differs from oracle: {candidate['id']}")
        require([item["id"] for item in candidate["observed_results"]] == case_ids, "duplicate or reordered behavior evidence")
        candidate["endpoint_groups"] = workspace["endpoint_groups"]
        units, edges, added_ids = apply_candidate(base_units, base_edges, candidate)
        require(len(units) <= package["budgets"]["semantic_nodes_per_candidate_max"] and len(edges) <= package["budgets"]["typed_edges_per_candidate_max"], "analysis budget exhausted")
        unit_map = {unit["id"]: unit for unit in units}
        capabilities, state, _dependencies, group_edges = direct_sets(units, edges, workspace)
        findings: list[dict[str, Any]] = []
        dispatch_context = len(context_closure({baseline["scope"]}, edges, workspace, True))
        if cfc(unit_map[baseline["scope"]]) > baseline["control_flow_complexity"] or dispatch_context > baseline["minimal_context_node_count"]:
            findings.append(finding(f"{candidate['id']}:dispatch-growth", "regression", "M23-POL-DISPATCH-NO-GROWTH", baseline["scope"], [baseline["scope"]], f"CFC {baseline['control_flow_complexity']}->{cfc(unit_map[baseline['scope']])}; context {baseline['minimal_context_node_count']}->{dispatch_context}"))
        transport_units = sorted(unit["id"] for unit in units if unit["group"] == "transport")
        forbidden_caps = sorted(capabilities.get("transport", set()) - set(policy["transport_capabilities"]))
        if forbidden_caps:
            contributors = sorted({source for source, target, kind in edges if source in transport_units and kind == "capability-use" and any(capability in target for capability in forbidden_caps)} | {target for source, target, kind in edges if source in transport_units and kind == "capability-use" and any(capability in target for capability in forbidden_caps)})
            findings.append(finding(f"{candidate['id']}:transport-capability", "violation", "M23-POL-TRANSPORT-CAPABILITY", "group:transport", contributors, f"declared capabilities {','.join(forbidden_caps)}"))
        forbidden_state = sorted(state.get("transport", set()) - set(policy["transport_state"]))
        if forbidden_state:
            contributors = sorted({endpoint for source, target, kind in edges if source in transport_units and kind in {"state-read", "state-write"} for endpoint in (source, target)})
            findings.append(finding(f"{candidate['id']}:transport-state", "violation", "M23-POL-TRANSPORT-STATE", "group:transport", contributors, f"state reads+writes {','.join(forbidden_state)}"))
        forbidden_edges = [edge for edge in group_edges if edge[1] not in policy["allowed_group_dependencies"][edge[0]]]
        if forbidden_edges:
            contributors = sorted({endpoint for _sg, _tg, source, target, _kind in forbidden_edges for endpoint in (source, target)})
            facts = ",".join(f"{sg}->{tg}" for sg, tg in sorted({(edge[0], edge[1]) for edge in forbidden_edges}))
            findings.append(finding(f"{candidate['id']}:transport-dependency", "violation", "M23-POL-GROUP-DEPENDENCY", "group:transport", contributors, f"forbidden group dependencies {facts}"))
        for identity in sorted(added_ids):
            unit = unit_map[identity]
            if unit["group"] == "contract":
                continue
            context = len(context_closure({identity}, edges, workspace, False))
            if cfc(unit) > policy["new_unit"]["control_flow_complexity_max"] or context > policy["new_unit"]["minimal_context_node_count_max"]:
                findings.append(finding(f"{candidate['id']}:new-unit:{identity}", "violation", "M23-POL-NEW-UNIT", identity, [identity], f"CFC={cfc(unit)} context={context}"))
        components = strongly_connected(set(unit_map), edges)
        new_cycles = [component for component in components if len(component) > 1 and component not in base_components]
        if new_cycles:
            contributors = sorted({identity for component in new_cycles for identity in component})
            findings.append(finding(f"{candidate['id']}:new-cycle", "violation", "M23-POL-NO-NEW-CYCLE", "workspace", contributors, "new dependency cycle"))
        candidate_coverage = candidate.get("coverage", workspace["coverage"])
        required_groups = set(policy["allowed_group_dependencies"]) | set(workspace["modules"].values()) | {unit["group"] for unit in units} | set(workspace["endpoint_groups"].values())
        covered_groups = set(candidate_coverage["analyzed"])
        units_covered = all(unit["group"] in covered_groups for unit in units)
        if policy["coverage_required"] and (covered_groups != required_groups or len(candidate_coverage["analyzed"]) != len(covered_groups) or not units_covered or candidate_coverage["unchecked"] != workspace["coverage"]["unchecked"]):
            findings.append(finding(f"{candidate['id']}:coverage", "incomplete", "M23-POL-COVERAGE", "workspace", [], "required coverage changed or incomplete"))
        if candidate["governance"] != policy["governance"]:
            findings.append(finding(f"{candidate['id']}:governance", "violation", "M23-POL-GOVERNANCE", "workspace", [], "policy, baseline, or exceptions changed"))
        rejected = bool(findings)
        if findings:
            inspections = []
            for item in findings:
                if item["scope"] not in {"workspace"} and item["scope"] not in inspections:
                    inspections.append(item["scope"])
            for item in findings:
                if item["rule"] == "M23-POL-GROUP-DEPENDENCY":
                    for contributor in item["contributors"]:
                        if contributor in unit_map and contributor not in inspections:
                            inspections.append(contributor)
        else:
            domain_additions = sorted(identity for identity in added_ids if unit_map[identity]["group"] == "domain")
            inspections = domain_additions or [baseline["scope"]]
        result = {
            "candidate": candidate["id"], "revision": candidate["revision"], "classification": "rejected" if rejected else "accepted",
            "behavior": f"passed:{len(case_ids)}/{len(case_ids)}", "architecture": "rejected" if rejected else "accepted",
            "published": not rejected, "committed": not rejected, "findings": findings, "accepted_debt": baseline["accepted_debt"],
            "coverage": coverage, "next_inspection_ids": inspections,
        }
        result["compact"] = compact(result, workspace["policy_revision"], workspace["baseline_revision"], baseline)
        results.append(result)
    return {"fixture_format": 1, "label": "non-normative M23 acceptance oracle; not an M24 protocol", "results": results}


def technical(package: dict[str, Any], workspace: dict[str, Any], candidates: dict[str, Any], expected: dict[str, Any]) -> None:
    budgets = package["budgets"]
    rules = ["M23-POL-GROUP-DEPENDENCY", "M23-POL-TRANSPORT-CAPABILITY", "M23-POL-TRANSPORT-STATE", "M23-POL-DISPATCH-NO-GROWTH", "M23-POL-NEW-UNIT", "M23-POL-NO-NEW-CYCLE", "M23-POL-COVERAGE", "M23-POL-GOVERNANCE"]
    requirements = ["APP-006", "LANG-006", "PROTO-006", "PROTO-007", "NFR-006", "NFR-007"]
    metrics = ["control_flow_complexity", "direct_dependency_set", "declared_capability_set", "state_read_set", "state_write_set", "dependency_component_size", "minimal_context_node_count"]
    require(package["use_case"] == "UC-007" and package["requirements"] == requirements, "use-case requirement traceability")
    require(package["metrics"] == metrics, "minimal metric set/order")
    require(package["rules"] == rules and list(package["traceability"]) == rules, "policy rule set/order")
    require(all(values and set(values) <= set(requirements) for values in package["traceability"].values()), "invalid rule traceability")
    require(set().union(*map(set, package["traceability"].values())) >= set(requirements) - {"NFR-007"}, "incomplete rule traceability")
    require(list(package["evidence_traceability"]) == requirements and all(package["evidence_traceability"][item] for item in requirements), "incomplete evidence traceability")
    require(set(budgets) == {"r1_operations", "candidates", "false_findings", "missed_required_findings", "semantic_nodes_per_candidate_max", "typed_edges_per_candidate_max", "structured_result_utf8_bytes_max", "compact_utf8_bytes_max", "compact_newline_lines_max", "required_contributor_truncation"}, "budget field set")
    require(budgets["false_findings"] == budgets["missed_required_findings"] == 0 and budgets["required_contributor_truncation"] is False, "finding/truncation budgets")
    require(package["policy"]["transport_capabilities"] == [] and package["policy"]["transport_state"] == [], "transport policy weakened")
    require(len(workspace["operations"]) == budgets["r1_operations"] == 24 and len({item["id"] for item in workspace["operations"]}) == 24, "operation count/identity")
    require(all(item["name"] != "CancelJob" and item["id"] != "job.cancel" for item in workspace["operations"]), "CancelJob present in R1")
    require(list(workspace["identity_pattern"]) == ["contract", "transport_registration", "transport_adapter", "domain_handler", "behavior_fixture"], "identity order")
    require(workspace["typed_edge_kinds"] == sorted(EDGE_KINDS), "typed edge kinds changed")
    require(workspace["endpoint_groups"] == {"contract:job.cancel": "contract", "capability:jobs_store.cancel_if_active": "persistence-adapter", "state:jobs": "persistence-adapter"}, "endpoint groups incomplete")
    required_groups = set(package["policy"]["allowed_group_dependencies"]) | set(workspace["modules"].values()) | {unit["group"] for unit in workspace["units"]} | set(workspace["operation_expansion"]["groups"]) | set(workspace["endpoint_groups"].values())
    require(set(workspace["coverage"]["analyzed"]) == required_groups and len(workspace["coverage"]["analyzed"]) == len(required_groups) and workspace["coverage"]["unchecked"] == [{"id": "external:job-api-clients", "reason": "client source unavailable"}], "coverage boundary")
    cases = candidates["behavior"]["cases"]
    oracle = [("malformed", "ValidationFailure", [], "unchanged", []), ("queued", "Cancelled", ["jobs_store.cancel_if_active"], "cancelled", []), ("running", "Cancelled", ["jobs_store.cancel_if_active"], "cancelled", []), ("missing", "NotFound", ["jobs_store.cancel_if_active"], "unchanged", []), ("completed", "NotCancellable", ["jobs_store.cancel_if_active"], "unchanged", []), ("cancelled", "NotCancellable", ["jobs_store.cancel_if_active"], "unchanged", [])]
    require([(item["id"], item["outcome"], item["calls"], item["state"], item["effects"]) for item in cases] == oracle, "behavior oracle mismatch")
    require(candidates["behavior"]["forbidden_capabilities"] == ["clock", "network", "telemetry"], "forbidden behavior capabilities")
    require([item["id"] for item in candidates["candidates"]] == ["valid", "centralized", "helper-split"] and len(candidates["candidates"]) == budgets["candidates"], "candidate set/order")
    base_units, base_edges = expand(workspace)
    for candidate in candidates["candidates"]:
        operation = candidate["operation"]
        require(operation["name"] == "CancelJob" and operation["id"] == "job.cancel", f"invalid candidate operation: {candidate['id']}")
        candidate["endpoint_groups"] = workspace["endpoint_groups"]
        units, _edges, _added_ids = apply_candidate(base_units, base_edges, candidate)
        unit_ids = {unit["id"] for unit in units}
        role_ids = [operation[key] for key in ("contract", "registration", "transport_entry", "implementation_owner", "behavior_fixture")]
        require(len(role_ids) == len(set(role_ids)) and all(identity in unit_ids for identity in role_ids), f"incomplete CancelJob roles: {candidate['id']}")
        require(len(workspace["operations"]) + 1 == 25, "candidate operation count")
    derived = evaluate(package, workspace, candidates)
    expected_matrix = {(result["candidate"], item["id"], item["rule"]) for result in expected["results"] for item in result["findings"]}
    actual_matrix = {(result["candidate"], item["id"], item["rule"]) for result in derived["results"] for item in result["findings"]}
    require(len(actual_matrix - expected_matrix) <= budgets["false_findings"], "false finding budget")
    require(len(expected_matrix - actual_matrix) <= budgets["missed_required_findings"], "missed finding budget")
    require(derived == expected, "derived result does not exactly match expected.json")
    required_rules = {"valid": [], "centralized": ["M23-POL-DISPATCH-NO-GROWTH", "M23-POL-TRANSPORT-CAPABILITY", "M23-POL-TRANSPORT-STATE", "M23-POL-GROUP-DEPENDENCY"], "helper-split": ["M23-POL-TRANSPORT-CAPABILITY", "M23-POL-TRANSPORT-STATE", "M23-POL-GROUP-DEPENDENCY"]}
    for result in derived["results"][1:]:
        require([item["rule"] for item in result["findings"]] == required_rules[result["candidate"]], "required finding set/order")
    for result in derived["results"]:
        require(all(item["contributors_truncated"] is budgets["required_contributor_truncation"] for item in result["findings"]), "required contributor truncation")
        require(len(canonical(result)) <= budgets["structured_result_utf8_bytes_max"], "structured output budget")
        text = result["compact"].encode()
        require(text.endswith(b"\n") and len(text) <= budgets["compact_utf8_bytes_max"] and text.count(b"\n") <= budgets["compact_newline_lines_max"], "compact output budget")


def mutation_tests(package: dict[str, Any], workspace: dict[str, Any], candidates: dict[str, Any], expected: dict[str, Any]) -> int:
    mutations: list[tuple[str, Callable[[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]], None]]] = []
    def add(name: str, operation: Callable[[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]], None]) -> None:
        mutations.append((name, operation))
    add("operation count", lambda p, w, c, e: w["operations"].pop())
    add("traceability", lambda p, w, c, e: p["traceability"]["M23-POL-COVERAGE"].remove("NFR-006"))
    add("NFR-007 evidence traceability", lambda p, w, c, e: p["evidence_traceability"].pop("NFR-007"))
    add("expected contributor", lambda p, w, c, e: e["results"][2]["findings"][1]["contributors"].pop())
    add("expected facts", lambda p, w, c, e: e["results"][2]["findings"][0].__setitem__("facts", "altered"))
    add("expected coverage", lambda p, w, c, e: e["results"][1]["coverage"].__setitem__("analyzed", 4))
    add("expected inspection", lambda p, w, c, e: e["results"][3]["next_inspection_ids"].pop())
    add("expected compact", lambda p, w, c, e: e["results"][1].__setitem__("compact", "accepted\n"))
    add("semantic edge", lambda p, w, c, e: c["candidates"][1]["added_edges"].pop(3))
    add("duplicate edge", lambda p, w, c, e: c["candidates"][0]["added_edges"].append(copy.deepcopy(c["candidates"][0]["added_edges"][0])))
    add("cycle", lambda p, w, c, e: c["candidates"][0]["added_edges"].append(["domain:handle:job.cancel", "transport:adapt:job.cancel", "calls"]))
    add("coverage loss", lambda p, w, c, e: c["candidates"][0].__setitem__("coverage", {**w["coverage"], "analyzed": w["coverage"]["analyzed"][:-1]}))
    add("coverage same-cardinality substitution", lambda p, w, c, e: c["candidates"][0].__setitem__("coverage", {**w["coverage"], "analyzed": w["coverage"]["analyzed"][:-1] + ["substitute"]}))
    add("behavior outcome", lambda p, w, c, e: c["behavior"]["cases"][1].__setitem__("outcome", "NotFound"))
    add("candidate behavior outcome", lambda p, w, c, e: c["candidates"][0]["observed_results"][1].__setitem__("outcome", "NotFound"))
    add("candidate behavior call", lambda p, w, c, e: c["candidates"][0]["observed_results"][1]["calls"].clear())
    add("candidate behavior state", lambda p, w, c, e: c["candidates"][0]["observed_results"][1].__setitem__("state", "unchanged"))
    add("candidate behavior effect", lambda p, w, c, e: c["candidates"][0]["observed_results"][1]["effects"].append("telemetry"))
    add("forbidden capability edge", lambda p, w, c, e: (w["endpoint_groups"].__setitem__("capability:telemetry.emit", "persistence-adapter"), c["candidates"][0]["added_edges"].append(["domain:handle:job.cancel", "capability:telemetry.emit", "capability-use"])))
    add("unknown graph endpoint", lambda p, w, c, e: c["candidates"][0]["added_edges"].append(["domain:handle:job.cancel", "unknown:endpoint", "calls"]))
    add("changed replacement", lambda p, w, c, e: c["candidates"][1]["changed_units"][0].__setitem__("id", "transport:missing"))
    add("helper contributor", lambda p, w, c, e: c["candidates"][2]["added_units"][4].__setitem__("capabilities", []))
    add("endpoint assignment", lambda p, w, c, e: w["endpoint_groups"].__setitem__("state:jobs", "transport"))
    add("baseline context", lambda p, w, c, e: w["baseline"].__setitem__("minimal_context_node_count", 99))
    add("governance", lambda p, w, c, e: c["candidates"][0]["governance"].__setitem__("baseline_revision", "other"))
    add("classification", lambda p, w, c, e: e["results"][1].__setitem__("classification", "rejected"))
    add("false findings budget", lambda p, w, c, e: p["budgets"].__setitem__("false_findings", 1))
    add("missed findings budget", lambda p, w, c, e: p["budgets"].__setitem__("missed_required_findings", 1))
    add("contributor truncation budget", lambda p, w, c, e: p["budgets"].__setitem__("required_contributor_truncation", True))
    add("contributor truncated", lambda p, w, c, e: e["results"][2]["findings"][0].__setitem__("contributors_truncated", True))
    add("semantic node budget", lambda p, w, c, e: p["budgets"].__setitem__("semantic_nodes_per_candidate_max", 1))
    add("typed edge budget", lambda p, w, c, e: p["budgets"].__setitem__("typed_edges_per_candidate_max", 1))
    add("structured output budget", lambda p, w, c, e: p["budgets"].__setitem__("structured_result_utf8_bytes_max", 1))
    add("compact byte budget", lambda p, w, c, e: p["budgets"].__setitem__("compact_utf8_bytes_max", 1))
    add("compact line budget", lambda p, w, c, e: p["budgets"].__setitem__("compact_newline_lines_max", 1))
    for name, operation in mutations:
        args = list(map(copy.deepcopy, (package, workspace, candidates, expected)))
        operation(*args)
        try:
            technical(*args)
        except (CheckError, KeyError, IndexError):
            continue
        raise CheckError(f"mutation survived: {name}")
    return len(mutations)


def validate_review_doc(package: dict[str, Any], digest: str, subject_digest: str, reviews: dict[str, Any]) -> None:
    require(reviews["fixture_set_digest"] == digest, "stale review digest")
    require(reviews["review_subject_digest"] == subject_digest, "stale review subject digest")
    reviewer_ids = [review.get("reviewer_id") for review in reviews["reviews"]]
    require(len(reviewer_ids) == len(set(reviewer_ids)), "duplicate reviewer")
    expected_classifications = {
        "r1": {"result": "accepted-baseline", "rules": []},
        "valid": {"result": "accepted", "rules": []},
        "centralized": {"result": "rejected", "rules": ["M23-POL-DISPATCH-NO-GROWTH", "M23-POL-TRANSPORT-CAPABILITY", "M23-POL-TRANSPORT-STATE", "M23-POL-GROUP-DEPENDENCY"]},
        "helper-split": {"result": "rejected", "rules": ["M23-POL-TRANSPORT-CAPABILITY", "M23-POL-TRANSPORT-STATE", "M23-POL-GROUP-DEPENDENCY"]},
    }
    valid = len(reviewer_ids) == 2 and all(
        review.get("decision") == "approve"
        and review.get("review_subject_digest") == subject_digest
        and review.get("reviewed_on") == "2026-07-22"
        and review.get("classifications") == expected_classifications
        and review.get("blockers") == []
        for review in reviews["reviews"]
    )
    require(reviews["status"] == ("accepted" if valid else "pending"), "review status inconsistent")
    require(package["status"] != "Accepted" or valid, "accepted package lacks two valid reviews")


def validate_reviews(package: dict[str, Any], digest: str, subject_digest: str) -> None:
    validate_review_doc(package, digest, subject_digest, load(SPECS / package["reviews_path"]))


def review_mutation_tests(package: dict[str, Any], digest: str, subject_digest: str) -> int:
    reviews = load(SPECS / package["reviews_path"])
    stale = copy.deepcopy(reviews)
    stale["review_subject_digest"] = "0" * 64
    duplicate = copy.deepcopy(reviews)
    duplicate["status"] = "accepted"
    duplicate["reviews"] = [
        {"reviewer_id": "same", "decision": "approve", "review_subject_digest": subject_digest},
        {"reviewer_id": "same", "decision": "approve", "review_subject_digest": subject_digest},
    ]
    for name, mutated in (("review digest", stale), ("duplicate review", duplicate)):
        try:
            validate_review_doc(package, digest, subject_digest, mutated)
        except CheckError:
            continue
        raise CheckError(f"mutation survived: {name}")
    return 2


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["check"])
    parser.parse_args()
    try:
        package = load(SPECS / "architecture-acceptance.json")
        paths = []
        for child in package["children"]:
            path = Path(child["path"])
            require(not path.is_absolute() and ".." not in path.parts and path.as_posix() == child["path"], "unsafe child path")
            paths.append(path)
            require(sha(SPECS / path) == child["sha256"], f"changed child hash: {path}")
        require(paths == [Path("architecture-acceptance-fixtures/workspace.json"), Path("architecture-acceptance-fixtures/candidates.json"), Path("architecture-acceptance-fixtures/expected.json")], "child order")
        digest = hashlib.sha256(b"".join((path.as_posix() + "\0" + sha(SPECS / path) + "\n").encode() for path in paths)).hexdigest()
        require(digest == package["fixture_set_digest"], "fixture-set digest")
        subject_digest = review_subject_digest(package)
        require(subject_digest == package["review_subject_digest"], "review-subject digest")
        workspace, candidates, expected = [load(SPECS / path) for path in paths]
        technical(package, workspace, candidates, expected)
        mutation_count = mutation_tests(package, workspace, candidates, expected) + review_mutation_tests(package, digest, subject_digest)
        validate_reviews(package, digest, subject_digest)
        print(f"M23 acceptance package: technical evidence valid; 24 operations, 3 candidates, 8 rules, {mutation_count} mutation tests")
        reviews = load(SPECS / package["reviews_path"])
        print("review gate: accepted" if reviews["status"] == "accepted" else "review gate: pending (0/2 independent reviews); package remains Proposed")
        return 0
    except (CheckError, OSError, json.JSONDecodeError) as error:
        print(f"architecture_acceptance: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
