//! `conformsTo(structure)` (R5 fhirpath.html, additional functions) with
//! `Environment::conforms_to` built from the real R5 `Registry`.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{
    Collection, ConformsTo, Environment, EvalError, Value, evaluate_parsed_with_environment, parse,
    resource_to_typed_value,
};
use fhir_parser::parse_json;

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
fn a_resource_conforms_to_its_base_definition() {
    let result = eval("conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')");
    assert_eq!(result.unwrap(), vec![Value::Bool(true)]);
}

#[test]
fn a_resource_does_not_conform_to_another_base_definition() {
    let result = eval("conformsTo('http://hl7.org/fhir/StructureDefinition/Person')");
    assert_eq!(result.unwrap(), vec![Value::Bool(false)]);
}

#[test]
fn a_complex_element_conforms_to_its_type_definition() {
    let result = eval("name.conformsTo('http://hl7.org/fhir/StructureDefinition/HumanName')");
    assert_eq!(result.unwrap(), vec![Value::Bool(true)]);
}

#[test]
fn a_primitive_conforms_to_its_own_type_only() {
    let base = "http://hl7.org/fhir/StructureDefinition";
    let result = eval(&format!("name.family.conformsTo('{base}/string')"));
    assert_eq!(result.unwrap(), vec![Value::Bool(true)]);
    let result = eval(&format!("name.family.conformsTo('{base}/code')"));
    assert_eq!(result.unwrap(), vec![Value::Bool(false)]);
}

#[test]
fn an_unresolved_structure_is_empty() {
    // fhirpath.html: "cannot be resolved to a valid profile, the result is empty".
    assert_eq!(eval("conformsTo('http://trash')").unwrap(), vec![]);
}

#[test]
fn an_input_that_is_not_a_single_item_is_empty() {
    let url = "'http://hl7.org/fhir/StructureDefinition/Patient'";
    assert_eq!(eval(&format!("{{}}.conformsTo({url})")).unwrap(), vec![]);
    assert_eq!(eval(&format!("(1 | 2).conformsTo({url})")).unwrap(), vec![]);
}

#[test]
fn an_empty_structure_is_empty() {
    assert_eq!(eval("conformsTo({})").unwrap(), vec![]);
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
