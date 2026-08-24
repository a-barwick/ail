//! Structured findings: one located, fact-carrying record per rejected check.
//!
//! A diagnostic code plus a byte span is not enough for a caller to converge.
//! A [`SourceFinding`] carries the file, the line and column, the source text
//! the checker read at that span, the expected and actual facts the checker
//! already computed, and the requirement those facts imply. Requirements are
//! restatements of compiler facts. Nothing here guesses a rewrite.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::Span;

/// Maximum bytes of source text a finding carries for one span or line.
const TEXT_LIMIT: usize = 240;

/// One located source range, with the text the checker read there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingLocation {
    /// Source-set path of the file the span belongs to.
    pub path: String,
    /// Half-open byte range inside that file.
    pub span: Span,
    /// One-based line of `span.start`.
    pub start_line: usize,
    /// One-based character column of `span.start`.
    pub start_column: usize,
    /// One-based line of `span.end`.
    pub end_line: usize,
    /// One-based character column of `span.end`.
    pub end_column: usize,
    /// Source text at the span, truncated to a bounded length.
    pub snippet: String,
    /// Whether `snippet` was truncated.
    pub snippet_truncated: bool,
    /// Full text of the line containing `span.start`, truncated the same way.
    pub line_text: String,
}

impl FindingLocation {
    /// Locate `span` inside `source`.
    ///
    /// Returns `None` when the span does not name a position in `source`.
    #[must_use]
    pub fn resolve(path: &str, source: &str, span: Span) -> Option<Self> {
        if span.start > source.len() {
            return None;
        }
        let start = floor_boundary(source, span.start);
        let end = floor_boundary(source, span.end.clamp(span.start, source.len()));
        let (start_line, start_column) = line_column(source, start);
        let (end_line, end_column) = line_column(source, end);
        let (snippet, snippet_truncated) = bounded(&source[start..end]);
        let (line_text, _) = bounded(line_at(source, start));
        Some(Self {
            path: path.to_owned(),
            span: Span::new(start, end),
            start_line,
            start_column,
            end_line,
            end_column,
            snippet,
            snippet_truncated,
            line_text,
        })
    }

    fn render(&self, indent: &str, into: &mut Vec<String>) {
        into.push(format!(
            "{indent}at {}:{}:{}-{}:{} bytes {}..{}",
            self.path,
            self.start_line,
            self.start_column,
            self.end_line,
            self.end_column,
            self.span.start,
            self.span.end
        ));
    }

    fn to_json(&self) -> Value {
        let mut value = Map::new();
        value.insert("path".into(), Value::String(self.path.clone()));
        value.insert("byte_start".into(), Value::from(self.span.start));
        value.insert("byte_end".into(), Value::from(self.span.end));
        value.insert("start_line".into(), Value::from(self.start_line));
        value.insert("start_column".into(), Value::from(self.start_column));
        value.insert("end_line".into(), Value::from(self.end_line));
        value.insert("end_column".into(), Value::from(self.end_column));
        value.insert("snippet".into(), Value::String(self.snippet.clone()));
        value.insert(
            "snippet_truncated".into(),
            Value::Bool(self.snippet_truncated),
        );
        value.insert("line_text".into(), Value::String(self.line_text.clone()));
        Value::Object(value)
    }
}

/// A second location a finding depends on, such as a policy contributor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedLocation {
    /// Why this location is part of the finding, for example `contributor`.
    pub role: String,
    /// Compiler name of the related entity.
    pub name: String,
    /// Located source range, when the compiler can resolve one.
    pub location: Option<FindingLocation>,
}

impl RelatedLocation {
    fn to_json(&self) -> Value {
        let mut value = Map::new();
        value.insert("role".into(), Value::String(self.role.clone()));
        value.insert("name".into(), Value::String(self.name.clone()));
        value.insert(
            "location".into(),
            self.location
                .as_ref()
                .map_or(Value::Null, FindingLocation::to_json),
        );
        Value::Object(value)
    }
}

/// One structured finding for one rejected check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFinding {
    /// Stable diagnostic code, for example `AIL.TYPE.FIELD_MISMATCH`.
    pub code: String,
    /// Diagnostic category, for example `type` or `architecture`.
    pub category: String,
    /// Primary source location, when the compiler has one.
    pub location: Option<FindingLocation>,
    /// What the checker required, keyed by fact name.
    pub expected: BTreeMap<String, String>,
    /// What the checker measured, keyed by fact name.
    pub actual: BTreeMap<String, String>,
    /// Other facts the checker already computed for this finding.
    pub facts: BTreeMap<String, String>,
    /// Additional named locations this finding depends on.
    pub related: Vec<RelatedLocation>,
    /// The requirement these facts imply, never a guessed rewrite.
    pub requirement: Option<String>,
}

impl SourceFinding {
    /// Build a finding with no location, facts, or requirement.
    #[must_use]
    pub fn new(code: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            category: category.into(),
            location: None,
            expected: BTreeMap::new(),
            actual: BTreeMap::new(),
            facts: BTreeMap::new(),
            related: Vec::new(),
            requirement: None,
        }
    }

    /// Fill [`Self::requirement`] from the facts this finding already carries.
    #[must_use]
    pub fn with_derived_requirement(mut self) -> Self {
        self.requirement =
            derive_requirement(&self.code, &self.expected, &self.actual, &self.facts);
        self
    }

    /// Render the finding as deterministic human-readable lines.
    #[must_use]
    pub fn render(&self) -> String {
        let mut lines = vec![format!("{} {} error", self.code, self.category)];
        if let Some(location) = &self.location {
            location.render("  ", &mut lines);
            if !location.snippet.is_empty() {
                lines.push(format!("  source: {}", one_line(&location.snippet)));
            }
            if location.start_line == location.end_line
                && location.line_text.trim() != location.snippet.trim()
            {
                lines.push(format!(
                    "  line {}: {}",
                    location.start_line,
                    location.line_text.trim_end()
                ));
            }
        }
        for (key, value) in &self.expected {
            lines.push(format!("  expected.{key}={value}"));
        }
        for (key, value) in &self.actual {
            lines.push(format!("  actual.{key}={value}"));
        }
        for (key, value) in &self.facts {
            lines.push(format!("  {key}={value}"));
        }
        for related in &self.related {
            lines.push(format!("  {}: {}", related.role, related.name));
            if let Some(location) = &related.location {
                location.render("    ", &mut lines);
            }
        }
        if let Some(requirement) = &self.requirement {
            lines.push(format!("  requires: {requirement}"));
        }
        lines.join("\n")
    }

    /// Render the finding as JSON.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut value = Map::new();
        value.insert("code".into(), Value::String(self.code.clone()));
        value.insert("category".into(), Value::String(self.category.clone()));
        value.insert(
            "location".into(),
            self.location
                .as_ref()
                .map_or(Value::Null, FindingLocation::to_json),
        );
        value.insert("expected".into(), string_map(&self.expected));
        value.insert("actual".into(), string_map(&self.actual));
        value.insert("facts".into(), string_map(&self.facts));
        value.insert(
            "related".into(),
            Value::Array(self.related.iter().map(RelatedLocation::to_json).collect()),
        );
        value.insert(
            "requirement".into(),
            self.requirement
                .as_ref()
                .map_or(Value::Null, |text| Value::String(text.clone())),
        );
        Value::Object(value)
    }
}

fn string_map(map: &BTreeMap<String, String>) -> Value {
    Value::Object(
        map.iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect(),
    )
}

fn one_line(text: &str) -> String {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    if lines.next().is_some() {
        format!("{first} ...")
    } else {
        first.to_owned()
    }
}

fn floor_boundary(source: &str, mut offset: usize) -> usize {
    let offset_limit = source.len();
    if offset > offset_limit {
        offset = offset_limit;
    }
    while offset > 0 && !source.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = source[line_start..offset].chars().count() + 1;
    (line, column)
}

fn line_at(source: &str, offset: usize) -> &str {
    let start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let end = source[start..]
        .find('\n')
        .map_or(source.len(), |index| start + index);
    &source[start..end]
}

fn bounded(text: &str) -> (String, bool) {
    if text.len() <= TEXT_LIMIT {
        return (text.to_owned(), false);
    }
    let cut = floor_boundary(text, TEXT_LIMIT);
    (format!("{}...", &text[..cut]), true)
}

fn value_of<'a>(map: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    map.get(key).map(String::as_str)
}

/// Derive the requirement implied by facts the checker already produced.
///
/// Every branch restates compiler facts. When the facts do not name a
/// requirement, this returns `None` rather than guessing one.
fn derive_requirement(
    code: &str,
    expected: &BTreeMap<String, String>,
    actual: &BTreeMap<String, String>,
    facts: &BTreeMap<String, String>,
) -> Option<String> {
    if let Some(requirement) = named_requirement(code, expected, actual, facts) {
        return Some(requirement);
    }
    let shared = expected
        .iter()
        .filter_map(|(key, want)| {
            let got = value_of(actual, key)?;
            Some(format!(
                "{key} must be {want} at this span; the checker measured {got}"
            ))
        })
        .collect::<Vec<_>>();
    if !shared.is_empty() {
        return Some(shared.join("; "));
    }
    if actual.is_empty() && expected.len() == 1 {
        let (key, want) = expected.iter().next()?;
        return Some(format!("{key} must be {want} at this span"));
    }
    None
}

#[allow(clippy::too_many_lines)]
fn named_requirement(
    code: &str,
    expected: &BTreeMap<String, String>,
    actual: &BTreeMap<String, String>,
    facts: &BTreeMap<String, String>,
) -> Option<String> {
    match code {
        "AIL.PARSE.EXPECTED_TOKEN" => {
            let want = value_of(expected, "token")?;
            let got = value_of(actual, "token")?;
            Some(format!(
                "this position must be {want}; the parser read {got}"
            ))
        }
        "AIL.MODULE.MISSING_IMPORT" => {
            let module = value_of(facts, "module")?;
            match value_of(facts, "source_set.modules") {
                Some(declared) => Some(format!(
                    "the source set must contain a file declaring module {module}; it declares {declared}"
                )),
                None => Some(format!(
                    "the source set must contain a file declaring module {module}"
                )),
            }
        }
        "AIL.MODULE.MISSING_IDENTITY" => {
            let requirement = value_of(facts, "requirement")?;
            Some(format!("this file must satisfy {requirement}"))
        }
        "AIL.MODULE.DUPLICATE_IDENTITY" => {
            let module = value_of(facts, "module")?;
            let existing = value_of(facts, "existing_path")?;
            Some(format!(
                "module {module} must be declared by one file; {existing} already declares it"
            ))
        }
        "AIL.MODULE.DUPLICATE_IMPORT" => {
            let module = value_of(facts, "module")?;
            Some(format!("this file must import {module} once"))
        }
        "AIL.MODULE.DUPLICATE_QUALIFIER" => {
            let qualifier = value_of(facts, "qualifier")?;
            let first = value_of(facts, "first_module")?;
            let second = value_of(facts, "second_module")?;
            Some(format!(
                "qualifier {qualifier} must name one module; {first} and {second} both claim it"
            ))
        }
        "AIL.MODULE.AMBIGUOUS_IMPORT" => {
            let declaration = value_of(facts, "declaration")?;
            let first = value_of(facts, "first_module")?;
            let second = value_of(facts, "second_module")?;
            Some(format!(
                "{declaration} must resolve to one module; {first} and {second} both export it"
            ))
        }
        "AIL.MODULE.INACCESSIBLE_DECLARATION" => {
            let declaration = value_of(facts, "declaration")?;
            let module = value_of(facts, "module")?;
            Some(format!(
                "this file must import {module} to reference {declaration}"
            ))
        }
        "AIL.MODULE.IMPORT_CYCLE" => {
            let cycle = value_of(facts, "cycle")?;
            Some(format!(
                "the import graph must be acyclic; it contains {cycle}"
            ))
        }
        "AIL.NAME.UNKNOWN_FUNCTION" => {
            let function = value_of(expected, "function")?;
            Some(format!(
                "the source set must declare a function named {function}"
            ))
        }
        "AIL.NAME.UNRESOLVED" => {
            let name = value_of(expected, "name")?;
            let role = value_of(expected, "role")?;
            Some(format!("the source set must declare a {role} named {name}"))
        }
        "AIL.NAME.DUPLICATE_DECLARATION" => {
            let name = value_of(expected, "name")?;
            let kind = value_of(expected, "kind")?;
            Some(format!("{kind} {name} must be declared once in this scope"))
        }
        "AIL.CALL.RECURSIVE_CYCLE" => {
            let cycle = value_of(actual, "cycle")?;
            Some(format!(
                "the AIL call graph must be acyclic; the cycle is {cycle}"
            ))
        }
        "AIL.CAPABILITY.UNKNOWN_INTERFACE" => {
            let interface = value_of(expected, "capability")?;
            match value_of(facts, "capability_environment.interfaces") {
                Some("") => Some(format!(
                    "the capability environment must declare interface {interface}; this check supplies none"
                )),
                Some(available) => Some(format!(
                    "the capability environment must declare interface {interface}; it declares {available}"
                )),
                None => Some(format!(
                    "the capability environment must declare interface {interface}"
                )),
            }
        }
        "AIL.CAPABILITY.UNKNOWN_OPERATION" => {
            let operation = value_of(expected, "operation")?;
            Some(format!(
                "the capability interface must declare operation {operation}"
            ))
        }
        "AIL.CAPABILITY.INVALID_EFFECT" => {
            let receiver = value_of(expected, "capability")?;
            Some(format!(
                "{receiver} must be a declared capability parameter of this function"
            ))
        }
        "AIL.CAPABILITY.DUPLICATE_EFFECT" => {
            let effect = value_of(expected, "effect")?;
            Some(format!("this function must declare effect {effect} once"))
        }
        "AIL.CAPABILITY.UNDECLARED_EFFECT" | "AIL.CAPABILITY.UNDECLARED_TRANSITIVE_EFFECT" => {
            let required = value_of(actual, "required_effect")?;
            let declared = value_of(expected, "declared_effects").unwrap_or("");
            if declared.is_empty() {
                Some(format!(
                    "this function must declare effect {required}; it declares none"
                ))
            } else {
                Some(format!(
                    "this function must declare effect {required}; it declares {declared}"
                ))
            }
        }
        "AIL.CAPABILITY.MISSING_TRANSITIVE_CAPABILITY" => {
            let receiver = value_of(expected, "receiver")?;
            let interface = value_of(expected, "interface")?;
            Some(format!(
                "this call must pass a capability of interface {interface} for parameter {receiver}"
            ))
        }
        "AIL.ARCH.HOTSPOT_GROWTH" => Some(format!(
            "control-flow complexity must stay at most {} and minimal review context at most {}; the candidate measured {} and {}",
            value_of(facts, "facts.base_cfc")?,
            value_of(facts, "facts.base_context")?,
            value_of(facts, "facts.candidate_cfc")?,
            value_of(facts, "facts.candidate_context")?
        )),
        "AIL.ARCH.NEW_UNIT" => Some(format!(
            "a new unit must have control-flow complexity at most {} and minimal review context at most {}; this unit measured {} and {}",
            value_of(facts, "facts.cfc_max")?,
            value_of(facts, "facts.context_max")?,
            value_of(facts, "facts.cfc")?,
            value_of(facts, "facts.context")?
        )),
        "AIL.ARCH.BOUNDARY" => {
            let source = value_of(facts, "facts.forbidden_group_edges.0.source")?;
            let target = value_of(facts, "facts.forbidden_group_edges.0.target")?;
            let kind = value_of(facts, "facts.forbidden_group_edges.0.kind")?;
            let source_group = value_of(facts, "facts.forbidden_group_edges.0.source_group")?;
            let target_group = value_of(facts, "facts.forbidden_group_edges.0.target_group")?;
            Some(format!(
                "group {source_group} must not depend on group {target_group}; remove the {kind} edge {source} -> {target}"
            ))
        }
        "AIL.ARCH.AUTHORITY" => Some(format!(
            "transport capabilities must be within {}; the candidate uses {}",
            value_of(facts, "facts.allowed")?,
            value_of(facts, "facts.actual")?
        )),
        "AIL.ARCH.STATE" => Some(format!(
            "transport state access must be within {}; the candidate reads {} and writes {}",
            value_of(facts, "facts.allowed")?,
            value_of(facts, "facts.reads").unwrap_or(""),
            value_of(facts, "facts.writes").unwrap_or("")
        )),
        "AIL.ARCH.CYCLE" => {
            let members = value_of(facts, "facts.components.0.members")?;
            Some(format!(
                "the unit graph must not gain a cycle; break the cycle among {members}"
            ))
        }
        "AIL.ARCH.STALE_BASELINE" => Some(format!(
            "the request must name baseline revision {}; it named {}",
            value_of(facts, "facts.required")?,
            value_of(facts, "facts.provided")?
        )),
        _ => None,
    }
}

/// Flatten a JSON value into deterministic dotted `key=value` facts.
pub(crate) fn flatten_json(prefix: &str, value: &Value, into: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                flatten_json(&format!("{prefix}.{key}"), child, into);
            }
        }
        Value::Array(items) => {
            if items
                .iter()
                .all(|item| matches!(item, Value::String(_) | Value::Number(_) | Value::Bool(_)))
            {
                into.insert(prefix.to_owned(), join_scalars(items));
            } else {
                for (index, item) in items.iter().enumerate() {
                    flatten_json(&format!("{prefix}.{index}"), item, into);
                }
            }
        }
        Value::Null => {}
        Value::String(text) => {
            into.insert(prefix.to_owned(), text.clone());
        }
        Value::Bool(_) | Value::Number(_) => {
            into.insert(prefix.to_owned(), value.to_string());
        }
    }
}

fn join_scalars(items: &[Value]) -> String {
    items
        .iter()
        .map(|item| match item {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Render findings as one JSON document for machine consumption.
#[must_use]
pub fn findings_document(status: &str, summary: &str, findings: &[SourceFinding]) -> String {
    let document = serde_json::json!({
        "status": status,
        "summary": summary,
        "findings": findings
            .iter()
            .map(SourceFinding::to_json)
            .collect::<Vec<_>>(),
    });
    format!("{document}\n")
}
