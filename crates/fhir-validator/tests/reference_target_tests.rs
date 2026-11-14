//! Reference target types (`REFERENCE_005`) and uncoded Codings under a required
//! binding (`TERMINOLOGY_001`), against the bundled R5 definitions.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// `(severity, path)` of every diagnostic with `code`.
fn found(json: &str, code: &str) -> Vec<(Severity, String)> {
    let resource = parse_json(json).expect("test JSON parses");
    validate(&resource, registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == code)
        .map(|d| (d.severity, d.path.to_string()))
        .collect()
}

fn patient(field: &str) -> String {
    format!(r#"{{"resourceType":"Patient","id":"p",{field}}}"#)
}

#[test]
fn reference_to_a_type_outside_target_profiles_is_an_error() {
    // Patient.managingOrganization is Reference(Organization).
    for reference in [
        "Practitioner/2",
        "Practitioner/2/_history/1",
        "http://example.org/fhir/Practitioner/2",
    ] {
        let json = patient(&format!(
            r#""managingOrganization":{{"reference":"{reference}"}}"#
        ));
        assert_eq!(
            found(&json, "REFERENCE_005"),
            [(
                Severity::Error,
                "Patient.managingOrganization.reference".to_owned()
            )],
            "{reference}",
        );
    }
}

#[test]
fn reference_to_an_allowed_type_is_clean() {
    for reference in [
        "Organization/2",
        "https://example.org/fhir/Organization/2/_history/3",
    ] {
        let json = patient(&format!(
            r#""managingOrganization":{{"reference":"{reference}"}}"#
        ));
        assert!(found(&json, "REFERENCE_005").is_empty(), "{reference}");
    }
    // Patient.generalPractitioner allows Organization, Practitioner and PractitionerRole.
    let json = patient(r#""generalPractitioner":[{"reference":"PractitionerRole/1"}]"#);
    assert!(found(&json, "REFERENCE_005").is_empty());
}

#[test]
fn nested_references_are_checked_at_every_depth() {
    // Backbone element: Patient.contact.organization is Reference(Organization).
    let json = patient(r#""contact":[{"organization":{"reference":"Patient/1"}}]"#);
    assert_eq!(
        found(&json, "REFERENCE_005"),
        [(
            Severity::Error,
            "Patient.contact[0].organization.reference".to_owned()
        )],
    );
    // Datatype: Identifier.assigner is Reference(Organization).
    let json = patient(r#""identifier":[{"assigner":{"reference":"Device/1"}}]"#);
    assert_eq!(
        found(&json, "REFERENCE_005"),
        [(
            Severity::Error,
            "Patient.identifier[0].assigner.reference".to_owned()
        )],
    );
}

#[test]
fn resource_target_logical_contained_and_urn_references_are_not_type_checked() {
    // Extension.value[x] Reference has no targetProfile; Provenance.target is Reference(Resource).
    let extension = patient(
        r#""extension":[{"url":"http://example.org/x","valueReference":{"reference":"Device/1"}}]"#,
    );
    assert!(found(&extension, "REFERENCE_005").is_empty());
    let provenance = r#"{"resourceType":"Provenance","target":[{"reference":"Device/1"}],
        "recorded":"2024-01-01T00:00:00Z","agent":[{"who":{"reference":"Device/1"}}]}"#;
    assert!(found(provenance, "REFERENCE_005").is_empty());
    for reference in [
        r##"{"reference":"#c1"}"##,
        r#"{"reference":"urn:uuid:a6f5d8e2-6a0f-4b4c-9b0a-1d2e3f4a5b6c"}"#,
        r#"{"identifier":{"value":"x"}}"#,
        r#"{"reference":"Practitioner?identifier=x"}"#,
    ] {
        let json = patient(&format!(
            r#""contained":[{{"resourceType":"Practitioner","id":"c1"}}],
            "managingOrganization":{reference}"#
        ));
        assert!(found(&json, "REFERENCE_005").is_empty(), "{reference}");
    }
}

#[test]
fn codeable_concept_coding_without_code_under_required_binding_is_reported() {
    let json = r#"{"resourceType":"Condition","subject":{"reference":"Patient/1"},
        "clinicalStatus":{"coding":[
            {"system":"http://terminology.hl7.org/CodeSystem/condition-clinical"}]}}"#;
    assert_eq!(
        found(json, "TERMINOLOGY_001"),
        [(Severity::Error, "Condition.clinicalStatus".to_owned())],
    );
}

#[test]
fn coding_without_code_under_required_binding_is_reported() {
    // Appointment...monthlyTemplate.dayOfWeek is a Coding bound (required) to days-of-week.
    let json = r#"{"resourceType":"Appointment","status":"proposed",
        "recurrenceTemplate":[{"recurrenceType":{"text":"monthly"},"monthlyTemplate":{
        "dayOfWeek":{"system":"http://hl7.org/fhir/days-of-week"},"monthInterval":1}}]}"#;
    assert_eq!(
        found(json, "TERMINOLOGY_001"),
        [(
            Severity::Error,
            "Appointment.recurrenceTemplate[0].monthlyTemplate.dayOfWeek".to_owned()
        )],
    );
    let coded = json.replace(r#"days-of-week""#, r#"days-of-week","code":"mon""#);
    assert!(found(&coded, "TERMINOLOGY_001").is_empty());
}
