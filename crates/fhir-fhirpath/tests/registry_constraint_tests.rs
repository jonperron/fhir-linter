//! Real R5 constraint expressions from the `Registry` parse and evaluate.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{Value, evaluate, resource_to_typed_value};
use fhir_parser::parse_json;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// Expression of constraint `key` on the root element of type `ty`.
fn constraint_expression(ty: &str, key: &str) -> TestResult<String> {
    let url = format!("http://hl7.org/fhir/StructureDefinition/{ty}");
    let sd = registry()
        .structure_definition(&url)
        .ok_or("type is not defined")?;
    let root = sd.element(ty).ok_or("no root element")?;
    let constraint = root
        .constraints
        .iter()
        .find(|c| &*c.key == key)
        .ok_or("constraint is not defined")?;
    Ok(constraint
        .expression
        .as_deref()
        .ok_or("constraint has no expression")?
        .to_owned())
}

/// Evaluate eld-11 on the single snapshot element of a StructureDefinition.
fn eld_11(element: &str) -> TestResult<Vec<Value>> {
    let json =
        format!(r#"{{"resourceType":"StructureDefinition","snapshot":{{"element":[{element}]}}}}"#);
    let resource = resource_to_typed_value(&parse_json(&json)?, &|parent, name| {
        registry().child_element(parent, name)
    });
    let element = evaluate("snapshot.element", &[resource])?;
    Ok(evaluate(
        &constraint_expression("ElementDefinition", "eld-11")?,
        &element,
    )?)
}

#[test]
fn eld_11_uses_a_double_quoted_string() -> TestResult {
    let expression = constraint_expression("ElementDefinition", "eld-11")?;
    assert!(expression.contains(r#"contains(":")"#), "{expression}");
    Ok(())
}

#[test]
fn eld_11_accepts_a_binding_on_a_coded_element() -> TestResult {
    let result = eld_11(
        r#"{"path":"Patient.gender","type":[{"code":"code"}],
        "binding":{"strength":"required"}}"#,
    )?;
    assert_eq!(result, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn eld_11_accepts_a_binding_on_a_type_with_a_colon() -> TestResult {
    let result = eld_11(
        r#"{"path":"X.y","type":[{"code":"http://hl7.org/fhirpath/System.String"}],
        "binding":{"strength":"required"}}"#,
    )?;
    assert_eq!(result, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn eld_11_as_written_holds_on_a_bound_boolean() -> TestResult {
    // `type.select(<Boolean>)` yields one `false` per type, so `exists()` is true:
    // the R5 expression cannot fail once a type is present.
    let result = eld_11(
        r#"{"path":"Patient.active","type":[{"code":"boolean"}],
        "binding":{"strength":"required"}}"#,
    )?;
    assert_eq!(result, [Value::Bool(true)]);
    Ok(())
}

#[test]
fn eld_11_on_several_types() -> TestResult {
    let result = eld_11(
        r#"{"path":"Observation.value[x]","type":[{"code":"string"},{"code":"Quantity"}],
        "binding":{"strength":"example"}}"#,
    )?;
    assert_eq!(result, [Value::Bool(true)]);
    Ok(())
}
