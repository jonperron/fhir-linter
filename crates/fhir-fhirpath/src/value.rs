//! FHIRPath runtime value.
//!
//! FHIRPath is a collection-oriented language: every expression evaluates to a
//! *collection* of zero or more values.  The `Collection` type alias is used
//! throughout the evaluator.
use std::fmt;
use std::sync::Arc;

use indexmap::IndexMap;

/// A single FHIRPath value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    /// FHIRPath 3.0 64-bit integer (distinct from Integer for type-system purposes).
    Long(i64),
    Decimal(f64),
    String(Arc<str>),
    /// ISO date/time/dateTime text, without the literal's `@` or `@T`.
    Date(Arc<str>),
    DateTime(Arc<str>),
    Time(Arc<str>),
    /// Quantity with a UCUM unit.
    Quantity(f64, Arc<str>),
    /// An object node (maps field name → collection).
    Object(Arc<IndexMap<Arc<str>, Collection>>),
}

/// A FHIRPath collection (ordered list of values, may be empty).
pub type Collection = Vec<Value>;

impl Value {
    /// Return `true` if this value is boolean-truthy in FHIRPath semantics.
    pub fn is_truthy(&self) -> bool {
        matches!(self, Value::Bool(true))
    }

    /// Human-readable type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "Boolean",
            Value::Integer(_) => "Integer",
            Value::Long(_) => "Long",
            Value::Decimal(_) => "Decimal",
            Value::String(_) => "String",
            Value::Date(_) => "Date",
            Value::DateTime(_) => "DateTime",
            Value::Time(_) => "Time",
            Value::Quantity(_, _) => "Quantity",
            Value::Object(_) => "Object",
        }
    }

    /// Convert to `Arc<str>` for string operations.
    pub fn as_string(&self) -> Option<Arc<str>> {
        match self {
            Value::String(s) => Some(Arc::clone(s)),
            Value::Date(s) | Value::DateTime(s) | Value::Time(s) => Some(Arc::clone(s)),
            _ => None,
        }
    }

    /// Coerce to `f64` (Integer, Long, or Decimal).
    pub fn as_decimal(&self) -> Option<f64> {
        match self {
            Value::Integer(n) | Value::Long(n) => Some(*n as f64),
            Value::Decimal(d) => Some(*d),
            _ => None,
        }
    }

    /// Coerce to `i64` only for Integer values (not Long).
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(n) => Some(*n),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Integer(n) => write!(f, "{n}"),
            Value::Long(n) => write!(f, "{n}"),
            Value::Decimal(d) => write!(f, "{d}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Date(s) | Value::DateTime(s) | Value::Time(s) => write!(f, "{s}"),
            Value::Quantity(n, u) => write!(f, "{n} '{u}'"),
            Value::Object(_) => write!(f, "{{…}}"),
        }
    }
}

/// FHIR type knowledge for building values: for the child `name` of the
/// element at definition path `parent`, its definition path and type code
/// (`("Patient", "birthDate")` gives `("date", "date")`), or `None` when
/// unknown. See `Registry::child_element` in `fhir-definitions`.
pub type ChildElement<'a> = dyn Fn(&str, &str) -> Option<(Arc<str>, Arc<str>)> + 'a;

/// Convert from a `fhir_parser::Value` node into a FHIRPath `Value`, without
/// type knowledge (FHIR date strings stay Strings).
///
/// Arrays become multiple items in a collection (handled by the evaluator);
/// individual values are mapped 1-to-1.
pub fn from_parser_value(v: &fhir_parser::Value) -> Vec<Value> {
    convert(v, None, &|_, _| None)
}

/// Build the root `Object` value from a `fhir_parser::Resource`, without type
/// knowledge; see [`resource_to_typed_value`].
pub fn resource_to_value(resource: &fhir_parser::Resource) -> Value {
    resource_to_typed_value(resource, &|_, _| None)
}

/// Build the root `Object` value from a `fhir_parser::Resource`, typing each
/// primitive by its declared FHIR type (R5 fhirpath.html §2.1.9.1): `date`
/// becomes a Date, `dateTime` and `instant` a DateTime, `time` a Time, so they
/// compare as temporals rather than as text.
pub fn resource_to_typed_value(
    resource: &fhir_parser::Resource,
    child_element: &ChildElement<'_>,
) -> Value {
    let mut map: IndexMap<Arc<str>, Collection> = IndexMap::new();
    // Include resourceType as a synthetic string field
    map.insert(
        Arc::from("resourceType"),
        vec![Value::String(Arc::clone(&resource.resource_type))],
    );
    if let Some(id) = &resource.id {
        map.insert(Arc::from("id"), vec![Value::String(Arc::clone(id))]);
    }
    let definition = &*resource.resource_type;
    for (k, node) in &resource.fields {
        let child = child_element(definition, k);
        map.insert(
            Arc::clone(k),
            convert(&node.value, child.as_ref(), child_element),
        );
    }
    Value::Object(Arc::new(map))
}

/// Convert one node whose element is `child` (definition path, type code).
fn convert(
    v: &fhir_parser::Value,
    child: Option<&(Arc<str>, Arc<str>)>,
    child_element: &ChildElement<'_>,
) -> Collection {
    match v {
        fhir_parser::Value::Null => vec![Value::Null],
        fhir_parser::Value::Bool(b) => vec![Value::Bool(*b)],
        fhir_parser::Value::Integer(n) => vec![Value::Integer(*n)],
        fhir_parser::Value::Decimal(d) => vec![Value::Decimal(*d)],
        fhir_parser::Value::Str(s) => vec![typed_string(s, child.map(|(_, ty)| &**ty))],
        fhir_parser::Value::Array(items) => items
            .iter()
            .flat_map(|n| convert(&n.value, child, child_element))
            .collect(),
        fhir_parser::Value::Object(fields) => {
            // A resource (contained, Bundle entry) is typed by its own resourceType.
            let definition = match fields.get("resourceType").map(|n| &n.value) {
                Some(fhir_parser::Value::Str(resource_type)) => Some(&**resource_type),
                _ => child.map(|(definition, _)| &**definition),
            };
            let map: IndexMap<Arc<str>, Collection> = fields
                .iter()
                .map(|(k, node)| {
                    let grandchild = definition.and_then(|d| child_element(d, k));
                    (
                        Arc::clone(k),
                        convert(&node.value, grandchild.as_ref(), child_element),
                    )
                })
                .collect();
            vec![Value::Object(Arc::new(map))]
        }
    }
}

/// A string as the FHIRPath type of its FHIR primitive type.
fn typed_string(s: &Arc<str>, ty: Option<&str>) -> Value {
    let s = Arc::clone(s);
    match ty {
        Some("date") => Value::Date(s),
        Some("dateTime" | "instant") => Value::DateTime(s),
        Some("time") => Value::Time(s),
        _ => Value::String(s),
    }
}
