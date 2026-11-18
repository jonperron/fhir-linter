//! Checks on StructureDefinition instances: properties a profile may not set
//! (elementdefinition.html, ElementDefinition.meaningWhenMissing: implicit meanings
//! "can only be specified on a resource, data type, or extension definition, and never
//! in a profile that applies to one of these"), on the official validator test cases.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{Resource, parse_json, parse_xml};
use fhir_validator::{Diagnostic, Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/validator")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn definition_findings(resource: &Resource) -> Vec<Diagnostic> {
    validate(resource, registry())
        .into_iter()
        .filter(|d| d.code.starts_with("DEFINITION_"))
        .collect()
}

fn xml(name: &str) -> Vec<Diagnostic> {
    definition_findings(&parse_xml(&fixture(name)).expect("parses"))
}

fn json(source: &str) -> Vec<Diagnostic> {
    definition_findings(&parse_json(source).expect("parses"))
}

fn assert_meaning_when_missing(found: &[Diagnostic], path: &str, element: &str) {
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(&*found[0].code, "DEFINITION_001");
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(found[0].path, path);
    assert!(
        found[0].message.contains("meaningWhenMissing") && found[0].message.contains(element),
        "{}",
        found[0].message
    );
}

#[test]
fn profile_on_a_resource_may_not_set_meaning_when_missing_xml() {
    assert_meaning_when_missing(
        &xml("profile-illegal-elements.xml"),
        "StructureDefinition.differential.element[0].meaningWhenMissing",
        "Patient.identifier",
    );
}

#[test]
fn profile_on_a_data_type_may_not_set_meaning_when_missing_xml() {
    assert_meaning_when_missing(
        &xml("sd-root-profile.xml"),
        "StructureDefinition.differential.element[0].meaningWhenMissing",
        "CodeableConcept",
    );
}

#[test]
fn specializations_may_set_meaning_when_missing_xml() {
    assert!(xml("sd-root-logical.xml").is_empty());
    assert!(xml("sd-root-specialization.xml").is_empty());
}

fn structure_definition(derivation: &str, type_name: &str, base: &str) -> String {
    format!(
        r#"{{
            "resourceType": "StructureDefinition",
            "url": "http://example.org/StructureDefinition/test",
            "name": "Test",
            "status": "draft",
            "kind": "complex-type",
            "abstract": false,
            "type": "{type_name}",
            "baseDefinition": "{base}",
            "derivation": "{derivation}",
            "differential": {{
                "element": [
                    {{"id": "{type_name}", "path": "{type_name}"}},
                    {{
                        "id": "{type_name}.id",
                        "path": "{type_name}.id",
                        "meaningWhenMissing": "none"
                    }}
                ]
            }}
        }}"#
    )
}

#[test]
fn profile_may_not_set_meaning_when_missing_json() {
    let source = structure_definition(
        "constraint",
        "Identifier",
        "http://hl7.org/fhir/StructureDefinition/Identifier",
    );
    assert_meaning_when_missing(
        &json(&source),
        "StructureDefinition.differential.element[1].meaningWhenMissing",
        "Identifier.id",
    );
}

#[test]
fn extension_definition_may_set_meaning_when_missing_json() {
    let source = structure_definition(
        "constraint",
        "Extension",
        "http://hl7.org/fhir/StructureDefinition/Extension|5.0.0",
    );
    assert!(json(&source).is_empty());
}

#[test]
fn profile_on_an_extension_definition_may_not_set_meaning_when_missing_json() {
    // A profile on an extension "applies to" the extension definition, so the rule holds.
    // The base need not be loaded: only the core Extension type exempts a definition.
    let base = "http://example.org/StructureDefinition/some-extension";
    let source = structure_definition("constraint", "Extension", base);
    assert_meaning_when_missing(
        &json(&source),
        "StructureDefinition.differential.element[1].meaningWhenMissing",
        "Extension.id",
    );
}

#[test]
fn core_structure_definitions_set_meaning_when_missing_without_findings() {
    let source = structure_definition(
        "specialization",
        "Identifier",
        "http://hl7.org/fhir/StructureDefinition/DataType",
    );
    assert!(json(&source).is_empty());
}

/// The test profile on Identifier, edited by `edit` before validation.
fn edited_profile(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<Diagnostic> {
    let source = structure_definition(
        "constraint",
        "Identifier",
        "http://hl7.org/fhir/StructureDefinition/Identifier",
    );
    let mut value: serde_json::Value = serde_json::from_str(&source).expect("valid JSON");
    edit(&mut value);
    json(&value.to_string())
}

#[test]
fn profile_without_meaning_when_missing_has_no_findings_json() {
    let found = edited_profile(|sd| {
        sd["differential"]["element"][1]
            .as_object_mut()
            .expect("element")
            .remove("meaningWhenMissing");
    });
    assert!(found.is_empty(), "{found:?}");
    let found = edited_profile(|sd| {
        sd.as_object_mut().expect("object").remove("differential");
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn profile_without_type_is_left_to_the_required_field_check_json() {
    // Without a type the extension exemption cannot be decided.
    let found = edited_profile(|sd| {
        sd.as_object_mut().expect("object").remove("type");
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn profile_without_base_definition_may_not_set_meaning_when_missing_json() {
    let found = edited_profile(|sd| {
        sd.as_object_mut().expect("object").remove("baseDefinition");
    });
    assert_meaning_when_missing(
        &found,
        "StructureDefinition.differential.element[1].meaningWhenMissing",
        "Identifier.id",
    );
}

#[test]
fn profile_in_a_bundle_entry_may_not_set_meaning_when_missing_json() {
    let profile = structure_definition(
        "constraint",
        "Identifier",
        "http://hl7.org/fhir/StructureDefinition/Identifier",
    );
    let source = format!(
        r#"{{"resourceType": "Bundle", "type": "collection", "entry": [
            {{"fullUrl": "http://example.org/StructureDefinition/test", "resource": {profile}}}
        ]}}"#
    );
    assert_meaning_when_missing(
        &json(&source),
        "Bundle.entry[0].resource.differential.element[1].meaningWhenMissing",
        "Identifier.id",
    );
}
