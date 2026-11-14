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
        ("Patient", "multipleBirthInteger") => "integer",
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
    // Each value carries its FHIR type; its value is the temporal.
    let value = |expr: &str| match one(expr).as_slice() {
        [Value::Primitive(value, _, Some(ty))] => ((**value).clone(), ty.to_string()),
        other => panic!("not a typed primitive: {other:?}"),
    };
    assert!(matches!(value("effectiveDateTime"), (Value::DateTime(_), t) if t == "dateTime"));
    assert!(matches!(value("issued"), (Value::DateTime(_), t) if t == "instant"));
    assert!(matches!(value("valueTime"), (Value::Time(_), t) if t == "time"));
    // 10:00+02:00 is 08:00Z: before 09:00Z, though "10" > "09" as text.
    assert_eq!(bools(&one("effectiveDateTime < issued")), [true]);
    assert_eq!(bools(&one("valueTime > @T10:00")), [true]);
    // Different precisions: unknown, so empty (N1 §6.2).
    assert!(one("effectiveDateTime = @2020-01-01T10:00+02:00").is_empty());
}

#[test]
fn numeric_primitives_carry_their_fhir_type() {
    let ctx = typed(r#"{"resourceType":"Patient","multipleBirthInteger":2}"#);
    let items = evaluate("multipleBirthInteger", &ctx).unwrap();
    assert!(
        matches!(
            items.as_slice(),
            [Value::Primitive(value, _, Some(ty))]
                if matches!(**value, Value::Integer(2)) && &**ty == "integer"
        ),
        "{items:?}"
    );
    assert_eq!(
        bools(&evaluate("multipleBirthInteger + 1 = 3", &ctx).unwrap()),
        [true]
    );
}

#[test]
fn string_functions_accept_typed_string_fields() {
    let ctx = typed(r#"{"resourceType":"Patient","gender":"male"}"#);
    let upper = evaluate("gender.upper()", &ctx).unwrap();
    assert!(
        matches!(upper.as_slice(), [Value::String(s)] if &**s == "MALE"),
        "{upper:?}"
    );
    let length = evaluate("gender.length()", &ctx).unwrap();
    assert!(
        matches!(length.as_slice(), [Value::Integer(4)]),
        "{length:?}"
    );
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
        matches!(
            issued.as_slice(),
            [Value::Primitive(value, _, Some(ty))]
                if matches!(**value, Value::DateTime(_)) && &**ty == "instant"
        ),
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
        <birthDate value="1974-12-25">
          <extension url="u"><valueString value="y"/></extension>
        </birthDate>
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
    // value ... Otherwise, the return value is empty." Empty for a complex
    // value; false for no value, as the R5 constraints and official suite expect.
    let ctx = typed(
        r#"{"resourceType":"Patient","birthDate":"1970-01-01",
        "_birthDate":{"extension":[{"url":"u","valueString":"x"}]},
        "name":[{"given":["a",null],
        "_given":[null,{"extension":[{"url":"u","valueString":"x"}]}]}]}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("name.given[0].hasValue()")), [true]);
    assert_eq!(bools(&one("birthDate.hasValue()")), [true]);
    // A primitive with extensions and no value, several items, no item.
    assert_eq!(bools(&one("name.given[1].hasValue()")), [false]);
    assert_eq!(bools(&one("name.given.hasValue()")), [false]);
    assert_eq!(bools(&one("{}.hasValue()")), [false]);
    // A complex value.
    assert!(one("name.first().hasValue()").is_empty());
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

#[test]
fn an_integer_with_extensions_still_indexes() {
    let ctx = untyped(
        r#"{"resourceType":"Patient","multipleBirthInteger":2,
        "_multipleBirthInteger":{"extension":[{"url":"u","valueString":"x"}]}}"#,
    );
    let third = evaluate("(1 | 2 | 3)[multipleBirthInteger]", &ctx).unwrap();
    assert_eq!(third, vec![Value::Integer(3)]);
    let two = evaluate("(1 | 2 | 3).take(multipleBirthInteger).count() = 2", &ctx).unwrap();
    assert_eq!(bools(&two), [true]);
}

#[test]
fn split_arrays_of_different_lengths_pair_by_index() {
    // More `_given` entries than values, and more values than `_given` entries.
    let ctx = untyped(
        r#"{"resourceType":"Patient","name":[
        {"given":["a"],"_given":[null,{"extension":[{"url":"u","valueString":"x"}]}]},
        {"given":["b","c"],"_given":[{"id":"g1"}]}]}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("name[0].given.count() = 2")), [true]);
    assert_eq!(bools(&one("name[0].given[1].extension.exists()")), [true]);
    assert_eq!(bools(&one("name[1].given.count() = 2")), [true]);
    assert_eq!(bools(&one("name[1].given[0].id = 'g1'")), [true]);
    assert_eq!(bools(&one("name[1].given[1] = 'c'")), [true]);
}

#[test]
fn a_non_object_split_element_carries_no_extensions() {
    let ctx = untyped(r#"{"resourceType":"Patient","active":true,"_active":5,"_gender":7}"#);
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("active")), [true]);
    assert_eq!(bools(&one("active.extension.exists()")), [false]);
    assert_eq!(bools(&one("gender.exists()")), [false]);
}

#[test]
fn root_id_extensions_keep_the_id_value() {
    let ctx = untyped(
        r#"{"resourceType":"Patient","id":"p1",
        "_id":{"extension":[{"url":"u","valueString":"x"}]}}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("id = 'p1'")), [true]);
    assert_eq!(bools(&one("id.extension('u').exists()")), [true]);
}

#[test]
fn aggregates_see_the_value_of_primitives_with_extensions() {
    let ctx = untyped(
        r#"{"resourceType":"Patient","active":true,
        "_active":{"extension":[{"url":"u","valueString":"x"}]},
        "multipleBirthInteger":4,
        "_multipleBirthInteger":{"extension":[{"url":"u","valueString":"x"}]}}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("active.allTrue()")), [true]);
    assert_eq!(bools(&one("multipleBirthInteger.sum() = 4")), [true]);
    assert_eq!(bools(&one("multipleBirthInteger.avg() = 4.0")), [true]);
    // Math functions see the value (primitive_values, owned path).
    assert_eq!(bools(&one("multipleBirthInteger.abs() = 4")), [true]);
}

#[test]
fn aggregates_skip_primitives_without_a_value() {
    // A primitive with only extensions has no value: it is not an input item
    // for aggregates (official suite, primitivesWithoutValue).
    let ctx = untyped(
        r#"{"resourceType":"Patient","x":[true,null],"n":[2,null],
        "_x":[null,{"extension":[{"url":"u","valueString":"a"}]}],
        "_n":[null,{"extension":[{"url":"u","valueString":"a"}]}]}"#,
    );
    let one = |expr: &str| evaluate(expr, &ctx).unwrap();
    assert_eq!(bools(&one("x.allTrue()")), [true]);
    assert_eq!(bools(&one("x.anyFalse()")), [false]);
    assert_eq!(one("n.sum()"), vec![Value::Integer(2)]);
    assert_eq!(one("n.avg()"), vec![Value::Decimal(2.0)]);
    assert_eq!(one("n.min()"), vec![Value::Integer(2)]);
    assert_eq!(one("n.max()"), vec![Value::Integer(2)]);
}

#[test]
fn get_value_gives_only_primitive_values() {
    let ctx = untyped(r#"{"resourceType":"Patient","name":[{"family":"F"}]}"#);
    assert!(evaluate("name.getValue()", &ctx).unwrap().is_empty());
    let family = evaluate("name.family.getValue()", &ctx).unwrap();
    assert_eq!(family, vec![Value::String(Arc::from("F"))]);
}
