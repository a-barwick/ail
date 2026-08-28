//! `ailc check` and `ailc publish` emit one located structured finding per error.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

fn ailc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ailc"))
}

fn unique(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ail-findings-{label}-{}-{nanos}",
        std::process::id()
    ))
}

struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(label: &str, files: &[(&str, &str)]) -> Self {
        let root = unique(label);
        fs::create_dir_all(&root).expect("workspace directory is creatable");
        for (name, source) in files {
            fs::write(root.join(name), source).expect("workspace source is writable");
        }
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn stderr_of(arguments: &[&str]) -> String {
    let output = ailc().args(arguments).output().expect("ailc runs").stderr;
    String::from_utf8(output).expect("ailc stderr is UTF-8")
}

fn run(arguments: &[&str]) -> std::process::Output {
    ailc().args(arguments).output().expect("ailc runs")
}

fn json_document(arguments: &[&str]) -> Value {
    let output = run(arguments);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "ailc --json stdout is one JSON document: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn only_finding(document: &Value) -> &Value {
    let findings = document["findings"]
        .as_array()
        .expect("document carries findings");
    assert_eq!(findings.len(), 1, "{document}");
    &findings[0]
}

fn finding_with_code<'a>(document: &'a Value, code: &str) -> &'a Value {
    document["findings"]
        .as_array()
        .expect("document carries findings")
        .iter()
        .find(|finding| finding["code"] == code)
        .unwrap_or_else(|| panic!("no {code} finding in {document}"))
}

const FIELD_MISMATCH_SOURCE: &str = concat!(
    "record Job {\n",
    "  job_id: Text;\n",
    "}\n",
    "\n",
    "fn make_job() -> Job {\n",
    "  let job = Job { job_id: 1 };\n",
    "  job\n",
    "}\n",
);

#[test]
fn a_type_error_carries_the_snippet_and_expected_versus_actual() {
    let workspace = Workspace::new("type", &[("types.ail", FIELD_MISMATCH_SOURCE)]);
    let path = workspace.path().to_str().expect("temp path is UTF-8");
    let stderr = stderr_of(&["check", path]);

    assert!(
        stderr.contains("AIL.TYPE.FIELD_MISMATCH type error"),
        "{stderr}"
    );
    assert!(
        stderr.contains("at types.ail:6:27-6:28 bytes 81..82"),
        "{stderr}"
    );
    assert!(stderr.contains("source: 1"), "{stderr}");
    assert!(
        stderr.contains("line 6:   let job = Job { job_id: 1 };"),
        "{stderr}"
    );
    assert!(stderr.contains("expected.type=Text"), "{stderr}");
    assert!(stderr.contains("actual.type=Int"), "{stderr}");
    assert!(
        stderr.contains("requires: type must be Text at this span; the checker measured Int"),
        "{stderr}"
    );

    let document = json_document(&["check", "--json", path]);
    let finding = only_finding(&document);
    assert_eq!(finding["code"], "AIL.TYPE.FIELD_MISMATCH");
    assert_eq!(finding["category"], "type");
    assert_eq!(finding["location"]["path"], "types.ail");
    assert_eq!(finding["location"]["start_line"], 6);
    assert_eq!(finding["location"]["start_column"], 27);
    assert_eq!(finding["location"]["byte_start"], 81);
    assert_eq!(finding["location"]["byte_end"], 82);
    assert_eq!(finding["location"]["snippet"], "1");
    assert_eq!(
        finding["location"]["line_text"],
        "  let job = Job { job_id: 1 };"
    );
    assert_eq!(finding["expected"]["type"], "Text");
    assert_eq!(finding["actual"]["type"], "Int");
}

#[test]
fn the_snippet_locates_the_source_the_caller_supplied() {
    // A leading blank line is not canonical form. The finding must still name
    // the line the caller can open, not the line of a canonical rewrite.
    let source = format!("\n{FIELD_MISMATCH_SOURCE}");
    let workspace = Workspace::new("noncanonical", &[("types.ail", source.as_str())]);
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = only_finding(&document);
    let location = &finding["location"];
    assert_eq!(location["start_line"], 7);
    assert_eq!(location["snippet"], "1");
    assert_eq!(finding["facts"]["source.canonical"], "false");

    let byte_start = usize::try_from(
        location["byte_start"]
            .as_u64()
            .expect("byte offsets are numbers"),
    )
    .expect("byte offsets fit a usize");
    let byte_end = usize::try_from(
        location["byte_end"]
            .as_u64()
            .expect("byte offsets are numbers"),
    )
    .expect("byte offsets fit a usize");
    assert_eq!(&source[byte_start..byte_end], "1");
}

#[test]
fn a_type_error_in_a_multi_file_workspace_names_the_file_that_holds_it() {
    let workspace = Workspace::new(
        "multifile",
        &[
            (
                "alpha.ail",
                "module alpha;\n\nrecord Job {\n  job_id: Text;\n}\n",
            ),
            (
                "beta.ail",
                "module beta;\n\nimport alpha;\n\nfn make_job() -> alpha.Job {\n  let job = alpha.Job { job_id: 1 };\n  job\n}\n",
            ),
        ],
    );
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = only_finding(&document);
    assert_eq!(finding["code"], "AIL.TYPE.FIELD_MISMATCH");
    assert_eq!(finding["location"]["path"], "beta.ail");
    assert_eq!(finding["location"]["start_line"], 6);
    assert_eq!(finding["location"]["snippet"], "1");

    let related = finding["related"]
        .as_array()
        .expect("related locations exist");
    let field = related
        .iter()
        .find(|entry| entry["name"] == "field:alpha.Job:job_id")
        .unwrap_or_else(|| panic!("declared field is related: {finding}"));
    assert_eq!(field["location"]["path"], "alpha.ail");
    assert_eq!(field["location"]["snippet"], "job_id: Text;");
}

#[test]
fn a_missing_import_names_the_import_and_the_file() {
    let path = examples_dir().join("composed-service/service.ail");
    let path = path.to_str().expect("example path is UTF-8");
    let stderr = stderr_of(&["check", path]);

    assert!(
        stderr.contains("AIL.MODULE.MISSING_IMPORT module error"),
        "{stderr}"
    );
    assert!(stderr.contains("at service.ail:2:1-2:24"), "{stderr}");
    assert!(
        stderr.contains("source: import domain as model;"),
        "{stderr}"
    );
    assert!(stderr.contains("module=domain"), "{stderr}");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.MODULE.MISSING_IMPORT");
    assert_eq!(finding["location"]["path"], "service.ail");
    assert_eq!(finding["location"]["snippet"], "import domain as model;");
    assert_eq!(finding["facts"]["module"], "domain");
    assert_eq!(finding["facts"]["source_set.modules"], "service");
    assert_eq!(
        finding["requirement"],
        "the source set must contain a file declaring module domain; it declares service"
    );
}

#[test]
fn an_inaccessible_declaration_names_the_module_that_must_be_imported() {
    // `tests` references `transport.dispatch` without importing `transport`.
    // Only `transport` can be imported to resolve that reference, so naming the
    // referring module `tests` would be a false fact.
    let workspace = Workspace::new(
        "inaccessible",
        &[
            (
                "transport.ail",
                "module transport;\n\nfn dispatch(value: Text) -> Text {\n  value\n}\n",
            ),
            (
                "tests.ail",
                "module tests;\n\nfn verify() -> Text {\n  transport.dispatch(\"x\")\n}\n",
            ),
        ],
    );
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.MODULE.INACCESSIBLE_DECLARATION");
    assert_eq!(finding["location"]["path"], "tests.ail");
    assert_eq!(finding["facts"]["declaration"], "transport.dispatch");
    assert_eq!(finding["facts"]["module"], "transport");
    assert_eq!(finding["facts"]["declaring_path"], "transport.ail");
    assert_eq!(finding["facts"]["referring_module"], "tests");
    assert_eq!(
        finding["requirement"],
        "this file must import transport to reference transport.dispatch"
    );

    let requirement = finding["requirement"]
        .as_str()
        .expect("requirement is a string");
    assert!(
        !requirement.contains("tests"),
        "the requirement must not name the referring module: {requirement}"
    );
}

#[test]
fn an_inaccessible_bare_reference_names_the_declaring_module() {
    let workspace = Workspace::new(
        "inaccessible-bare",
        &[
            ("a.ail", "module a;\n\nfn hidden() -> Text {\n  \"a\"\n}\n"),
            ("b.ail", "module b;\n\nfn run() -> Text {\n  hidden()\n}\n"),
        ],
    );
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.MODULE.INACCESSIBLE_DECLARATION");
    assert_eq!(finding["facts"]["module"], "a");
    assert_eq!(finding["facts"]["referring_module"], "b");
    assert_eq!(
        finding["requirement"],
        "this file must import a to reference hidden"
    );
}

#[test]
fn an_inaccessible_dotted_module_reference_names_the_whole_module() {
    let workspace = Workspace::new(
        "inaccessible-dotted",
        &[
            (
                "deep.ail",
                "module a.deep;\n\nfn hidden() -> Text {\n  \"a\"\n}\n",
            ),
            (
                "b.ail",
                "module b;\n\nfn run() -> Text {\n  a.deep.hidden()\n}\n",
            ),
        ],
    );
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.MODULE.INACCESSIBLE_DECLARATION");
    assert_eq!(finding["facts"]["module"], "a.deep");
    assert_eq!(finding["facts"]["declaring_path"], "deep.ail");
    assert_eq!(
        finding["requirement"],
        "this file must import a.deep to reference a.deep.hidden"
    );
}

#[test]
fn an_inaccessible_declaration_with_several_owners_names_every_candidate() {
    // Two modules declare `hidden` and the bare reference names neither. The
    // checker knows the candidates but not which one is meant, so it reports
    // both instead of picking one.
    let workspace = Workspace::new(
        "inaccessible-ambiguous",
        &[
            ("a.ail", "module a;\n\nfn hidden() -> Text {\n  \"a\"\n}\n"),
            ("c.ail", "module c;\n\nfn hidden() -> Text {\n  \"c\"\n}\n"),
            ("b.ail", "module b;\n\nfn run() -> Text {\n  hidden()\n}\n"),
        ],
    );
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.MODULE.INACCESSIBLE_DECLARATION");
    assert_eq!(finding["facts"]["declaring_modules"], "a,c");
    assert_eq!(finding["facts"]["module"], Value::Null);
    assert_eq!(
        finding["requirement"],
        "this file must import the module that declares hidden; a,c declare it"
    );
}

#[test]
fn an_architecture_failure_names_the_rule_and_the_violating_source() {
    let path = examples_dir().join("architecture-denied");
    let path = path.to_str().expect("example path is UTF-8");
    let stderr = stderr_of(&["check", path]);

    assert!(
        stderr.contains("AIL.ARCH.BOUNDARY architecture error"),
        "{stderr}"
    );
    assert!(stderr.contains("rule=M23-POL-GROUP-DEPENDENCY"), "{stderr}");
    assert!(stderr.contains("at transport.ail:"), "{stderr}");
    assert!(
        stderr.contains("source: fn dispatch(value: Text) -> Text {"),
        "{stderr}"
    );
    assert!(stderr.contains("contributor: domain:work"), "{stderr}");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.ARCH.BOUNDARY");
    assert_eq!(finding["category"], "architecture");
    assert_eq!(finding["facts"]["rule"], "M23-POL-GROUP-DEPENDENCY");
    assert_eq!(finding["facts"]["scope"], "group:transport");
    assert_eq!(
        finding["facts"]["facts.forbidden_group_edges.0.source"],
        "transport:dispatch"
    );
    assert_eq!(finding["location"]["path"], "transport.ail");
    assert_eq!(finding["location"]["start_line"], 4);
    assert_eq!(
        finding["requirement"],
        "group transport must not depend on group domain; the candidate has a calls edge transport:dispatch -> domain:work"
    );
    let related = finding["related"]
        .as_array()
        .expect("related locations exist");
    assert_eq!(related[0]["name"], "domain:work");
    assert_eq!(related[0]["location"]["path"], "domain.ail");
}

#[test]
fn a_parse_error_locates_the_token_the_parser_expected() {
    let workspace = Workspace::new("parse", &[("broken.ail", "record Job {\n  job_id\n}\n")]);
    let path = workspace.path().to_str().expect("temp path is UTF-8");
    let stderr = stderr_of(&["check", path]);

    assert!(
        stderr.contains("AIL.PARSE.EXPECTED_TOKEN parse error"),
        "{stderr}"
    );
    assert!(stderr.contains("at broken.ail:2:9-2:9"), "{stderr}");
    assert!(stderr.contains("line 2:   job_id"), "{stderr}");
    assert!(stderr.contains("expected.token=:"), "{stderr}");
    assert!(stderr.contains("actual.token=}"), "{stderr}");

    let document = json_document(&["check", "--json", path]);
    let finding = only_finding(&document);
    assert_eq!(finding["code"], "AIL.PARSE.EXPECTED_TOKEN");
    assert_eq!(finding["location"]["path"], "broken.ail");
    assert_eq!(finding["location"]["start_line"], 2);
    assert_eq!(finding["location"]["start_column"], 9);
    assert_eq!(
        finding["requirement"],
        "this position must be :; the parser read }"
    );
}

#[test]
fn an_unknown_capability_interface_names_the_environment_check_supplies() {
    let path = examples_dir().join("batch-lookup");
    let path = path.to_str().expect("example path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.CAPABILITY.UNKNOWN_INTERFACE");
    assert_eq!(finding["location"]["path"], "service.ail");
    assert_eq!(
        finding["location"]["snippet"],
        "dependency: capability DependencyClient"
    );
    assert_eq!(finding["expected"]["capability"], "DependencyClient");
    assert_eq!(finding["facts"]["capability_environment.interfaces"], "");
    assert_eq!(finding["facts"].get("capability_environment.path"), None);
    assert_eq!(
        finding["requirement"],
        "the capability environment must declare interface DependencyClient; this check supplies none"
    );
}

#[test]
fn an_undeclared_capability_names_the_loaded_file_path_and_digest() {
    let path = examples_dir().join("capability-undeclared");
    let path = path.to_str().expect("example path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.CAPABILITY.UNKNOWN_INTERFACE");
    assert_eq!(finding["location"]["path"], "store.ail");
    assert_eq!(finding["expected"]["capability"], "Clock");
    assert_eq!(
        finding["facts"]["capability_environment.interfaces"],
        "JobsStore"
    );
    assert_eq!(
        finding["facts"]["capability_environment.path"],
        "capabilities.json"
    );
    let digest = finding["facts"]["capability_environment.digest"]
        .as_str()
        .expect("digest fact is a string");
    assert!(digest.starts_with("sha256:"), "{digest}");
    assert_eq!(
        finding["requirement"],
        "the capability environment must declare interface Clock; it declares JobsStore"
    );
}

#[test]
fn a_recursive_cycle_names_the_cycle_and_both_declarations() {
    let workspace = Workspace::new(
        "recursion",
        &[(
            "cycle.ail",
            "fn first(value: Text) -> Text {\n  second(value)\n}\n\nfn second(value: Text) -> Text {\n  first(value)\n}\n",
        )],
    );
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = finding_with_code(&document, "AIL.CALL.RECURSIVE_CYCLE");
    assert_eq!(finding["location"]["path"], "cycle.ail");
    assert_eq!(finding["location"]["snippet"], "first(value)");
    assert_eq!(finding["actual"]["cycle"], "first, second, first");
    assert_eq!(
        finding["requirement"],
        "the AIL call graph must be acyclic; the cycle is first, second, first"
    );
    let names = finding["related"]
        .as_array()
        .expect("related locations exist")
        .iter()
        .map(|entry| entry["name"].as_str().unwrap_or_default().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["function:first", "function:second"]);
}

#[test]
fn a_long_snippet_is_bounded_and_says_so() {
    let padding = "x".repeat(300);
    let source =
        format!("record R {{\n  n: Int;\n}}\n\nfn f() -> R {{\n  R {{ n: \"{padding}\" }}\n}}\n");
    let workspace = Workspace::new("bounded", &[("long.ail", source.as_str())]);
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let document = json_document(&["check", "--json", path]);
    let finding = only_finding(&document);
    let snippet = finding["location"]["snippet"]
        .as_str()
        .expect("snippet is a string");
    assert_eq!(finding["location"]["snippet_truncated"], true);
    assert_eq!(snippet.len(), 243, "{snippet}");
    assert!(snippet.ends_with("..."), "{snippet}");
    assert!(snippet.starts_with("\"xxx"), "{snippet}");
}

/// Verbs that would make a requirement an edit instruction instead of a
/// constraint. A finding reports what must hold and what the checker measured.
/// Choosing the repair is the caller's work, and the checker has no fact that
/// justifies picking one.
const EDIT_VERBS: [&str; 22] = [
    "add", "avoid", "break", "change", "consider", "delete", "drop", "edit", "extract", "fix",
    "insert", "instead", "move", "prefer", "remove", "rename", "replace", "rewrite", "should",
    "split", "try", "wrap",
];

/// Temporary workspaces covering one rejection class each.
fn failing_workspaces() -> Vec<Workspace> {
    vec![
        Workspace::new("rule-type", &[("types.ail", FIELD_MISMATCH_SOURCE)]),
        Workspace::new(
            "rule-parse",
            &[("broken.ail", "record Job {\n  job_id\n}\n")],
        ),
        Workspace::new(
            "rule-recursion",
            &[(
                "cycle.ail",
                "fn first(value: Text) -> Text {\n  second(value)\n}\n\nfn second(value: Text) -> Text {\n  first(value)\n}\n",
            )],
        ),
        Workspace::new(
            "rule-name",
            &[(
                "names.ail",
                "fn run(value: Text) -> Text {\n  missing(value)\n}\n",
            )],
        ),
        Workspace::new(
            "rule-unresolved",
            &[("types.ail", "record R {\n  n: Missing;\n}\n")],
        ),
        Workspace::new(
            "rule-duplicate",
            &[(
                "dup.ail",
                "fn run(value: Text) -> Text {\n  value\n}\n\nfn run(other: Text) -> Text {\n  other\n}\n",
            )],
        ),
        Workspace::new(
            "rule-effect",
            &[(
                "effect.ail",
                "fn run(value: Text) -> Text effects { store.mark } {\n  value\n}\n",
            )],
        ),
        Workspace::new(
            "rule-inaccessible",
            &[
                (
                    "transport.ail",
                    "module transport;\n\nfn dispatch(value: Text) -> Text {\n  value\n}\n",
                ),
                (
                    "tests.ail",
                    "module tests;\n\nfn verify() -> Text {\n  transport.dispatch(\"x\")\n}\n",
                ),
            ],
        ),
        Workspace::new(
            "rule-ambiguous-owner",
            &[
                ("a.ail", "module a;\n\nfn hidden() -> Text {\n  \"a\"\n}\n"),
                ("c.ail", "module c;\n\nfn hidden() -> Text {\n  \"c\"\n}\n"),
                ("b.ail", "module b;\n\nfn run() -> Text {\n  hidden()\n}\n"),
            ],
        ),
    ]
}

/// Every code and every requirement `ailc check` emits for these inputs.
fn all_findings(inputs: &[&Path]) -> (Vec<String>, Vec<(String, String)>) {
    let mut codes = Vec::new();
    let mut requirements = Vec::new();
    for input in inputs {
        let path = input.to_str().expect("input path is UTF-8");
        let document = json_document(&["check", "--json", path]);
        for finding in document["findings"]
            .as_array()
            .expect("document carries findings")
        {
            let code = finding["code"]
                .as_str()
                .expect("code is a string")
                .to_owned();
            codes.push(code.clone());
            if let Some(requirement) = finding["requirement"].as_str() {
                requirements.push((code, requirement.to_owned()));
            }
        }
    }
    codes.sort_unstable();
    codes.dedup();
    (codes, requirements)
}

#[test]
fn requirement_is_a_constraint_and_never_an_edit() {
    let workspaces = failing_workspaces();
    let examples = [
        examples_dir().join("architecture-denied"),
        examples_dir().join("composed-service/service.ail"),
        examples_dir().join("batch-lookup"),
    ];
    let inputs = workspaces
        .iter()
        .map(Workspace::path)
        .chain(examples.iter().map(PathBuf::as_path))
        .collect::<Vec<_>>();
    let (codes, requirements) = all_findings(&inputs);

    assert!(
        codes.len() >= 8,
        "this rule must be checked against a wide spread of codes, saw {codes:?}"
    );
    assert!(
        requirements.len() >= 8,
        "this rule must be checked against real requirements, saw {requirements:?}"
    );

    for (code, requirement) in &requirements {
        assert!(
            requirement.contains(" must "),
            "{code} requirement must state a constraint with `must`: {requirement}"
        );
        for word in requirement.split_whitespace() {
            let word = word
                .trim_matches(|character: char| !character.is_ascii_alphabetic())
                .to_ascii_lowercase();
            assert!(
                !EDIT_VERBS.contains(&word.as_str()),
                "{code} requirement prescribes an edit with `{word}`: {requirement}"
            );
        }
    }
}

#[test]
fn a_passing_workspace_reports_no_findings() {
    let path = examples_dir().join("composed-service");
    let path = path.to_str().expect("example path is UTF-8");
    let output = run(&["check", "--json", path]);
    assert!(output.status.success());

    let document: Value =
        serde_json::from_slice(&output.stdout).expect("ok document is one JSON document");
    assert_eq!(document["status"], "ok");
    assert_eq!(
        document["findings"].as_array().map(Vec::len),
        Some(0),
        "{document}"
    );
}

#[test]
fn publish_reports_the_same_findings_as_check_and_writes_nothing() {
    let workspace = Workspace::new("publish", &[("types.ail", FIELD_MISMATCH_SOURCE)]);
    let path = workspace.path().to_str().expect("temp path is UTF-8");

    let checked = json_document(&["check", "--json", path]);
    let published = json_document(&["publish", "--json", path]);
    assert_eq!(checked["findings"], published["findings"]);
    assert_eq!(published["status"], "failed");
    assert!(
        !workspace.path().join(".ail").exists(),
        "a rejected publish must write no revision"
    );
}
