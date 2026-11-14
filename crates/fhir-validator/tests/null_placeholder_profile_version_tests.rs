//! Audit 11.6.3 V1 (null placeholders in repeating primitives) and V10 (`meta.profile`
//! version), checked against the R5 Registry.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{Resource, parse_json, parse_xml};
use fhir_validator::validate;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// `"<severity> <code> <path>"` for every diagnostic of `resource`, invariants aside.
fn diagnostics(resource: &Resource) -> Vec<String> {
    validate(resource, registry())
        .iter()
        .filter(|d| !d.code.starts_with("INVARIANT_"))
        .map(|d| format!("{:?} {} {}", d.severity, d.code, d.path))
        .collect()
}

fn json_patient(name: &str) -> Vec<String> {
    let source = format!(r#"{{"resourceType": "Patient", "name": [{name}]}}"#);
    diagnostics(&parse_json(&source).expect("parses"))
}

const EXTENSION: &str = r#"{"url": "http://example.org/e", "valueString": "x"}"#;

#[test]
fn json_null_placeholder_with_extension_is_accepted() {
    let found = json_patient(&format!(
        r#"{{"given": ["Jim", null], "_given": [null, {{"extension": [{EXTENSION}]}}]}}"#
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn json_null_without_extension_is_still_an_error() {
    let found = json_patient(r#"{"given": ["Jim", null]}"#);
    assert!(
        found.contains(&"Error TYPE_001 Patient.name[0].given[1]".to_owned()),
        "{found:?}"
    );
}

#[test]
fn json_null_whose_extension_slot_is_null_is_still_an_error() {
    let found = json_patient(&format!(
        r#"{{"given": [null, "Jim"], "_given": [null, {{"extension": [{EXTENSION}]}}]}}"#
    ));
    assert!(
        found.contains(&"Error TYPE_001 Patient.name[0].given[0]".to_owned()),
        "{found:?}"
    );
}

#[test]
fn json_null_beyond_a_short_extension_array_is_still_an_error() {
    let found = json_patient(&format!(
        r#"{{"given": ["Jim", null], "_given": [{{"extension": [{EXTENSION}]}}]}}"#
    ));
    assert!(
        found.contains(&"Error TYPE_001 Patient.name[0].given[1]".to_owned()),
        "{found:?}"
    );
}

#[test]
fn json_null_whose_extension_is_an_empty_object_is_still_an_error() {
    // json.html: objects are never empty, so `{}` carries no id or extension.
    let found = json_patient(r#"{"given": ["Jim", null], "_given": [null, {}]}"#);
    assert!(
        found.contains(&"Error TYPE_001 Patient.name[0].given[1]".to_owned()),
        "{found:?}"
    );
}

#[test]
fn json_null_placeholder_is_accepted_at_the_resource_root() {
    let source = format!(
        r#"{{"resourceType": "Questionnaire", "status": "draft",
        "subjectType": ["Patient", null], "_subjectType": [null, {{"extension": [{EXTENSION}]}}]}}"#
    );
    let found = diagnostics(&parse_json(&source).expect("parses"));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn xml_repeating_primitive_with_only_an_extension_is_accepted() {
    let source = r#"<Patient xmlns="http://hl7.org/fhir"><name><given value="Jim"/><given>
        <extension url="http://example.org/e"><valueString value="x"/></extension>
        </given></name></Patient>"#;
    let found = diagnostics(&parse_xml(source).expect("parses"));
    assert!(found.is_empty(), "{found:?}");
}

fn vitalsigns_observation(profile: &str) -> Vec<String> {
    let source = format!(
        r#"{{"resourceType": "Observation", "meta": {{"profile": ["{profile}"]}},
        "status": "final", "code": {{"text": "x"}}}}"#
    );
    diagnostics(&parse_json(&source).expect("parses"))
}

const VITALSIGNS: &str = "http://hl7.org/fhir/StructureDefinition/vitalsigns";

#[test]
fn profile_version_other_than_the_loaded_one_warns_and_validates() {
    let found = vitalsigns_observation(&format!("{VITALSIGNS}|1.0.0"));
    assert!(
        found.contains(&"Warning PROFILE_003 Observation.meta.profile[0]".to_owned()),
        "{found:?}"
    );
    // The profile's own rules still apply: vitalsigns requires `subject`.
    assert!(
        found.contains(&"Error CARDINALITY_001 Observation.subject".to_owned()),
        "{found:?}"
    );
}

#[test]
fn profile_version_matching_the_loaded_one_does_not_warn() {
    let profiles = [
        format!("{VITALSIGNS}|5.0.0"),
        // references.html: a canonical may end in `#fragment` after its version.
        format!("{VITALSIGNS}|5.0.0#vs"),
        VITALSIGNS.to_owned(),
    ];
    for profile in profiles {
        let found = vitalsigns_observation(&profile);
        // No unknown-profile or version warning (the missing VSCat category is PROFILE_006).
        let warned = |d: &String| d.contains("PROFILE_001") || d.contains("PROFILE_003");
        assert!(!found.iter().any(warned), "{found:?}");
        assert!(
            found.contains(&"Error CARDINALITY_001 Observation.subject".to_owned()),
            "{found:?}"
        );
    }
}
