//! FHIR environment variables (R5 fhirpath.html §2.1.9.1.5): `%resource`,
//! `%rootResource`, the FHIRPath `%context`, and the spec constants.
use std::error::Error;
use std::sync::Arc;

use fhir_fhirpath::{
    Collection, Environment, EvalError, Value, evaluate, evaluate_parsed_with_environment, parse,
    resource_to_value,
};
use fhir_parser::parse_json;

type TestResult = Result<(), Box<dyn Error>>;

fn resource(json: &str) -> Result<Value, Box<dyn Error>> {
    Ok(resource_to_value(&parse_json(json)?))
}

fn patient() -> Result<[Value; 1], Box<dyn Error>> {
    Ok([resource(
        r#"{"resourceType":"Patient","id":"p1","active":true}"#,
    )?])
}

fn strings(result: &Collection) -> Vec<String> {
    result
        .iter()
        .map(|v| v.as_string().map(|s| s.to_string()).unwrap_or_default())
        .collect()
}

fn container() -> Result<Value, Box<dyn Error>> {
    resource(
        r##"{"resourceType":"Observation","id":"obs","status":"final",
        "code":{"text":"x"},
        "contained":[{"resourceType":"Patient","id":"pat"}],
        "subject":{"reference":"#pat"}}"##,
    )
}

fn child(value: &Value, name: &str) -> Result<Value, Box<dyn Error>> {
    let Value::Object(map) = value else {
        return Err("not an object".into());
    };
    map.get(name)
        .and_then(|values| values.first())
        .cloned()
        .ok_or_else(|| format!("no {name}").into())
}

fn eval_env(
    expression: &str,
    context: &[Value],
    environment: &Environment<'_>,
) -> Result<Collection, EvalError> {
    evaluate_parsed_with_environment(&parse(expression)?, context, &|_, _| false, environment)
}

#[test]
fn resource_defaults_to_a_resource_input() -> TestResult {
    assert_eq!(strings(&evaluate("%resource.id", &patient()?)?), ["p1"]);
    Ok(())
}

#[test]
fn root_resource_defaults_to_a_resource_input() -> TestResult {
    assert_eq!(strings(&evaluate("%rootResource.id", &patient()?)?), ["p1"]);
    Ok(())
}

#[test]
fn root_resource_is_the_container_when_set() -> TestResult {
    let root = container()?;
    let contained = child(&root, "contained")?;
    let environment = Environment {
        resource: Some(&contained),
        root_resource: Some(&root),
        ..Environment::default()
    };
    let input = [contained.clone()];
    let resource_id = eval_env("%resource.id", &input, &environment)?;
    let root_id = eval_env("%rootResource.id", &input, &environment)?;
    assert_eq!(strings(&resource_id), ["pat"]);
    assert_eq!(strings(&root_id), ["obs"]);
    Ok(())
}

#[test]
fn root_resource_defaults_to_the_set_resource() -> TestResult {
    let root = container()?;
    let environment = Environment {
        resource: Some(&root),
        root_resource: None,
        ..Environment::default()
    };
    let subject = [child(&root, "subject")?];
    let result = eval_env("%rootResource.id", &subject, &environment)?;
    assert_eq!(strings(&result), ["obs"]);
    // On a nested node, %resource is the set resource, not inferred from the input.
    let contained = [child(&root, "contained")?];
    let result = eval_env("%resource.id", &contained, &environment)?;
    assert_eq!(strings(&result), ["obs"]);
    Ok(())
}

#[test]
fn ref_1_resolves_a_contained_reference_on_a_nested_node() -> TestResult {
    let root = container()?;
    let environment = Environment {
        resource: Some(&root),
        root_resource: Some(&root),
        ..Environment::default()
    };
    let ref_1 = "reference.exists()  implies (reference.startsWith('#').not() or \
        (reference.substring(1).trace('url') in %rootResource.contained.id.trace('ids')) or \
        (reference='#' and %rootResource!=%resource))";
    let subject = [child(&root, "subject")?];
    let result = eval_env(ref_1, &subject, &environment)?;
    assert_eq!(result, vec![Value::Bool(true)]);
    Ok(())
}

#[test]
fn context_is_the_original_input() -> TestResult {
    let result = evaluate("active.select(%context.id)", &patient()?)?;
    assert_eq!(strings(&result), ["p1"]);
    Ok(())
}

#[test]
fn delimited_name_is_the_same_variable() -> TestResult {
    let patient = patient()?;
    assert_eq!(strings(&evaluate("%`resource`.id", &patient)?), ["p1"]);
    assert_eq!(strings(&evaluate("%'resource'.id", &patient)?), ["p1"]);
    Ok(())
}

#[test]
fn resource_is_unset_on_a_non_resource_input() -> TestResult {
    let input = [Value::String(Arc::from("x"))];
    assert!(evaluate("%resource", &input).is_err());
    assert!(evaluate("%rootResource", &input).is_err());
    // Two resources are not "the" resource either.
    let [one] = patient()?;
    assert!(evaluate("%resource", &[one.clone(), one]).is_err());
    Ok(())
}

#[test]
fn constants_are_the_spec_urls() -> TestResult {
    let cases = [
        ("%ucum", "http://unitsofmeasure.org"),
        ("%sct", "http://snomed.info/sct"),
        ("%loinc", "http://loinc.org"),
        (
            "%`vs-administrative-gender`",
            "http://hl7.org/fhir/ValueSet/administrative-gender",
        ),
        (
            "%`ext-patient-birthTime`",
            "http://hl7.org/fhir/StructureDefinition/patient-birthTime",
        ),
    ];
    for (expression, url) in cases {
        assert_eq!(strings(&evaluate(expression, &[])?), [url], "{expression}");
    }
    Ok(())
}

#[test]
fn unknown_variable_is_an_error() -> TestResult {
    let patient = patient()?;
    assert!(evaluate("%nope", &patient).is_err());
    assert!(evaluate("%`vs-`", &patient).is_err());
    assert!(evaluate("%`ext-`", &patient).is_err());
    Ok(())
}
