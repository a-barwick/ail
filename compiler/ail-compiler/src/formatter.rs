use std::collections::BTreeMap;
use std::fmt::Write;

use crate::{
    Block, Declaration, Effect, Expr, FunctionDecl, MatchArm, Parameter, ParameterType, RecordDecl,
    RecordFieldValue, SourceUnit, VariantDecl,
};

pub(crate) fn format(unit: &SourceUnit) -> String {
    let records = unit
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Record(record) => Some((
                record.name.as_str(),
                record
                    .fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>(),
            )),
            Declaration::Variant(_) | Declaration::Function(_) => None,
        })
        .collect::<BTreeMap<_, _>>();

    let mut output = String::new();
    for (index, declaration) in unit.declarations.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        match declaration {
            Declaration::Record(record) => format_record(&mut output, record),
            Declaration::Variant(variant) => format_variant(&mut output, variant),
            Declaration::Function(function) => format_function(&mut output, function, &records),
        }
    }
    output
}

fn format_record(output: &mut String, record: &RecordDecl) {
    writeln!(output, "record {} {{", record.name).expect("writing to String cannot fail");
    for field in &record.fields {
        writeln!(output, "  {}: {};", field.name, field.ty).expect("writing to String cannot fail");
    }
    output.push_str("}\n");
}

fn format_variant(output: &mut String, variant: &VariantDecl) {
    writeln!(output, "variant {} {{", variant.name).expect("writing to String cannot fail");
    for case in &variant.cases {
        if let Some(payload) = &case.payload {
            writeln!(output, "  {}({payload});", case.name).expect("writing to String cannot fail");
        } else {
            writeln!(output, "  {};", case.name).expect("writing to String cannot fail");
        }
    }
    output.push_str("}\n");
}

fn format_function(
    output: &mut String,
    function: &FunctionDecl,
    records: &BTreeMap<&str, Vec<&str>>,
) {
    write!(output, "fn {}(", function.name).expect("writing to String cannot fail");
    format_joined(output, &function.parameters, format_parameter);
    write!(output, ") -> {}", function.result_type).expect("writing to String cannot fail");
    if !function.effects.is_empty() {
        output.push_str(" effects { ");
        format_joined(output, &function.effects, format_effect);
        output.push_str(" }");
    }
    output.push_str(" {\n");
    format_block_body(output, &function.body, records, 1);
    output.push_str("}\n");
}

fn format_parameter(output: &mut String, parameter: &Parameter) {
    match &parameter.ty {
        ParameterType::Named(ty) => {
            write!(output, "{}: {ty}", parameter.name).expect("writing to String cannot fail");
        }
        ParameterType::Capability(ty) => {
            write!(output, "{}: capability {ty}", parameter.name)
                .expect("writing to String cannot fail");
        }
    }
}

fn format_effect(output: &mut String, effect: &Effect) {
    write!(output, "{}.{}", effect.receiver, effect.operation)
        .expect("writing to String cannot fail");
}

fn format_block_body(
    output: &mut String,
    block: &Block,
    records: &BTreeMap<&str, Vec<&str>>,
    indent: usize,
) {
    for binding in &block.bindings {
        write_indent(output, indent);
        write!(output, "let {} = ", binding.name).expect("writing to String cannot fail");
        format_expression(output, &binding.value, records, indent);
        output.push_str(";\n");
    }
    write_indent(output, indent);
    format_expression(output, &block.tail, records, indent);
    output.push('\n');
}

fn format_expression(
    output: &mut String,
    expression: &Expr,
    records: &BTreeMap<&str, Vec<&str>>,
    indent: usize,
) {
    match expression {
        Expr::Text { value, .. } => {
            output.push_str(
                &serde_json::to_string(value).expect("a Rust string always encodes as JSON"),
            );
        }
        Expr::Integer { spelling, .. } => {
            let canonical = spelling
                .parse::<u128>()
                .map_or_else(|_| spelling.as_str().to_owned(), |value| value.to_string());
            output.push_str(&canonical);
        }
        Expr::Name { name, .. } => output.push_str(name),
        Expr::Record { name, fields, .. } => {
            write!(output, "{name} {{ ").expect("writing to String cannot fail");
            let mut ordered = fields.iter().collect::<Vec<_>>();
            if let Some(field_order) = records.get(name.as_str()) {
                ordered.sort_by_key(|field| {
                    field_order
                        .iter()
                        .position(|expected| *expected == field.name)
                        .unwrap_or(usize::MAX)
                });
            }
            format_joined(output, &ordered, |output, field| {
                format_record_field(output, field, records, indent);
            });
            output.push_str(" }");
        }
        Expr::Variant {
            type_name,
            case,
            payload,
            ..
        } => {
            write!(output, "{type_name}::{case}").expect("writing to String cannot fail");
            if let Some(payload) = payload {
                output.push('(');
                format_expression(output, payload, records, indent);
                output.push(')');
            }
        }
        Expr::CapabilityCall {
            receiver,
            operation,
            arguments,
            ..
        } => {
            write!(output, "{receiver}.{operation}(").expect("writing to String cannot fail");
            format_joined(output, arguments, |output, argument| {
                format_expression(output, argument, records, indent);
            });
            output.push(')');
        }
        Expr::FieldAccess { target, field, .. } => {
            format_expression(output, target, records, indent);
            write!(output, ".{field}").expect("writing to String cannot fail");
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            output.push_str("if ");
            format_expression(output, condition, records, indent);
            output.push_str(" {\n");
            format_block_body(output, then_branch, records, indent + 1);
            write_indent(output, indent);
            output.push_str("} else {\n");
            format_block_body(output, else_branch, records, indent + 1);
            write_indent(output, indent);
            output.push('}');
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            output.push_str("match ");
            format_expression(output, scrutinee, records, indent);
            output.push_str(" {\n");
            for arm in arms {
                format_match_arm(output, arm, records, indent + 1);
            }
            write_indent(output, indent);
            output.push('}');
        }
    }
}

fn format_record_field(
    output: &mut String,
    field: &RecordFieldValue,
    records: &BTreeMap<&str, Vec<&str>>,
    indent: usize,
) {
    write!(output, "{}: ", field.name).expect("writing to String cannot fail");
    format_expression(output, &field.value, records, indent);
}

fn format_match_arm(
    output: &mut String,
    arm: &MatchArm,
    records: &BTreeMap<&str, Vec<&str>>,
    indent: usize,
) {
    write_indent(output, indent);
    write!(output, "{}::{}", arm.type_name, arm.case).expect("writing to String cannot fail");
    if let Some(binding) = &arm.binding {
        write!(output, "({binding})").expect("writing to String cannot fail");
    }
    output.push_str(" => {\n");
    format_block_body(output, &arm.body, records, indent + 1);
    write_indent(output, indent);
    output.push_str("},\n");
}

fn write_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push_str("  ");
    }
}

fn format_joined<T>(output: &mut String, values: &[T], mut formatter: impl FnMut(&mut String, &T)) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        formatter(output, value);
    }
}
