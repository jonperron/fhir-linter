//! Strict navigation (N1 §3): with `Environment::strict` set from the real R5
//! `Registry`, a path step that names no element of a typed node is an error;
//! unset, it yields empty.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, Environment, EvalError, HasElement, Value, evaluate_parsed_with_environment, parse,
    resource_to_typed_value,
};
use fhir_parser::parse_json;

type TestResult = Result<(), Box<dyn Error>>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("R5 definitions")
    })
}

fn has_element(type_name: &str, name: &str) -> Option<bool> {
    registry().has_element(type_name, name)
}

fn is_subtype(child: &str, ancestor: &str) -> bool {
    registry().type_ancestors(child).any(|a| a == ancestor)
}

fn observation() -> Result<Value, Box<dyn Error>> {
    let resource = parse_json(
        r#"{"resourceType":"Observation","status":"final",
        "text":{"status":"generated","div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">w</div>"},
        "code":{"text":"weight"},
        "valueQuantity":{"value":185,"unit":"lbs"},
        "component":[{"code":{"text":"c"},"valueString":"x"}]}"#,
    )?;
    let registry = registry();
    Ok(resource_to_typed_value(&resource, &|parent, name| {
        registry.child_element(parent, name)
    }))
}

fn eval(expression: &str, strict: bool) -> Result<Collection, EvalError> {
    let input = [observation().expect("observation parses")];
    let environment = Environment {
        is_subtype: Some(&is_subtype),
        strict: strict.then_some(&has_element as &HasElement<'_>),
        ..Environment::default()
    };
    evaluate_parsed_with_environment(
        &parse(expression)?,
        &input,
        &|base, suffix| registry().is_choice_variant(base, suffix),
        &environment,
    )
}

fn assert_unresolved(expression: &str, message: &str) {
    match eval(expression, true) {
        Err(EvalError::Type(got)) => assert_eq!(got, message),
        other => panic!("{expression}: expected a type error, got {other:?}"),
    }
}

#[test]
fn strict_rejects_a_choice_type_suffix() {
    assert_unresolved(
        "Observation.valueQuantity.unit",
        "valueQuantity is not an element of Observation",
    );
}

#[test]
fn strict_rejects_an_unknown_element() {
    assert_unresolved(
        "Observation.code.text1",
        "text1 is not an element of CodeableConcept",
    );
}

#[test]
fn strict_rejects_a_leading_name_that_is_not_the_type() {
    assert_unresolved(
        "Encounter.status",
        "Encounter is not an element of Observation",
    );
}

#[test]
fn strict_resolves_declared_elements_and_the_choice_name() -> TestResult {
    assert_eq!(eval("Observation.value.unit", true)?.len(), 1);
    assert_eq!(eval("DomainResource.text", true)?.len(), 1);
    // A declared element that is absent is empty, not an error.
    assert_eq!(eval("Observation.subject", true)?.len(), 0);
    // An empty input resolves nothing and fails nothing.
    assert_eq!(eval("Observation.subject.reference1", true)?.len(), 0);
    Ok(())
}

#[test]
fn strict_does_not_check_backbone_children() -> TestResult {
    // A BackboneElement's children are defined at its path, not under its type.
    assert_eq!(eval("Observation.component.value", true)?.len(), 1);
    assert_eq!(eval("Observation.component.valueString", true)?.len(), 1);
    Ok(())
}

#[test]
fn strict_fails_only_when_no_item_can_resolve_the_name() -> TestResult {
    // The CodeableConcept has no `valueString`, but the backbone may.
    let mixed = "(Observation.code | Observation.component).valueString";
    assert_eq!(eval(mixed, true)?.len(), 1);
    assert!(eval("(Observation.code | Observation.value).text1", true).is_err());
    Ok(())
}

#[test]
fn lenient_navigation_yields_empty_for_unknown_names() -> TestResult {
    assert_eq!(eval("Observation.valueQuantity.unit", false)?.len(), 1);
    assert_eq!(eval("Observation.code.text1", false)?.len(), 0);
    assert_eq!(eval("Encounter.status", false)?.len(), 0);
    Ok(())
}
