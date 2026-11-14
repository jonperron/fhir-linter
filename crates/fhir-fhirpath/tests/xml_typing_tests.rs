//! XML primitives typed by their element's declared FHIR type (R5 fhirpath.html §2.1.9.1):
//! a boolean, integer or decimal read from XML text evaluates as it does from JSON.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_fhirpath::{Collection, Value, evaluate, resource_to_typed_value};
use fhir_parser::{Resource, parse_json, parse_xml};

type TestResult = Result<(), Box<dyn Error>>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("R5 definitions")
    })
}

fn eval(resource: &Resource, expression: &str) -> Result<Collection, Box<dyn Error>> {
    let registry = registry();
    let value = resource_to_typed_value(resource, &|parent, name| {
        registry.child_element(parent, name)
    });
    Ok(evaluate(expression, &[value])?)
}

/// Whether `expression` is true on `resource`.
fn holds(resource: &Resource, expression: &str) -> Result<bool, Box<dyn Error>> {
    Ok(matches!(
        eval(resource, expression)?.as_slice(),
        [Value::Bool(true)]
    ))
}

const XML_PATIENT: &str = r#"<Patient xmlns="http://hl7.org/fhir"><active value="true"/>
    <multipleBirthInteger value="3"/></Patient>"#;
const JSON_PATIENT: &str = r#"{"resourceType":"Patient","active":true,"multipleBirthInteger":3}"#;

#[test]
fn xml_boolean_is_a_boolean() -> TestResult {
    let xml = parse_xml(XML_PATIENT)?;
    assert!(holds(&xml, "active = true")?);
    assert!(holds(&xml, "active and true")?);
    Ok(())
}

#[test]
fn xml_integer_and_decimal_evaluate_as_json_ones() -> TestResult {
    let xml = parse_xml(XML_PATIENT)?;
    let json = parse_json(JSON_PATIENT)?;
    let expressions = [
        "multipleBirthInteger > 2",
        "multipleBirthInteger + 1 = 4",
        "multipleBirthInteger is integer",
    ];
    for expression in expressions {
        assert!(holds(&json, expression)?, "JSON {expression}");
        assert!(holds(&xml, expression)?, "XML {expression}");
    }
    let xml = parse_xml(
        r#"<RiskAssessment xmlns="http://hl7.org/fhir"><prediction>
        <relativeRisk value="2.5"/></prediction></RiskAssessment>"#,
    )?;
    let json =
        parse_json(r#"{"resourceType":"RiskAssessment","prediction":[{"relativeRisk":2.5}]}"#)?;
    let expression = "prediction.relativeRisk > 2.4";
    assert!(holds(&json, expression)?);
    assert!(holds(&xml, expression)?);
    Ok(())
}

#[test]
fn ill_formed_xml_text_stays_a_string() -> TestResult {
    let xml = parse_xml(
        r#"<Patient xmlns="http://hl7.org/fhir"><active value="yes"/>
        <multipleBirthInteger value="007"/></Patient>"#,
    )?;
    assert!(holds(&xml, "active = 'yes'")?);
    assert!(holds(&xml, "multipleBirthInteger = '007'")?);
    Ok(())
}

#[test]
fn xml_text_is_typed_with_extensions_in_contained_and_for_int_subtypes() -> TestResult {
    let xml = parse_xml(
        r#"<Patient xmlns="http://hl7.org/fhir">
        <contained><Patient><active value="false"/></Patient></contained>
        <active value="true"><extension url="http://example.org/x">
        <valueString value="s"/></extension></active>
        </Patient>"#,
    )?;
    assert!(holds(&xml, "active = true")?);
    assert!(holds(&xml, "active.extension.exists()")?);
    assert!(holds(&xml, "contained.active = false")?);
    let xml = parse_xml(r#"<Group xmlns="http://hl7.org/fhir"><quantity value="5"/></Group>"#)?;
    // `Group.quantity` is an unsignedInt.
    assert!(holds(&xml, "quantity + 1 = 6")?);
    Ok(())
}
