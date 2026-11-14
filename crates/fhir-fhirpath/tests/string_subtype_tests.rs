//! FHIR primitive types whose FHIRPath type is `System.String` (R5
//! `profiles-types.json`): a String value does not record which of them it is,
//! so `is`, `as` and `ofType` with one of these names are an error, never a
//! guessed `false` (a false `dom-3` came from `ofType(canonical)`).
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use fhir_fhirpath::{Value, evaluate, resource_to_value};
use fhir_parser::parse_json;

type TestResult = Result<(), Box<dyn Error>>;

fn types_bundle() -> Result<[Value; 1], Box<dyn Error>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5/profiles-types.json");
    Ok([resource_to_value(&parse_json(&std::fs::read_to_string(
        path,
    )?)?)])
}

/// The R5 primitive types whose `value` element is `System.String`.
fn string_subtypes() -> Result<Vec<String>, Box<dyn Error>> {
    let bundle = types_bundle()?;
    let paths = evaluate(
        "entry.resource.where(kind = 'primitive-type').snapshot.element\
            .where(path.endsWith('.value') \
                and type.code = 'http://hl7.org/fhirpath/System.String').path",
        &bundle,
    )?;
    let names: Vec<String> = paths
        .iter()
        .filter_map(Value::as_string)
        .filter_map(|p| p.strip_suffix(".value").map(str::to_owned))
        .collect();
    assert!(names.iter().any(|n| n == "canonical"), "{names:?}");
    Ok(names)
}

#[test]
fn string_subtypes_on_a_string_are_an_error() -> TestResult {
    let input = [Value::String(Arc::from("#q"))];
    for name in string_subtypes()?.iter().filter(|n| *n != "string") {
        for expression in [
            format!("ofType({name})"),
            format!("is({name})"),
            format!("as({name})"),
            format!("$this is {name}"),
            format!("$this as {name}"),
        ] {
            assert!(evaluate(&expression, &input).is_err(), "{expression}");
        }
    }
    Ok(())
}

#[test]
fn other_primitive_types_on_a_string_are_answered() -> TestResult {
    // The reverse direction: the evaluator's list holds no other R5 primitive.
    let string_subtypes = string_subtypes()?;
    let primitives = evaluate(
        "entry.resource.where(kind = 'primitive-type').name",
        &types_bundle()?,
    )?;
    assert!(primitives.len() > string_subtypes.len(), "{primitives:?}");
    let input = [Value::String(Arc::from("#q"))];
    for name in primitives.iter().filter_map(Value::as_string) {
        if !string_subtypes.iter().any(|s| s.as_str() == &*name) {
            assert_eq!(evaluate(&format!("ofType({name})"), &input)?, [], "{name}");
        }
    }
    Ok(())
}

#[test]
fn string_and_other_types_stay_answered() -> TestResult {
    let input = [Value::String(Arc::from("#q"))];
    assert_eq!(evaluate("ofType(string)", &input)?, input);
    assert_eq!(evaluate("ofType(integer)", &input)?, []);
    // A non-String value is certainly not a canonical.
    let object = [resource_to_value(&parse_json(
        r#"{"resourceType":"Patient"}"#,
    )?)];
    assert_eq!(evaluate("ofType(canonical)", &object)?, []);
    assert_eq!(
        evaluate("$this is uri", &[Value::Integer(1)])?,
        [Value::Bool(false)]
    );
    Ok(())
}
