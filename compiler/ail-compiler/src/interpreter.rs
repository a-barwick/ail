//! Deterministic tree-walking execution for the accepted M17 core slice.

use std::collections::BTreeMap;

use crate::{Block, CapabilityEnvironment, Declaration, Expr, ParameterType, SourceUnit, Span};

/// One immutable value accepted or produced by the M17 interpreter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    Unit,
    Text(String),
    Int(u128),
    Bool(bool),
    Bytes(Vec<u8>),
    Record {
        type_name: String,
        fields: BTreeMap<String, RuntimeValue>,
    },
    Variant {
        type_name: String,
        case: String,
        payload: Option<Box<RuntimeValue>>,
    },
}

impl RuntimeValue {
    /// Construct one record value from deterministic field pairs.
    #[must_use]
    pub fn record(
        type_name: impl Into<String>,
        fields: impl IntoIterator<Item = (impl Into<String>, RuntimeValue)>,
    ) -> Self {
        Self::Record {
            type_name: type_name.into(),
            fields: fields
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
        }
    }

    /// Construct one closed variant value.
    #[must_use]
    pub fn variant(
        type_name: impl Into<String>,
        case: impl Into<String>,
        payload: Option<RuntimeValue>,
    ) -> Self {
        Self::Variant {
            type_name: type_name.into(),
            case: case.into(),
            payload: payload.map(Box::new),
        }
    }

    /// Return one record field when this is a record value.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&Self> {
        let Self::Record { fields, .. } = self else {
            return None;
        };
        fields.get(name)
    }

    /// Return the exact named runtime type.
    #[must_use]
    pub fn type_name(&self) -> &str {
        match self {
            Self::Unit => "Unit",
            Self::Text(_) => "Text",
            Self::Int(_) => "Int",
            Self::Bool(_) => "Bool",
            Self::Bytes(_) => "Bytes",
            Self::Record { type_name, .. } | Self::Variant { type_name, .. } => type_name,
        }
    }
}

/// A capability implementation supplied by the embedding host.
pub trait CapabilityProvider {
    /// Whether the named instance and interface are available for this execution.
    fn supports(&self, receiver: &str, interface: &str) -> bool;

    /// Execute one operation after all arguments have been evaluated.
    ///
    /// # Errors
    ///
    /// Returns a structured runtime fault when the supplied instance cannot
    /// complete the operation under its declared contract.
    fn call(
        &mut self,
        receiver: &str,
        interface: &str,
        operation: &str,
        arguments: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault>;
}

/// One capability invocation in observable execution order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedCapabilityCall {
    pub receiver: String,
    pub interface: String,
    pub operation: String,
    pub arguments: Vec<RuntimeValue>,
    /// `None` only when the supplied capability returned a fault.
    pub result: Option<RuntimeValue>,
}

/// One structured deterministic runtime failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFault {
    pub code: &'static str,
    pub span: Span,
    pub expected: BTreeMap<String, String>,
    pub actual: BTreeMap<String, String>,
}

impl RuntimeFault {
    /// Construct one fault with deterministic expected and actual fact maps.
    pub fn new(
        code: &'static str,
        span: Span,
        expected: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        actual: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            code,
            span,
            expected: expected
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            actual: actual
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
enum RuntimeBinding {
    Value(RuntimeValue),
    Capability(String),
}

/// Successful evaluation before revision metadata is attached by the protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InterpreterSuccess {
    pub value: RuntimeValue,
    pub calls: Vec<ObservedCapabilityCall>,
}

/// Failed evaluation before revision metadata is attached by the protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InterpreterFailure {
    pub fault: RuntimeFault,
    pub calls: Vec<ObservedCapabilityCall>,
}

pub(crate) fn interpret(
    unit: &SourceUnit,
    function_name: &str,
    arguments: Vec<RuntimeValue>,
    environment: &CapabilityEnvironment,
    capabilities: &mut dyn CapabilityProvider,
) -> Result<InterpreterSuccess, InterpreterFailure> {
    let Some(function) = unit.declarations.iter().find_map(|declaration| {
        let Declaration::Function(function) = declaration else {
            return None;
        };
        (function.name == function_name).then_some(function)
    }) else {
        return Err(failure(RuntimeFault::new(
            "AIL.RUNTIME.UNKNOWN_FUNCTION",
            Span::empty(0),
            [("function", function_name)],
            std::iter::empty::<(&str, &str)>(),
        )));
    };

    let value_parameter_count = function
        .parameters
        .iter()
        .filter(|parameter| matches!(parameter.ty, ParameterType::Named(_)))
        .count();
    if arguments.len() != value_parameter_count {
        return Err(failure(RuntimeFault::new(
            "AIL.RUNTIME.ARGUMENT_COUNT",
            function.span,
            [("count", value_parameter_count.to_string())],
            [("count", arguments.len().to_string())],
        )));
    }

    let mut values = arguments.into_iter();
    let mut locals = BTreeMap::new();
    for parameter in &function.parameters {
        match &parameter.ty {
            ParameterType::Named(expected) => {
                let value = values.next().expect("value argument count was checked");
                if value.type_name() != expected || !value_matches_type(unit, &value, expected) {
                    return Err(failure(RuntimeFault::new(
                        "AIL.RUNTIME.ARGUMENT_TYPE",
                        parameter.span,
                        [("type", expected.as_str())],
                        [("type", value.type_name())],
                    )));
                }
                locals.insert(parameter.name.clone(), RuntimeBinding::Value(value));
            }
            ParameterType::Capability(interface) => {
                if !capabilities.supports(&parameter.name, interface) {
                    return Err(failure(RuntimeFault::new(
                        "AIL.RUNTIME.MISSING_CAPABILITY",
                        parameter.span,
                        [
                            ("receiver", parameter.name.as_str()),
                            ("interface", interface.as_str()),
                        ],
                        std::iter::empty::<(&str, &str)>(),
                    )));
                }
                locals.insert(
                    parameter.name.clone(),
                    RuntimeBinding::Capability(interface.clone()),
                );
            }
        }
    }

    let mut evaluator = Evaluator {
        unit,
        environment,
        capabilities,
        calls: Vec::new(),
    };
    match evaluator.eval_block(&function.body, &locals) {
        Ok(value) => Ok(InterpreterSuccess {
            value,
            calls: evaluator.calls,
        }),
        Err(fault) => Err(InterpreterFailure {
            fault,
            calls: evaluator.calls,
        }),
    }
}

fn failure(fault: RuntimeFault) -> InterpreterFailure {
    InterpreterFailure {
        fault,
        calls: Vec::new(),
    }
}

struct Evaluator<'a> {
    unit: &'a SourceUnit,
    environment: &'a CapabilityEnvironment,
    capabilities: &'a mut dyn CapabilityProvider,
    calls: Vec<ObservedCapabilityCall>,
}

impl Evaluator<'_> {
    fn eval_block(
        &mut self,
        block: &Block,
        outer: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        let mut locals = outer.clone();
        for binding in &block.bindings {
            let value = self.eval_expr(&binding.value, &locals)?;
            locals.insert(binding.name.clone(), RuntimeBinding::Value(value));
        }
        self.eval_expr(&block.tail, &locals)
    }

    fn eval_expr(
        &mut self,
        expression: &Expr,
        locals: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        match expression {
            Expr::Text { value, .. } => Ok(RuntimeValue::Text(value.clone())),
            Expr::Integer { spelling, span } => spelling
                .parse::<u128>()
                .map(RuntimeValue::Int)
                .map_err(|_| {
                    RuntimeFault::new(
                        "AIL.RUNTIME.INTEGER_OVERFLOW",
                        *span,
                        [("range", "0..=u128::MAX")],
                        [("spelling", spelling.as_str())],
                    )
                }),
            Expr::Name { name, span } => match locals.get(name) {
                Some(RuntimeBinding::Value(value)) => Ok(value.clone()),
                Some(RuntimeBinding::Capability(_)) => Err(RuntimeFault::new(
                    "AIL.RUNTIME.CAPABILITY_AS_VALUE",
                    *span,
                    [("kind", "value")],
                    [("kind", "capability")],
                )),
                None => Err(RuntimeFault::new(
                    "AIL.RUNTIME.UNRESOLVED_NAME",
                    *span,
                    [("name", name.as_str())],
                    std::iter::empty::<(&str, &str)>(),
                )),
            },
            Expr::Record { name, fields, .. } => {
                self.eval_record(name, fields, expression.span(), locals)
            }
            Expr::Variant {
                type_name,
                case,
                payload,
                ..
            } => Ok(RuntimeValue::Variant {
                type_name: type_name.clone(),
                case: case.clone(),
                payload: match payload {
                    Some(payload) => Some(Box::new(self.eval_expr(payload, locals)?)),
                    None => None,
                },
            }),
            Expr::CapabilityCall {
                receiver,
                operation,
                arguments,
                span,
            } => self.eval_call(receiver, operation, arguments, *span, locals),
            Expr::FieldAccess {
                target,
                field,
                span,
            } => self.eval_field_access(target, field, *span, locals),
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => self.eval_if(condition, then_branch, else_branch, locals),
            Expr::Match {
                scrutinee, arms, ..
            } => self.eval_match(scrutinee, arms, expression.span(), locals),
        }
    }

    fn eval_record(
        &mut self,
        name: &str,
        fields: &[crate::RecordFieldValue],
        span: Span,
        locals: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        let declared_fields = self
            .record(name, span)?
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<Vec<_>>();
        let mut values = BTreeMap::new();
        for declared_name in declared_fields {
            let Some(field) = fields.iter().find(|field| field.name == declared_name) else {
                return Err(RuntimeFault::new(
                    "AIL.RUNTIME.INVALID_RECORD",
                    span,
                    [("field", declared_name)],
                    [("field", "missing")],
                ));
            };
            values.insert(field.name.clone(), self.eval_expr(&field.value, locals)?);
        }
        Ok(RuntimeValue::Record {
            type_name: name.to_owned(),
            fields: values,
        })
    }

    fn eval_call(
        &mut self,
        receiver: &str,
        operation: &str,
        arguments: &[Expr],
        span: Span,
        locals: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        let arguments = arguments
            .iter()
            .map(|argument| self.eval_expr(argument, locals))
            .collect::<Result<Vec<_>, _>>()?;
        if crate::semantics::intrinsic_signature(receiver, operation).is_some() {
            return eval_intrinsic(receiver, operation, &arguments, span);
        }
        let Some(RuntimeBinding::Capability(interface)) = locals.get(receiver) else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.INVALID_CAPABILITY",
                span,
                [("receiver", receiver)],
                std::iter::empty::<(&str, &str)>(),
            ));
        };
        self.calls.push(ObservedCapabilityCall {
            receiver: receiver.to_owned(),
            interface: interface.clone(),
            operation: operation.to_owned(),
            arguments: arguments.clone(),
            result: None,
        });
        let result = self
            .capabilities
            .call(receiver, interface, operation, &arguments)?;
        let expected = self.capability_result_type(interface, operation, span)?;
        if !value_matches_type(self.unit, &result, expected) {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.CAPABILITY_RESULT",
                span,
                [("type", expected)],
                [("type", result.type_name())],
            ));
        }
        self.calls
            .last_mut()
            .expect("call was recorded before invocation")
            .result = Some(result.clone());
        Ok(result)
    }

    fn capability_result_type(
        &self,
        interface: &str,
        operation: &str,
        span: Span,
    ) -> Result<&str, RuntimeFault> {
        self.environment
            .interface(interface)
            .and_then(|interface| interface.operation(operation))
            .map(|operation| operation.result.as_str())
            .ok_or_else(|| {
                RuntimeFault::new(
                    "AIL.RUNTIME.CAPABILITY_CONTRACT",
                    span,
                    [("operation", format!("{interface}.{operation}"))],
                    std::iter::empty::<(&str, String)>(),
                )
            })
    }

    fn eval_field_access(
        &mut self,
        target: &Expr,
        field: &str,
        span: Span,
        locals: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        let target = self.eval_expr(target, locals)?;
        let RuntimeValue::Record { fields, .. } = target else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.FIELD_TARGET",
                span,
                [("kind", "record")],
                [("type", target.type_name())],
            ));
        };
        fields.get(field).cloned().ok_or_else(|| {
            RuntimeFault::new(
                "AIL.RUNTIME.UNKNOWN_FIELD",
                span,
                [("field", field)],
                std::iter::empty::<(&str, &str)>(),
            )
        })
    }

    fn eval_if(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: &Block,
        locals: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        match self.eval_expr(condition, locals)? {
            RuntimeValue::Bool(true) => self.eval_block(then_branch, locals),
            RuntimeValue::Bool(false) => self.eval_block(else_branch, locals),
            actual => Err(RuntimeFault::new(
                "AIL.RUNTIME.IF_CONDITION",
                condition.span(),
                [("type", "Bool")],
                [("type", actual.type_name())],
            )),
        }
    }

    fn eval_match(
        &mut self,
        scrutinee: &Expr,
        arms: &[crate::MatchArm],
        span: Span,
        locals: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        let value = self.eval_expr(scrutinee, locals)?;
        let RuntimeValue::Variant {
            type_name,
            case,
            payload,
        } = value
        else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.MATCH_TARGET",
                scrutinee.span(),
                [("kind", "variant")],
                [("type", value.type_name())],
            ));
        };
        let Some(arm) = arms
            .iter()
            .find(|arm| arm.type_name == type_name && arm.case == case)
        else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.NON_EXHAUSTIVE_MATCH",
                span,
                [("case", format!("{type_name}::{case}"))],
                std::iter::empty::<(&str, String)>(),
            ));
        };
        let mut arm_locals = locals.clone();
        if let Some(binding) = &arm.binding {
            let Some(payload) = payload else {
                return Err(RuntimeFault::new(
                    "AIL.RUNTIME.MATCH_PAYLOAD",
                    arm.span,
                    [("payload", "present")],
                    [("payload", "missing")],
                ));
            };
            arm_locals.insert(binding.clone(), RuntimeBinding::Value(*payload));
        }
        self.eval_block(&arm.body, &arm_locals)
    }

    fn record(&self, name: &str, span: Span) -> Result<&crate::RecordDecl, RuntimeFault> {
        self.unit
            .declarations
            .iter()
            .find_map(|declaration| {
                let Declaration::Record(record) = declaration else {
                    return None;
                };
                (record.name == name).then_some(record)
            })
            .ok_or_else(|| {
                RuntimeFault::new(
                    "AIL.RUNTIME.UNKNOWN_RECORD",
                    span,
                    [("record", name)],
                    std::iter::empty::<(&str, &str)>(),
                )
            })
    }
}

fn value_matches_type(unit: &SourceUnit, value: &RuntimeValue, expected: &str) -> bool {
    match expected {
        "Unit" => matches!(value, RuntimeValue::Unit),
        "Text" => matches!(value, RuntimeValue::Text(_)),
        "Int" => matches!(value, RuntimeValue::Int(_)),
        "Bool" => matches!(value, RuntimeValue::Bool(_)),
        "Bytes" => matches!(value, RuntimeValue::Bytes(_)),
        _ => unit
            .declarations
            .iter()
            .any(|declaration| match (declaration, value) {
                (Declaration::Record(record), RuntimeValue::Record { type_name, fields })
                    if record.name == expected && type_name == expected =>
                {
                    fields.len() == record.fields.len()
                        && record.fields.iter().all(|field| {
                            fields
                                .get(&field.name)
                                .is_some_and(|value| value_matches_type(unit, value, &field.ty))
                        })
                }
                (
                    Declaration::Variant(variant),
                    RuntimeValue::Variant {
                        type_name,
                        case,
                        payload,
                    },
                ) if variant.name == expected && type_name == expected => variant
                    .cases
                    .iter()
                    .find(|candidate| candidate.name == *case)
                    .is_some_and(|candidate| match (&candidate.payload, payload) {
                        (None, None) => true,
                        (Some(expected), Some(actual)) => {
                            value_matches_type(unit, actual, expected)
                        }
                        _ => false,
                    }),
                _ => false,
            }),
    }
}

fn eval_intrinsic(
    namespace: &str,
    operation: &str,
    arguments: &[RuntimeValue],
    span: Span,
) -> Result<RuntimeValue, RuntimeFault> {
    let invalid = || {
        RuntimeFault::new(
            "AIL.RUNTIME.INTRINSIC_ARGUMENT",
            span,
            [("operation", format!("{namespace}.{operation}"))],
            [("argument_types", runtime_types(arguments).join(", "))],
        )
    };
    match (namespace, operation, arguments) {
        ("text", "is_empty", [RuntimeValue::Text(value)]) => {
            Ok(RuntimeValue::Bool(value.is_empty()))
        }
        (
            "text",
            "byte_length_between",
            [
                RuntimeValue::Text(value),
                RuntimeValue::Int(minimum),
                RuntimeValue::Int(maximum),
            ],
        ) => Ok(RuntimeValue::Bool(
            (*minimum..=*maximum).contains(&(value.len() as u128)),
        )),
        ("text", "first_ascii_alphanumeric", [RuntimeValue::Text(value)]) => {
            Ok(RuntimeValue::Bool(
                value
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_alphanumeric()),
            ))
        }
        (
            "text",
            "rest_ascii_alphanumeric_or",
            [RuntimeValue::Text(value), RuntimeValue::Text(allowed)],
        ) => Ok(RuntimeValue::Bool(value.as_bytes().get(1..).is_some_and(
            |rest| {
                rest.iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || allowed.as_bytes().contains(byte))
            },
        ))),
        ("text", "scalar_count_gt", [RuntimeValue::Text(value), RuntimeValue::Int(limit)]) => {
            Ok(RuntimeValue::Bool(value.chars().count() as u128 > *limit))
        }
        ("text", "contains_control", [RuntimeValue::Text(value)]) => {
            Ok(RuntimeValue::Bool(value.chars().any(char::is_control)))
        }
        ("bytes", "length_gt", [RuntimeValue::Bytes(value), RuntimeValue::Int(limit)]) => {
            Ok(RuntimeValue::Bool(value.len() as u128 > *limit))
        }
        _ => Err(invalid()),
    }
}

fn runtime_types(values: &[RuntimeValue]) -> Vec<&str> {
    values.iter().map(RuntimeValue::type_name).collect()
}
