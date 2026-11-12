//! FHIR data model mapping (R5 fhirpath.html §2.1.9.1): typed primitives, the
//! primitive split representation, `hasValue()`, `children()` and nulls.
use std::sync::Arc;

use fhir_fhirpath::{Collection, Value, evaluate, resource_to_typed_value, resource_to_value};
use fhir_parser::{parse_json, parse_xml};

/// A tiny stand-in for `Registry::child_element`, enough for these resources.
fn child_element(parent: &str, name: &str) -> Option<(Arc<str>, Arc<str>)> {
    let ty = match (parent, name) {
        ("Patient", "birthDate") => "date",
        ("Patient", "contained") => "Resource",
        ("Observation", "effectiveDateTime") => "dateTime",
        ("Observation", "issued") => "instant",
        ("Observation", "valueTime") => "time",
        ("Patient" | "Observation", "extension") | ("date" | "dateTime", "extension") => {
            "Extension"
        }
        ("Extension", "valueDateTime") => "dateTime",
        _ => return None,
    };
    Some((Arc::from(ty), Arc::from(ty)))
}

fn typed(json: &str) -> Vec<Value> {
    vec![resource_to_typed_value(
        &parse_json(json).unwrap(),
        &child_element,
    )]
}

fn bools(result: &Collection) -> Vec<bool> {
    result
        .iter()
        .map(|v| match v {
            Value::Bool(b) => *b,
            other => panic!("not a Boolean: {other:?}"),
        })
        .collect()
}

#[test]
fn declared_temporal_types_become_temporal_values() {
    let ctx = typed(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"c"},
        "effectiveDateTime":"2020-01-01T10:00:00+02:00","issued":"2020-01-01T09:00:00Z",
        "valueTime":"10:30:00"}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert!(matches!(
        one("effectiveDateTime").as_slice(),
        [Value::DateTime(_)]
    ));
    assert!(matches!(one("issued").as_slice(), [Value::DateTime(_)]));
    assert!(matches!(one("valueTime").as_slice(), [Value::Time(_)]));
    // 10:00+02:00 is 08:00Z: before 09:00Z, though "10" > "09" as text.
    assert_eq!(bools(&one("effectiveDateTime < issued")), [true]);
    assert_eq!(bools(&one("valueTime > @T10:00")), [true]);
    // Different precisions: unknown, so empty (N1 §6.2).
    assert!(one("effectiveDateTime = @2020-01-01T10:00+02:00").is_empty());
}

#[test]
fn date_against_zoned_date_time_is_empty() {
    // User decision (PLAN 11.7): a Date against a zoned DateTime is empty.
    let ctx = typed(r#"{"resourceType":"Patient","birthDate":"2020-01-01"}"#);
    assert!(
        evaluate("birthDate < @2020-06-01T00:00:00Z", &ctx)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        bools(&evaluate("birthDate < @2020-06-01", &ctx).unwrap()),
        [true]
    );
}

#[test]
fn contained_resources_are_typed_by_their_resource_type() {
    let ctx = typed(
        r#"{"resourceType":"Patient","contained":[{"resourceType":"Observation",
        "status":"final","code":{"text":"c"},"issued":"2020-01-01T09:00:00Z"}]}"#,
    );
    let issued = evaluate("contained.issued", &ctx).unwrap();
    assert!(
        matches!(issued.as_slice(), [Value::DateTime(_)]),
        "{issued:?}"
    );
}

#[test]
fn untyped_values_keep_strings() {
    let json = r#"{"resourceType":"Patient","birthDate":"2020-01-01"}"#;
    let ctx = vec![resource_to_value(&parse_json(json).unwrap())];
    let birth_date = evaluate("birthDate", &ctx).unwrap();
    assert!(matches!(birth_date.as_slice(), [Value::String(_)]));
}

#[test]
fn xml_temporals_are_typed_too() {
    let xml = r#"<Patient xmlns="http://hl7.org/fhir"><birthDate value="1974-12-25"/></Patient>"#;
    let ctx = vec![resource_to_typed_value(
        &parse_xml(xml).unwrap(),
        &child_element,
    )];
    assert_eq!(
        bools(&evaluate("birthDate = @1974-12-25", &ctx).unwrap()),
        [true]
    );
}
