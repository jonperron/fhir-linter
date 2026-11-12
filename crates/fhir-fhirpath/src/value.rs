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
    /// Quantity with a UCUM unit, or a calendar duration unit given by its
    /// singular keyword (N1 §4.1.8: `4 days` has the unit `'day'`).
    Quantity(f64, Arc<str>),
    /// An object node (maps field name → collection).
    Object(Arc<IndexMap<Arc<str>, Collection>>),
    /// A FHIR primitive that has an `id` or extensions (JSON `_name`, or an XML
    /// element with children): its value (`Null` when it has none) and its own
    /// children. Operators and functions on values see the value; navigation
    /// (`extension`, `id`, `children()`) sees the children.
    Primitive(Box<Value>, Arc<IndexMap<Arc<str>, Collection>>),
}

/// A FHIRPath collection (ordered list of values, may be empty).
pub type Collection = Vec<Value>;

impl Value {
    /// Return `true` if this value is boolean-truthy in FHIRPath semantics.
    pub fn is_truthy(&self) -> bool {
        matches!(self.primitive_value(), Value::Bool(true))
    }

    /// The value a [`Value::Primitive`] carries; any other value is itself.
    pub fn primitive_value(&self) -> &Value {
        match self {
            Value::Primitive(value, _) => value,
            other => other,
        }
    }

    /// The children of an object or of a primitive with an id or extensions.
    pub(crate) fn fields(&self) -> Option<&IndexMap<Arc<str>, Collection>> {
        match self {
            Value::Object(fields) | Value::Primitive(_, fields) => Some(fields),
            _ => None,
        }
    }

    /// The child elements, for `children()` and `descendants()`: every field
    /// but `resourceType`, which names the type in JSON and is not an element.
    pub(crate) fn child_elements(&self) -> impl Iterator<Item = &Value> {
        self.fields()
            .into_iter()
            .flatten()
            .filter(|(key, _)| &***key != "resourceType")
            .flat_map(|(_, values)| values)
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
            Value::Primitive(value, _) => value.type_name(),
        }
    }

    /// Convert to `Arc<str>` for string operations.
    pub fn as_string(&self) -> Option<Arc<str>> {
        match self.primitive_value() {
            Value::String(s) => Some(Arc::clone(s)),
            Value::Date(s) | Value::DateTime(s) | Value::Time(s) => Some(Arc::clone(s)),
            _ => None,
        }
    }

    /// Coerce to `f64` (Integer, Long, or Decimal).
    pub fn as_decimal(&self) -> Option<f64> {
        match self.primitive_value() {
            Value::Integer(n) | Value::Long(n) => Some(*n as f64),
            Value::Decimal(d) => Some(*d),
            _ => None,
        }
    }

    /// Coerce to `i64` only for Integer values (not Long).
    pub fn as_integer(&self) -> Option<i64> {
        match self.primitive_value() {
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
            Value::Primitive(value, _) => write!(f, "{value}"),
        }
    }
}

/// N1 §4.1.8 calendar durations: (singular keyword, definite UCUM unit).
const CALENDAR_DURATIONS: [(&str, &str); 8] = [
    ("year", "a"),
    ("month", "mo"),
    ("week", "wk"),
    ("day", "d"),
    ("hour", "h"),
    ("minute", "min"),
    ("second", "s"),
    ("millisecond", "ms"),
];

/// The unit a calendar duration keyword (singular or plural) stands for.
pub(crate) fn calendar_unit(keyword: &str) -> Option<&'static str> {
    let singular = keyword.strip_suffix('s').unwrap_or(keyword);
    CALENDAR_DURATIONS
        .iter()
        .find(|(unit, _)| *unit == singular)
        .map(|(unit, _)| *unit)
}

/// Whether two quantity units match (N1 §6.1.1, §6.1.2): the same unit, or
/// a calendar duration and its UCUM unit. Under `=` that pairing holds only
/// for seconds and below; under `~` it always holds.
pub(crate) fn same_quantity_unit(a: &str, b: &str, equivalent: bool) -> bool {
    a == b
        || CALENDAR_DURATIONS.iter().any(|(calendar, ucum)| {
            (equivalent || matches!(*calendar, "second" | "millisecond"))
                && ((a, b) == (*calendar, *ucum) || (b, a) == (*calendar, *ucum))
        })
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
    let fields = object_fields(
        &resource.fields,
        Some(&resource.resource_type),
        child_element,
    );
    for (key, mut values) in fields {
        // `_id` carries the extensions of the id the parser lifted out of `fields`.
        if let (Some(id), [Value::Primitive(value, _)]) = (&resource.id, values.as_mut_slice())
            && &*key == "id"
        {
            **value = Value::String(Arc::clone(id));
        }
        map.insert(key, values);
    }
    Value::Object(Arc::new(map))
}

/// Convert one instance value and, for a primitive, its JSON `_name` element,
/// whose element is `child` (definition path and type code, as returned by
/// [`ChildElement`]). An array becomes one item per value; a JSON null, or
/// nothing at all, gives an empty collection.
pub fn node_to_typed_value(
    value: Option<&fhir_parser::Value>,
    element: Option<&fhir_parser::Value>,
    child: Option<&(Arc<str>, Arc<str>)>,
    child_element: &ChildElement<'_>,
) -> Collection {
    match (value, element) {
        (Some(value), None) => convert(value, child, child_element),
        (value, Some(element)) => split(value, element, child, child_element),
        (None, None) => vec![],
    }
}

/// Convert one node whose element is `child` (definition path, type code).
fn convert(
    v: &fhir_parser::Value,
    child: Option<&(Arc<str>, Arc<str>)>,
    child_element: &ChildElement<'_>,
) -> Collection {
    match v {
        // A JSON null (an array placeholder) is not an item.
        fhir_parser::Value::Null => vec![],
        fhir_parser::Value::Bool(b) => vec![Value::Bool(*b)],
        fhir_parser::Value::Integer(n) => vec![Value::Integer(*n)],
        fhir_parser::Value::Decimal(d) => vec![Value::Decimal(*d)],
        fhir_parser::Value::Str(s) => vec![typed_string(s, child.map(|(_, ty)| &**ty))],
        fhir_parser::Value::Array(items) => items
            .iter()
            .flat_map(|n| convert(&n.value, child, child_element))
            .collect(),
        // An XML primitive with an id or extensions: `value` holds its value.
        fhir_parser::Value::Object(fields)
            if child.is_some_and(|(_, ty)| ty.starts_with(|c: char| c.is_ascii_lowercase())) =>
        {
            let value = fields
                .get("value")
                .and_then(|node| convert(&node.value, child, child_element).pop())
                .unwrap_or(Value::Null);
            let mut element = element_fields(fields, child, child_element);
            element.shift_remove("value");
            vec![Value::Primitive(Box::new(value), Arc::new(element))]
        }
        fhir_parser::Value::Object(fields) => {
            // A resource (contained, Bundle entry) is typed by its own resourceType.
            let definition = match fields.get("resourceType").map(|n| &n.value) {
                Some(fhir_parser::Value::Str(resource_type)) => Some(&**resource_type),
                _ => child.map(|(definition, _)| &**definition),
            };
            vec![Value::Object(Arc::new(object_fields(
                fields,
                definition,
                child_element,
            )))]
        }
    }
}

/// The children of an object at `definition`, with each JSON `_name` merged
/// into the primitive `name` it belongs to (R5 json.html, "the split
/// representation of primitives" is not visible to FHIRPath).
fn object_fields(
    fields: &IndexMap<Arc<str>, fhir_parser::Node>,
    definition: Option<&str>,
    child_element: &ChildElement<'_>,
) -> IndexMap<Arc<str>, Collection> {
    let mut map = IndexMap::new();
    for (key, node) in fields {
        let (name, value, element) = match key.strip_prefix('_') {
            // Merged when its primitive is reached.
            Some(bare) if fields.contains_key(bare) => continue,
            // Extensions on a primitive with no value.
            Some(bare) => (Arc::from(bare), None, Some(&node.value)),
            None => (
                Arc::clone(key),
                Some(&node.value),
                fields.get(&*format!("_{key}")).map(|n| &n.value),
            ),
        };
        let child = definition.and_then(|d| child_element(d, &name));
        let values = node_to_typed_value(value, element, child.as_ref(), child_element);
        if !values.is_empty() {
            map.insert(name, values);
        }
    }
    map
}

/// Pair a JSON primitive (or array of them) with its `_name` element(s), by index.
fn split(
    value: Option<&fhir_parser::Value>,
    element: &fhir_parser::Value,
    child: Option<&(Arc<str>, Arc<str>)>,
    child_element: &ChildElement<'_>,
) -> Collection {
    let values: Vec<&fhir_parser::Value> = match value {
        Some(fhir_parser::Value::Array(items)) => items.iter().map(|n| &n.value).collect(),
        Some(v) => vec![v],
        None => vec![],
    };
    let elements: Vec<&fhir_parser::Value> = match element {
        fhir_parser::Value::Array(items) => items.iter().map(|n| &n.value).collect(),
        e => vec![e],
    };
    (0..values.len().max(elements.len()))
        .flat_map(|i| {
            let value = values.get(i).copied();
            match (value, elements.get(i)) {
                // Only an object is an element; anything else carries no extensions.
                (None | Some(fhir_parser::Value::Null), Some(fhir_parser::Value::Object(f))) => {
                    let element = Arc::new(element_fields(f, child, child_element));
                    vec![Value::Primitive(Box::new(Value::Null), element)]
                }
                (
                    Some(
                        v @ (fhir_parser::Value::Str(_)
                        | fhir_parser::Value::Bool(_)
                        | fhir_parser::Value::Integer(_)
                        | fhir_parser::Value::Decimal(_)),
                    ),
                    Some(fhir_parser::Value::Object(f)),
                ) => {
                    let element = Arc::new(element_fields(f, child, child_element));
                    convert(v, child, child_element)
                        .into_iter()
                        .map(|v| Value::Primitive(Box::new(v), Arc::clone(&element)))
                        .collect()
                }
                (Some(v), _) => convert(v, child, child_element),
                (None, _) => vec![],
            }
        })
        .collect()
}

/// The element children (`id`, `extension`) of a primitive, typed against
/// the primitive type's own definition (`date.extension`).
fn element_fields(
    fields: &IndexMap<Arc<str>, fhir_parser::Node>,
    child: Option<&(Arc<str>, Arc<str>)>,
    child_element: &ChildElement<'_>,
) -> IndexMap<Arc<str>, Collection> {
    object_fields(fields, child.map(|(_, ty)| &**ty), child_element)
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
