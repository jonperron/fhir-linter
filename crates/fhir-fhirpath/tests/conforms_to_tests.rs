//! `conformsTo(structure)` (R5 fhirpath.html, additional functions) with
//! `Environment::conforms_to` built from the real R5 `Registry`.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, ConformsTo, Environment, EvalError, Value, evaluate_parsed_with_environment, parse,
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

fn conforms_to(fhir_type: &str, url: &str) -> Option<Option<bool>> {
    registry().conforms_to(fhir_type, url)
}

fn patient() -> Value {
    let resource = parse_json(r#"{"resourceType":"Patient","name":[{"family":"Doe"}]}"#)
        .expect("patient parses");
    let registry = registry();
    resource_to_typed_value(&resource, &|parent, name| {
        registry.child_element(parent, name)
    })
}

fn eval_with(expression: &str, hook: Option<&ConformsTo<'_>>) -> Result<Collection, EvalError> {
    let environment = Environment {
        conforms_to: hook,
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
    eval_with(expression, Some(&conforms_to))
}

#[test]
fn a_resource_conforms_to_its_base_definition() -> TestResult {
    let result = eval("conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')");
    assert_eq!(result?, vec![Value::Bool(true)]);
    Ok(())
}

#[test]
fn a_resource_does_not_conform_to_another_base_definition() -> TestResult {
    let result = eval("conformsTo('http://hl7.org/fhir/StructureDefinition/Person')");
    assert_eq!(result?, vec![Value::Bool(false)]);
    Ok(())
}

#[test]
fn a_complex_element_conforms_to_its_type_definition() -> TestResult {
    let result = eval("name.conformsTo('http://hl7.org/fhir/StructureDefinition/HumanName')");
    assert_eq!(result?, vec![Value::Bool(true)]);
    Ok(())
}

#[test]
fn a_primitive_conforms_to_its_own_type_only() -> TestResult {
    let base = "http://hl7.org/fhir/StructureDefinition";
    let result = eval(&format!("name.family.conformsTo('{base}/string')"));
    assert_eq!(result?, vec![Value::Bool(true)]);
    let result = eval(&format!("name.family.conformsTo('{base}/code')"));
    assert_eq!(result?, vec![Value::Bool(false)]);
    Ok(())
}

#[test]
fn an_unresolved_structure_is_empty() -> TestResult {
    // fhirpath.html: "cannot be resolved to a valid profile, the result is empty".
    assert_eq!(eval("conformsTo('http://trash')")?, vec![]);
    Ok(())
}

#[test]
fn an_input_that_is_not_a_single_item_is_empty() -> TestResult {
    let url = "'http://hl7.org/fhir/StructureDefinition/Patient'";
    assert_eq!(eval(&format!("{{}}.conformsTo({url})"))?, vec![]);
    assert_eq!(eval(&format!("(1 | 2).conformsTo({url})"))?, vec![]);
    Ok(())
}

#[test]
fn a_versioned_canonical_resolves_to_the_loaded_version_only() -> TestResult {
    let base = "http://hl7.org/fhir/StructureDefinition";
    let result = eval(&format!("conformsTo('{base}/Patient|5.0.0')"));
    assert_eq!(result?, vec![Value::Bool(true)]);
    // Another version is not loaded: it cannot be resolved.
    assert_eq!(
        eval(&format!("conformsTo('{base}/Patient|4.0.1')"))?,
        vec![]
    );
    Ok(())
}

#[test]
fn a_structure_that_is_not_a_string_is_an_error() {
    let result = eval("conformsTo(1)");
    assert!(matches!(result, Err(EvalError::Type(_))), "{result:?}");
}

#[test]
fn a_null_input_is_empty() -> TestResult {
    let result = evaluate_parsed_with_environment(
        &parse("conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')")?,
        &[Value::Null],
        &|base, suffix| registry().is_choice_variant(base, suffix),
        &Environment {
            conforms_to: Some(&conforms_to),
            ..Environment::default()
        },
    );
    assert_eq!(result?, vec![]);
    Ok(())
}

#[test]
fn an_empty_structure_is_empty() -> TestResult {
    assert_eq!(eval("conformsTo({})")?, vec![]);
    Ok(())
}

#[test]
fn a_constraint_profile_is_an_error() {
    let result = eval("conformsTo('http://hl7.org/fhir/StructureDefinition/vitalsigns')");
    assert!(matches!(result, Err(EvalError::Type(_))), "{result:?}");
}

#[test]
fn an_input_without_a_fhir_type_is_an_error() {
    let result = eval("1.conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')");
    assert!(matches!(result, Err(EvalError::Type(_))), "{result:?}");
}

#[test]
fn without_the_hook_conforms_to_is_an_error() {
    let result = eval_with(
        "conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')",
        None,
    );
    assert!(matches!(result, Err(EvalError::Type(_))), "{result:?}");
}

#[test]
fn conforms_to_takes_one_argument() {
    assert!(eval("conformsTo()").is_err());
}
