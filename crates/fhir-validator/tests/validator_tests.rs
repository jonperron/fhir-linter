//! Integration tests for the FHIR validator against the bundled R5 definitions.
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
}

fn load_registry() -> Registry {
    Registry::from_definitions_dir(definitions_dir()).expect("failed to load R5 definitions")
}

// ── Unknown resource type ─────────────────────────────────────────────────────

#[test]
fn unknown_resource_type_produces_structure_000() {
    let registry = load_registry();
    let resource = parse_json(r#"{"resourceType":"NonExistentThing","id":"x"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code.as_ref(), "STRUCTURE_000");
    assert_eq!(diagnostics[0].severity, Severity::Error);
}

// ── Structural validation ─────────────────────────────────────────────────────

#[test]
fn unknown_element_produces_structure_001() {
    let registry = load_registry();
    let resource =
        parse_json(r#"{"resourceType":"Patient","id":"p1","unknownFooBarBaz":"value"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "STRUCTURE_001")
        .collect();
    assert!(
        !errors.is_empty(),
        "Expected STRUCTURE_001 for unknown element"
    );
    assert!(
        errors.iter().any(|d| d.path.contains("unknownFooBarBaz")),
        "Error path should mention the unknown field",
    );
}

#[test]
fn valid_patient_produces_no_structural_errors() {
    let registry = load_registry();
    let resource = parse_json(r#"{"resourceType":"Patient","id":"p1","active":true}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let structural_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "STRUCTURE_001")
        .collect();
    assert!(
        structural_errors.is_empty(),
        "Unexpected STRUCTURE_001: {structural_errors:?}",
    );
}

// ── Cardinality validation ─────────────────────────────────────────────────────

#[test]
fn missing_required_field_produces_cardinality_001() {
    let registry = load_registry();
    // Observation requires `status` (min=1) and `code` (min=1)
    let resource = parse_json(r#"{"resourceType":"Observation","id":"o1"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "CARDINALITY_001")
        .collect();
    assert!(
        !errors.is_empty(),
        "Expected CARDINALITY_001 for missing required fields on Observation",
    );
}

#[test]
fn patient_with_no_required_fields_passes_cardinality() {
    // Patient has no required fields at the base level.
    let registry = load_registry();
    let resource = parse_json(r#"{"resourceType":"Patient","id":"p1"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "CARDINALITY_001" && d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Unexpected CARDINALITY_001 for Patient: {errors:?}",
    );
}

// ── Type validation ───────────────────────────────────────────────────────────

#[test]
fn invalid_birth_date_produces_type_003() {
    let registry = load_registry();
    let resource =
        parse_json(r#"{"resourceType":"Patient","id":"p1","birthDate":"not-a-date"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == "TYPE_003")
        .collect();
    assert!(
        !errors.is_empty(),
        "Expected TYPE_003 for invalid birthDate"
    );
}

#[test]
fn valid_birth_date_produces_no_type_error() {
    let registry = load_registry();
    let resource =
        parse_json(r#"{"resourceType":"Patient","id":"p1","birthDate":"1990-06-15"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let type_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_ref().starts_with("TYPE_"))
        .collect();
    assert!(
        type_errors.is_empty(),
        "Unexpected type errors: {type_errors:?}",
    );
}

// ── Combined: valid patient has no errors ─────────────────────────────────────

#[test]
fn validates_valid_patient_returns_no_errors() {
    let registry = load_registry();
    let resource = parse_json(
        r#"{"resourceType":"Patient","id":"example","active":true,"birthDate":"1990-01-01"}"#,
    )
    .unwrap();
    let diagnostics = validate(&resource, &registry);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for valid Patient, got: {errors:#?}",
    );
}

// ── Missing required field has correct path ────────────────────────────────────

#[test]
fn missing_required_field_path_is_correct() {
    let registry = load_registry();
    let resource = parse_json(r#"{"resourceType":"Observation","id":"o1"}"#).unwrap();
    let diagnostics = validate(&resource, &registry);
    let status_error = diagnostics
        .iter()
        .find(|d| d.code.as_ref() == "CARDINALITY_001" && d.path.contains("status"));
    assert!(
        status_error.is_some(),
        "Expected CARDINALITY_001 with path containing 'status'",
    );
}

// ── Terminology validation ────────────────────────────────────────────────────

fn codes(registry: &Registry, json: &str, code: &str) -> usize {
    let resource = parse_json(json).unwrap();
    validate(&resource, registry)
        .iter()
        .filter(|d| d.code.as_ref() == code)
        .count()
}

#[test]
fn invalid_code_with_required_binding_produces_terminology_001() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Observation","status":"bogus","code":{"text":"x"}}"#;
    assert_eq!(codes(&registry, json, "TERMINOLOGY_001"), 1);
}

#[test]
fn valid_code_with_required_binding_passes_terminology() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Observation","status":"final","code":{"text":"x"}}"#;
    assert_eq!(codes(&registry, json, "TERMINOLOGY_001"), 0);
}

// ── Reference validation ──────────────────────────────────────────────────────

#[test]
fn reference_to_unknown_type_produces_reference_001() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Patient","managingOrganization":{"reference":"Bogus/1"}}"#;
    assert_eq!(codes(&registry, json, "REFERENCE_001"), 1);
}

#[test]
fn reference_to_known_type_and_absolute_url_pass() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Patient","generalPractitioner":[{"reference":"Practitioner/1"},{"reference":"http://x.org/anything/2"}]}"#;
    assert_eq!(codes(&registry, json, "REFERENCE_001"), 0);
}

#[test]
fn unresolved_contained_reference_produces_reference_002() {
    let registry = load_registry();
    let json = r##"{"resourceType":"Patient","managingOrganization":{"reference":"#org"}}"##;
    assert_eq!(codes(&registry, json, "REFERENCE_002"), 1);
}

#[test]
fn resolved_contained_reference_passes() {
    let registry = load_registry();
    let json = r##"{"resourceType":"Patient","contained":[{"resourceType":"Organization","id":"org"}],"managingOrganization":{"reference":"#org"}}"##;
    assert_eq!(codes(&registry, json, "REFERENCE_002"), 0);
}

// ── Profile validation ────────────────────────────────────────────────────────

#[test]
fn unknown_profile_produces_profile_001_warning() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Patient","meta":{"profile":["http://example.org/p"]}}"#;
    assert_eq!(codes(&registry, json, "PROFILE_001"), 1);
}

#[test]
fn profile_for_other_resource_type_produces_profile_002() {
    let registry = load_registry();
    let json = r#"{"resourceType":"Patient","meta":{"profile":["http://hl7.org/fhir/StructureDefinition/Observation"]}}"#;
    assert_eq!(codes(&registry, json, "PROFILE_002"), 1);
}

#[test]
fn conditional_and_abstract_references() {
    let registry = load_registry();
    let ok = r#"{"resourceType":"Patient","managingOrganization":{"reference":"Organization?identifier=1"}}"#;
    assert_eq!(codes(&registry, ok, "REFERENCE_001"), 0);
    let abs =
        r#"{"resourceType":"Patient","managingOrganization":{"reference":"DomainResource/1"}}"#;
    assert_eq!(codes(&registry, abs, "REFERENCE_001"), 1);
}

// ── Nested validation ─────────────────────────────────────────────────────────

fn codes_for(json: &str) -> Vec<(String, String)> {
    let resource = parse_json(json).unwrap();
    validate(&resource, &load_registry())
        .into_iter()
        .map(|d| (d.code.to_string(), d.path))
        .collect()
}

#[test]
fn unknown_element_in_complex_type_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"family":"A","bogus":1}]}"#);
    assert!(found.contains(&("STRUCTURE_001".into(), "Patient.name[0].bogus".into())));
}

#[test]
fn unknown_element_in_backbone_element_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","contact":[{"bogus":1}]}"#);
    assert!(found.contains(&("STRUCTURE_001".into(), "Patient.contact[0].bogus".into())));
}

#[test]
fn missing_required_nested_element_is_reported() {
    let found = codes_for(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"component":[{"valueString":"v"}]}"#,
    );
    assert!(found.contains(&(
        "CARDINALITY_001".into(),
        "Observation.component[0].code".into()
    )));
}

#[test]
fn bad_primitive_in_nested_element_is_reported() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"period":{"start":"nope"}}]}"#);
    assert!(
        found
            .iter()
            .any(|(c, p)| c == "TYPE_003" && p == "Patient.name[0].period.start")
    );
}

#[test]
fn valid_nested_resource_has_no_nested_errors() {
    let found = codes_for(
        r#"{"resourceType":"Patient","name":[{"family":"A","given":["B"]}],"contact":[{"name":{"family":"C"}}]}"#,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn content_reference_element_is_not_unknown() {
    let found = codes_for(
        r#"{"resourceType":"Questionnaire","status":"draft","item":[{"linkId":"1","type":"group","item":[{"linkId":"2","type":"string"}]}]}"#,
    );
    assert!(
        !found.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{found:?}"
    );
}

#[test]
fn required_child_present_only_as_primitive_extension_is_not_missing() {
    let found = codes_for(
        r#"{"resourceType":"Questionnaire","status":"draft","item":[{"_linkId":{"extension":[{"url":"http://example.org/x","valueString":"v"}]},"type":"string"}]}"#,
    );
    assert!(
        !found.iter().any(|(code, _)| code == "CARDINALITY_001"),
        "{found:?}"
    );
}
