//! `resolve()` and `memberOf()` (R5 fhirpath.html §2.1.9.1.7 and §2.1.9.1.8).
use std::error::Error;
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, Environment, EvalError, MemberOf, Value, evaluate,
    evaluate_parsed_with_environment, parse, resource_to_typed_value, resource_to_value,
};
use fhir_parser::parse_json;

type TestResult = Result<(), Box<dyn Error>>;

fn resource(json: &str) -> Result<Value, Box<dyn Error>> {
    Ok(resource_to_value(&parse_json(json)?))
}

fn container() -> Result<Value, Box<dyn Error>> {
    resource(
        r##"{"resourceType":"Observation","id":"obs","status":"final",
        "code":{"coding":[{"system":"http://example.org/cs","code":"a"}]},
        "contained":[{"resourceType":"Patient","id":"pat"},
            {"resourceType":"Group","id":"grp","type":"person","membership":"enumerated",
             "member":[{"entity":{"reference":"#"}}]}],
        "subject":{"reference":"#pat"},
        "performer":[{"reference":"Patient/elsewhere"},{"reference":"#missing"}]}"##,
    )
}

fn eval_env(
    expression: &str,
    context: &[Value],
    environment: &Environment<'_>,
) -> Result<Collection, EvalError> {
    evaluate_parsed_with_environment(&parse(expression)?, context, &|_, _| false, environment)
}

fn bools(result: &Collection) -> Vec<bool> {
    result.iter().map(Value::is_truthy).collect()
}

#[test]
fn resolve_finds_a_contained_resource_from_a_reference() -> TestResult {
    let root = [container()?];
    assert_eq!(
        bools(&evaluate("subject.resolve() is Patient", &root)?),
        [true]
    );
    assert_eq!(
        bools(&evaluate("subject.reference.resolve().id = 'pat'", &root)?),
        [true]
    );
    Ok(())
}

#[test]
fn resolve_errors_on_references_it_cannot_follow() -> TestResult {
    // Empty would let an invariant fail on a reference it merely cannot see.
    let root = [container()?];
    for expression in [
        "performer[0].resolve()",
        "performer[1].resolve()",
        "code.coding.system.resolve()",
        "code.resolve()",
    ] {
        assert!(
            matches!(evaluate(expression, &root), Err(EvalError::Type(_))),
            "{expression}",
        );
    }
    assert_eq!(evaluate("{}.resolve()", &root)?, vec![]);
    Ok(())
}

#[test]
fn resolve_hash_names_the_root_resource() -> TestResult {
    let root = container()?;
    let environment = Environment {
        root_resource: Some(&root),
        ..Environment::default()
    };
    let root = [root.clone()];
    let groups = evaluate("contained.ofType(Group)", &root)?;
    let result = eval_env("member.entity.resolve().id", &groups, &environment)?;
    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].as_string().map(|s| s.to_string()).as_deref(),
        Some("obs")
    );
    Ok(())
}

#[test]
fn resolve_without_a_root_resource_is_an_error() -> TestResult {
    // Neither the environment nor the input names a resource.
    let root = [container()?];
    let subject = evaluate("subject", &root)?;
    assert!(matches!(
        evaluate("resolve()", &subject),
        Err(EvalError::Type(_))
    ));
    Ok(())
}

const RESOURCE_TYPES: &str = "http://hl7.org/fhir/ValueSet/resource-types";

fn member_of(url: &str, system: Option<&str>, code: &str) -> Option<bool> {
    let known =
        url == RESOURCE_TYPES && matches!(system, None | Some("http://hl7.org/fhir/fhir-types"));
    known.then_some(matches!(code, "Patient" | "Group"))
}

/// Evaluate on `json` typed by the R5 definitions, so `code` elements carry their type.
fn eval_member_of(expression: &str, json: &str) -> Result<Collection, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    let registry = Registry::from_definitions_dir(dir)?;
    let typed = resource_to_typed_value(&parse_json(json)?, &|parent, name| {
        registry.child_element(parent, name)
    });
    let environment = Environment {
        member_of: Some(&member_of as &MemberOf<'_>),
        ..Environment::default()
    };
    Ok(eval_env(expression, &[typed], &environment)?)
}

#[test]
fn member_of_is_empty_for_a_plain_string() -> TestResult {
    let expression = format!("'Patient'.memberOf('{RESOURCE_TYPES}')");
    assert_eq!(
        eval_member_of(&expression, r#"{"resourceType":"Basic"}"#)?,
        vec![]
    );
    Ok(())
}

#[test]
fn member_of_asks_the_environment_for_a_code() -> TestResult {
    let json = r#"{"resourceType":"OperationDefinition","parameter":[{"type":"Patient"},
        {"type":"string"}]}"#;
    let yes = format!("parameter[0].type.memberOf('{RESOURCE_TYPES}')");
    let no = format!("parameter[1].type.memberOf('{RESOURCE_TYPES}')");
    assert_eq!(bools(&eval_member_of(&yes, json)?), [true]);
    assert_eq!(bools(&eval_member_of(&no, json)?), [false]);
    Ok(())
}

#[test]
fn member_of_checks_codings_and_concepts() -> TestResult {
    let json = r#"{"resourceType":"Basic",
        "code":{"coding":[{"system":"http://example.org/other","code":"Patient"},
            {"system":"http://hl7.org/fhir/fhir-types","code":"Group"}]},
        "extension":[{"url":"http://example.org/x",
            "valueCoding":{"system":"http://hl7.org/fhir/fhir-types","code":"Basic"}}]}"#;
    let concept = format!("code.memberOf('{RESOURCE_TYPES}')");
    let first = format!("code.coding.first().memberOf('{RESOURCE_TYPES}')");
    let coding = format!("extension.valueCoding.memberOf('{RESOURCE_TYPES}')");
    assert_eq!(bools(&eval_member_of(&concept, json)?), [true]);
    // The ValueSet's membership is undecided for a coding of another system.
    assert_eq!(eval_member_of(&first, json)?, vec![]);
    assert_eq!(bools(&eval_member_of(&coding, json)?), [false]);
    Ok(())
}

#[test]
fn member_of_a_concept_is_false_only_when_every_coding_is_out() -> TestResult {
    let concept =
        |codings: &str| format!(r#"{{"resourceType":"Basic","code":{{"coding":[{codings}]}}}}"#);
    let out = r#"{"system":"http://hl7.org/fhir/fhir-types","code":"Basic"}"#;
    let undecided = r#"{"system":"http://example.org/other","code":"Patient"}"#;
    let expression = format!("code.memberOf('{RESOURCE_TYPES}')");
    let all_out = concept(&format!("{out},{out}"));
    assert_eq!(bools(&eval_member_of(&expression, &all_out)?), [false]);
    let mixed = concept(&format!("{out},{undecided}"));
    assert_eq!(eval_member_of(&expression, &mixed)?, vec![]);
    Ok(())
}

#[test]
fn member_of_several_items_is_an_error() -> TestResult {
    let json = r#"{"resourceType":"OperationDefinition","parameter":[{"type":"Patient"},
        {"type":"Group"}]}"#;
    let expression = format!("parameter.type.memberOf('{RESOURCE_TYPES}')");
    let result = eval_member_of(&expression, json);
    assert!(result.is_err(), "{result:?}");
    Ok(())
}

#[test]
fn member_of_is_empty_when_membership_cannot_be_decided() -> TestResult {
    let json = r#"{"resourceType":"OperationDefinition","parameter":[{"type":"Patient"}]}"#;
    let unknown = "parameter.type.memberOf('http://example.org/ValueSet/unknown')";
    assert_eq!(eval_member_of(unknown, json)?, vec![]);
    // No terminology in the environment.
    let without = format!("parameter.type.memberOf('{RESOURCE_TYPES}')");
    assert_eq!(evaluate(&without, &[resource(json)?])?, vec![]);
    assert_eq!(
        eval_member_of(&format!("{{}}.memberOf('{RESOURCE_TYPES}')"), json)?,
        vec![]
    );
    Ok(())
}
