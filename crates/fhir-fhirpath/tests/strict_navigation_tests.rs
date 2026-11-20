//! Strict navigation (N1 §3): with `Environment::strict` set from the real R5
//! `Registry`, a path step that names no element of a typed node is an error;
//! unset, it yields empty.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, Environment, EvalError, Strict, Value, evaluate_parsed_with_environment, parse,
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

const OBSERVATION: &str = r#"{"resourceType":"Observation","status":"final",
    "text":{"status":"generated","div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">w</div>"},
    "code":{"text":"weight"},
    "valueQuantity":{"value":185,"unit":"lbs"},
    "component":[{"code":{"text":"c"},"valueString":"x"}]}"#;

const PATIENT: &str = r#"{"resourceType":"Patient",
    "birthDate":"1970-01-01",
    "_birthDate":{"extension":[{"url":"http://example.org/e",
        "valueTiming":{"repeat":{"frequency":2}}}]},
    "contact":[{"name":{"family":"Doe"}}],
    "contained":[{"resourceType":"Questionnaire","status":"draft",
        "item":[{"linkId":"c1","type":"group"}]}]}"#;

const QUESTIONNAIRE: &str = r#"{"resourceType":"Questionnaire","status":"draft",
    "item":[{"linkId":"1","type":"group",
        "item":[{"linkId":"1.1","type":"group",
            "item":[{"linkId":"1.1.1","type":"string"}]}]}]}"#;

fn typed(json: &str) -> Result<Value, Box<dyn Error>> {
    let resource = parse_json(json)?;
    let registry = registry();
    Ok(resource_to_typed_value(&resource, &|parent, name| {
        registry.child_element(parent, name)
    }))
}

fn eval(expression: &str, strict: bool) -> Result<Collection, EvalError> {
    eval_on(OBSERVATION, expression, strict)
}

fn eval_on(json: &str, expression: &str, strict: bool) -> Result<Collection, EvalError> {
    let input = [typed(json).expect("resource parses")];
    let navigation = Strict::new(&has_element);
    let environment = Environment {
        is_subtype: Some(&is_subtype),
        strict: strict.then_some(&navigation),
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
    assert_unresolved_on(OBSERVATION, expression, message);
}

fn assert_unresolved_on(json: &str, expression: &str, message: &str) {
    match eval_on(json, expression, true) {
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
fn strict_checks_backbone_children_at_their_definition_path() -> TestResult {
    // A BackboneElement's children are defined at its path, not under its type.
    assert_eq!(eval("Observation.component.value", true)?.len(), 1);
    assert_unresolved(
        "Observation.component.valueString",
        "valueString is not an element of Observation.component",
    );
    assert_unresolved_on(
        PATIENT,
        "Patient.contact.nmae",
        "nmae is not an element of Patient.contact",
    );
    assert_eq!(
        eval_on(PATIENT, "Patient.contact.name.family", true)?.len(),
        1
    );
    // Reached through a function, the backbone still knows its path.
    assert_unresolved_on(
        PATIENT,
        "Patient.contact.first().nmae",
        "nmae is not an element of Patient.contact",
    );
    // A contained resource starts its own paths.
    assert_eq!(
        eval_on(PATIENT, "Patient.contained.item.linkId", true)?.len(),
        1
    );
    assert_unresolved_on(
        PATIENT,
        "Patient.contained.item.lnkId",
        "lnkId is not an element of Questionnaire.item",
    );
    Ok(())
}

#[test]
fn strict_checks_backbones_in_bundle_entries_and_complex_types() -> TestResult {
    let bundle = r#"{"resourceType":"Bundle","type":"collection",
        "entry":[{"resource":{"resourceType":"MedicationRequest","status":"active",
            "intent":"order","subject":{"reference":"Patient/1"},
            "dosageInstruction":[{"timing":{"repeat":{"frequency":2}}}]}}]}"#;
    let repeat = "Bundle.entry.resource.dosageInstruction.timing.repeat";
    assert_eq!(
        eval_on(bundle, &format!("{repeat}.frequency"), true)?.len(),
        1
    );
    // The entry is a backbone of Bundle; the resource in it restarts the path.
    assert_unresolved_on(
        bundle,
        "Bundle.entry.fullUrl1",
        "fullUrl1 is not an element of Bundle.entry",
    );
    assert_unresolved_on(
        bundle,
        &format!("{repeat}.frequency1"),
        "frequency1 is not an element of MedicationRequest.dosageInstruction.timing.repeat",
    );
    Ok(())
}

#[test]
fn strict_checks_below_a_primitive_extension() -> TestResult {
    // A primitive's children (`id`, `extension`) are typed by its own definition.
    let extension = "Patient.birthDate.extension";
    // A backbone inside an extension value is reached through the primitive.
    let repeat = format!("{extension}.value.repeat");
    assert_eq!(
        eval_on(PATIENT, &format!("{repeat}.frequency"), true)?.len(),
        1
    );
    assert_unresolved_on(
        PATIENT,
        &format!("{repeat}.frequency1"),
        "frequency1 is not an element of Patient.birthDate.extension.valueTiming.repeat",
    );
    assert_eq!(
        eval_on(PATIENT, &format!("{extension}.url"), true)?.len(),
        1
    );
    assert_unresolved_on(
        PATIENT,
        &format!("{extension}.urll"),
        "urll is not an element of Extension",
    );
    Ok(())
}

#[test]
fn strict_finds_a_context_backbone_through_the_environment_resources() -> TestResult {
    let patient = typed(PATIENT)?;
    let Value::Object(fields, _) = &patient else {
        return Err("the patient is not an object".into());
    };
    let contact = fields
        .get("contact")
        .and_then(|c| c.first())
        .ok_or("no contact")?;
    // One `Strict` for every environment: its index serves only the resources it
    // was built from, the others are indexed per evaluation.
    let navigation = Strict::new(&has_element);
    let eval_contact = |resource: Option<&Value>, root_resource: Option<&Value>| {
        let environment = Environment {
            resource,
            root_resource,
            is_subtype: Some(&is_subtype),
            strict: Some(&navigation),
            ..Environment::default()
        };
        evaluate_parsed_with_environment(
            &parse("nmae")?,
            std::slice::from_ref(contact),
            &|base, suffix| registry().is_choice_variant(base, suffix),
            &environment,
        )
    };
    let message = "nmae is not an element of Patient.contact";
    for (resource, root_resource) in [(Some(&patient), None), (None, Some(&patient))] {
        match eval_contact(resource, root_resource) {
            Err(EvalError::Type(got)) => assert_eq!(got, message),
            other => panic!("expected a type error, got {other:?}"),
        }
    }
    // Outside any resource, the backbone's path is unknown: not checked.
    assert_eq!(eval_contact(None, None)?.len(), 0);
    Ok(())
}

#[test]
fn strict_follows_content_references() -> TestResult {
    let q = QUESTIONNAIRE;
    assert_eq!(eval_on(q, "Questionnaire.item.item.linkId", true)?.len(), 1);
    assert_eq!(
        eval_on(q, "Questionnaire.item.item.item.linkId", true)?.len(),
        1
    );
    assert_unresolved_on(
        q,
        "Questionnaire.item.item.lnkId",
        "lnkId is not an element of Questionnaire.item.item",
    );
    Ok(())
}

#[test]
fn lenient_navigation_leaves_backbone_children_unchanged() -> TestResult {
    assert_eq!(eval("Observation.component.valueString", false)?.len(), 1);
    assert_eq!(eval_on(PATIENT, "Patient.contact.nmae", false)?.len(), 0);
    assert_eq!(
        eval_on(PATIENT, "Patient.contact.name.family", false)?.len(),
        1
    );
    let q = QUESTIONNAIRE;
    assert_eq!(
        eval_on(q, "Questionnaire.item.item.linkId", false)?.len(),
        1
    );
    assert_eq!(eval_on(q, "Questionnaire.item.item.lnkId", false)?.len(), 0);
    Ok(())
}

#[test]
fn strict_fails_only_when_no_item_can_resolve_the_name() -> TestResult {
    // The CodeableConcept has no `text1`, but a primitive is not checked.
    let mixed = "(Observation.code | Observation.status).text1";
    assert_eq!(eval(mixed, true)?.len(), 0);
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

#[test]
fn strict_resolves_a_leading_name_in_a_parameter_against_the_whole_input() -> TestResult {
    // N1 types `$this` by the input collection: the Narrative and CodeableConcept
    // descendants have no `unit`, the Quantity has (core dom-3 asks `reference` so).
    assert_eq!(eval("descendants().where(unit = 'lbs')", true)?.len(), 1);
    assert_eq!(eval("descendants().select(unit)", true)?.len(), 1);
    Ok(())
}

#[test]
fn strict_rejects_a_leading_name_in_a_parameter_no_input_item_resolves() {
    assert_unresolved(
        "Observation.code.where(bogus.exists())",
        "bogus is not an element of CodeableConcept",
    );
}

#[test]
fn strict_types_repeat_items_by_the_input_and_every_projection() -> TestResult {
    // Core qrs-2: the answers `repeat()` reaches have no `answer`, the input items do.
    let response = r#"{"resourceType":"QuestionnaireResponse","status":"completed",
        "item":[{"linkId":"1","answer":[{"valueString":"x"}]}]}"#;
    let expression = "QuestionnaireResponse.item.repeat(answer | item)";
    assert_eq!(eval_on(response, expression, true)?.len(), 1);
    Ok(())
}

#[test]
fn strict_resolves_names_of_a_nested_resource_by_its_resource_element() -> TestResult {
    // Core dom-2: `contained` is typed Resource, and a contained Bundle has no
    // `contained` of its own.
    let patient = r#"{"resourceType":"Patient",
        "contained":[{"resourceType":"Bundle","type":"collection"}]}"#;
    assert_eq!(
        eval_on(patient, "contained.contained.empty()", true)?,
        vec![Value::Bool(true)]
    );
    Ok(())
}
