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

/// Heart-rate example from the R5 spec (`observation-example-heart-rate`).
const VITAL_SIGNS_HEART_RATE: &str = r#"{
  "resourceType": "Observation",
  "id": "heart-rate",
  "meta": {"profile": ["http://hl7.org/fhir/StructureDefinition/vitalsigns"]},
  "status": "final",
  "category": [{
    "coding": [{
      "system": "http://terminology.hl7.org/CodeSystem/observation-category",
      "code": "vital-signs",
      "display": "Vital Signs"
    }],
    "text": "Vital Signs"
  }],
  "code": {
    "coding": [{"system": "http://loinc.org", "code": "8867-4", "display": "Heart rate"}],
    "text": "Heart rate"
  },
  "subject": {"reference": "Patient/example"},
  "effectiveDateTime": "1999-07-02",
  "valueQuantity": {
    "value": 44,
    "unit": "beats/minute",
    "system": "http://unitsofmeasure.org",
    "code": "/min"
  }
}"#;

#[test]
fn core_vitalsigns_profile_is_known() {
    let registry = load_registry();
    assert_eq!(codes(&registry, VITAL_SIGNS_HEART_RATE, "PROFILE_001"), 0);
}

#[test]
fn valid_vitalsigns_observation_has_no_errors() {
    let registry = load_registry();
    let resource = parse_json(VITAL_SIGNS_HEART_RATE).unwrap();
    let errors: Vec<_> = validate(&resource, &registry)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn vitalsigns_profile_cardinality_is_enforced() {
    // `subject` is optional on Observation but 1..1 in the vitalsigns profile.
    let json =
        VITAL_SIGNS_HEART_RATE.replace(r#""subject": {"reference": "Patient/example"},"#, "");
    let found = codes_for(&json);
    assert!(
        found.contains(&("CARDINALITY_001".into(), "Observation.subject".into())),
        "{found:?}",
    );
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

#[test]
fn top_level_choice_value_is_type_checked() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""valueBoolean":"yes"}"#
    ));
    assert!(
        found
            .iter()
            .any(|(code, path)| code == "TYPE_001" && path == "Observation.valueBoolean"),
        "{found:?}"
    );
    // Only the suffix-selected type is checked, not every declared type.
    let valid = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""valueBoolean":true}"#
    ));
    assert!(
        !valid.iter().any(|(_, path)| path.contains("value")),
        "{valid:?}"
    );
}

#[test]
fn choice_suffix_must_be_an_allowed_type() {
    let bad = codes_for(r#"{"resourceType":"Observation","status":"final","valueAddress":{}}"#);
    assert!(
        bad.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{bad:?}"
    );
    let ok =
        codes_for(r#"{"resourceType":"Observation","status":"final","code":{},"valueString":"x"}"#);
    assert!(!ok.iter().any(|(_, path)| path.contains("value")), "{ok:?}");
}

#[test]
fn literal_choice_placeholder_key_is_unknown() {
    let top =
        codes_for(r#"{"resourceType":"Observation","status":"final","code":{},"value[x]":"x"}"#);
    assert!(
        top.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{top:?}"
    );
    let nested = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""component":[{"code":{},"value[x]":"x"}]}"#,
    ));
    assert!(
        nested.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{nested:?}"
    );
}

#[test]
fn dotted_top_level_key_is_unknown() {
    let found = codes_for(
        r#"{"resourceType":"Observation","status":"final","code":{},"component.code":{}}"#,
    );
    assert!(
        found.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{found:?}"
    );
}

#[test]
fn top_level_required_primitive_present_only_as_extension_is_not_missing() {
    let found = codes_for(r#"{"resourceType":"Observation","_status":{"id":"a"},"code":{}}"#);
    assert!(
        !found.iter().any(|(code, _)| code == "CARDINALITY_001"),
        "{found:?}"
    );
}

fn assert_has(found: &[(String, String)], wanted_code: &str, wanted_path: &str) {
    assert!(
        found
            .iter()
            .any(|(code, path)| code == wanted_code && path == wanted_path),
        "{found:?}"
    );
}

#[test]
fn required_complex_element_present_only_as_extension_is_missing() {
    let found = codes_for(r#"{"resourceType":"Observation","status":"final","_code":{"id":"a"}}"#);
    assert_has(&found, "CARDINALITY_001", "Observation.code");
}

#[test]
fn primitive_extension_array_over_max_is_cardinality_002() {
    let found = codes_for(r#"{"resourceType":"Patient","_birthDate":[{},{}]}"#);
    assert_has(&found, "CARDINALITY_002", "Patient.birthDate");
}

#[test]
fn punctuated_underscore_keys_are_unknown() {
    let obs = r#"{"resourceType":"Observation","status":"final","code":{},"#;
    for key in ["_a.b", "_value[x]"] {
        let top = codes_for(&format!(r#"{obs}"{key}":{{}}}}"#));
        assert_has(&top, "STRUCTURE_001", &format!("Observation.{key}"));
        let nested = codes_for(&format!(
            r#"{obs}"component":[{{"code":{{}},"{key}":{{}}}}]}}"#
        ));
        assert_has(
            &nested,
            "STRUCTURE_001",
            &format!("Observation.component[0].{key}"),
        );
    }
    let plain = codes_for(&format!(r#"{obs}"_status":{{}}}}"#));
    assert!(
        !plain.iter().any(|(code, _)| code == "STRUCTURE_001"),
        "{plain:?}"
    );
}

#[test]
fn nested_complex_child_present_only_as_extension_is_missing() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{},"#,
        r#""component":[{"_code":{"id":"a"}}]}"#,
    ));
    assert_has(&found, "CARDINALITY_001", "Observation.component[0].code");
}

#[test]
fn nested_primitive_extension_array_over_max_is_cardinality_002() {
    let found = codes_for(r#"{"resourceType":"Patient","contact":[{"_gender":[{},{}]}]}"#);
    assert_has(&found, "CARDINALITY_002", "Patient.contact[0].gender");
}

#[test]
fn nested_integer_beyond_i64_produces_type_001() {
    let found = codes_for(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},
            "component":[{"code":{"text":"y"},"valueInteger":12345678901234567890}]}"#,
    );
    assert_has(&found, "TYPE_001", "Observation.component[0].valueInteger");
}

// ── Resource and element ids ──────────────────────────────────────────────────

#[test]
fn invalid_resource_id_produces_type_005() {
    let found = codes_for(r#"{"resourceType":"Patient","id":"has space!"}"#);
    assert_has(&found, "TYPE_005", "Patient.id");
}

#[test]
fn valid_resource_id_produces_no_type_error() {
    let found = codes_for(r#"{"resourceType":"Patient","id":"p-1.a"}"#);
    assert!(
        !found.iter().any(|(code, _)| code.starts_with("TYPE_")),
        "{found:?}"
    );
}

#[test]
fn non_string_element_id_produces_type_001() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"id":5,"family":"x"}]}"#);
    assert_has(&found, "TYPE_001", "Patient.name[0].id");
}

#[test]
fn element_id_is_a_string_not_an_id() {
    let found = codes_for(r#"{"resourceType":"Patient","name":[{"id":"n:1","family":"x"}]}"#);
    assert!(
        !found.iter().any(|(code, _)| code.starts_with("TYPE_")),
        "{found:?}"
    );
}

// ── Choice element cardinality ────────────────────────────────────────────────

const IMMUNIZATION: &str = concat!(
    r#""resourceType":"Immunization","status":"completed","#,
    r#""vaccineCode":{"text":"x"},"patient":{"reference":"Patient/1"}"#,
);

fn occurrence_errors(extra: &str) -> Vec<(String, String)> {
    codes_for(&format!("{{{IMMUNIZATION}{extra}}}"))
        .into_iter()
        .filter(|(c, p)| c.starts_with("CARDINALITY_") && p.contains("occurrence"))
        .collect()
}

#[test]
fn missing_required_choice_produces_cardinality_001() {
    assert_eq!(
        occurrence_errors(""),
        [(
            "CARDINALITY_001".to_string(),
            "Immunization.occurrence[x]".to_string()
        )]
    );
}

#[test]
fn present_required_choice_produces_no_cardinality_error() {
    assert!(occurrence_errors(r#","occurrenceDateTime":"2021-01-01""#).is_empty());
}

#[test]
fn required_primitive_choice_present_only_as_extension_is_not_missing() {
    let ext = r#","_occurrenceString":{"extension":[{"url":"http://e","valueBoolean":true}]}"#;
    assert!(occurrence_errors(ext).is_empty());
}

#[test]
fn malformed_choice_extension_does_not_satisfy_required_choice() {
    assert_eq!(
        occurrence_errors(r#","_occurrenceString":5"#),
        [(
            "CARDINALITY_001".to_string(),
            "Immunization.occurrence[x]".to_string()
        )]
    );
}

#[test]
fn malformed_primitive_extension_does_not_satisfy_required_element() {
    let found = codes_for(r#"{"resourceType":"Observation","code":{"text":"x"},"_status":5}"#);
    assert_has(&found, "CARDINALITY_001", "Observation.status");
}

#[test]
fn extension_array_without_objects_does_not_satisfy_required_element() {
    for ext in ["[]", "[null]"] {
        let found = codes_for(&format!(
            r#"{{"resourceType":"Observation","code":{{"text":"x"}},"_status":{ext}}}"#
        ));
        assert_has(&found, "CARDINALITY_001", "Observation.status");
        let choice = occurrence_errors(&format!(r#","_occurrenceString":{ext}"#));
        assert_eq!(choice.len(), 1, "{choice:?}");
    }
}

#[test]
fn two_variants_of_one_choice_produce_cardinality_002() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"#,
        r#""valueString":"a","valueBoolean":true}"#,
    ));
    assert!(
        found.contains(&("CARDINALITY_002".into(), "Observation.valueBoolean".into())),
        "{found:?}"
    );
}

#[test]
fn missing_nested_required_choice_produces_cardinality_001() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Claim","status":"active","type":{"text":"x"},"#,
        r#""use":"claim","patient":{"reference":"Patient/1"},"#,
        r#""created":"2021-01-01","diagnosis":[{"sequence":1}]}"#,
    ));
    assert!(
        found.contains(&(
            "CARDINALITY_001".into(),
            "Claim.diagnosis[0].diagnosis[x]".into()
        )),
        "{found:?}"
    );
}

#[test]
fn value_and_extension_of_one_variant_count_once() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"#,
        r#""valueString":"a","_valueString":{"id":"e"}}"#,
    ));
    assert!(
        !found.iter().any(|(c, _)| c == "CARDINALITY_002"),
        "{found:?}"
    );
}

#[test]
fn two_variants_of_a_nested_choice_produce_cardinality_002() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"#,
        r#""component":[{"code":{"text":"c"},"valueString":"a","#,
        r#""valueBoolean":true}]}"#,
    ));
    assert!(
        found.contains(&(
            "CARDINALITY_002".into(),
            "Observation.component[0].valueBoolean".into()
        )),
        "{found:?}"
    );
}

#[test]
fn complex_choice_present_only_as_extension_is_missing() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Claim","status":"active","type":{"text":"x"},"#,
        r#""use":"claim","patient":{"reference":"Patient/1"},"#,
        r#""created":"2021-01-01","diagnosis":[{"sequence":1,"#,
        r#""_diagnosisCodeableConcept":{}}]}"#,
    ));
    assert!(
        found.contains(&(
            "CARDINALITY_001".into(),
            "Claim.diagnosis[0].diagnosis[x]".into()
        )),
        "{found:?}"
    );
}

#[test]
fn variants_of_different_choices_do_not_conflict() {
    let found = codes_for(concat!(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"x"},"#,
        r#""valueString":"a","effectiveDateTime":"2021-01-01"}"#,
    ));
    assert!(
        !found.iter().any(|(c, _)| c == "CARDINALITY_002"),
        "{found:?}"
    );
}
