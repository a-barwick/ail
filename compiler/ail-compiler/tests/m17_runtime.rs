use ail_compiler::{
    CapabilityEnvironment, CapabilityInterface, CapabilityOperation, CapabilityProvider,
    ExecutionRequest, ExecutionResponse, InspectionRequest, RenameRequest, RenameResponse,
    RuntimeFault, RuntimeValue, TypeCheckStatus, Workspace, check_source, format_source,
};

const SERVICE: &str = include_str!("../../../specs/runtime-fixtures/job-service.ail");

fn environment() -> CapabilityEnvironment {
    let mut jobs = CapabilityInterface::new();
    jobs.insert_operation(
        "insert_if_absent",
        CapabilityOperation::new(["Job"], "InsertOutcome"),
    );
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("JobsStore", jobs);
    environment
}

fn workspace() -> Workspace {
    Workspace::new(
        "m17-job-service",
        "m17-r1",
        "specs/runtime-fixtures/job-service.ail",
        SERVICE,
        environment(),
    )
    .expect("reference service creates a revision")
}

fn priority(case: &str) -> RuntimeValue {
    RuntimeValue::variant("Priority", case, None)
}

fn priority_option(case: Option<&str>) -> RuntimeValue {
    match case {
        Some(case) => RuntimeValue::variant("PriorityOption", "Some", Some(priority(case))),
        None => RuntimeValue::variant("PriorityOption", "None", None),
    }
}

fn request(job_id: &str, task: &str, payload: &[u8], priority: Option<&str>) -> RuntimeValue {
    RuntimeValue::record(
        "CreateJobRequest",
        [
            ("job_id", RuntimeValue::Text(job_id.to_owned())),
            ("task", RuntimeValue::Text(task.to_owned())),
            ("payload", RuntimeValue::Bytes(payload.to_vec())),
            ("priority", priority_option(priority)),
        ],
    )
}

fn execute_create_job(
    workspace: &Workspace,
    request: RuntimeValue,
    provider: &mut dyn CapabilityProvider,
) -> ExecutionResponse {
    workspace.execute(
        ExecutionRequest {
            revision_id: "m17-r1".to_owned(),
            function: "create_job".to_owned(),
            arguments: vec![request],
        },
        provider,
    )
}

#[derive(Debug)]
struct StoreProvider {
    outcome: RuntimeValue,
    invocations: usize,
}

impl StoreProvider {
    fn new(case: &str) -> Self {
        Self {
            outcome: RuntimeValue::variant("InsertOutcome", case, None),
            invocations: 0,
        }
    }
}

impl CapabilityProvider for StoreProvider {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        receiver == "jobs" && interface == "JobsStore"
    }

    fn call(
        &mut self,
        receiver: &str,
        interface: &str,
        operation: &str,
        arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        assert_eq!(
            (receiver, interface, operation),
            ("jobs", "JobsStore", "insert_if_absent")
        );
        assert_eq!(arguments.len(), 1);
        self.invocations += 1;
        Ok(self.outcome.clone())
    }
}

#[derive(Debug, Default)]
struct NoCapabilities;

impl CapabilityProvider for NoCapabilities {
    fn supports(&self, _receiver: &str, _interface: &str) -> bool {
        false
    }

    fn call(
        &mut self,
        _receiver: &str,
        _interface: &str,
        _operation: &str,
        _arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        panic!("a missing capability must never be called")
    }
}

fn completed(response: ExecutionResponse) -> ail_compiler::ExecutionSuccess {
    let ExecutionResponse::Completed(success) = response else {
        panic!("execution should complete")
    };
    success
}

fn result_case(value: &RuntimeValue) -> &str {
    let RuntimeValue::Variant {
        type_name, case, ..
    } = value
    else {
        panic!("result is a variant")
    };
    assert_eq!(type_name, "CreateJobResult");
    case
}

#[test]
fn reference_service_is_canonical_idempotent_and_statically_valid() {
    let canonical = format_source(SERVICE).expect("reference service formats");
    assert_eq!(canonical, SERVICE);
    assert_eq!(format_source(&canonical).unwrap(), canonical);

    let check = check_source(SERVICE, "m17-static", &environment());
    assert!(check.parse_diagnostics.is_empty());
    assert!(check.diagnostics.is_empty());
    assert_eq!(check.type_result.status, TypeCheckStatus::Ok);

    let workspace = workspace();
    let handles = workspace.handles("m17-r1").expect("revision handles");
    assert!(handles.len() > 100);
    for handle in handles {
        workspace
            .inspect(InspectionRequest {
                revision_id: "m17-r1".to_owned(),
                handle,
            })
            .expect("every indexed runtime construct inspects");
    }
}

#[test]
fn exhaustive_match_and_branch_type_diagnostics_are_stable() {
    let non_exhaustive = "variant Choice {\n  A;\n  B;\n}\n\nfn choose(value: Choice) -> Text {\n  match value {\n    Choice::A => {\n      \"a\"\n    },\n  }\n}\n";
    let check = check_source(
        non_exhaustive,
        "match-diagnostic",
        &CapabilityEnvironment::new(),
    );
    assert_eq!(check.diagnostics[0].code, "AIL.TYPE.NON_EXHAUSTIVE_MATCH");

    let if_mismatch =
        "fn choose(flag: Bool) -> Text {\n  if flag {\n    \"a\"\n  } else {\n    1\n  }\n}\n";
    let check = check_source(if_mismatch, "if-diagnostic", &CapabilityEnvironment::new());
    assert_eq!(check.diagnostics[0].code, "AIL.TYPE.IF_BRANCH_MISMATCH");

    let arm_mismatch = "variant Choice {\n  A;\n  B;\n}\n\nfn choose(value: Choice) -> Text {\n  match value {\n    Choice::A => {\n      \"a\"\n    },\n    Choice::B => {\n      1\n    },\n  }\n}\n";
    let check = check_source(
        arm_mismatch,
        "arm-diagnostic",
        &CapabilityEnvironment::new(),
    );
    assert_eq!(check.diagnostics[0].code, "AIL.TYPE.MATCH_ARM_MISMATCH");
}

#[test]
fn invalid_requests_make_zero_capability_calls() {
    let workspace = workspace();
    let mut provider = StoreProvider::new("Inserted");
    let success = completed(execute_create_job(
        &workspace,
        request("", "", &[0; 4097], None),
        &mut provider,
    ));

    assert_eq!(result_case(&success.value), "Invalid");
    assert!(success.calls.is_empty());
    assert_eq!(provider.invocations, 0);
}

#[test]
fn valid_requests_make_exactly_one_ordered_store_call() {
    let workspace = workspace();
    let mut provider = StoreProvider::new("Inserted");
    let success = completed(execute_create_job(
        &workspace,
        request("job-1", "rebuild", b"payload", Some("High")),
        &mut provider,
    ));

    assert_eq!(result_case(&success.value), "Created");
    assert_eq!(provider.invocations, 1);
    assert_eq!(success.calls.len(), 1);
    let call = &success.calls[0];
    assert_eq!(
        (
            call.receiver.as_str(),
            call.interface.as_str(),
            call.operation.as_str(),
        ),
        ("jobs", "JobsStore", "insert_if_absent")
    );
    assert_eq!(call.arguments.len(), 1);
    assert_eq!(
        call.arguments[0].field("job_id"),
        Some(&RuntimeValue::Text("job-1".to_owned()))
    );
    assert_eq!(call.arguments[0].field("priority"), Some(&priority("High")));
    assert_eq!(
        call.result,
        Some(RuntimeValue::variant("InsertOutcome", "Inserted", None))
    );
}

#[test]
fn store_outcomes_map_to_the_closed_public_result() {
    let workspace = workspace();
    for (outcome, expected) in [
        ("Inserted", "Created"),
        ("Duplicate", "AlreadyExists"),
        ("UnavailableBeforeCommit", "PersistenceUnavailable"),
    ] {
        let mut provider = StoreProvider::new(outcome);
        let success = completed(execute_create_job(
            &workspace,
            request("job-1", "task", b"", Some("Low")),
            &mut provider,
        ));
        assert_eq!(result_case(&success.value), expected);
        assert_eq!(success.calls.len(), 1);
        assert_eq!(provider.invocations, 1);
    }
}

#[test]
fn v1_requests_and_stored_jobs_adapt_explicitly_to_normal_priority() {
    let workspace = workspace();
    let v1_request = RuntimeValue::record(
        "CreateJobRequestV1",
        [
            ("job_id", RuntimeValue::Text("legacy".to_owned())),
            ("task", RuntimeValue::Text("task".to_owned())),
            ("payload", RuntimeValue::Bytes(vec![1, 2])),
        ],
    );
    let request = completed(workspace.execute(
        ExecutionRequest {
            revision_id: "m17-r1".to_owned(),
            function: "adapt_create_job_request_v1".to_owned(),
            arguments: vec![v1_request],
        },
        &mut NoCapabilities,
    ));
    assert_eq!(
        request.value.field("priority"),
        Some(&priority_option(Some("Normal")))
    );

    let stored = RuntimeValue::record(
        "StoredJobV1",
        [
            ("job_id", RuntimeValue::Text("legacy".to_owned())),
            ("task", RuntimeValue::Text("task".to_owned())),
            ("payload", RuntimeValue::Bytes(vec![1, 2])),
        ],
    );
    let job = completed(workspace.execute(
        ExecutionRequest {
            revision_id: "m17-r1".to_owned(),
            function: "adapt_stored_job".to_owned(),
            arguments: vec![stored],
        },
        &mut NoCapabilities,
    ));
    assert_eq!(job.value.field("priority"), Some(&priority("Normal")));
}

#[test]
fn execution_is_revision_scoped_and_retains_old_revisions() {
    let source = "fn identity(value: Text) -> Text {\n  value\n}\n";
    let mut workspace = Workspace::new(
        "revision-execution",
        "exec-r1",
        "main.ail",
        source,
        CapabilityEnvironment::new(),
    )
    .unwrap();
    let function = workspace
        .handles("exec-r1")
        .unwrap()
        .into_iter()
        .find(|handle| {
            workspace
                .inspect(InspectionRequest {
                    revision_id: "exec-r1".to_owned(),
                    handle: handle.clone(),
                })
                .is_ok_and(|inspection| inspection.semantic_kind == "function")
        })
        .unwrap();
    let RenameResponse::Committed(rename) = workspace.rename(RenameRequest {
        base_revision_id: "exec-r1".to_owned(),
        handle: function,
        new_name: "renamed".to_owned(),
    }) else {
        panic!("function rename commits")
    };

    let mut capabilities = NoCapabilities;
    let old = workspace.execute(
        ExecutionRequest {
            revision_id: "exec-r1".to_owned(),
            function: "identity".to_owned(),
            arguments: vec![RuntimeValue::Text("old".to_owned())],
        },
        &mut capabilities,
    );
    let new = workspace.execute(
        ExecutionRequest {
            revision_id: rename.revision.revision_id,
            function: "renamed".to_owned(),
            arguments: vec![RuntimeValue::Text("new".to_owned())],
        },
        &mut capabilities,
    );
    assert_eq!(completed(old).value, RuntimeValue::Text("old".to_owned()));
    assert_eq!(completed(new).value, RuntimeValue::Text("new".to_owned()));
}

#[test]
fn runtime_faults_and_repeated_results_are_deterministic() {
    let workspace = workspace();
    let malformed = RuntimeValue::record(
        "CreateJobRequest",
        [("job_id", RuntimeValue::Text("job-1".to_owned()))],
    );
    let execution_request = ExecutionRequest {
        revision_id: "m17-r1".to_owned(),
        function: "create_job".to_owned(),
        arguments: vec![malformed],
    };
    let left = workspace.execute(execution_request.clone(), &mut NoCapabilities);
    let right = workspace.execute(execution_request, &mut NoCapabilities);
    assert_eq!(left, right);
    let ExecutionResponse::Failed(failure) = left else {
        panic!("malformed host value faults")
    };
    assert_eq!(failure.fault.code, "AIL.RUNTIME.ARGUMENT_TYPE");
    assert!(failure.calls.is_empty());

    let valid = request("job-1", "task", b"", Some("Normal"));
    let left = execute_create_job(&workspace, valid.clone(), &mut NoCapabilities);
    let right = execute_create_job(&workspace, valid, &mut NoCapabilities);
    assert_eq!(left, right);
    let ExecutionResponse::Failed(failure) = left else {
        panic!("missing capability faults")
    };
    assert_eq!(failure.fault.code, "AIL.RUNTIME.MISSING_CAPABILITY");
    assert!(failure.calls.is_empty());
}
