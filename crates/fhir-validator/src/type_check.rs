use std::sync::Arc;

use fhir_definitions::StructureDefinition;
use fhir_parser::{Resource, Span, Value};

use crate::diagnostic::Diagnostic;
use crate::nested::resolve;

/// Validate primitive type constraints for each resource field.
///
/// Emits `TYPE_001` through `TYPE_005` for format and type mismatches.
pub fn validate(resource: &Resource, sd: &StructureDefinition, diagnostics: &mut Vec<Diagnostic>) {
    let resource_type = resource.resource_type.as_ref();

    for (field_name, node) in &resource.fields {
        let path = format!("{resource_type}.{field_name}");

        let Some((elem, resolved)) = resolve(sd, &path, field_name) else {
            continue;
        };
        // A choice element is checked against the type its suffix selects.
        let chosen: Vec<Arc<str>> = if elem.path.ends_with("[x]") {
            resolved.into_iter().collect()
        } else {
            elem.types.clone()
        };

        for fhir_type in &chosen {
            check_type(
                &node.value,
                fhir_type.as_ref(),
                &path,
                node.span,
                diagnostics,
            );
        }
    }
}

pub(crate) fn check_type(
    value: &Value,
    fhir_type: &str,
    path: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Recurse into arrays.
    if let Value::Array(items) = value {
        for (i, item) in items.iter().enumerate() {
            let item_path = format!("{path}[{i}]");
            check_type(&item.value, fhir_type, &item_path, item.span, diagnostics);
        }
        return;
    }

    let expected = |what: &str| ("TYPE_001", format!("Expected {what} at '{path}'"));
    let string_format = |is_valid: fn(&str) -> bool| match value {
        Value::Str(s) if is_valid(s) => None,
        Value::Str(s) => Some((
            "TYPE_003",
            format!("Invalid {fhir_type} format at '{path}': '{s}'"),
        )),
        _ => Some(expected(&format!("{fhir_type} string"))),
    };

    let error: Option<(&str, String)> = match fhir_type {
        "boolean" if !matches!(value, Value::Bool(_)) => Some(expected("boolean")),
        "integer" if !matches!(value, Value::Integer(_)) => Some(expected("integer")),
        "unsignedInt" => match value {
            Value::Integer(n) if *n < 0 => Some((
                "TYPE_002",
                format!("Expected non-negative integer at '{path}', got {n}"),
            )),
            Value::Integer(_) => None,
            _ => Some(expected("integer")),
        },
        "positiveInt" => match value {
            Value::Integer(n) if *n <= 0 => Some((
                "TYPE_002",
                format!("Expected positive integer at '{path}', got {n}"),
            )),
            Value::Integer(_) => None,
            _ => Some(expected("integer")),
        },
        "decimal" if !matches!(value, Value::Decimal(_) | Value::Integer(_)) => {
            Some(expected("decimal number"))
        }
        "date" => string_format(is_valid_date),
        "dateTime" | "instant" => string_format(is_valid_datetime),
        "time" => string_format(is_valid_time),
        // Official regex `[0]|[-+]?[1-9][0-9]*`, within i64 range.
        "integer64" => string_format(|s| {
            s.parse::<i64>().is_ok()
                && (s == "0" || !s.trim_start_matches(['+', '-']).starts_with('0'))
        }),
        "code" => match value {
            Value::Str(s) if s.chars().any(char::is_whitespace) => Some((
                "TYPE_004",
                format!("Code at '{path}' must not contain whitespace: '{s}'"),
            )),
            Value::Str(_) => None,
            _ => Some(expected("code string")),
        },
        "id" => match value {
            Value::Str(s) if !is_valid_id(s) => {
                Some(("TYPE_005", format!("Invalid id format at '{path}': '{s}'")))
            }
            Value::Str(_) => None,
            _ => Some(expected("id string")),
        },
        "string" | "markdown" | "base64Binary" | "xhtml" | "uri" | "url" | "canonical" | "oid"
        | "uuid"
            if !matches!(value, Value::Str(_)) =>
        {
            Some(expected("string"))
        }
        // Complex and unknown types — skip deep validation here.
        _ => None,
    };

    if let Some((code, message)) = error {
        diagnostics.push(Diagnostic::error(code, message, path.to_owned()).with_location(span));
    }
}

/// FHIR date: `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`.
fn is_valid_date(s: &str) -> bool {
    // Non-ASCII input would make the byte slicing below panic.
    if !s.is_ascii() {
        return false;
    }
    let digits = |r: std::ops::Range<usize>| s[r].bytes().all(|c| c.is_ascii_digit());
    let in_range = |r: std::ops::Range<usize>, max: u8| {
        digits(r.clone()) && s[r].parse::<u8>().is_ok_and(|n| (1..=max).contains(&n))
    };
    match s.len() {
        4 => digits(0..4),
        7 => digits(0..4) && &s[4..5] == "-" && in_range(5..7, 12),
        10 => {
            digits(0..4)
                && &s[4..5] == "-"
                && &s[7..8] == "-"
                && in_range(5..7, 12)
                && in_range(8..10, 31)
        }
        _ => false,
    }
}

/// FHIR dateTime: date with optional `T` followed by a valid time component.
fn is_valid_datetime(s: &str) -> bool {
    if is_valid_date(s) {
        return true;
    }
    if let Some(t_pos) = s.find('T') {
        return is_valid_date(&s[..t_pos]) && is_valid_time(&s[t_pos + 1..]);
    }
    false
}

/// `hh:mm:ss[...]`, shared by `time` and the time part of `dateTime`.
/// Only the leading `hh:mm:ss` is checked (fraction / timezone are not validated).
fn is_valid_time(s: &str) -> bool {
    // Non-ASCII input would make the byte slicing below panic.
    if !s.is_ascii() || s.len() < 8 {
        return false;
    }
    let b = s.as_bytes();
    if b[2] != b':' || b[5] != b':' {
        return false;
    }
    let part = |r: std::ops::Range<usize>, max: u8| {
        s[r.clone()].bytes().all(|c| c.is_ascii_digit())
            && s[r].parse::<u8>().is_ok_and(|n| n <= max)
    };
    part(0..2, 23) && part(3..5, 59) && part(6..8, 59)
}

/// FHIR id: `[A-Za-z0-9\-\.]{1,64}`.
fn is_valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use fhir_definitions::{ElementDefinition, StructureDefinition};
    use fhir_parser::{Node, Span};
    use indexmap::IndexMap;

    fn make_elem(path: &str, type_code: &str) -> ElementDefinition {
        ElementDefinition {
            path: Arc::from(path),
            min: 0,
            max: Arc::from("1"),
            types: vec![Arc::from(type_code)],
            constraints: vec![],
            binding: None,
        }
    }

    fn make_sd(resource_type: &str, field: &str, type_code: &str) -> StructureDefinition {
        let path = format!("{resource_type}.{field}");
        let snapshot = vec![make_elem(resource_type, ""), make_elem(&path, type_code)];
        StructureDefinition {
            url: Arc::from(
                format!("http://hl7.org/fhir/StructureDefinition/{resource_type}").as_str(),
            ),
            name: Arc::from(resource_type),
            kind: Arc::from("resource"),
            is_abstract: false,
            base_definition: None,
            snapshot,
        }
    }

    fn make_resource(resource_type: &str, field: &str, value: Value) -> fhir_parser::Resource {
        let mut map: IndexMap<Arc<str>, Node> = IndexMap::new();
        map.insert(
            Arc::from(field),
            Node {
                value,
                span: Span::default(),
            },
        );
        fhir_parser::Resource {
            resource_type: Arc::from(resource_type),
            id: None,
            fields: map,
        }
    }

    #[test]
    fn valid_boolean_produces_no_error() {
        let sd = make_sd("Patient", "active", "boolean");
        let resource = make_resource("Patient", "active", Value::Bool(true));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn string_for_boolean_produces_type_001() {
        let sd = make_sd("Patient", "active", "boolean");
        let resource = make_resource("Patient", "active", Value::Str(Arc::from("true")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_001");
    }

    #[test]
    fn valid_date_produces_no_error() {
        let sd = make_sd("Patient", "birthDate", "date");
        for s in &["1990", "1990-01", "1990-01-15"] {
            let resource = make_resource("Patient", "birthDate", Value::Str(Arc::from(*s)));
            let mut d = vec![];
            validate(&resource, &sd, &mut d);
            assert!(d.is_empty(), "Expected no error for date '{s}'");
        }
    }

    #[test]
    fn invalid_date_produces_type_003() {
        let sd = make_sd("Patient", "birthDate", "date");
        let resource = make_resource("Patient", "birthDate", Value::Str(Arc::from("not-a-date")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_003");
    }

    #[test]
    fn integer_beyond_i64_produces_type_001() {
        for type_code in ["integer", "unsignedInt", "positiveInt"] {
            let sd = make_sd("Obs", "n", type_code);
            let source = r#"{"resourceType": "Obs", "n": 12345678901234567890}"#;
            let resource = fhir_parser::parse_json(source).unwrap();
            let mut d = vec![];
            validate(&resource, &sd, &mut d);
            assert_eq!(d.len(), 1, "{type_code}: {d:?}");
            assert_eq!(d[0].code.as_ref(), "TYPE_001");
        }
    }

    #[test]
    fn integer64_requires_a_digit_string() {
        let sd = make_sd("Obs", "big", "integer64");
        for (value, expected) in [
            (Value::Str(Arc::from("-123")), None),
            (Value::Str(Arc::from("12x")), Some("TYPE_003")),
            (Value::Str(Arc::from("-")), Some("TYPE_003")),
            (Value::Str(Arc::from("+1")), None),
            (Value::Str(Arc::from("0")), None),
            (Value::Str(Arc::from("01")), Some("TYPE_003")),
            (Value::Str(Arc::from("-0")), Some("TYPE_003")),
            (
                Value::Str(Arc::from("99999999999999999999")),
                Some("TYPE_003"),
            ),
            (Value::Integer(5), Some("TYPE_001")),
        ] {
            let resource = make_resource("Obs", "big", value);
            let mut d = vec![];
            validate(&resource, &sd, &mut d);
            assert_eq!(d.first().map(|x| x.code.as_ref()), expected, "{d:?}");
        }
    }

    #[test]
    fn code_with_whitespace_produces_type_004() {
        let sd = make_sd("Obs", "status", "code");
        let resource = make_resource("Obs", "status", Value::Str(Arc::from("fin al")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_004");
    }

    #[test]
    fn valid_code_produces_no_error() {
        let sd = make_sd("Obs", "status", "code");
        let resource = make_resource("Obs", "status", Value::Str(Arc::from("final")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert!(d.is_empty());
    }

    #[test]
    fn negative_unsigned_int_produces_type_002() {
        let sd = make_sd("Claim", "priority", "unsignedInt");
        let resource = make_resource("Claim", "priority", Value::Integer(-1));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_002");
    }

    #[test]
    fn zero_positive_int_produces_type_002() {
        let sd = make_sd("Claim", "priority", "positiveInt");
        let resource = make_resource("Claim", "priority", Value::Integer(0));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_002");
    }

    #[test]
    fn valid_id_produces_no_error() {
        let sd = make_sd("Patient", "id2", "id");
        for s in &["abc", "ABC-123", "a.b.c", "x-1"] {
            let resource = make_resource("Patient", "id2", Value::Str(Arc::from(*s)));
            let mut d = vec![];
            validate(&resource, &sd, &mut d);
            assert!(d.is_empty(), "Expected no error for id '{s}'");
        }
    }

    #[test]
    fn id_with_invalid_chars_produces_type_005() {
        let sd = make_sd("Patient", "id2", "id");
        let resource = make_resource("Patient", "id2", Value::Str(Arc::from("hello world")));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_005");
    }

    #[test]
    fn array_elements_are_each_checked() {
        let sd = make_sd("Patient", "birthDate", "date");
        let items = vec![
            Node {
                value: Value::Str(Arc::from("1990-01-01")),
                span: Span::default(),
            },
            Node {
                value: Value::Str(Arc::from("not-a-date")),
                span: Span::default(),
            },
        ];
        let resource = make_resource("Patient", "birthDate", Value::Array(items));
        let mut d = vec![];
        validate(&resource, &sd, &mut d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_003");
    }

    #[test]
    fn non_ascii_date_and_time_do_not_panic() {
        for s in [
            "abcé-0",
            "２０２４-01-01",
            "é024-01-01",
            "é:00:00",
            "2024-01-01Té:00:00",
        ] {
            assert!(!is_valid_date(s));
            assert!(!is_valid_datetime(s));
            assert!(!is_valid_time(s));
        }
    }

    #[test]
    fn valid_time_and_datetime() {
        assert!(is_valid_time("12:30:59"));
        assert!(!is_valid_time("24:00:00"));
        assert!(is_valid_datetime("2024-01-01T12:30:59Z"));
        assert!(!is_valid_datetime("2024-01-01T+1:30:59Z"));
    }
}
