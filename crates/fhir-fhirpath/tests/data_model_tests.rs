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
        ("Patient", "gender") => "code",
        ("Patient", "name") => "HumanName",
        ("HumanName", "given") => "string",
        ("Extension", "valueString") => "string",
        ("code" | "string", "extension") => "Extension",
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

fn untyped(json: &str) -> Vec<Value> {
    vec![resource_to_value(&parse_json(json).unwrap())]
}

const BIRTH_DATE_EXTENSION: &str = r#"{"resourceType":"Patient","birthDate":"1970-01-01",
    "_birthDate":{"id":"b1","extension":[{"url":"u","valueString":"x"}]}}"#;

#[test]
fn json_primitive_extensions_are_found_on_the_primitive() {
    for ctx in [untyped(BIRTH_DATE_EXTENSION), typed(BIRTH_DATE_EXTENSION)] {
        let one = |expr: &str| evaluate(expr, &ctx).unwrap();
        assert_eq!(bools(&one("birthDate.extension.exists()")), [true]);
        assert_eq!(bools(&one("birthDate.extension('u').exists()")), [true]);
        assert_eq!(bools(&one("birthDate.id = 'b1'")), [true]);
        // The split representation is not visible (R5 fhirpath.html §2.1.9.1).
        assert_eq!(bools(&one("_birthDate.exists()")), [false]);
        assert_eq!(bools(&one("children().count() = 1")), [true]);
        // The value still behaves as the primitive.
        assert_eq!(bools(&one("birthDate.toString() = '1970-01-01'")), [true]);
        assert_eq!(bools(&one("birthDate.toString().length() = 10")), [true]);
    }
    let ctx = typed(BIRTH_DATE_EXTENSION);
    assert_eq!(bools(&evaluate("birthDate < @1971", &ctx).unwrap()), [true]);
    assert_eq!(
        bools(&evaluate("birthDate.extension('u').valueString = 'x'", &ctx).unwrap()),
        [true]
    );
}

#[test]
fn repeating_primitive_extensions_pair_by_index() {
    let ctx = untyped(
        r#"{"resourceType":"Patient","name":[{"given":["a",null,"c"],
        "_given":[null,{"extension":[{"url":"u","valueString":"x"}]},null]}]}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("name.given.count() = 3")), [true]);
    assert_eq!(bools(&one("name.given[1].extension.exists()")), [true]);
    assert_eq!(bools(&one("name.given[2] = 'c'")), [true]);
    assert_eq!(bools(&one("name.given[0].extension.exists()")), [false]);
    // A primitive with extensions but no value compares as empty.
    assert!(one("name.given[1] = 'b'").is_empty());
    assert!(one("name.given[1] < 'x'").is_empty());
    assert!(one("name.given[1].length()").is_empty());
}

#[test]
fn extensions_without_a_value_still_make_the_element_present() {
    let ctx = untyped(
        r#"{"resourceType":"Patient",
        "_birthDate":{"extension":[{"url":"u","valueString":"x"}]}}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("birthDate.exists()")), [true]);
    assert_eq!(bools(&one("birthDate.extension('u').exists()")), [true]);
    assert!(one("birthDate = '1970'").is_empty());
}

#[test]
fn xml_primitives_with_extensions_behave_as_primitives() {
    let xml = r#"<Patient xmlns="http://hl7.org/fhir">
        <gender value="male"><extension url="u"><valueString value="x"/></extension></gender>
        <birthDate value="1974-12-25"><extension url="u"><valueString value="y"/></extension></birthDate>
        </Patient>"#;
    let ctx = vec![resource_to_typed_value(
        &parse_xml(xml).unwrap(),
        &child_element,
    )];
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("gender = 'male'")), [true]);
    assert_eq!(bools(&one("gender.extension('u').exists()")), [true]);
    assert_eq!(bools(&one("birthDate = @1974-12-25")), [true]);
    assert_eq!(bools(&one("birthDate.extension.exists()")), [true]);
}

#[test]
fn complex_value_never_equals_a_primitive() {
    // N1 §6.1.1; testHasCodingValue3 in the official suite.
    let ctx = untyped(r#"{"resourceType":"Patient","name":[{"family":"Peter"}]}"#);
    assert_eq!(
        bools(&evaluate("name.first() = 'Peter'", &ctx).unwrap()),
        [false]
    );
    assert_eq!(
        bools(&evaluate("name.first() != 'Peter'", &ctx).unwrap()),
        [true]
    );
}

#[test]
fn has_value_is_true_only_for_one_primitive_with_a_value() {
    // R5 fhirpath.html hasValue(): "Returns true if the input collection
    // contains a single value which is a FHIR primitive, and it has a primitive
    // value ... Otherwise, the return value is empty."
    let ctx = typed(
        r#"{"resourceType":"Patient","birthDate":"1970-01-01",
        "_birthDate":{"extension":[{"url":"u","valueString":"x"}]},
        "name":[{"given":["a",null],
        "_given":[null,{"extension":[{"url":"u","valueString":"x"}]}]}]}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("name.given[0].hasValue()")), [true]);
    assert_eq!(bools(&one("birthDate.hasValue()")), [true]);
    // A primitive with extensions and no value.
    assert!(one("name.given[1].hasValue()").is_empty());
    // A complex value, several items, no item.
    assert!(one("name.first().hasValue()").is_empty());
    assert!(one("name.given.hasValue()").is_empty());
    assert!(one("{}.hasValue()").is_empty());
    assert!(one("birthDate.extension.hasValue()").is_empty());
}

#[test]
fn resource_type_is_not_a_child() {
    // resourceType names the type in JSON; it is not an element.
    let ctx = untyped(
        r#"{"resourceType":"Patient","active":true,
        "contained":[{"resourceType":"Basic","id":"b"}]}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("children().count() = 2")), [true]);
    // contained, its id, and active: no resourceType at either level.
    assert_eq!(bools(&one("descendants().count() = 3")), [true]);
    assert_eq!(
        bools(&one("descendants().where($this = 'Basic').empty()")),
        [true]
    );
    // The type name still leads a path.
    assert_eq!(bools(&one("Patient.active")), [true]);
}

#[test]
fn null_placeholders_are_not_items() {
    let ctx = untyped(
        r#"{"resourceType":"Patient","name":[{"given":["a",null,"c"]}],
        "gender":null}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("name.given.count() = 2")), [true]);
    assert_eq!(bools(&one("name.given[1] = 'c'")), [true]);
    assert_eq!(bools(&one("gender.exists()")), [false]);
    assert_eq!(bools(&one("name.children().count() = 2")), [true]);
}
