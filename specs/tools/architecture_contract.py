#!/usr/bin/env python3
"""Dependency-free executable verifier for the M24 architecture contract.

This evaluator deliberately does not call the M23 evaluator or renderer.  M23
is used only for its graph expansion, candidate application, CFG, SCC, and
one-hop closure helpers; all protocol records and policy decisions below are
M24 semantics.
"""
from __future__ import annotations

import argparse, copy, hashlib, json, sys
from pathlib import Path
from typing import Any, Callable

import architecture_acceptance as m23

ROOT = Path(__file__).resolve().parents[2]
SPECS = ROOT / "specs"
LIMITS = {"semantic_nodes": 512, "typed_edges": 2048, "structured_bytes": 65536,
          "compact_bytes": 2048, "compact_lines": 12}
CODES = ["AIL.ARCH.HOTSPOT_GROWTH", "AIL.ARCH.AUTHORITY", "AIL.ARCH.STATE",
         "AIL.ARCH.BOUNDARY", "AIL.ARCH.NEW_UNIT", "AIL.ARCH.CYCLE",
         "AIL.ARCH.COVERAGE_INCOMPLETE", "AIL.ARCH.STALE_BASELINE",
         "AIL.ARCH.GOVERNANCE_UNAUTHORIZED", "AIL.ARCH.ANALYSIS_INCOMPLETE"]
RULE_CODE = {"M23-POL-DISPATCH-NO-GROWTH": CODES[0], "M23-POL-TRANSPORT-CAPABILITY": CODES[1],
             "M23-POL-TRANSPORT-STATE": CODES[2], "M23-POL-GROUP-DEPENDENCY": CODES[3],
             "M23-POL-NEW-UNIT": CODES[4], "M23-POL-NO-NEW-CYCLE": CODES[5],
             "M23-POL-COVERAGE": CODES[6], "M24-BASELINE-COMPATIBILITY": CODES[7],
             "M23-POL-GOVERNANCE": CODES[8], "M24-ANALYSIS-BUDGET": CODES[9]}
MODULE_GROUP = {"contracts": "contract", "transport": "transport", "domain": "domain",
                "adapters": "persistence-adapter", "tests": "verification"}
GROUP_MODULE = {v: k for k, v in MODULE_GROUP.items()}
EDGE_KIND_ORDER = {kind: index for index, kind in enumerate(
                   ["calls", "type-use", "verifies", "capability-use", "state-read", "state-write", "delegates"])}
SHAPE_KEYS: dict[str, list[str]] = {
 "AnalysisIdentity": ["workspace_id","revision_id","semantic_model_version","policy_revision","baseline_revision","analysis_scope"],
 "Coverage": ["analyzed_groups","unchecked_boundaries","complete_for_policy"],
 "BudgetUse": ["semantic_nodes","typed_edges","structured_bytes","compact_bytes","compact_lines","exhausted"],
 "ScopeMetrics": ["scope_kind","identity","unit_ids","control_flow_complexity","direct_dependency_set","declared_capability_set","state_read_set","state_write_set","dependency_component_size","minimal_context_node_count"],
 "Finding": ["id","code","classification","disposition","rule","scope","contributors","facts","exception"],
 "PolicySelector": ["scope_kind","scope_identity"], "PolicyRule": ["id","selector","classification","disposition","comparison","value"],
 "BaselineMatch": ["baseline_revision","scope","metrics","accepted_debt"],
 "Exception": ["id","rule","scope","contributors","policy_revision","expires_after_review_boundary"],
 "ScopeChange": ["scope_kind","identity","base","candidate"],
 "ArchitectureSnapshot": ["format","analysis","scopes","coverage","budgets","active_policies","baseline_match","accepted_debt","exceptions","findings","classification","compact"],
 "ArchitectureDelta": ["format","base_snapshot_digest","candidate_snapshot_digest","base_revision_id","candidate_revision_id","scope_changes","findings","classification","publication","commit","compact"],
 "GovernanceChange": ["kind","rule","scope","exception_ids"],
 "GovernanceAuthorization": ["authorization_id","candidate_revision_id","candidate_graph_digest","policy_revision","baseline_revision","kind","exception_ids","rule","scope","review_boundary"],
 "ArchitectureRequest": ["base_revision_id","candidate_revision_id","analysis_scope","policy_revision","baseline_revision","review_boundary","requested_governance_changes","authorization_id"],
 "ArchitectureEvaluationInput": ["request","governance_authorizations","active_exceptions","active_policy_revision","active_baseline_revision"],
 "ArchitectureSnapshotRequest": ["revision_id","analysis_scope"],
 "ArchitectureSnapshotInput": ["request","active_exceptions","active_policy_revision","active_baseline_revision"],
 "CompletionEvidence": ["base_revision_id","revision_id","base_snapshot_digest","snapshot_digest","delta_digest","policy_revision","baseline_revision","coverage","budgets","behavior_validation","commit"],
 "ArchitectureSuccess": ["status","snapshot","delta","completion"],
 "ArchitectureSnapshotResponse": ["status","snapshot","snapshot_digest"],
 "ArchitectureIncompleteFailure": ["status","analysis","coverage","budgets","diagnostics","edits","current_revision_id","published_child_revision_id"],
 "ArchitectureFailure": ["status","base_revision_id","current_revision_id","snapshot","delta","diagnostics","edits","published_child_revision_id"]}

class CheckError(Exception): pass
def require(v: bool, msg: str) -> None:
    if not v: raise CheckError(msg)
def unique(pairs: list[tuple[str,Any]]) -> dict[str,Any]:
    out={}
    for k,v in pairs:
        require(k not in out, f"duplicate key: {k}"); out[k]=v
    return out
def canonical(v: Any) -> bytes: return (json.dumps(v,ensure_ascii=False,indent=2)+"\n").encode()
def load(path: Path) -> dict[str,Any]:
    raw=path.read_bytes(); require(raw.endswith(b"\n") and raw==raw.decode().encode(),f"encoding/newline: {path}")
    v=json.loads(raw,object_pairs_hook=unique); require(isinstance(v,dict) and raw==canonical(v),f"noncanonical JSON: {path}"); return v
def file_digest(path: Path) -> str: return hashlib.sha256(path.read_bytes()).hexdigest()
def object_digest(v: Any) -> str: return "sha256:"+hashlib.sha256(canonical(v)).hexdigest()
def unit_module(identity: str, group: str, workspace: dict[str,Any]) -> str:
    # Membership is fixture data: templates plus explicit exceptional units.
    explicit={u["id"]:u["module"] for u in workspace["units"]}
    if identity in explicit: return explicit[identity]
    require(group in GROUP_MODULE, f"unit has no semantic module membership: {identity}")
    return GROUP_MODULE[group]
def graph_digest(units: list[dict[str,Any]], edges: list[list[str]], workspace: dict[str,Any]) -> str:
    require(len({unit["id"] for unit in units})==len(units),"duplicate unit identity in candidate graph digest")
    require(len({tuple(edge) for edge in edges})==len(edges),"duplicate edge in candidate graph digest")
    projected=[]
    for unit in units:
        projected.append({"id":unit["id"],"module":unit_module(unit["id"],unit["group"],workspace),"group":unit["group"],
                          "cfg":{"nodes":unit["cfg"]["nodes"],"edges":unit["cfg"]["edges"]},
                          "capabilities":sorted(set(unit["capabilities"])),"state_reads":sorted(set(unit["state_reads"])),
                          "state_writes":sorted(set(unit["state_writes"]))})
    projected.sort(key=lambda unit:unit["id"].encode())
    normalized_edges=sorted((tuple(edge) for edge in edges),key=lambda edge:(edge[0].encode(),edge[1].encode(),EDGE_KIND_ORDER[edge[2]]))
    return object_digest({"units":projected,"edges":[list(edge) for edge in normalized_edges]})
def policies(package: dict[str,Any]) -> list[dict[str,Any]]:
    p=package["policy"]
    return [
      {"id":"M23-POL-GROUP-DEPENDENCY","selector":{"scope_kind":"architecture-group","scope_identity":"*"},"classification":"violation","disposition":"deny","comparison":"subset-by-source-group","value":p["allowed_group_dependencies"]},
      {"id":"M23-POL-TRANSPORT-CAPABILITY","selector":{"scope_kind":"architecture-group","scope_identity":"group:transport"},"classification":"violation","disposition":"deny","comparison":"set-subset","value":p["transport_capabilities"]},
      {"id":"M23-POL-TRANSPORT-STATE","selector":{"scope_kind":"architecture-group","scope_identity":"group:transport"},"classification":"violation","disposition":"deny","comparison":"read-and-write-set-subset","value":p["transport_state"]},
      {"id":"M23-POL-DISPATCH-NO-GROWTH","selector":{"scope_kind":"executable-unit","scope_identity":"transport:dispatch"},"classification":"regression","disposition":"deny","comparison":"both-less-than-or-equal-baseline","value":p["dispatch_no_growth"]},
      {"id":"M23-POL-NEW-UNIT","selector":{"scope_kind":"executable-unit","scope_identity":"new-non-contract-unit"},"classification":"violation","disposition":"deny","comparison":"both-less-than-or-equal","value":p["new_unit"]},
      {"id":"M23-POL-NO-NEW-CYCLE","selector":{"scope_kind":"dependency-component","scope_identity":"new-component"},"classification":"violation","disposition":"deny","comparison":"new-component-member-count-less-than-or-equal","value":1},
      {"id":"M23-POL-COVERAGE","selector":{"scope_kind":"architecture-group","scope_identity":"*"},"classification":"incomplete","disposition":"deny","comparison":"complete-for-policy-equals","value":True},
      {"id":"M23-POL-GOVERNANCE","selector":{"scope_kind":"governance","scope_identity":"trusted-input"},"classification":"violation","disposition":"deny","comparison":"trusted-revisions-and-exact-exceptions-equal","value":{"policy_revision":"m23-policy-r1","baseline_revision":"m23-baseline-r1","exceptions":[]}},
    ]
def component_id(members: list[str]) -> str:
    joined="\0".join(sorted(members)); return "component:sha256:"+hashlib.sha256(joined.encode()).hexdigest()
def ordered_findings(items: list[dict[str,Any]]) -> list[dict[str,Any]]:
    rank={c:i for i,c in enumerate(CODES)}
    return sorted(items,key=lambda f:(rank[f["code"]],f["scope"],f["contributors"],f["id"]))
def finding(fid:str,rule:str,scope:str,contributors:list[str],facts:dict[str,Any], exception:dict[str,Any]|None=None)->dict[str,Any]:
    classification="regression" if rule=="M23-POL-DISPATCH-NO-GROWTH" else "incomplete" if rule in {"M23-POL-COVERAGE","M24-BASELINE-COMPATIBILITY","M24-ANALYSIS-BUDGET"} else "violation"
    return {"id":fid,"code":RULE_CODE[rule],"classification":classification,"disposition":"deny","rule":rule,"scope":scope,"contributors":sorted(set(contributors)),"facts":facts,"exception":exception}

def metric_record(kind:str, identity:str, members:list[str], units:list[dict[str,Any]], edges:list[list[str]], workspace:dict[str,Any], comps:list[list[str]], baseline:bool)->dict[str,Any]:
    um={u["id"]:u for u in units}; members=sorted(members); member_set=set(members)
    vals=[(i,m23.cfc(um[i])) for i in members]; maxv=max(v for _,v in vals)
    deps=sorted({t for s,t,k in edges if s in member_set and k in m23.DEPENDENCY_KINDS and t not in member_set})
    caps=sorted({t.split(":",1)[1].split(".",1)[0] for s,t,k in edges if s in member_set and k=="capability-use"}); reads=sorted({t.split(":",1)[1] for s,t,k in edges if s in member_set and k=="state-read"}); writes=sorted({t.split(":",1)[1] for s,t,k in edges if s in member_set and k=="state-write"})
    sizes={i:len(next(c for c in comps if i in c)) for i in members}
    # Aggregate members are all selected before the one-hop closure.
    context=len(m23.context_closure(member_set,edges,workspace,baseline))
    return {"scope_kind":kind,"identity":identity,"unit_ids":members,
      "control_flow_complexity":{"maximum":maxv,"sum":sum(v for _,v in vals),"contributors":[{"unit_id":i,"value":v} for i,v in vals]},
      "direct_dependency_set":deps,"declared_capability_set":caps,"state_read_set":reads,"state_write_set":writes,
      "dependency_component_size":len(members) if kind=="dependency-component" else max(sizes.values()),"minimal_context_node_count":context}

def scope_records(owner:str, units:list[dict[str,Any]], edges:list[list[str]], workspace:dict[str,Any], extra_scopes:list[str])->list[dict[str,Any]]:
    um={u["id"]:u for u in units}; require(owner in um,"analysis scope is not an executable unit")
    comps=m23.strongly_connected(set(um),edges); comp=next(c for c in comps if owner in c); module=unit_module(owner,um[owner]["group"],workspace); group=um[owner]["group"]
    specs=[("executable-unit",owner,[owner]),("module","module:"+module,[u["id"] for u in units if unit_module(u["id"],u["group"],workspace)==module]),("dependency-component",component_id(comp),comp),("architecture-group","group:"+group,[u["id"] for u in units if u["group"]==group])]
    # A finding scope outside the owner's four scopes adds exactly that policy scope.
    for scope in sorted(set(extra_scopes)):
        if scope.startswith("group:") and scope not in {x[1] for x in specs}:
            g=scope.split(":",1)[1]; specs.append(("architecture-group",scope,[u["id"] for u in units if u["group"]==g]))
        elif scope in um and scope not in {x[1] for x in specs}: specs.append(("executable-unit",scope,[scope]))
    order={"executable-unit":0,"module":1,"dependency-component":2,"architecture-group":3}
    specs.sort(key=lambda x:(order[x[0]],x[1]))
    return [metric_record(k,i,m,units,edges,workspace,comps,i=="transport:dispatch") for k,i,m in specs]

def evaluate(package:dict[str,Any], workspace:dict[str,Any], candidates:dict[str,Any], candidate_id:str, mutation:dict[str,Any]|None=None, evaluation_input:dict[str,Any]|None=None)->dict[str,Any]:
    mutation=mutation or {}; evaluation_input=evaluation_input or evaluation_input_for(workspace,candidates,candidate_id)
    request=evaluation_input["request"]
    base_units,base_edges=m23.expand(workspace)
    cand=None if candidate_id=="r1" else next(c for c in candidates["candidates"] if c["id"]==candidate_id)
    if cand:
        cand=copy.deepcopy(cand); cand["endpoint_groups"]=workspace["endpoint_groups"]
        units,edges,added=m23.apply_candidate(base_units,base_edges,cand)
    else: units,edges,added=base_units,base_edges,set()
    # Executable fixture mutations are applied to semantic input, not trusted labels.
    for value in ([mutation["add_unit"]] if mutation.get("add_unit") else [])+mutation.get("add_units",[]):
        u=copy.deepcopy(value); units.append(u); added.add(u["id"])
    if mutation.get("change_cfg"):
        for u in units:
            if u["id"]==mutation["change_cfg"]["unit_id"]: u["cfg"]={"nodes":mutation["change_cfg"]["nodes"],"edges":mutation["change_cfg"]["edges"]}
    if mutation.get("change_unit_facts"):
        change=mutation["change_unit_facts"]
        for u in units:
            if u["id"]==change["unit_id"]:
                for key in ("capabilities","state_reads","state_writes"): u[key]=copy.deepcopy(change[key])
    edges+=copy.deepcopy(mutation.get("add_edges",[]))
    endpoint_groups={**workspace["endpoint_groups"],**mutation.get("endpoint_groups",{})}
    m23.validate_graph(units,edges,endpoint_groups)
    for u in units:
        caps=sorted({t.split(":",1)[1].split(".",1)[0] for s,t,k in edges if s==u["id"] and k=="capability-use"})
        reads=sorted({t.split(":",1)[1] for s,t,k in edges if s==u["id"] and k=="state-read"})
        writes=sorted({t.split(":",1)[1] for s,t,k in edges if s==u["id"] and k=="state-write"})
        require(u["capabilities"]==caps and u["state_reads"]==reads and u["state_writes"]==writes,f"stale authority/state facts: {u['id']}")
    coverage=copy.deepcopy(workspace["coverage"]); coverage.update(mutation.get("coverage",{}))
    owner=mutation.get("analysis_scope", {"r1":"transport:dispatch","valid":"domain:handle:job.cancel","centralized":"transport:dispatch","helper-split":"transport:cancel_decision"}[candidate_id])
    findings=[]; um={u["id"]:u for u in units}; base_comps=m23.strongly_connected({u["id"] for u in base_units},base_edges)
    if m23.cfc(um["transport:dispatch"])>4 or len(m23.context_closure({"transport:dispatch"},edges,workspace,True))>3:
        findings.append(finding(f"{candidate_id}:dispatch-growth","M23-POL-DISPATCH-NO-GROWTH","transport:dispatch",["transport:dispatch"],{"base_cfc":4,"candidate_cfc":m23.cfc(um["transport:dispatch"]),"base_context":3,"candidate_context":len(m23.context_closure({"transport:dispatch"},edges,workspace,True))}))
    transport={u["id"] for u in units if u["group"]=="transport"}
    cap_edges=[e for e in edges if e[0] in transport and e[2]=="capability-use"]
    if cap_edges: findings.append(finding(f"{candidate_id}:transport-capability","M23-POL-TRANSPORT-CAPABILITY","group:transport",[x for e in cap_edges for x in e[:2]],{"actual":sorted({e[1].split(":",1)[1].split(".",1)[0] for e in cap_edges}),"allowed":[]}))
    state_edges=[e for e in edges if e[0] in transport and e[2] in {"state-read","state-write"}]
    if state_edges: findings.append(finding(f"{candidate_id}:transport-state","M23-POL-TRANSPORT-STATE","group:transport",[x for e in state_edges for x in e[:2]],{"reads":sorted({e[1].split(":",1)[1] for e in state_edges if e[2]=="state-read"}),"writes":sorted({e[1].split(":",1)[1] for e in state_edges if e[2]=="state-write"}),"allowed":[]}))
    endpoint_group={**workspace["endpoint_groups"],**{u["id"]:u["group"] for u in units}}
    forbidden=[]
    for s,t,k in edges:
        sg,tg=endpoint_group.get(s),endpoint_group.get(t)
        if k in m23.DEPENDENCY_KINDS and sg and tg and sg!=tg and tg not in package["policy"]["allowed_group_dependencies"][sg]: forbidden.append((sg,tg,s,t,k))
    if forbidden: findings.append(finding(f"{candidate_id}:group-dependency","M23-POL-GROUP-DEPENDENCY","group:"+sorted(forbidden)[0][0],[x for e in forbidden for x in e[2:4]],{"forbidden_group_edges":[{"source_group":a,"target_group":b,"source":s,"target":t,"kind":k} for a,b,s,t,k in sorted(forbidden)]}))
    for identity in sorted(added):
        u=um[identity]; context=len(m23.context_closure({identity},edges,workspace,False))
        if u["group"]!="contract" and (m23.cfc(u)>4 or context>12): findings.append(finding(f"{candidate_id}:new-unit:{identity}","M23-POL-NEW-UNIT",identity,[identity],{"cfc":m23.cfc(u),"cfc_max":4,"context":context,"context_max":12}))
    comps=m23.strongly_connected(set(um),edges); newcycles=[c for c in comps if len(c)>1 and c not in base_comps]
    if newcycles: findings.append(finding(f"{candidate_id}:new-cycle","M23-POL-NO-NEW-CYCLE",component_id(newcycles[0]),[x for c in newcycles for x in c],{"components":[{"id":component_id(c),"members":c} for c in newcycles]}))
    required=workspace["coverage"]["analyzed"]
    if coverage["analyzed"]!=required or not coverage.get("complete_for_policy",False): findings.append(finding(f"{candidate_id}:coverage","M23-POL-COVERAGE","workspace",[],{"required_groups":required,"analyzed_groups":coverage["analyzed"],"unchecked_boundaries":coverage["unchecked"]}))
    requested=request["requested_governance_changes"]; baseline_revision=request["baseline_revision"]
    require(evaluation_input["active_policy_revision"]==workspace["policy_revision"] and evaluation_input["active_baseline_revision"]==workspace["baseline_revision"],"trusted governance context mismatch")
    if baseline_revision!=workspace["baseline_revision"]: findings.append(finding(f"{candidate_id}:stale-baseline","M24-BASELINE-COMPATIBILITY","transport:dispatch",[],{"required":workspace["baseline_revision"],"provided":baseline_revision}))
    if requested:
        auth=next((a for a in evaluation_input["governance_authorizations"] if a["authorization_id"]==request["authorization_id"]),None)
        changes_ok=len(requested)==1
        change=requested[0]
        actual_revision=cand["revision"] if cand else workspace["revision"]
        binding={"candidate_revision_id":actual_revision,"candidate_graph_digest":graph_digest(units,edges,workspace),"policy_revision":evaluation_input["active_policy_revision"],"baseline_revision":evaluation_input["active_baseline_revision"],"kind":change["kind"],"exception_ids":sorted(change["exception_ids"]),"rule":change["rule"],"scope":change["scope"],"review_boundary":request["review_boundary"]}
        request_bound=request["candidate_revision_id"]==actual_revision and request["policy_revision"]==evaluation_input["active_policy_revision"] and request["baseline_revision"]==evaluation_input["active_baseline_revision"]
        if not changes_ok or not request_bound or auth is None or any(auth.get(k)!=v for k,v in binding.items()): findings.append(finding(f"{candidate_id}:governance","M23-POL-GOVERNANCE","governance",[],{"requested_changes":requested,"authorization_id":request["authorization_id"],"trusted_authorization_matched":False}))
    # Exceptions are trusted records and exact matching suppresses only their finding.
    exceptions=copy.deepcopy(evaluation_input["active_exceptions"]); boundary=request["review_boundary"]
    kept=[]
    for f in findings:
        match=next((e for e in exceptions if e["rule"]==f["rule"] and e["scope"]==f["scope"] and e["contributors"]==f["contributors"] and e["policy_revision"]==workspace["policy_revision"] and e["expires_after_review_boundary"]>=boundary),None)
        if match: f["disposition"]="record"; f["exception"]=match; kept.append(f)
        else: kept.append(f)
    findings=ordered_findings(kept)
    node_count=len({u["id"] for u in units}|{x for e in edges for x in e[:2]})
    exhausted=None
    scopes=scope_records(owner,units,edges,workspace,[f["scope"] for f in findings])
    coverage_record={"analyzed_groups":coverage["analyzed"],"unchecked_boundaries":coverage["unchecked"],"complete_for_policy":not any(f["code"]==CODES[6] for f in findings)}
    debt=[{"rule":"M23-POL-DISPATCH-NO-GROWTH","scope":"transport:dispatch","baseline_revision":workspace["baseline_revision"],"metrics":{"control_flow_complexity":4,"minimal_context_node_count":3}}]
    analysis={"workspace_id":"m23-job-service","revision_id":cand["revision"] if cand else workspace["revision"],"semantic_model_version":"m24-v1","policy_revision":workspace["policy_revision"],"baseline_revision":baseline_revision,"analysis_scope":owner}
    budget={"semantic_nodes":node_count,"typed_edges":len(edges),"structured_bytes":0,"compact_bytes":0,"compact_lines":0,"exhausted":exhausted}
    classification="incomplete" if any(f["classification"]=="incomplete" and f["disposition"]=="deny" for f in findings) else "rejected" if any(f["disposition"]=="deny" for f in findings) else "accepted"
    snapshot={"format":"ail.architecture.snapshot.v1","analysis":analysis,"scopes":scopes,"coverage":coverage_record,"budgets":budget,"active_policies":policies(package),"baseline_match":{"baseline_revision":workspace["baseline_revision"],"scope":"transport:dispatch","metrics":{"control_flow_complexity":4,"minimal_context_node_count":3},"accepted_debt":True} if baseline_revision==workspace["baseline_revision"] else None,"accepted_debt":debt,"exceptions":exceptions,"findings":findings,"classification":classification,"compact":""}
    compact_lines=[f"{classification} {analysis['revision_id']} scope={owner}",f"policy={analysis['policy_revision']} baseline={baseline_revision}"]+[f"{f['code']} {f['scope']} contributors={','.join(f['contributors']) or '-'}" for f in findings]+[f"coverage={len(coverage_record['analyzed_groups'])} unchecked={len(coverage_record['unchecked_boundaries'])}"]
    snapshot["compact"]="\n".join(compact_lines)+"\n"; budget["compact_bytes"]=len(snapshot["compact"].encode()); budget["compact_lines"]=snapshot["compact"].count("\n")
    # Measure with structured_bytes zero and exhausted at its final value. Node
    # and edge limits have precedence. A compact failure can itself make the
    # structured representation exceed its earlier limit, so recheck it after
    # installing the preliminary compact exhaustion value.
    if budget["semantic_nodes"]>LIMITS["semantic_nodes"]: exhausted="semantic_nodes"
    elif budget["typed_edges"]>LIMITS["typed_edges"]: exhausted="typed_edges"
    else:
        budget["exhausted"]=None
        measured=len(canonical(snapshot))
        if measured>LIMITS["structured_bytes"]: exhausted="structured_bytes"
        elif budget["compact_bytes"]>LIMITS["compact_bytes"]: exhausted="compact_bytes"
        elif budget["compact_lines"]>LIMITS["compact_lines"]: exhausted="compact_lines"
        if exhausted in {"compact_bytes","compact_lines"}:
            budget["exhausted"]=exhausted
            if len(canonical(snapshot))>LIMITS["structured_bytes"]: exhausted="structured_bytes"
    budget["exhausted"]=exhausted
    budget["structured_bytes"]=len(canonical(snapshot))
    return {"snapshot":snapshot,"units":units,"edges":edges}

def delta(base:dict[str,Any],candidate:dict[str,Any],base_revision:str,candidate_revision:str)->dict[str,Any]:
    bm={(s["scope_kind"],s["identity"]):s for s in base["scopes"]}; cm={(s["scope_kind"],s["identity"]):s for s in candidate["scopes"]}
    # Scope changes identify the exact canonical records by digest rather than
    # embedding both records again; both full records are in the bound snapshots.
    changes=[{"scope_kind":k[0],"identity":k[1],"base":object_digest(bm[k]) if k in bm else None,"candidate":object_digest(cm[k]) if k in cm else None} for k in sorted(set(bm)|set(cm),key=lambda x:({"executable-unit":0,"module":1,"dependency-component":2,"architecture-group":3}[x[0]],x[1])) if bm.get(k)!=cm.get(k)]
    denied=any(f["disposition"]=="deny" for f in candidate["findings"])
    text=f"{candidate['classification']} {base_revision}->{candidate_revision} changes={len(changes)} findings={len(candidate['findings'])} publication={'none' if denied else candidate_revision}\n"
    return {"format":"ail.architecture.delta.v1","base_snapshot_digest":object_digest(base),"candidate_snapshot_digest":object_digest(candidate),"base_revision_id":base_revision,"candidate_revision_id":candidate_revision,"scope_changes":changes,"findings":candidate["findings"],"classification":candidate["classification"],"publication":"not-published" if denied else "published","commit":"rolled-back" if denied else "committed","compact":text}

def response(package:dict[str,Any],workspace:dict[str,Any],candidates:dict[str,Any],candidate_id:str,mutation:dict[str,Any]|None=None,evaluation_input:dict[str,Any]|None=None)->dict[str,Any]:
    base=evaluate(package,workspace,candidates,"r1",{"analysis_scope":(mutation or {}).get("analysis_scope","transport:dispatch")})["snapshot"]
    ev=evaluate(package,workspace,candidates,candidate_id,mutation,evaluation_input)
    snap=ev["snapshot"]; rev=snap["analysis"]["revision_id"]; d=delta(base,snap,workspace["revision"],rev)
    coverage_incomplete=not snap["coverage"]["complete_for_policy"]
    exhausted=snap["budgets"]["exhausted"]
    if coverage_incomplete or exhausted:
        if exhausted:
            used=snap["budgets"][exhausted]
            diagnostics=[finding(f"{candidate_id}:analysis-budget","M24-ANALYSIS-BUDGET","workspace",[],{"exhausted":exhausted,"used":used,"limit":LIMITS[exhausted]})]
        else:
            diagnostics=[f for f in snap["findings"] if f["code"]==CODES[6]]
        result={"status":"incomplete","analysis":snap["analysis"],"coverage":snap["coverage"],"budgets":snap["budgets"],"diagnostics":diagnostics,"edits":[],"current_revision_id":workspace["revision"],"published_child_revision_id":None}
        require(len(canonical(result))<=LIMITS["structured_bytes"],"incomplete response must remain bounded")
        return result
    denied=any(f["disposition"]=="deny" for f in snap["findings"])
    if denied: return {"status":"failure","base_revision_id":workspace["revision"],"current_revision_id":workspace["revision"],"snapshot":snap,"delta":d,"diagnostics":snap["findings"],"edits":[],"published_child_revision_id":None}
    completion={"base_revision_id":workspace["revision"],"revision_id":rev,"base_snapshot_digest":object_digest(base),"snapshot_digest":object_digest(snap),"delta_digest":object_digest(d),"policy_revision":workspace["policy_revision"],"baseline_revision":workspace["baseline_revision"],"coverage":snap["coverage"],"budgets":snap["budgets"],"behavior_validation":{"status":"not-applicable" if candidate_id=="r1" else "passed","cases_passed":0 if candidate_id=="r1" else 6,"cases_total":0 if candidate_id=="r1" else 6},"commit":"committed"}
    return {"status":"success","snapshot":snap,"delta":d,"completion":completion}

def budget_units()->list[dict[str,Any]]:
    return [{"id":f"budget:node:{i:03}","group":"contract","cfg":{"nodes":1,"edges":0},"capabilities":[],"state_reads":[],"state_writes":[]} for i in range(513)]
def budget_edges(workspace:dict[str,Any],candidates:dict[str,Any])->list[list[str]]:
    units,_=m23.expand(workspace); valid=next(c for c in candidates["candidates"] if c["id"]=="valid")
    ids=sorted({u["id"] for u in units+valid["added_units"]}); existing={tuple(e) for e in valid["added_edges"]}
    result=[]
    for source in ids:
        for target in ids:
            edge=[source,target,"verifies"]
            if tuple(edge) not in existing: result.append(edge)
            if len(result)==2049: return result
    raise CheckError("insufficient unique budget edges")
def compact_line_units()->list[dict[str,Any]]:
    return [{"id":f"domain:line-budget:{i}","group":"domain","cfg":{"nodes":8,"edges":12},"capabilities":[],"state_reads":[],"state_writes":[]} for i in range(10)]
SCENARIOS=[
 ("boundary-edge","valid",{"add_edges":[["transport:dispatch","state:jobs","type-use"]]}),
 ("authority-edge","valid",{"add_edges":[["transport:dispatch","capability:jobs_store.cancel_if_active","capability-use"]],"change_unit_facts":{"unit_id":"transport:dispatch","capabilities":["jobs_store"],"state_reads":[],"state_writes":[]}}),
 ("state-edge","valid",{"add_edges":[["transport:dispatch","state:jobs","state-read"]],"change_unit_facts":{"unit_id":"transport:dispatch","capabilities":[],"state_reads":["jobs"],"state_writes":[]}}),
 ("hotspot-cfg-growth","valid",{"change_cfg":{"unit_id":"transport:dispatch","nodes":8,"edges":11}}),
 ("new-unit-cfg-context","valid",{"add_unit":{"id":"domain:oversized","group":"domain","cfg":{"nodes":8,"edges":12},"capabilities":[],"state_reads":[],"state_writes":[]}}),
 ("new-cycle-edge","valid",{"add_edges":[["domain:handle:job.cancel","transport:adapt:job.cancel","calls"]]}),
 ("coverage-loss","valid",{"coverage":{"analyzed":["contract","transport","domain","persistence-adapter"],"complete_for_policy":False}}),
 ("stale-baseline","valid",{}),
 ("candidate-owned-policy-mutation","valid",{}),
 ("candidate-owned-baseline-mutation","valid",{}),
 ("candidate-owned-exception-mutation","centralized",{}),
 ("analysis-node-budget","valid",{"add_units":budget_units()}),
 ("analysis-edge-budget","valid",{"generated_budget_edges":True}),
 ("encoded-output-budget","valid",{"add_unit":{"id":"domain:"+"x"*66000,"group":"domain","cfg":{"nodes":8,"edges":12},"capabilities":[],"state_reads":[],"state_writes":[]}}),
 ("compact-byte-budget","valid",{"add_unit":{"id":"domain:"+"x"*3000,"group":"domain","cfg":{"nodes":8,"edges":12},"capabilities":[],"state_reads":[],"state_writes":[]}}),
 ("compact-line-budget","valid",{"add_units":compact_line_units()}),
]
def exact_exception(expiry="review-2026-07-22",contributors=None)->dict[str,Any]:
    return {"id":"exception-dispatch-growth","rule":"M23-POL-DISPATCH-NO-GROWTH","scope":"transport:dispatch","contributors":sorted(set(contributors or ["transport:dispatch"])),"policy_revision":"m23-policy-r1","expires_after_review_boundary":expiry}
SCENARIOS += [("trusted-exact-exception","valid",{"change_cfg":{"unit_id":"transport:dispatch","nodes":8,"edges":11}}),
              ("expired-exception","centralized",{}),
              ("contributor-mismatch-exception","centralized",{}),
              ("authorized-governance-change","valid",{}),
              ("unknown-authorization-id","valid",{}),
              ("authorization-binding-mismatch","valid",{}),
              ("authorization-kind-mismatch","valid",{})]

def evaluation_input_for(workspace:dict[str,Any],candidates:dict[str,Any],candidate_id:str)->dict[str,Any]:
    revision=workspace["revision"] if candidate_id=="r1" else next(c["revision"] for c in candidates["candidates"] if c["id"]==candidate_id)
    request={"base_revision_id":workspace["revision"],"candidate_revision_id":revision,"analysis_scope":{"r1":"transport:dispatch","valid":"domain:handle:job.cancel","centralized":"transport:dispatch","helper-split":"transport:cancel_decision"}[candidate_id],"policy_revision":workspace["policy_revision"],"baseline_revision":workspace["baseline_revision"],"review_boundary":"review-2026-07-22","requested_governance_changes":[],"authorization_id":None}
    return {"request":request,"governance_authorizations":[],"active_exceptions":[],"active_policy_revision":workspace["policy_revision"],"active_baseline_revision":workspace["baseline_revision"]}

def scenario_input(workspace:dict[str,Any],candidates:dict[str,Any],sid:str,cid:str,units:list[dict[str,Any]],edges:list[list[str]])->dict[str,Any]:
    value=evaluation_input_for(workspace,candidates,cid); req=value["request"]
    changes={
      "candidate-owned-policy-mutation":{"kind":"policy","rule":"M23-POL-TRANSPORT-CAPABILITY","scope":"group:transport","exception_ids":[]},
      "candidate-owned-baseline-mutation":{"kind":"baseline","rule":"M23-POL-DISPATCH-NO-GROWTH","scope":"transport:dispatch","exception_ids":[]},
      "candidate-owned-exception-mutation":{"kind":"exception","rule":"M23-POL-DISPATCH-NO-GROWTH","scope":"transport:dispatch","exception_ids":["candidate-exception"]},
      "authorized-governance-change":{"kind":"policy","rule":"M23-POL-TRANSPORT-CAPABILITY","scope":"group:transport","exception_ids":[]},
      "unknown-authorization-id":{"kind":"policy","rule":"M23-POL-TRANSPORT-CAPABILITY","scope":"group:transport","exception_ids":[]},
      "authorization-binding-mismatch":{"kind":"policy","rule":"M23-POL-TRANSPORT-CAPABILITY","scope":"group:transport","exception_ids":[]},
      "authorization-kind-mismatch":{"kind":"baseline","rule":"M23-POL-TRANSPORT-CAPABILITY","scope":"group:transport","exception_ids":[]}}
    if sid=="stale-baseline": req["baseline_revision"]="m23-baseline-stale"
    if sid in changes:
        req["requested_governance_changes"]=[changes[sid]]; req["authorization_id"]="auth-governance-valid" if sid!="unknown-authorization-id" else "auth-unknown"
    if sid in {"trusted-exact-exception","expired-exception","contributor-mismatch-exception"}:
        value["active_exceptions"]=[exact_exception("review-2026-07-21" if sid=="expired-exception" else "review-2026-07-22", ["transport:dispatch","transport:adapt:job.cancel"] if sid=="contributor-mismatch-exception" else None)]
    if sid in {"authorized-governance-change","authorization-binding-mismatch","authorization-kind-mismatch"}:
        change=changes[sid]
        auth={"authorization_id":"auth-governance-valid","candidate_revision_id":req["candidate_revision_id"],"candidate_graph_digest":graph_digest(units,edges,workspace),"policy_revision":req["policy_revision"],"baseline_revision":req["baseline_revision"],"kind":change["kind"],"exception_ids":sorted(change["exception_ids"]),"rule":change["rule"],"scope":change["scope"],"review_boundary":req["review_boundary"]}
        if sid=="authorization-binding-mismatch": auth["candidate_graph_digest"]="sha256:"+"0"*64
        if sid=="authorization-kind-mismatch": auth["kind"]="policy"
        value["governance_authorizations"]=[auth]
    return value

def derive_fixtures(package,workspace,candidates):
    results={"fixture_format":2,"accepted_input_fixture_set_digest":package["fixture_set_digest"],"operations":[]}
    for cid in ["r1","valid","centralized","helper-split"]:
        inp=evaluation_input_for(workspace,candidates,cid)
        if cid=="r1":
            snapshot_input={"request":{"revision_id":workspace["revision"],"analysis_scope":"transport:dispatch"},"active_exceptions":[],"active_policy_revision":workspace["policy_revision"],"active_baseline_revision":workspace["baseline_revision"]}
            snap=evaluate(package,workspace,candidates,"r1",evaluation_input=inp)["snapshot"]
            expected={"status":"success","snapshot":snap,"snapshot_digest":object_digest(snap)}
            results["operations"].append({"candidate":cid,"operation":"architecture_snapshot","input":snapshot_input,"expected_response":expected})
        else: results["operations"].append({"candidate":cid,"operation":"validate_architecture_change","input":inp,"expected_response":response(package,workspace,candidates,cid,evaluation_input=inp)})
    snapshot_input={"request":{"revision_id":workspace["revision"],"analysis_scope":"transport:dispatch"},"active_exceptions":[],"active_policy_revision":workspace["policy_revision"],"active_baseline_revision":workspace["baseline_revision"]}
    incomplete=response(package,workspace,candidates,"r1",{"coverage":{"analyzed":["contract","transport","domain","persistence-adapter"],"complete_for_policy":False}},evaluation_input_for(workspace,candidates,"r1"))
    results["operations"].append({"candidate":"r1-coverage-incomplete","operation":"architecture_snapshot","input":snapshot_input,"expected_response":incomplete})
    rejects={"fixture_format":2,"scenarios":[]}
    for sid,cid,mut in SCENARIOS:
        mut=copy.deepcopy(mut)
        if mut.pop("generated_budget_edges",False): mut["add_edges"]=budget_edges(workspace,candidates)
        # Governance and trusted context are protocol input, never semantic mutation.
        if sid in {"authorized-governance-change","authorization-binding-mismatch","authorization-kind-mismatch"}:
            preview=evaluate(package,workspace,candidates,cid,mut,evaluation_input_for(workspace,candidates,cid)); units,edges=preview["units"],preview["edges"]
        else: units,edges=[],[]
        inp=scenario_input(workspace,candidates,sid,cid,units,edges)
        rejects["scenarios"].append({"id":sid,"candidate":cid,"operation":"validate_architecture_change","input":inp,"semantic_mutation":mut,"expected_response":response(package,workspace,candidates,cid,mut,inp)})
    return results,rejects

def exact_keys(v:dict[str,Any],shape:str)->None: require(list(v)==SHAPE_KEYS[shape],f"{shape} field shape/order")
def walk_input(operation:str, inp:dict[str,Any])->None:
    if operation=="architecture_snapshot":
        exact_keys(inp,"ArchitectureSnapshotInput"); exact_keys(inp["request"],"ArchitectureSnapshotRequest")
    else:
        exact_keys(inp,"ArchitectureEvaluationInput"); exact_keys(inp["request"],"ArchitectureRequest")
        for change in inp["request"]["requested_governance_changes"]: exact_keys(change,"GovernanceChange"); require(change["exception_ids"]==sorted(set(change["exception_ids"])),"governance exception set")
        for auth in inp["governance_authorizations"]: exact_keys(auth,"GovernanceAuthorization"); require(auth["exception_ids"]==sorted(set(auth["exception_ids"])),"authorization exception set")
    for exception in inp["active_exceptions"]: exact_keys(exception,"Exception"); require(exception["contributors"]==sorted(set(exception["contributors"])),"exception contributor set")

def walk_protocol(resp:dict[str,Any], operation:str)->None:
    if resp["status"]=="incomplete":
        exact_keys(resp,"ArchitectureIncompleteFailure"); exact_keys(resp["analysis"],"AnalysisIdentity"); exact_keys(resp["coverage"],"Coverage"); exact_keys(resp["budgets"],"BudgetUse")
        for x in resp["diagnostics"]: exact_keys(x,"Finding")
        require(resp["edits"]==[] and resp["published_child_revision_id"] is None and len(canonical(resp))<=LIMITS["structured_bytes"],"bounded incomplete failure")
        return
    if operation=="architecture_snapshot":
        exact_keys(resp,"ArchitectureSnapshotResponse"); require(resp["status"]=="success","snapshot response status")
        s=resp["snapshot"]
        require(resp["snapshot_digest"]==object_digest(s),"snapshot response digest")
    else:
        exact_keys(resp,"ArchitectureSuccess" if resp["status"]=="success" else "ArchitectureFailure"); s=resp["snapshot"]
    exact_keys(s,"ArchitectureSnapshot"); exact_keys(s["analysis"],"AnalysisIdentity"); exact_keys(s["coverage"],"Coverage"); exact_keys(s["budgets"],"BudgetUse")
    for x in s["scopes"]: exact_keys(x,"ScopeMetrics")
    for x in s["active_policies"]: exact_keys(x,"PolicyRule"); exact_keys(x["selector"],"PolicySelector")
    if s["baseline_match"]: exact_keys(s["baseline_match"],"BaselineMatch")
    for x in s["exceptions"]: exact_keys(x,"Exception")
    for x in s["findings"]: exact_keys(x,"Finding"); require(x["code"] in CODES and x["contributors"]==sorted(set(x["contributors"])),"finding enum/order")
    if operation=="architecture_snapshot":
        require(len(canonical(resp))<=LIMITS["structured_bytes"],"full snapshot response budget"); return
    d=resp["delta"]; exact_keys(d,"ArchitectureDelta")
    for x in d["scope_changes"]: exact_keys(x,"ScopeChange")
    for x in d["findings"]: exact_keys(x,"Finding")
    if resp["status"]=="success":
        exact_keys(resp["completion"],"CompletionEvidence"); exact_keys(resp["completion"]["coverage"],"Coverage"); exact_keys(resp["completion"]["budgets"],"BudgetUse")
    else: require(resp["edits"]==[] and resp["published_child_revision_id"] is None and resp["current_revision_id"]==resp["base_revision_id"],"atomic rollback")
    require(len(canonical(resp))<=LIMITS["structured_bytes"],"full operation response budget")

def validate(contract,protocol,results,rejections,locks=True):
    require(contract["status"]==protocol["status"]=="Accepted","status"); require(contract["diagnostic_codes"]==CODES,"diagnostic precedence")
    rule_ids=["M24-LANG-001","M24-LANG-002"]+[f"M24-PROTO-{n:03}" for n in range(1,7)]
    require([r["id"] for r in contract["rules"]]==rule_ids,"contract rule order")
    require(contract["accepted_metrics"]==["control_flow_complexity","direct_dependency_set","declared_capability_set","state_read_set","state_write_set","dependency_component_size","minimal_context_node_count"],"metric contract")
    require(contract["aggregate_scopes"]==["executable-unit","module","dependency-component","architecture-group"],"scope contract")
    membership=contract["module_membership"]
    require(membership["role_templates"]=={"contract":"contracts","transport_registration":"transport","transport_adapter":"transport","domain_handler":"domain","behavior_fixture":"tests"} and membership["explicit_units"]=={"domain:shared_policy":"domain","transport:dispatch":"transport","transport:cancel_decision":"transport","transport:cancel_store":"transport","transport:cancel_result":"transport"} and membership["endpoint_module"]=="adapters","module membership contract")
    documented=(SPECS/"architecture.md").read_text()
    require(all(f"### {rid} " in documented for rid in rule_ids),"rule/documentation equality")
    require(protocol["canonical_digests"]=={
      "algorithm":"SHA-256",
      "encoding":"lowercase sha256:<hex> over canonical JSON bytes; snapshot/delta objects omit their own digest",
      "budget_bytes":"structured_bytes is canonical snapshot bytes with budgets.structured_bytes zero and budgets.exhausted at its final value; full response must be <=65536",
      "candidate_graph":{
        "object_keys":["units","edges"],
        "unit_keys":["id","module","group","cfg","capabilities","state_reads","state_writes"],
        "cfg_keys":["nodes","edges"],
        "unit_order":"UTF-8 identity byte order",
        "set_order":"deduplicated UTF-8 byte order",
        "edge_shape":"[source, target, kind]",
        "edge_order":"UTF-8 source, UTF-8 target, then edge-kind precedence",
        "edge_kind_precedence":["calls","type-use","verifies","capability-use","state-read","state-write","delegates"],
        "duplicates":"duplicate unit identities and duplicate edge triples are rejected before digesting",
        "encoding":"canonical UTF-8 JSON with two-space indentation and one final LF",
        "digest":"lowercase sha256:<hex>"}},"canonical digest contract")
    require(protocol["shapes"]=={k:{"required":v,"optional":[]} for k,v in SHAPE_KEYS.items()},"protocol shapes exact")
    require(protocol["operations"]=={"architecture_snapshot":{"input":"ArchitectureSnapshotInput","success":"ArchitectureSnapshotResponse","failure":"ArchitectureIncompleteFailure"},"validate_architecture_change":{"input":"ArchitectureEvaluationInput","success":"ArchitectureSuccess","failure":["ArchitectureFailure","ArchitectureIncompleteFailure"]}},"protocol operations exact")
    if locks:
        for p,d in contract["accepted_input_digests"].items(): require(file_digest(ROOT/p)==d,f"accepted M23 input changed: {p}")
    package=load(SPECS/"architecture-acceptance.json"); workspace=load(SPECS/"architecture-acceptance-fixtures/workspace.json"); candidates=load(SPECS/"architecture-acceptance-fixtures/candidates.json"); expected=load(SPECS/"architecture-acceptance-fixtures/expected.json")
    m23.technical(package,workspace,copy.deepcopy(candidates),expected)
    derived=derive_fixtures(package,workspace,candidates); require(results==derived[0],"full result derivation"); require(rejections==derived[1],"executable rejection derivation")
    for item in results["operations"]: walk_input(item["operation"],item["input"]); walk_protocol(item["expected_response"],item["operation"])
    for item in rejections["scenarios"]: walk_input(item["operation"],item["input"]); walk_protocol(item["expected_response"],item["operation"])
    require(any(x["id"]=="trusted-exact-exception" and x["expected_response"]["snapshot"]["findings"][0]["disposition"]=="record" for x in rejections["scenarios"]),"trusted exact exception fixture")
    require(any(x["id"]=="authorized-governance-change" and x["expected_response"]["status"]=="success" for x in rejections["scenarios"]),"authorized governance fixture")
    require(any(x["id"]=="authorization-kind-mismatch" and x["expected_response"]["status"]=="failure" for x in rejections["scenarios"]),"authorization kind binding fixture")

def mutations(contract,protocol,results,rejections)->int:
    changes=[]
    def add(name,fn): changes.append((name,fn))
    add("shape field",lambda c,p,r,x:p["shapes"]["AnalysisIdentity"]["required"].pop()); add("shape order",lambda c,p,r,x:p["shapes"]["Finding"]["required"].reverse())
    add("module membership",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["scopes"][1].__setitem__("identity","module:transport"))
    add("aggregate set",lambda c,p,r,x:r["operations"][2]["expected_response"]["snapshot"]["scopes"][-1]["direct_dependency_set"].pop())
    add("scope order",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["scopes"].reverse())
    add("finding code",lambda c,p,r,x:r["operations"][2]["expected_response"]["snapshot"]["findings"][0].__setitem__("code",CODES[1]))
    add("finding disposition",lambda c,p,r,x:r["operations"][2]["expected_response"]["snapshot"]["findings"][0].__setitem__("disposition","warn"))
    add("finding order",lambda c,p,r,x:r["operations"][2]["expected_response"]["snapshot"]["findings"].reverse())
    add("snapshot digest",lambda c,p,r,x:r["operations"][1]["expected_response"]["delta"].__setitem__("candidate_snapshot_digest","sha256:"+"0"*64))
    add("delta digest",lambda c,p,r,x:r["operations"][1]["expected_response"]["completion"].__setitem__("delta_digest","sha256:"+"0"*64))
    add("policy value",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["active_policies"][1]["value"].append("jobs_store"))
    add("baseline debt",lambda c,p,r,x:r["operations"][0]["expected_response"]["snapshot"]["accepted_debt"].clear())
    add("exception contributor",lambda c,p,r,x:x["scenarios"][16]["input"]["active_exceptions"][0]["contributors"].append("x"))
    add("governance request",lambda c,p,r,x:x["scenarios"][8]["input"]["request"]["requested_governance_changes"].clear())
    add("semantic nodes",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["budgets"].__setitem__("semantic_nodes",0))
    add("typed edges",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["budgets"].__setitem__("typed_edges",0))
    add("structured bytes",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["budgets"].__setitem__("structured_bytes",0))
    add("compact bytes",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["budgets"].__setitem__("compact_bytes",0))
    add("compact lines",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["budgets"].__setitem__("compact_lines",0))
    add("budget exhausted",lambda c,p,r,x:x["scenarios"][11]["expected_response"]["budgets"].__setitem__("exhausted",None))
    add("compact",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"].__setitem__("compact","accepted\n"))
    add("rollback current",lambda c,p,r,x:x["scenarios"][0]["expected_response"].__setitem__("current_revision_id","arch-r2-valid"))
    add("rollback edits",lambda c,p,r,x:x["scenarios"][0]["expected_response"]["edits"].append({}))
    add("published child",lambda c,p,r,x:x["scenarios"][0]["expected_response"].__setitem__("published_child_revision_id","arch-r2-valid"))
    add("coverage",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["coverage"].__setitem__("complete_for_policy",False))
    add("analysis scope",lambda c,p,r,x:r["operations"][1]["input"]["request"].__setitem__("analysis_scope","transport:dispatch"))
    add("component id",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["scopes"][2].__setitem__("identity","component:bad"))
    add("CFC aggregate",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["scopes"][1]["control_flow_complexity"].__setitem__("sum",0))
    add("capability namespace",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"]["scopes"][-1]["declared_capability_set"].__setitem__(0,"jobs_store.cancel_if_active"))
    add("failure diagnostic",lambda c,p,r,x:x["scenarios"][0]["expected_response"]["diagnostics"].clear())
    add("classification",lambda c,p,r,x:r["operations"][1]["expected_response"]["snapshot"].__setitem__("classification","rejected"))
    add("publication",lambda c,p,r,x:r["operations"][1]["expected_response"]["delta"].__setitem__("publication","not-published"))
    add("completion coverage",lambda c,p,r,x:r["operations"][1]["expected_response"]["completion"]["coverage"].__setitem__("analyzed_groups",[]))
    add("exception expiry",lambda c,p,r,x:x["scenarios"][17]["input"]["active_exceptions"][0].__setitem__("expires_after_review_boundary","review-2099"))
    add("exception overbroad",lambda c,p,r,x:x["scenarios"][18]["input"]["active_exceptions"][0].__setitem__("contributors",["transport:dispatch"]))
    for name,fn in changes:
        vals=list(map(copy.deepcopy,(contract,protocol,results,rejections))); fn(*vals)
        try: validate(*vals,locks=False)
        except (CheckError,KeyError,IndexError,m23.CheckError): continue
        raise CheckError("mutation survived: "+name)
    # A semantic edge cannot be hidden behind stale duplicate unit facts.
    package=load(SPECS/"architecture-acceptance.json"); workspace=load(SPECS/"architecture-acceptance-fixtures/workspace.json"); candidates=load(SPECS/"architecture-acceptance-fixtures/candidates.json")
    try:
        evaluate(package,workspace,candidates,"valid",{"add_edges":[["transport:dispatch","capability:jobs_store.cancel_if_active","capability-use"]]})
    except CheckError as error: require("stale authority/state facts" in str(error),"stale duplicate unit-fact rejection")
    else: raise CheckError("mutation survived: stale duplicate unit facts")
    units,edges=m23.expand(workspace)
    digest=graph_digest(units,edges,workspace)
    reordered_units=[{**u,"capabilities":list(reversed(u["capabilities"])),"state_reads":list(reversed(u["state_reads"])),"state_writes":list(reversed(u["state_writes"]))} for u in reversed(units)]
    require(graph_digest(reordered_units,list(reversed(edges)),workspace)==digest,"canonical graph digest normalization")
    valid=next(c for c in candidates["candidates"] if c["id"]=="valid"); candidate=copy.deepcopy(valid); candidate["endpoint_groups"]=workspace["endpoint_groups"]
    candidate_units,candidate_edges,_=m23.apply_candidate(units,edges,candidate)
    inp=scenario_input(workspace,candidates,"authorized-governance-change","valid",list(reversed(candidate_units)),list(reversed(candidate_edges)))
    require(response(package,workspace,candidates,"valid",evaluation_input=inp)["status"]=="success","normalized digest authorization match")
    return len(changes)+1

def main()->int:
    ap=argparse.ArgumentParser(); ap.add_argument("command",choices=["check","generate"]); args=ap.parse_args()
    try:
        paths=["architecture-contract.json","architecture-protocol.json","architecture-fixtures/results.json","architecture-fixtures/rejections.json"]
        if args.command=="generate":
            p=load(SPECS/"architecture-acceptance.json"); w=load(SPECS/"architecture-acceptance-fixtures/workspace.json"); c=load(SPECS/"architecture-acceptance-fixtures/candidates.json"); r,x=derive_fixtures(p,w,c)
            (SPECS/paths[2]).write_bytes(canonical(r)); (SPECS/paths[3]).write_bytes(canonical(x)); print("generated M24 fixtures"); return 0
        vals=[load(SPECS/p) for p in paths]; validate(*vals); count=mutations(*vals)
        print(f"M24 architecture contract: valid; 8 rules, {len(SHAPE_KEYS)} shapes, 5 operation results, {len(SCENARIOS)} executable scenarios, {count} mutation tests")
        print("accepted M23 inputs: digest-locked; M24 evaluator independent of M23 evaluate/result/renderer; M24 remains Active")
        return 0
    except (CheckError,m23.CheckError,OSError,json.JSONDecodeError) as e: print("architecture_contract:",e,file=sys.stderr); return 1
if __name__=="__main__": raise SystemExit(main())
