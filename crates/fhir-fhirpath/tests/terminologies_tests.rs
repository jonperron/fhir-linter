//! `%terminologies` (R5 fhirpath.html §2.1.9.4) with
//! `Environment::terminologies` set to the real R5 `Registry`.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, Environment, EvalError, Terminologies, Value, evaluate_parsed_with_environment,
    parse, resource_to_typed_value,
};
use fhir_parser::parse_json;

type TestResult = Result<(), Box<dyn Error>>;

const GENDER: &str = "http://hl7.org/fhir/ValueSet/administrative-gender";
const ADDRESS_USE_V2: &str = "http://hl7.org/fhir/ConceptMap/cm-address-use-v2";

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("R5 definitions")
    })
}

fn patient() -> Value {
    let resource = parse_json(
        r#"{"resourceType":"Patient","gender":"male","address":[{"use":"home"}],
        "contact":[{"relationship":[{"text":"friend"}]}],
        "maritalStatus":{"coding":[
            {"system":"http://hl7.org/fhir/administrative-gender","code":"xyz"},
            {"system":"http://hl7.org/fhir/administrative-gender","code":"female"}]}}"#,
    )
    .expect("patient parses");
    let registry = registry();
    resource_to_typed_value(&resource, &|parent, name| {
        registry.child_element(parent, name)
    })
}

fn eval_with(
    expression: &str,
    terminologies: Option<&dyn Terminologies>,
) -> Result<Collection, EvalError> {
    let environment = Environment {
        terminologies,
        ..Environment::default()
    };
    evaluate_parsed_with_environment(
        &parse(expression)?,
        &[patient()],
        &|base, suffix| registry().is_choice_variant(base, suffix),
        &environment,
    )
}

fn eval(expression: &str) -> Result<Collection, EvalError> {
    eval_with(expression, Some(registry()))
}

fn validate(coded: &str) -> String {
    format!("%terminologies.validateVS('{GENDER}', {coded}).parameter.where(name = 'result').value")
}

#[test]
fn expand_lists_the_codes_with_system_and_display() -> TestResult {
    let expansion = format!("%terminologies.expand('{GENDER}').expansion");
    assert_eq!(eval(&format!("{expansion}.total"))?, [Value::Integer(4)]);
    let male = eval(&format!("{expansion}.contains.where(code = 'male')"))?;
    let display = eval(&format!(
        "{expansion}.contains.where(code = 'male').select(system & '|' & display)"
    ))?;
    assert_eq!(male.len(), 1);
    assert_eq!(
        display,
        [Value::String(
            "http://hl7.org/fhir/administrative-gender|Male".into()
        )]
    );
    Ok(())
}

#[test]
fn validate_vs_answers_for_a_code_a_coding_and_a_codeable_concept() -> TestResult {
    assert_eq!(eval(&validate("gender"))?, [Value::Bool(true)]);
    assert_eq!(
        eval(&validate("maritalStatus.coding.first()"))?,
        [Value::Bool(false)]
    );
    // A CodeableConcept is valid when one of its codings is.
    assert_eq!(eval(&validate("maritalStatus"))?, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn translate_returns_result_and_matches() -> TestResult {
    let translate = format!("%terminologies.translate('{ADDRESS_USE_V2}', address.use)");
    assert_eq!(
        eval(&format!(
            "{translate}.parameter.where(name = 'match').part.where(name = 'relationship').value"
        ))?,
        [Value::fhir_string("equivalent", "code")]
    );
    assert_eq!(
        eval(&format!(
            "{translate}.parameter.where(name = 'result').value"
        ))?,
        [Value::Bool(true)]
    );
    // A code the map does not list: no match, and so no translation.
    let unmapped = format!("%terminologies.translate('{ADDRESS_USE_V2}', gender).parameter");
    assert_eq!(
        eval(&format!("{unmapped}.where(name = 'result').value"))?,
        [Value::Bool(false)]
    );
    assert_eq!(eval(&format!("{unmapped}.where(name = 'match')"))?, []);
    Ok(())
}

#[test]
fn empty_multiple_or_invalid_parameters_give_empty() -> TestResult {
    // fhirpath.html: empty "if any of the parameters are empty, or a collection
    // with more than one value, or one or more of the parameters are not valid".
    for expression in [
        "%terminologies.expand({})".to_owned(),
        format!("%terminologies.expand('{GENDER}' | 'http://x')"),
        format!("%terminologies.validateVS('{GENDER}', {{}})"),
        // A plain String is not "a resource element that is a code".
        format!("%terminologies.validateVS('{GENDER}', 'male')"),
        // A CodeableConcept with text alone has no code to validate.
        format!("%terminologies.validateVS('{GENDER}', contact.relationship)"),
        format!("%terminologies.translate('{ADDRESS_USE_V2}', maritalStatus)"),
        "%terminologies.expand(1)".to_owned(),
        format!("%terminologies.expand('{GENDER}', 1)"),
    ] {
        assert_eq!(eval(&expression)?, [], "{expression}");
    }
    Ok(())
}

#[test]
fn empty_params_are_accepted_and_others_are_errors() -> TestResult {
    let expand = format!("%terminologies.expand('{GENDER}', '').expansion.total");
    assert_eq!(eval(&expand)?, [Value::Integer(4)]);
    assert!(
        eval(&format!(
            "%terminologies.expand('{GENDER}', 'activeOnly=true')"
        ))
        .is_err()
    );
    Ok(())
}

#[test]
fn an_unset_or_undecided_service_is_an_error() {
    let expand = format!("%terminologies.expand('{GENDER}')");
    assert!(eval_with(&expand, None).is_err());
    // include.filter: the Registry cannot enumerate it.
    assert!(eval("%terminologies.expand('http://hl7.org/fhir/ValueSet/example-filter')").is_err());
    assert!(eval("%terminologies.expand('http://hl7.org/fhir/ValueSet/no-such')").is_err());
    // An actual resource is not looked up by its url.
    assert!(eval("%terminologies.expand(%resource)").is_err());
}

#[test]
fn other_functions_and_redefinition_are_errors() {
    assert!(matches!(
        eval("%terminologies.lookup(gender)"),
        Err(EvalError::UndefinedFunction(_))
    ));
    assert!(eval("defineVariable('terminologies', 1).select(%terminologies)").is_err());
    assert!(eval(&format!("%terminologies.expand('{GENDER}', '', '')")).is_err());
}
