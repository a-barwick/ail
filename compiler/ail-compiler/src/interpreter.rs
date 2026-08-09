//! Deterministic tree-walking execution for the accepted M17 core slice.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Block, CapabilityEnvironment, CapabilityOperationKind, Declaration, Expr, ParameterType,
    SourceUnit, Span, TypeRef, ValueType,
};

/// Opaque source-level cancellation authority supplied by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationToken {
    pub id: String,
}

impl CancellationToken {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// One immutable value accepted or produced by the M17 interpreter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    Unit,
    Text(String),
    Int(u128),
    Bool(bool),
    Bytes(Vec<u8>),
    List(Vec<RuntimeValue>),
    Cancellation(CancellationToken),
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

    #[must_use]
    pub fn list(values: impl IntoIterator<Item = RuntimeValue>) -> Self {
        Self::List(values.into_iter().collect())
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
            Self::List(_) => "List",
            Self::Cancellation(_) => "Cancellation",
            Self::Record { type_name, .. } | Self::Variant { type_name, .. } => type_name,
        }
    }
}

/// A capability implementation supplied by the embedding host.
#[allow(clippy::missing_errors_doc)]
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

    /// Whether this provider implements the separate outbound path.
    fn supports_outbound(&self, _receiver: &str, _interface: &str, _operation: &str) -> bool {
        false
    }

    /// # Errors
    /// Returns an unsupported fault unless the provider overrides this method.
    fn call_outbound(
        &mut self,
        request: &OutboundCapabilityRequest,
    ) -> Result<OutboundProviderOutcome, RuntimeFault> {
        Err(RuntimeFault::new(
            "AIL.RUNTIME.OUTBOUND_UNSUPPORTED",
            Span::empty(0),
            [("operation", request.operation.as_str())],
            std::iter::empty::<(&str, &str)>(),
        ))
    }

    fn supports_outbound_batch(&self, _receiver: &str, _interface: &str, _operation: &str) -> bool {
        false
    }
    fn start_outbound(
        &mut self,
        request: &OutboundCapabilityRequest,
    ) -> Result<OutboundRequestHandle, RuntimeFault> {
        Err(RuntimeFault::new(
            "AIL.RUNTIME.OUTBOUND_BATCH_UNSUPPORTED",
            Span::empty(0),
            [("operation", request.operation.as_str())],
            std::iter::empty::<(&str, &str)>(),
        ))
    }
    fn check_outbound(
        &mut self,
        _handles: &[OutboundRequestHandle],
    ) -> Result<OutboundBatchCheck, RuntimeFault> {
        Err(RuntimeFault::new(
            "AIL.RUNTIME.OUTBOUND_BATCH_UNSUPPORTED",
            Span::empty(0),
            std::iter::empty::<(&str, &str)>(),
            std::iter::empty::<(&str, &str)>(),
        ))
    }
    fn cancel_outbound(&mut self, _handle: &OutboundRequestHandle) -> Result<(), RuntimeFault> {
        Ok(())
    }
    fn collect_outbound(
        &mut self,
        _handle: &OutboundRequestHandle,
    ) -> Result<OutboundProviderOutcome, RuntimeFault> {
        Err(RuntimeFault::new(
            "AIL.RUNTIME.OUTBOUND_BATCH_UNSUPPORTED",
            Span::empty(0),
            std::iter::empty::<(&str, &str)>(),
            std::iter::empty::<(&str, &str)>(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutboundRequestHandle(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundBatchCheck {
    pub completed: Vec<OutboundRequestHandle>,
    pub cancelled: bool,
}

fn request_batch_cancellation(
    provider: &mut dyn CapabilityProvider,
    active: &BTreeMap<OutboundRequestHandle, (usize, usize)>,
) {
    for handle in active.keys() {
        let _ = provider.cancel_outbound(handle);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundCapabilityRequest {
    pub receiver: String,
    pub interface: String,
    pub operation: String,
    pub arguments: Vec<RuntimeValue>,
    pub timeout_ms: u64,
    pub cancellation: CancellationToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboundProviderOutcome {
    Returned(RuntimeValue),
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedOutboundCall {
    pub effect: String,
    pub timeout_ms: u64,
    pub cancellation_token_identity: String,
    pub outcome: Option<OutboundProviderOutcome>,
    pub batch_index: Option<usize>,
    pub start_order: Option<usize>,
    pub completion_order: Option<usize>,
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
    pub outbound: Option<ObservedOutboundCall>,
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
        .filter(|parameter| matches!(parameter.ty, ParameterType::Value(_)))
        .count();
    if arguments.len() != value_parameter_count {
        return Err(failure(RuntimeFault::new(
            "AIL.RUNTIME.ARGUMENT_COUNT",
            function.span,
            [("count", value_parameter_count.to_string())],
            [("count", arguments.len().to_string())],
        )));
    }

    let value_parameters = function
        .parameters
        .iter()
        .filter_map(|parameter| match &parameter.ty {
            ParameterType::Value(ty) => Some((parameter, ty)),
            ParameterType::Capability(_) => None,
        })
        .collect::<Vec<_>>();
    for (index, ((parameter, expected), value)) in
        value_parameters.iter().zip(&arguments).enumerate()
    {
        if let Err(mismatch) =
            validate_runtime_value(unit, value, expected, &format!("argument[{index}]"))
        {
            return Err(failure(mismatch.into_fault(parameter.span)));
        }
    }

    let mut values = arguments.into_iter();
    let mut locals = BTreeMap::new();
    for parameter in &function.parameters {
        match &parameter.ty {
            ParameterType::Value(_) => {
                let value = values.next().expect("value argument count was checked");
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

    #[allow(clippy::too_many_lines)]
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
            Expr::Call {
                function,
                arguments,
                span,
            } => self.eval_function_call(function, arguments, *span, locals),
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
            Expr::Map {
                binding,
                source,
                body,
                ..
            } => {
                let RuntimeValue::List(values) = self.eval_expr(source, locals)? else {
                    return Err(RuntimeFault::new(
                        "AIL.RUNTIME.MAP_SOURCE",
                        source.span(),
                        [("kind", "list")],
                        std::iter::empty::<(&str, &str)>(),
                    ));
                };
                let mut output = Vec::with_capacity(values.len());
                for (index, value) in values.into_iter().enumerate() {
                    let mut nested = locals.clone();
                    nested.insert(binding.clone(), RuntimeBinding::Value(value));
                    match self.eval_block(body, &nested) {
                        Ok(value) => output.push(value),
                        Err(mut fault) => {
                            fault
                                .actual
                                .insert("map_index".to_owned(), index.to_string());
                            return Err(fault);
                        }
                    }
                }
                Ok(RuntimeValue::List(output))
            }
            Expr::ParallelMap {
                binding,
                source,
                limit,
                body,
                span,
                ..
            } => self.eval_parallel_map(
                binding,
                source,
                usize::try_from(*limit).unwrap_or(usize::MAX),
                body,
                *span,
                locals,
            ),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn eval_parallel_map(
        &mut self,
        binding: &str,
        source: &Expr,
        limit: usize,
        body: &Block,
        span: Span,
        locals: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        let RuntimeValue::List(values) = self.eval_expr(source, locals)? else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.PARALLEL_MAP_SOURCE",
                span,
                [("kind", "list")],
                std::iter::empty::<(&str, &str)>(),
            ));
        };
        if limit == 0 {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.PARALLEL_MAP_LIMIT",
                span,
                [("minimum", "1")],
                [("limit", limit.to_string())],
            ));
        }
        let Expr::CapabilityCall {
            receiver,
            operation,
            arguments,
            ..
        } = &body.tail
        else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.PARALLEL_MAP_BODY",
                span,
                [("body", "direct outbound call")],
                std::iter::empty::<(&str, &str)>(),
            ));
        };
        let Some(RuntimeBinding::Capability(interface)) = locals.get(receiver) else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.INVALID_CAPABILITY",
                span,
                [("receiver", receiver)],
                std::iter::empty::<(&str, &str)>(),
            ));
        };
        let signature = self
            .environment
            .interface(interface)
            .and_then(|i| i.operation(operation))
            .ok_or_else(|| {
                RuntimeFault::new(
                    "AIL.RUNTIME.CAPABILITY_CONTRACT",
                    span,
                    [("operation", operation)],
                    std::iter::empty::<(&str, &str)>(),
                )
            })?;
        let CapabilityOperationKind::Outbound(metadata) = &signature.kind else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.OUTBOUND_BATCH_UNSUPPORTED",
                span,
                [("operation", operation)],
                std::iter::empty::<(&str, &str)>(),
            ));
        };
        let mut prepared_items = Vec::with_capacity(values.len());
        for value in values {
            let mut nested = locals.clone();
            nested.insert(binding.to_owned(), RuntimeBinding::Value(value));
            let timeout_expression = &arguments[metadata.timeout_argument_index];
            let timeout_value = self.eval_expr(timeout_expression, &nested)?;
            let cancellation_expression = &arguments[metadata.cancellation_argument_index];
            let cancellation_value = self.eval_expr(cancellation_expression, &nested)?;
            let timeout_ms = match &timeout_value {
                RuntimeValue::Int(v) => u64::try_from(*v).ok(),
                _ => None,
            }
            .filter(|v| *v > 0 && u128::from(*v) <= metadata.maximum_timeout_ms)
            .ok_or_else(|| {
                RuntimeFault::new(
                    "AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT",
                    span,
                    [("maximum", metadata.maximum_timeout_ms.to_string())],
                    [(
                        "argument_index",
                        metadata.timeout_argument_index.to_string(),
                    )],
                )
            })?;
            let cancellation = match &cancellation_value {
                RuntimeValue::Cancellation(value) => value.clone(),
                _ => {
                    return Err(RuntimeFault::new(
                        "AIL.RUNTIME.ARGUMENT_TYPE",
                        span,
                        [("type", "Cancellation")],
                        std::iter::empty::<(&str, &str)>(),
                    ));
                }
            };
            prepared_items.push((
                nested,
                timeout_value,
                cancellation_value,
                timeout_ms,
                cancellation,
            ));
        }
        let mut requests = Vec::with_capacity(prepared_items.len());
        for (nested, timeout_value, cancellation_value, timeout_ms, cancellation) in prepared_items
        {
            let mut prepared = vec![None; arguments.len()];
            prepared[metadata.timeout_argument_index] = Some(timeout_value);
            prepared[metadata.cancellation_argument_index] = Some(cancellation_value);
            let evaluated = arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| {
                    if let Some(value) = prepared[index].take() {
                        Ok(value)
                    } else {
                        self.eval_expr(argument, &nested)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            requests.push(OutboundCapabilityRequest {
                receiver: receiver.clone(),
                interface: interface.clone(),
                operation: operation.clone(),
                arguments: evaluated,
                timeout_ms,
                cancellation,
            });
        }
        if requests.is_empty() {
            return Ok(RuntimeValue::List(Vec::new()));
        }
        if !self
            .capabilities
            .supports_outbound_batch(receiver, interface, operation)
        {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.OUTBOUND_BATCH_UNSUPPORTED",
                span,
                [("operation", operation)],
                std::iter::empty::<(&str, &str)>(),
            ));
        }
        let mut results = vec![None; requests.len()];
        let mut active = BTreeMap::new();
        let mut next = 0;
        let mut completion = 0;
        while next < requests.len() || !active.is_empty() {
            while next < requests.len() && active.len() < limit {
                let handle = match self.capabilities.start_outbound(&requests[next]) {
                    Ok(handle) => handle,
                    Err(fault) => {
                        request_batch_cancellation(self.capabilities, &active);
                        self.synthesize_active_cancellations(&active, signature, metadata);
                        return Err(fault);
                    }
                };
                self.calls.push(ObservedCapabilityCall {
                    receiver: receiver.clone(),
                    interface: interface.clone(),
                    operation: operation.clone(),
                    arguments: requests[next].arguments.clone(),
                    result: None,
                    outbound: Some(ObservedOutboundCall {
                        effect: format!("{receiver}.{operation}"),
                        timeout_ms: requests[next].timeout_ms,
                        cancellation_token_identity: requests[next].cancellation.id.clone(),
                        outcome: None,
                        batch_index: Some(next),
                        start_order: Some(next),
                        completion_order: None,
                    }),
                });
                let call_index = self.calls.len() - 1;
                if active.contains_key(&handle) {
                    request_batch_cancellation(self.capabilities, &active);
                    return Err(RuntimeFault::new(
                        "AIL.RUNTIME.OUTBOUND_HOST_CONTRACT",
                        span,
                        [("handle", "unique")],
                        [("handle", handle.0)],
                    ));
                }
                active.insert(handle, (next, call_index));
                next += 1;
            }
            if active.is_empty() {
                break;
            }
            let handles = active.keys().cloned().collect::<Vec<_>>();
            let checked = match self.capabilities.check_outbound(&handles) {
                Ok(checked) => checked,
                Err(fault) => {
                    request_batch_cancellation(self.capabilities, &active);
                    return Err(fault);
                }
            };
            let mut reported = BTreeSet::new();
            for handle in &checked.completed {
                if !reported.insert(handle.clone()) {
                    request_batch_cancellation(self.capabilities, &active);
                    return Err(RuntimeFault::new(
                        "AIL.RUNTIME.OUTBOUND_HOST_CONTRACT",
                        span,
                        [("handle", "reported once")],
                        [("handle", handle.0.as_str())],
                    ));
                }
                if !active.contains_key(handle) {
                    request_batch_cancellation(self.capabilities, &active);
                    return Err(RuntimeFault::new(
                        "AIL.RUNTIME.OUTBOUND_HOST_CONTRACT",
                        span,
                        [("handle", "known active")],
                        [("handle", handle.0.as_str())],
                    ));
                }
            }
            for handle in checked.completed {
                let (index, call_index) = active[&handle];
                let outcome = match self.capabilities.collect_outbound(&handle) {
                    Ok(outcome) => outcome,
                    Err(fault) => {
                        request_batch_cancellation(self.capabilities, &active);
                        return Err(fault);
                    }
                };
                let value = self.closed_outcome(signature, metadata, &outcome);
                active.remove(&handle);
                {
                    let outbound = self.calls[call_index].outbound.as_mut().unwrap();
                    outbound.outcome = Some(outcome);
                    outbound.completion_order = Some(completion);
                }
                completion += 1;
                if let Err(fault) = self.validate_capability_result(&value, &signature.result, span)
                {
                    request_batch_cancellation(self.capabilities, &active);
                    return Err(fault);
                }
                self.calls[call_index].result = Some(value.clone());
                results[index] = Some(value);
            }
            if checked.cancelled {
                request_batch_cancellation(self.capabilities, &active);
                self.synthesize_active_cancellations(&active, signature, metadata);
                for (index, _) in active.values().copied() {
                    results[index] = Some(self.closed_outcome(
                        signature,
                        metadata,
                        &OutboundProviderOutcome::Cancelled,
                    ));
                }
                for result in results.iter_mut().skip(next) {
                    *result = Some(self.closed_outcome(
                        signature,
                        metadata,
                        &OutboundProviderOutcome::Cancelled,
                    ));
                }
                break;
            }
        }
        Ok(RuntimeValue::List(
            results
                .into_iter()
                .map(|v| v.expect("all batch slots closed"))
                .collect(),
        ))
    }

    fn synthesize_active_cancellations(
        &mut self,
        active: &BTreeMap<OutboundRequestHandle, (usize, usize)>,
        signature: &crate::CapabilityOperation,
        metadata: &crate::OutboundCapabilityMetadata,
    ) {
        for (_, call_index) in active.values().copied() {
            let outcome = OutboundProviderOutcome::Cancelled;
            let result = self.closed_outcome(signature, metadata, &outcome);
            self.calls[call_index].result = Some(result);
            self.calls[call_index].outbound.as_mut().unwrap().outcome = Some(outcome);
        }
    }

    fn closed_outcome(
        &self,
        signature: &crate::CapabilityOperation,
        metadata: &crate::OutboundCapabilityMetadata,
        outcome: &OutboundProviderOutcome,
    ) -> RuntimeValue {
        match outcome {
            OutboundProviderOutcome::Returned(value) => value.clone(),
            OutboundProviderOutcome::TimedOut => RuntimeValue::variant(
                &signature.result,
                variant_case_name(
                    self.unit,
                    &signature.result,
                    &metadata.timed_out_case_identity,
                )
                .unwrap(),
                None,
            ),
            OutboundProviderOutcome::Cancelled => RuntimeValue::variant(
                &signature.result,
                variant_case_name(
                    self.unit,
                    &signature.result,
                    &metadata.cancelled_case_identity,
                )
                .unwrap(),
                None,
            ),
        }
    }

    fn eval_function_call(
        &mut self,
        function_name: &str,
        arguments: &[Expr],
        span: Span,
        caller_bindings: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        let function = self
            .unit
            .declarations
            .iter()
            .find_map(|declaration| {
                let Declaration::Function(function) = declaration else {
                    return None;
                };
                (function.name == function_name).then_some(function)
            })
            .ok_or_else(|| {
                RuntimeFault::new(
                    "AIL.RUNTIME.UNKNOWN_FUNCTION",
                    span,
                    [("function", function_name)],
                    std::iter::empty::<(&str, &str)>(),
                )
            })?;

        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            values.push(self.eval_expr(argument, caller_bindings)?);
        }
        let mut values = values.into_iter();
        let mut callee_locals = BTreeMap::new();
        for parameter in &function.parameters {
            match &parameter.ty {
                ParameterType::Value(expected) => {
                    let value = values.next().ok_or_else(|| {
                        RuntimeFault::new(
                            "AIL.RUNTIME.ARGUMENT_COUNT",
                            span,
                            [("function", function_name)],
                            [("count", arguments.len().to_string())],
                        )
                    })?;
                    validate_runtime_value(
                        self.unit,
                        &value,
                        expected,
                        &format!("call {function_name} argument"),
                    )
                    .map_err(|mismatch| mismatch.into_fault(span))?;
                    callee_locals.insert(parameter.name.clone(), RuntimeBinding::Value(value));
                }
                ParameterType::Capability(expected) => {
                    let Some(RuntimeBinding::Capability(actual)) =
                        caller_bindings.get(&parameter.name)
                    else {
                        return Err(RuntimeFault::new(
                            "AIL.RUNTIME.MISSING_CAPABILITY",
                            span,
                            [("receiver", parameter.name.as_str())],
                            std::iter::empty::<(&str, &str)>(),
                        ));
                    };
                    if actual != expected {
                        return Err(RuntimeFault::new(
                            "AIL.RUNTIME.CAPABILITY_INTERFACE",
                            span,
                            [("interface", expected.as_str())],
                            [("interface", actual.as_str())],
                        ));
                    }
                    callee_locals.insert(
                        parameter.name.clone(),
                        RuntimeBinding::Capability(actual.clone()),
                    );
                }
            }
        }
        if values.next().is_some() {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.ARGUMENT_COUNT",
                span,
                [("function", function_name)],
                [("count", arguments.len().to_string())],
            ));
        }
        self.eval_block(&function.body, &callee_locals)
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

    #[allow(clippy::too_many_lines)]
    fn eval_call(
        &mut self,
        receiver: &str,
        operation: &str,
        arguments: &[Expr],
        span: Span,
        locals: &BTreeMap<String, RuntimeBinding>,
    ) -> Result<RuntimeValue, RuntimeFault> {
        if crate::semantics::intrinsic_signature(receiver, operation).is_some() {
            let values = arguments
                .iter()
                .map(|argument| self.eval_expr(argument, locals))
                .collect::<Result<Vec<_>, _>>()?;
            return eval_intrinsic(receiver, operation, &values, span);
        }
        let Some(RuntimeBinding::Capability(interface)) = locals.get(receiver) else {
            return Err(RuntimeFault::new(
                "AIL.RUNTIME.INVALID_CAPABILITY",
                span,
                [("receiver", receiver)],
                std::iter::empty::<(&str, &str)>(),
            ));
        };
        let signature = self
            .environment
            .interface(interface)
            .and_then(|candidate| candidate.operation(operation))
            .ok_or_else(|| {
                RuntimeFault::new(
                    "AIL.RUNTIME.CAPABILITY_CONTRACT",
                    span,
                    [("operation", operation)],
                    std::iter::empty::<(&str, &str)>(),
                )
            })?;
        if let CapabilityOperationKind::Outbound(metadata) = &signature.kind {
            // Controls are evaluated and validated before any non-control argument can
            // perform outside work. Remaining arguments retain source order.
            let timeout_value = self.eval_expr(
                arguments
                    .get(metadata.timeout_argument_index)
                    .ok_or_else(|| {
                        RuntimeFault::new(
                            "AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT",
                            span,
                            [("argument", "timeout")],
                            [("value", "missing")],
                        )
                    })?,
                locals,
            )?;
            let cancellation_value = self.eval_expr(
                arguments
                    .get(metadata.cancellation_argument_index)
                    .ok_or_else(|| {
                        RuntimeFault::new(
                            "AIL.RUNTIME.ARGUMENT_TYPE",
                            span,
                            [("type", "Cancellation")],
                            [("value", "missing")],
                        )
                    })?,
                locals,
            )?;
            let valid_timeout = matches!(&timeout_value, RuntimeValue::Int(value) if *value > 0 && *value <= metadata.maximum_timeout_ms && u64::try_from(*value).is_ok());
            if !valid_timeout {
                return Err(RuntimeFault::new(
                    "AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT",
                    span,
                    [("maximum", metadata.maximum_timeout_ms.to_string())],
                    [(
                        "argument_index",
                        metadata.timeout_argument_index.to_string(),
                    )],
                ));
            }
            if !matches!(cancellation_value, RuntimeValue::Cancellation(_)) {
                return Err(RuntimeFault::new(
                    "AIL.RUNTIME.ARGUMENT_TYPE",
                    span,
                    [("type", "Cancellation")],
                    [(
                        "argument_index",
                        metadata.cancellation_argument_index.to_string(),
                    )],
                ));
            }
            let mut evaluated = vec![None; arguments.len()];
            evaluated[metadata.timeout_argument_index] = Some(timeout_value);
            evaluated[metadata.cancellation_argument_index] = Some(cancellation_value);
            let arguments = arguments
                .iter()
                .enumerate()
                .map(|(index, expression)| {
                    if let Some(value) = evaluated[index].take() {
                        Ok(value)
                    } else {
                        self.eval_expr(expression, locals)
                    }
                })
                .collect::<Result<Vec<_>, RuntimeFault>>()?;
            let timeout = arguments
                .get(metadata.timeout_argument_index)
                .and_then(|value| {
                    let RuntimeValue::Int(value) = value else {
                        return None;
                    };
                    u64::try_from(value.to_owned()).ok()
                });
            let valid_timeout = timeout
                .filter(|value| *value > 0 && u128::from(*value) <= metadata.maximum_timeout_ms);
            let Some(timeout_ms) = valid_timeout else {
                return Err(RuntimeFault::new(
                    "AIL.RUNTIME.OUTBOUND_TIMEOUT_ARGUMENT",
                    span,
                    [
                        ("maximum", metadata.maximum_timeout_ms.to_string()),
                        (
                            "argument_index",
                            metadata.timeout_argument_index.to_string(),
                        ),
                    ],
                    [(
                        "value",
                        arguments
                            .get(metadata.timeout_argument_index)
                            .map_or_else(|| "missing".to_owned(), |value| format!("{value:?}")),
                    )],
                ));
            };
            let Some(RuntimeValue::Cancellation(cancellation)) =
                arguments.get(metadata.cancellation_argument_index)
            else {
                return Err(RuntimeFault::new(
                    "AIL.RUNTIME.ARGUMENT_TYPE",
                    span,
                    [("type", "Cancellation")],
                    [(
                        "type",
                        arguments
                            .get(metadata.cancellation_argument_index)
                            .map_or("missing", RuntimeValue::type_name),
                    )],
                ));
            };
            if !self
                .capabilities
                .supports_outbound(receiver, interface, operation)
            {
                return Err(RuntimeFault::new(
                    "AIL.RUNTIME.OUTBOUND_UNSUPPORTED",
                    span,
                    [("operation", operation)],
                    std::iter::empty::<(&str, &str)>(),
                ));
            }
            let request = OutboundCapabilityRequest {
                receiver: receiver.to_owned(),
                interface: interface.clone(),
                operation: operation.to_owned(),
                arguments: arguments.clone(),
                timeout_ms,
                cancellation: cancellation.clone(),
            };
            self.calls.push(ObservedCapabilityCall {
                receiver: receiver.to_owned(),
                interface: interface.clone(),
                operation: operation.to_owned(),
                arguments: arguments.clone(),
                result: None,
                outbound: Some(ObservedOutboundCall {
                    effect: format!("{receiver}.{operation}"),
                    timeout_ms,
                    cancellation_token_identity: cancellation.id.clone(),
                    outcome: None,
                    batch_index: None,
                    start_order: None,
                    completion_order: None,
                }),
            });
            let outcome = self.capabilities.call_outbound(&request)?;
            self.calls
                .last_mut()
                .expect("outbound call recorded")
                .outbound
                .as_mut()
                .expect("outbound facts")
                .outcome = Some(outcome.clone());
            let result = match &outcome {
                OutboundProviderOutcome::Returned(value) => value.clone(),
                OutboundProviderOutcome::TimedOut => RuntimeValue::variant(
                    &signature.result,
                    variant_case_name(
                        self.unit,
                        &signature.result,
                        &metadata.timed_out_case_identity,
                    )
                    .expect("validated outbound timeout case identity"),
                    None,
                ),
                OutboundProviderOutcome::Cancelled => RuntimeValue::variant(
                    &signature.result,
                    variant_case_name(
                        self.unit,
                        &signature.result,
                        &metadata.cancelled_case_identity,
                    )
                    .expect("validated outbound cancellation case identity"),
                    None,
                ),
            };
            self.validate_capability_result(&result, &signature.result, span)?;
            let call = self.calls.last_mut().expect("outbound call recorded");
            call.result = Some(result.clone());
            return Ok(result);
        }
        let arguments = arguments
            .iter()
            .map(|argument| self.eval_expr(argument, locals))
            .collect::<Result<Vec<_>, _>>()?;
        self.calls.push(ObservedCapabilityCall {
            receiver: receiver.to_owned(),
            interface: interface.clone(),
            operation: operation.to_owned(),
            arguments: arguments.clone(),
            result: None,
            outbound: None,
        });
        let result = self
            .capabilities
            .call(receiver, interface, operation, &arguments)?;
        let expected = self.capability_result_type(interface, operation, span)?;
        let expected_type = TypeRef::named(expected, span);
        validate_runtime_value(self.unit, &result, &expected_type, "capability result").map_err(
            |mismatch| {
                let mut fault = mismatch.into_fault(span);
                if fault.code == "AIL.RUNTIME.ARGUMENT_TYPE" {
                    fault.code = "AIL.RUNTIME.CAPABILITY_RESULT";
                }
                fault
            },
        )?;
        self.calls
            .last_mut()
            .expect("call was recorded before invocation")
            .result = Some(result.clone());
        Ok(result)
    }

    fn validate_capability_result(
        &self,
        result: &RuntimeValue,
        expected: &str,
        span: Span,
    ) -> Result<(), RuntimeFault> {
        validate_runtime_value(
            self.unit,
            result,
            &TypeRef::named(expected, span),
            "capability result",
        )
        .map_err(|mismatch| {
            let mut fault = mismatch.into_fault(span);
            if fault.code == "AIL.RUNTIME.ARGUMENT_TYPE" {
                fault.code = "AIL.RUNTIME.CAPABILITY_RESULT";
            }
            fault
        })
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

fn variant_case_name<'a>(
    unit: &'a SourceUnit,
    variant_name: &str,
    identity: &str,
) -> Option<&'a str> {
    unit.declarations.iter().find_map(|declaration| {
        let Declaration::Variant(variant) = declaration else {
            return None;
        };
        (variant.name == variant_name)
            .then_some(variant)?
            .cases
            .iter()
            .find_map(|case| {
                (case.identity.as_deref() == Some(identity)).then_some(case.name.as_str())
            })
    })
}

struct RuntimeValueMismatch {
    code: &'static str,
    expected: BTreeMap<String, String>,
    actual: BTreeMap<String, String>,
}

impl RuntimeValueMismatch {
    fn type_mismatch(expected: &TypeRef, value: &RuntimeValue, path: &str) -> Self {
        Self {
            code: "AIL.RUNTIME.ARGUMENT_TYPE",
            expected: BTreeMap::from([
                ("type".to_owned(), expected.to_string()),
                ("value_path".to_owned(), path.to_owned()),
            ]),
            actual: BTreeMap::from([
                ("type".to_owned(), value.type_name().to_owned()),
                ("value_path".to_owned(), path.to_owned()),
            ]),
        }
    }

    fn into_fault(self, span: Span) -> RuntimeFault {
        RuntimeFault {
            code: self.code,
            span,
            expected: self.expected,
            actual: self.actual,
        }
    }
}

fn validate_runtime_value(
    unit: &SourceUnit,
    value: &RuntimeValue,
    expected: &TypeRef,
    path: &str,
) -> Result<(), RuntimeValueMismatch> {
    match &expected.value {
        ValueType::List {
            element,
            max_length,
            ..
        } => {
            let RuntimeValue::List(values) = value else {
                return Err(RuntimeValueMismatch::type_mismatch(expected, value, path));
            };
            if values.len() as u128 > *max_length {
                return Err(RuntimeValueMismatch {
                    code: "AIL.RUNTIME.LIST_CARDINALITY",
                    expected: BTreeMap::from([
                        ("element_type".to_owned(), element.to_string()),
                        ("maximum".to_owned(), max_length.to_string()),
                    ]),
                    actual: BTreeMap::from([
                        ("count".to_owned(), values.len().to_string()),
                        ("value_path".to_owned(), path.to_owned()),
                    ]),
                });
            }
            for (index, item) in values.iter().enumerate() {
                if let Err(mut mismatch) =
                    validate_runtime_value(unit, item, element, &format!("{path}[{index}]"))
                {
                    if mismatch.code == "AIL.RUNTIME.ARGUMENT_TYPE" {
                        let actual_type = mismatch
                            .actual
                            .get("type")
                            .cloned()
                            .unwrap_or_else(|| item.type_name().to_owned());
                        mismatch.code = "AIL.RUNTIME.LIST_ELEMENT";
                        mismatch
                            .expected
                            .insert("element_type".to_owned(), element.to_string());
                        mismatch
                            .actual
                            .insert("actual_type".to_owned(), actual_type);
                        mismatch
                            .actual
                            .insert("index".to_owned(), index.to_string());
                    }
                    return Err(mismatch);
                }
            }
            Ok(())
        }
        ValueType::Named(name) => validate_named_runtime_value(unit, value, name, expected, path),
    }
}

fn validate_named_runtime_value(
    unit: &SourceUnit,
    value: &RuntimeValue,
    expected_name: &str,
    expected: &TypeRef,
    path: &str,
) -> Result<(), RuntimeValueMismatch> {
    let builtin_matches = match expected_name {
        "Unit" => matches!(value, RuntimeValue::Unit),
        "Text" => matches!(value, RuntimeValue::Text(_)),
        "Int" => matches!(value, RuntimeValue::Int(_)),
        "Bool" => matches!(value, RuntimeValue::Bool(_)),
        "Bytes" => matches!(value, RuntimeValue::Bytes(_)),
        "Cancellation" => matches!(value, RuntimeValue::Cancellation(_)),
        _ => false,
    };
    if builtin_matches {
        return Ok(());
    }

    for declaration in &unit.declarations {
        match (declaration, value) {
            (Declaration::Record(record), RuntimeValue::Record { type_name, fields })
                if record.name == expected_name && type_name == expected_name =>
            {
                if fields.len() != record.fields.len() {
                    return Err(RuntimeValueMismatch::type_mismatch(expected, value, path));
                }
                for field in &record.fields {
                    let Some(field_value) = fields.get(&field.name) else {
                        return Err(RuntimeValueMismatch::type_mismatch(expected, value, path));
                    };
                    validate_runtime_value(
                        unit,
                        field_value,
                        &field.ty,
                        &format!("{path}.{}", field.name),
                    )?;
                }
                return Ok(());
            }
            (
                Declaration::Variant(variant),
                RuntimeValue::Variant {
                    type_name,
                    case,
                    payload,
                },
            ) if variant.name == expected_name && type_name == expected_name => {
                let Some(candidate) = variant
                    .cases
                    .iter()
                    .find(|candidate| candidate.name == *case)
                else {
                    return Err(RuntimeValueMismatch::type_mismatch(expected, value, path));
                };
                return match (&candidate.payload, payload) {
                    (None, None) => Ok(()),
                    (Some(payload_type), Some(actual)) => validate_runtime_value(
                        unit,
                        actual,
                        payload_type,
                        &format!("{path}::{case}"),
                    ),
                    _ => Err(RuntimeValueMismatch::type_mismatch(expected, value, path)),
                };
            }
            _ => {}
        }
    }
    Err(RuntimeValueMismatch::type_mismatch(expected, value, path))
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
