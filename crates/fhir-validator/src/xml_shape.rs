//! Reshape a resource parsed from XML into the JSON form the checks expect.
//!
//! In XML every primitive is text and a primitive carrying an `id` or extension is an
//! object holding `value`. The reshape types each primitive from its Registry element type
//! and moves the non-value part into `_name`, so the checks see one shape for both formats.
//! Text that does not fit its type is left as a string so `TYPE_001` still reports it.
//! Arrays stay as the XML parser grouped them (repeated siblings only).

use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Value};
use indexmap::IndexMap;
use std::sync::Arc;

use crate::nested::{complex_type, is_primitive, resolve};

// shortcut: contentReference and contained resources are not reshaped, matching nested::walk,
// which does not descend into them either; upgrade together with it.

type Fields = IndexMap<Arc<str>, Node>;

pub(crate) fn to_json_shape(
    resource: &Resource,
    sd: &StructureDefinition,
    registry: &Registry,
) -> Resource {
    let name = resource.resource_type.as_ref();
    Resource {
        fields: reshape_fields(&resource.fields, sd, name, registry),
        resource_type: Arc::clone(&resource.resource_type),
        id: resource.id.clone(),
        from_xml: false,
    }
}

fn reshape_fields(
    fields: &Fields,
    sd: &StructureDefinition,
    def_path: &str,
    registry: &Registry,
) -> Fields {
    let prefix = format!("{def_path}.");
    let mut out = Fields::with_capacity(fields.len());
    for (name, node) in fields {
        let Some((elem, Some(ty))) = resolve(sd, &format!("{prefix}{name}"), name) else {
            out.insert(Arc::clone(name), node.clone());
            continue;
        };
        if is_primitive(&ty) {
            let items = match &node.value {
                Value::Array(items) => items.as_slice(),
                _ => std::slice::from_ref(node),
            };
            let (values, extensions): (Vec<Node>, Vec<Node>) = items
                .iter()
                .map(|item| split_primitive(item, &ty, registry))
                .unzip();
            let wrap = |nodes: Vec<Node>| match &node.value {
                Value::Array(_) => Node {
                    value: Value::Array(nodes),
                    span: node.span,
                },
                _ => nodes.into_iter().next().unwrap_or_else(|| node.clone()),
            };
            if values.iter().any(|n| !matches!(n.value, Value::Null)) {
                out.insert(Arc::clone(name), wrap(values));
            }
            if extensions.iter().any(|n| !matches!(n.value, Value::Null)) {
                out.insert(Arc::from(format!("_{name}")), wrap(extensions));
            }
            continue;
        }
        let (type_sd, type_path) = if ty.as_ref() == "BackboneElement" || ty.as_ref() == "Element" {
            (Some(sd), elem.path.to_string())
        } else {
            let type_sd = complex_type(&ty, registry);
            (
                type_sd,
                type_sd.map(|t| t.name.to_string()).unwrap_or_default(),
            )
        };
        let reshaped = match type_sd {
            Some(type_sd) => reshape_complex(node, type_sd, &type_path, registry),
            None => node.clone(),
        };
        out.insert(Arc::clone(name), reshaped);
    }
    out
}

fn reshape_complex(
    node: &Node,
    sd: &StructureDefinition,
    def_path: &str,
    registry: &Registry,
) -> Node {
    let value = match &node.value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| reshape_complex(item, sd, def_path, registry))
                .collect(),
        ),
        Value::Object(fields) => Value::Object(reshape_fields(fields, sd, def_path, registry)),
        other => other.clone(),
    };
    Node {
        value,
        span: node.span,
    }
}

/// Split one XML primitive into its typed value and its `_name` part (`Null` when absent).
/// The `_name` part is reshaped as an `Element`, so its extension values are typed too.
/// An element with neither value nor `_name` part is kept as is, so the type check reports it.
fn split_primitive(node: &Node, ty: &str, registry: &Registry) -> (Node, Node) {
    let null = Node {
        value: Value::Null,
        span: node.span,
    };
    match &node.value {
        Value::Str(text) => (
            Node {
                value: typed(text, ty),
                span: node.span,
            },
            null,
        ),
        Value::Object(fields) => {
            let rest: Fields = fields
                .iter()
                .filter(|(k, _)| k.as_ref() != "value")
                .map(|(k, v)| (Arc::clone(k), v.clone()))
                .collect();
            let value = fields.get("value").map_or_else(
                || null.clone(),
                |v| match &v.value {
                    Value::Str(text) => Node {
                        value: typed(text, ty),
                        span: v.span,
                    },
                    _ => v.clone(),
                },
            );
            if rest.is_empty() && matches!(value.value, Value::Null) {
                return (node.clone(), null);
            }
            let extension = if rest.is_empty() {
                null
            } else {
                let rest = match complex_type("Element", registry) {
                    Some(element_sd) => reshape_fields(&rest, element_sd, "Element", registry),
                    None => rest,
                };
                Node {
                    value: Value::Object(rest),
                    span: node.span,
                }
            };
            (value, extension)
        }
        _ => (node.clone(), null),
    }
}

/// The JSON value for XML text of primitive type `ty`; the text itself when it does not fit.
fn typed(text: &Arc<str>, ty: &str) -> Value {
    let keep = || Value::Str(Arc::clone(text));
    match ty {
        "boolean" => match text.as_ref() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => keep(),
        },
        // Official integer regex `[0]|[-+]?[1-9][0-9]*`.
        "integer" | "unsignedInt" | "positiveInt" => {
            let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
            let canonical = text.as_ref() == "0"
                || (digits.bytes().all(|b| b.is_ascii_digit()) && !digits.starts_with('0'));
            match text.parse::<i64>() {
                Ok(n) if canonical => Value::Integer(n),
                _ => keep(),
            }
        }
        "decimal" if is_decimal(text) => match text.parse::<f64>() {
            Ok(n) if n.is_finite() => Value::Decimal(n),
            _ => keep(),
        },
        _ => keep(),
    }
}

/// Official decimal regex `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`.
fn is_decimal(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (mantissa, exponent) = s
        .split_once(['e', 'E'])
        .map_or((s, None), |(m, e)| (m, Some(e)));
    let (int, frac) = mantissa
        .split_once('.')
        .map_or((mantissa, None), |(i, f)| (i, Some(f)));
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    (int == "0" || (digits(int) && !int.starts_with('0')))
        && frac.is_none_or(digits)
        && exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_converts_only_well_formed_text() {
        let v = |t: &str, ty| typed(&Arc::from(t), ty);
        assert!(matches!(v("2", "integer"), Value::Integer(2)));
        assert!(matches!(v("007", "integer"), Value::Str(_)));
        assert!(matches!(v("1.50", "decimal"), Value::Decimal(_)));
        assert!(matches!(v("1e999", "decimal"), Value::Str(_)));
        assert!(matches!(v("NaN", "decimal"), Value::Str(_)));
        assert!(matches!(v("false", "boolean"), Value::Bool(false)));
        assert!(matches!(v("yes", "boolean"), Value::Str(_)));
    }
}
