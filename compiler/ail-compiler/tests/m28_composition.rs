use ail_compiler::{
    CapabilityEnvironment, CapabilityInterface, CapabilityOperation, CapabilityProvider,
    EvolutionCoverage, EvolutionSource, EvolutionWorkspace, ExecutionResponse, RuntimeFault,
    RuntimeValue, TypeCheckStatus, check_source, format_source,
};

#[derive(Default)]
struct TestCapabilities {
    calls: Vec<String>,
}

impl CapabilityProvider for TestCapabilities {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        receiver == "store" && interface == "Store"
    }

    fn call(
        &mut self,
        _receiver: &str,
        _interface: &str,
        operation: &str,
        arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        let RuntimeValue::Text(value) = &arguments[0] else {
            unreachable!("static checking requires Text")
        };
        self.calls.push(value.clone());
        match operation {
            "mark" => Ok(RuntimeValue::Text(value.clone())),
            _ => unreachable!("test interface has one operation"),
        }
    }
}

fn environment() -> CapabilityEnvironment {
    let mut store = CapabilityInterface::new();
    store.insert_operation("mark", CapabilityOperation::new(["Text"], "Text"));
    let mut environment = CapabilityEnvironment::new();
    environment.insert_interface("Store", store);
    environment
}

fn no_coverage() -> EvolutionCoverage {
    EvolutionCoverage {
        declared_complete: true,
        ..EvolutionCoverage::default()
    }
}

fn example_sources() -> Vec<EvolutionSource> {
    vec![
        EvolutionSource::new(
            "domain.ail",
            include_str!("../../examples/composed-service/domain.ail"),
        ),
        EvolutionSource::new(
            "service.ail",
            include_str!("../../examples/composed-service/service.ail"),
        ),
        EvolutionSource::new(
            "validation.ail",
            include_str!("../../examples/composed-service/validation.ail"),
        ),
    ]
}

#[test]
fn multi_file_service_imports_calls_and_executes() {
    let workspace = EvolutionWorkspace::new(
        "composed-service",
        "r1",
        example_sources(),
        &CapabilityEnvironment::new(),
        no_coverage(),
    )
    .expect("example must compile");
    let mut capabilities = TestCapabilities::default();

    let accepted = workspace.execute(
        "r1",
        "handle",
        vec![RuntimeValue::record(
            "domain.Request",
            [("name", RuntimeValue::Text("build".to_owned()))],
        )],
        &mut capabilities,
    );
    assert!(matches!(
        accepted,
        ExecutionResponse::Completed(result)
            if result.value == RuntimeValue::variant(
                "domain.Response",
                "Accepted",
                Some(RuntimeValue::Text("build".to_owned()))
            )
    ));

    let rejected = workspace.execute(
        "r1",
        "handle",
        vec![RuntimeValue::record(
            "domain.Request",
            [("name", RuntimeValue::Text(String::new()))],
        )],
        &mut capabilities,
    );
    assert!(matches!(
        rejected,
        ExecutionResponse::Completed(result)
            if result.value == RuntimeValue::variant("domain.Response", "Rejected", None)
    ));
}

#[test]
fn module_headers_and_imports_format_canonically() {
    let source = concat!(
        "module service; import validation; import domain; ",
        "fn handle(value: Text) -> Text { value }",
    );
    assert_eq!(
        format_source(source).expect("module source parses"),
        concat!(
            "module service;\n",
            "import domain;\n",
            "import validation;\n\n",
            "fn handle(value: Text) -> Text {\n",
            "  value\n",
            "}\n",
        )
    );

    let import_without_module = "import domain; fn run() -> Text { \"ok\" }";
    assert_eq!(
        format_source(import_without_module).expect("imports remain representable before linking"),
        "import domain;\n\nfn run() -> Text {\n  \"ok\"\n}\n"
    );
    assert!(
        module_failure(vec![("service.ail", import_without_module)])
            .contains(&"AIL.MODULE.MISSING_IDENTITY")
    );
}

#[test]
fn aliases_and_qualified_references_format_canonically() {
    let source = concat!(
        "module service.api; import right.models as right; import left.models as left; ",
        "fn echo(value: left.Item) -> right.Result { right.make(value) }",
    );
    assert_eq!(
        format_source(source).expect("qualified source parses"),
        concat!(
            "module service.api;\n",
            "import left.models as left;\n",
            "import right.models as right;\n\n",
            "fn echo(value: left.Item) -> right.Result {\n",
            "  right.make(value)\n",
            "}\n",
        )
    );
}

#[test]
fn function_calls_check_resolution_arguments_effects_and_recursion() {
    let unknown = check_source(
        "fn run(value: Text) -> Text { missing(value) }\n",
        "unknown",
        &CapabilityEnvironment::new(),
    );
    assert_eq!(unknown.diagnostics[0].code, "AIL.NAME.UNKNOWN_FUNCTION");

    let wrong_arguments = check_source(
        concat!(
            "fn take(value: Text) -> Text { value }\n\n",
            "fn run() -> Text { take(1, 2) }\n",
        ),
        "arguments",
        &CapabilityEnvironment::new(),
    );
    assert!(
        wrong_arguments
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AIL.TYPE.FUNCTION_ARGUMENTS")
    );
    assert!(
        wrong_arguments
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AIL.TYPE.FUNCTION_ARGUMENT")
    );

    let transitive = check_source(
        concat!(
            "fn write(value: Text, store: capability Store) -> Text effects { store.mark } {\n",
            "  store.mark(value)\n",
            "}\n\n",
            "fn run(value: Text, store: capability Store) -> Text {\n",
            "  write(value)\n",
            "}\n",
        ),
        "effects",
        &environment(),
    );
    assert!(
        transitive
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "AIL.CAPABILITY.UNDECLARED_TRANSITIVE_EFFECT" })
    );

    let recursion = check_source(
        concat!(
            "fn first(value: Text) -> Text { second(value) }\n\n",
            "fn second(value: Text) -> Text { first(value) }\n",
        ),
        "recursion",
        &CapabilityEnvironment::new(),
    );
    assert_eq!(recursion.type_result.status, TypeCheckStatus::Error);
    assert!(
        recursion
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AIL.CALL.RECURSIVE_CYCLE")
    );
}

#[test]
fn qualification_preserves_capability_intrinsic_and_field_parsing() {
    let source = concat!(
        "module store;\n",
        "record Request { enabled: Bool; }\n\n",
        "variant Outcome { Ready; Waiting; }\n\n",
        "record State { outcome: Outcome; }\n\n",
        "fn use_store(value: Text, store: capability Store) -> Text effects { store.mark } {\n",
        "  store.mark(value)\n",
        "}\n\n",
        "fn empty(value: Text) -> Bool { text.is_empty(value) }\n\n",
        "fn choose(request: Request) -> Text {\n",
        "  if request.enabled { \"yes\" } else { \"no\" }\n",
        "}\n\n",
        "fn inspect(state: State) -> Text {\n",
        "  match state.outcome {\n",
        "    Outcome::Ready => { \"ready\" },\n",
        "    Outcome::Waiting => { \"waiting\" },\n",
        "  }\n",
        "}\n",
    );
    let result = check_source(source, "dotted-parsing", &environment());
    assert_eq!(result.type_result.status, TypeCheckStatus::Ok);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn interpreter_evaluates_call_arguments_left_to_right() {
    let source = concat!(
        "fn first(store: capability Store) -> Text effects { store.mark } { store.mark(\"first\") }\n\n",
        "fn second(store: capability Store) -> Text effects { store.mark } { store.mark(\"second\") }\n\n",
        "fn choose(left: Text, right: Text) -> Text { left }\n\n",
        "fn run(store: capability Store) -> Text effects { store.mark } { choose(first(), second()) }\n",
    );
    let workspace = ail_compiler::Workspace::new("calls", "r1", "calls.ail", source, environment())
        .expect("source parses");
    let mut capabilities = TestCapabilities::default();
    let result = workspace.execute(
        ail_compiler::ExecutionRequest {
            revision_id: "r1".to_owned(),
            function: "run".to_owned(),
            arguments: Vec::new(),
        },
        &mut capabilities,
    );
    assert!(matches!(result, ExecutionResponse::Completed(_)));
    assert_eq!(capabilities.calls, ["first", "second"]);
}

fn module_failure(sources: Vec<(&str, &str)>) -> Vec<&'static str> {
    EvolutionWorkspace::new(
        "invalid-modules",
        "r1",
        sources
            .into_iter()
            .map(|(path, source)| EvolutionSource::new(path, source))
            .collect(),
        &CapabilityEnvironment::new(),
        no_coverage(),
    )
    .expect_err("source set must be rejected")
    .diagnostics
    .into_iter()
    .map(|diagnostic| diagnostic.code)
    .collect()
}

#[test]
fn modules_reject_invalid_identity_import_visibility_and_cycles() {
    assert!(
        module_failure(vec![
            ("a.ail", "module a;\nfn a() -> Text { \"a\" }\n"),
            ("legacy.ail", "fn legacy() -> Text { \"legacy\" }\n"),
        ])
        .contains(&"AIL.MODULE.MISSING_IDENTITY")
    );

    assert!(
        module_failure(vec![
            ("a.ail", "module same;\nfn a() -> Text { \"a\" }\n"),
            ("b.ail", "module same;\nfn b() -> Text { \"b\" }\n"),
        ])
        .contains(&"AIL.MODULE.DUPLICATE_IDENTITY")
    );

    assert!(
        module_failure(vec![
            ("a.ail", "module a;\nfn a() -> Text { \"a\" }\n"),
            (
                "b.ail",
                "module b;\nimport missing;\nfn b() -> Text { \"b\" }\n"
            ),
        ])
        .contains(&"AIL.MODULE.MISSING_IMPORT")
    );

    assert!(
        module_failure(vec![
            ("a.ail", "module a;\nfn hidden() -> Text { \"a\" }\n"),
            ("b.ail", "module b;\nfn use() -> Text { hidden() }\n"),
        ])
        .contains(&"AIL.MODULE.INACCESSIBLE_DECLARATION")
    );

    for hidden_call in ["a.hidden()", "a.deep.hidden()"] {
        let module = if hidden_call.starts_with("a.deep") {
            "a.deep"
        } else {
            "a"
        };
        assert!(
            module_failure(vec![
                (
                    "a.ail",
                    &format!("module {module};\nfn hidden() -> Text {{ \"a\" }}\n")
                ),
                (
                    "b.ail",
                    &format!("module b;\nfn use() -> Text {{ {hidden_call} }}\n")
                ),
            ])
            .contains(&"AIL.MODULE.INACCESSIBLE_DECLARATION")
        );
    }

    assert!(
        module_failure(vec![(
            "a.ail",
            "module a;\nfn use() -> Text { unknown_function() }\n"
        ),])
        .contains(&"AIL.NAME.UNKNOWN_FUNCTION")
    );

    assert!(
        module_failure(vec![
            ("a.ail", "module a;\nimport b;\nfn a() -> Text { b() }\n"),
            ("b.ail", "module b;\nimport a;\nfn b() -> Text { a() }\n"),
        ])
        .contains(&"AIL.MODULE.IMPORT_CYCLE")
    );

    assert!(
        module_failure(vec![
            ("a.ail", "module a;\nfn one() -> Text { \"a\" }\n"),
            ("b.ail", "module b;\nfn two() -> Text { \"b\" }\n"),
            (
                "c.ail",
                "module c;\nimport a;\nimport a;\nfn three() -> Text { one() }\n",
            ),
        ])
        .contains(&"AIL.MODULE.DUPLICATE_IMPORT")
    );

    let ambiguous = module_failure(vec![
        ("a.ail", "module a;\nfn shared() -> Text { \"a\" }\n"),
        ("b.ail", "module b;\nfn shared() -> Text { \"b\" }\n"),
        (
            "c.ail",
            "module c;\nimport a;\nimport b;\nfn use() -> Text { shared() }\n",
        ),
    ]);
    assert!(ambiguous.contains(&"AIL.MODULE.AMBIGUOUS_IMPORT"));
}

#[test]
fn duplicate_alias_rejection_is_source_order_independent() {
    let sources = vec![
        ("a.ail", "module a;\nfn one() -> Text { \"a\" }\n"),
        ("b.ail", "module b;\nfn two() -> Text { \"b\" }\n"),
        (
            "c.ail",
            concat!(
                "module c;\n",
                "import a as dependency;\n",
                "import b as dependency;\n",
                "fn run() -> Text { dependency.one() }\n",
            ),
        ),
    ];
    let forward = module_failure(sources.clone());
    let reverse = module_failure(sources.into_iter().rev().collect());
    assert_eq!(forward, reverse);
    assert!(forward.contains(&"AIL.MODULE.DUPLICATE_QUALIFIER"));
}

fn aliased_collision_sources() -> Vec<EvolutionSource> {
    vec![
        EvolutionSource::new(
            "left.ail",
            concat!(
                "module left;\n",
                "record Item identity \"left.item\" { text identity \"text\": Text; }\n\n",
                "variant Result identity \"left.result\" { Value identity \"value\"(Item); }\n\n",
                "fn read(item: Item) -> Text { item.text }\n",
            ),
        ),
        EvolutionSource::new(
            "right.ail",
            concat!(
                "module right;\n",
                "record Item identity \"right.item\" { number identity \"number\": Int; }\n\n",
                "variant Result identity \"right.result\" { Value identity \"value\"(Text); }\n\n",
                "fn make(value: Text) -> Result { Result::Value(value) }\n",
            ),
        ),
        EvolutionSource::new(
            "service.ail",
            concat!(
                "module service;\n",
                "import left as input;\n",
                "import right as output;\n\n",
                "fn text(item: input.Item) -> Text { input.read(item) }\n\n",
                "fn copy(item: input.Item) -> input.Item {\n",
                "  input.Item { text: input.read(item) }\n",
                "}\n\n",
                "fn direct(item: input.Item) -> output.Result {\n",
                "  output.Result::Value(service.text(item))\n",
                "}\n\n",
                "fn run(item: input.Item) -> output.Result {\n",
                "  output.make(service.text(service.copy(item)))\n",
                "}\n",
            ),
        ),
    ]
}

#[test]
fn aliases_disambiguate_calls_types_variants_execution_and_graph_facts() {
    let sources = aliased_collision_sources();
    let forward = EvolutionWorkspace::new(
        "qualified-collisions",
        "r1",
        sources.clone(),
        &CapabilityEnvironment::new(),
        no_coverage(),
    )
    .expect("source order does not affect qualification");
    let workspace = EvolutionWorkspace::new(
        "qualified-collisions",
        "r1",
        sources.into_iter().rev().collect(),
        &CapabilityEnvironment::new(),
        no_coverage(),
    )
    .expect("aliases make every colliding reference explicit");
    assert_eq!(
        forward.revision("r1"),
        workspace.revision("r1"),
        "source-set revisions must be independent of input order"
    );

    let mut capabilities = TestCapabilities::default();
    assert!(matches!(
        workspace.execute(
            "r1",
            "service.run",
            vec![RuntimeValue::record(
                "left.Item",
                [("text", RuntimeValue::Text("qualified".to_owned()))],
            )],
            &mut capabilities,
        ),
        ExecutionResponse::Completed(result)
            if result.value == RuntimeValue::variant(
                "right.Result",
                "Value",
                Some(RuntimeValue::Text("qualified".to_owned())),
            )
    ));
    assert!(matches!(
        workspace.execute(
            "r1",
            "service.run",
            vec![RuntimeValue::record(
                "right.Item",
                [("text", RuntimeValue::Text("wrong module".to_owned()))],
            )],
            &mut capabilities,
        ),
        ExecutionResponse::Failed(failure)
            if failure.fault.code == "AIL.RUNTIME.ARGUMENT_TYPE"
    ));

    let graph = workspace.graph("r1").expect("revision is retained");
    assert!(graph.iter().any(|edge| {
        edge.site.path == "service.ail"
            && edge.kind == "signature-input"
            && edge.target == "left.item"
    }));
    assert!(graph.iter().any(|edge| {
        edge.site.path == "service.ail"
            && edge.kind == "signature-output"
            && edge.target == "right.result"
    }));
    assert!(graph.iter().any(|edge| {
        edge.site.path == "service.ail" && edge.kind == "constructs" && edge.target == "left.item"
    }));
    assert!(graph.iter().any(|edge| {
        edge.site.path == "service.ail"
            && edge.kind == "constructs"
            && edge.target == "right.result"
    }));
}

#[test]
fn explicit_module_qualification_disambiguates_plain_imports() {
    let workspace = EvolutionWorkspace::new(
        "qualified-plain-imports",
        "r1",
        vec![
            EvolutionSource::new(
                "a.ail",
                concat!(
                    "module a;\n",
                    "record Item { value: Text; }\n\n",
                    "fn read(item: Item) -> Text { item.value }\n",
                ),
            ),
            EvolutionSource::new(
                "b.ail",
                concat!(
                    "module b;\n",
                    "record Item { value: Text; }\n\n",
                    "fn read(item: Item) -> Text { item.value }\n",
                ),
            ),
            EvolutionSource::new(
                "service.ail",
                concat!(
                    "module service;\n",
                    "import a;\n",
                    "import b;\n\n",
                    "fn run(item: a.Item) -> Text { b.read(b.Item { value: a.read(item) }) }\n",
                ),
            ),
        ],
        &CapabilityEnvironment::new(),
        no_coverage(),
    )
    .expect("qualified references resolve colliding plain imports");
    let mut capabilities = TestCapabilities::default();
    assert!(matches!(
        workspace.execute(
            "r1",
            "service.run",
            vec![RuntimeValue::record(
                "a.Item",
                [("value", RuntimeValue::Text("selected".to_owned()))],
            )],
            &mut capabilities,
        ),
        ExecutionResponse::Completed(result)
            if result.value == RuntimeValue::Text("selected".to_owned())
    ));
}

#[test]
fn qualified_local_calls_participate_in_recursion_detection() {
    let failure = EvolutionWorkspace::new(
        "qualified-recursion",
        "r1",
        vec![EvolutionSource::new(
            "loop.ail",
            concat!(
                "module loop;\n",
                "fn first(value: Text) -> Text { loop.second(value) }\n\n",
                "fn second(value: Text) -> Text { loop.first(value) }\n",
            ),
        )],
        &CapabilityEnvironment::new(),
        no_coverage(),
    )
    .expect_err("qualification must not hide recursion");
    assert!(
        failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AIL.CALL.RECURSIVE_CYCLE")
    );
}

#[test]
fn independent_modules_may_reuse_names_and_entry_selection_is_explicit_when_needed() {
    let sources = vec![
        EvolutionSource::new("z.ail", "module z;\nfn shared() -> Text { \"z\" }\n"),
        EvolutionSource::new("a.ail", "module a;\nfn shared() -> Text { \"a\" }\n"),
    ];
    let workspace = EvolutionWorkspace::new(
        "duplicate-source-names",
        "r1",
        sources.into_iter().rev().collect(),
        &CapabilityEnvironment::new(),
        no_coverage(),
    )
    .expect("independent module namespaces do not collide");
    let mut capabilities = TestCapabilities::default();

    for (selector, expected) in [("a.shared", "a"), ("z.shared", "z")] {
        assert!(matches!(
            workspace.execute("r1", selector, Vec::new(), &mut capabilities),
            ExecutionResponse::Completed(result)
                if result.value == RuntimeValue::Text(expected.to_owned())
                    && result.function_handle.local_id.starts_with(
                        if selector.starts_with('a') { "a.ail#shared:" } else { "z.ail#shared:" }
                    )
        ));
    }
    assert!(matches!(
        workspace.execute("r1", "shared", Vec::new(), &mut capabilities),
        ExecutionResponse::Failed(failure)
            if failure.fault.code == "AIL.RUNTIME.AMBIGUOUS_FUNCTION"
    ));
}

#[test]
fn module_qualified_linking_preserves_identity_graph_targets() {
    let workspace = EvolutionWorkspace::new(
        "duplicate-schema-names",
        "r1",
        vec![
            EvolutionSource::new(
                "a.ail",
                concat!(
                    "module a;\n",
                    "record Item identity \"a.item.v1\" { value identity \"value\": Text; }\n\n",
                    "fn make() -> Item { Item { value: \"a\" } }\n",
                ),
            ),
            EvolutionSource::new(
                "b.ail",
                concat!(
                    "module b;\n",
                    "record Item identity \"b.item.v1\" { value identity \"value\": Text; }\n\n",
                    "fn make() -> Item { Item { value: \"b\" } }\n",
                ),
            ),
        ],
        &CapabilityEnvironment::new(),
        no_coverage(),
    )
    .expect("independent schemas may share a source name");

    let constructs = workspace
        .graph("r1")
        .unwrap()
        .iter()
        .filter(|edge| edge.kind == "constructs")
        .map(|edge| (edge.site.path.as_str(), edge.target.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(constructs, [("a.ail", "a.item.v1"), ("b.ail", "b.item.v1")]);
}

#[test]
fn cross_module_calls_enforce_multi_hop_effects() {
    let leaf = EvolutionSource::new(
        "leaf.ail",
        concat!(
            "module leaf;\n",
            "fn mark(value: Text, store: capability Store) -> Text effects { store.mark } {\n",
            "  store.mark(value)\n",
            "}\n",
        ),
    );
    let middle = EvolutionSource::new(
        "middle.ail",
        concat!(
            "module middle;\n",
            "import leaf as writer;\n\n",
            "fn forward(value: Text, store: capability Store) -> Text effects { store.mark } {\n",
            "  writer.mark(value)\n",
            "}\n",
        ),
    );
    let invalid = EvolutionSource::new(
        "service.ail",
        concat!(
            "module service;\n",
            "import middle as pipeline;\n\n",
            "fn run(value: Text, store: capability Store) -> Text {\n",
            "  pipeline.forward(value)\n",
            "}\n",
        ),
    );
    let failure = EvolutionWorkspace::new(
        "transitive-effects",
        "invalid",
        vec![leaf.clone(), middle.clone(), invalid],
        &environment(),
        no_coverage(),
    )
    .expect_err("the entry point must declare its reachable effect");
    assert!(
        failure
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "AIL.CAPABILITY.UNDECLARED_TRANSITIVE_EFFECT" })
    );

    let valid = EvolutionSource::new(
        "service.ail",
        concat!(
            "module service;\n",
            "import middle as pipeline;\n\n",
            "fn run(value: Text, store: capability Store) -> Text effects { store.mark } {\n",
            "  pipeline.forward(value)\n",
            "}\n",
        ),
    );
    let workspace = EvolutionWorkspace::new(
        "transitive-effects",
        "valid",
        vec![valid, middle, leaf],
        &environment(),
        no_coverage(),
    )
    .expect("the complete effect declaration is valid");
    let mut capabilities = TestCapabilities::default();
    assert!(matches!(
        workspace.execute(
            "valid",
            "service.run",
            vec![RuntimeValue::Text("work".to_owned())],
            &mut capabilities,
        ),
        ExecutionResponse::Completed(result)
            if result.value == RuntimeValue::Text("work".to_owned())
    ));
    assert_eq!(capabilities.calls, ["work"]);
}
