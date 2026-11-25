//! A code system URI that is the canonical URL of a ValueSet (datatypes.html#Coding,
//! valueset-definitions.html#ValueSet.compose.include.system: the system is a code system).
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{Resource, parse_json, parse_xml};
use fhir_validator::{Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

fn findings(resource: &Resource) -> Vec<(Severity, String)> {
    validate(resource, registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == "TERMINOLOGY_003")
        .map(|d| (d.severity, d.path))
        .collect()
}

fn basic(system: &str) -> Resource {
    let json = format!(
        r#"{{"resourceType":"Basic","code":{{"coding":[{{"system":"{system}","code":"x"}}]}}}}"#
    );
    parse_json(&json).unwrap()
}

/// An official case; a missing fixture fails (run `git submodule update --init`).
fn official(name: &str) -> Resource {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/validator")
        .join(name);
    let source = std::fs::read_to_string(path).expect("fixtures submodule checked out");
    parse_xml(&source).unwrap()
}

#[test]
fn coding_system_naming_a_value_set_is_an_error_in_json() {
    assert_eq!(
        findings(&basic("http://hl7.org/fhir/ValueSet/account-type")),
        vec![(Severity::Error, "Basic.code.coding[0].system".to_owned())]
    );
}

#[test]
fn versioned_coding_system_naming_a_value_set_is_an_error_at_any_version() {
    // A versioned canonical still names the ValueSet, whatever version is loaded.
    for version in ["5.0.0", "9.9"] {
        let system = format!("http://hl7.org/fhir/ValueSet/account-type|{version}");
        assert_eq!(
            findings(&basic(&system)),
            vec![(Severity::Error, "Basic.code.coding[0].system".to_owned())],
            "{version}"
        );
    }
}

#[test]
fn coding_system_naming_a_value_set_is_an_error_in_xml() {
    let resource = official("code-wrong-system-vs.xml");
    assert_eq!(
        findings(&resource),
        vec![(Severity::Error, "Basic.code.coding.system".to_owned())]
    );
}

#[test]
fn value_set_include_system_naming_a_value_set_is_an_error() {
    let resource = official("vs-wrong-vs.xml");
    assert_eq!(
        findings(&resource),
        vec![(
            Severity::Error,
            "ValueSet.compose.include.system".to_owned()
        )]
    );
    let json = r#"{"resourceType":"ValueSet","status":"active","compose":{
        "include":[{"system":"http://hl7.org/fhir/FHIR-version"}],
        "exclude":[{"system":"http://hl7.org/fhir/ValueSet/FHIR-version"}]}}"#;
    assert_eq!(
        findings(&parse_json(json).unwrap()),
        vec![(
            Severity::Error,
            "ValueSet.compose.exclude[0].system".to_owned()
        )]
    );
}

#[test]
fn code_systems_and_unknown_uris_are_not_reported() {
    for system in [
        "http://hl7.org/fhir/account-type",
        "http://snomed.info/sct",
        "http://example.org/ValueSet/unknown",
    ] {
        assert!(findings(&basic(system)).is_empty(), "{system}");
    }
}

#[test]
fn coding_system_with_an_extension_in_xml_is_still_checked() {
    let source = r#"<Basic xmlns="http://hl7.org/fhir"><code><coding>
        <system value="http://hl7.org/fhir/ValueSet/account-type">
          <extension url="http://example.org/x"><valueString value="y"/></extension>
        </system><code value="x"/></coding></code></Basic>"#;
    assert_eq!(
        findings(&parse_xml(source).unwrap()),
        vec![(Severity::Error, "Basic.code.coding.system".to_owned())]
    );
}
