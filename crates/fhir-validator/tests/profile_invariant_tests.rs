//! The constraints of a `meta.profile` profile (vitalsigns `vs-1`, `vs-2`, `vs-3`), at the
//! root, on a choice element and in a backbone element, against the bundled R5 definitions.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Diagnostic, Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

fn diagnostics(json: &str) -> Vec<Diagnostic> {
    validate(&parse_json(json).expect("test JSON parses"), registry())
}

/// `code path` of every invariant error.
fn invariant_errors(json: &str) -> Vec<String> {
    diagnostics(json)
        .into_iter()
        .filter(|d| d.code.starts_with("INVARIANT_") && d.severity == Severity::Error)
        .map(|d| format!("{} {}", d.code, d.path))
        .collect()
}

/// A vitalsigns heart rate; `rest` replaces its effective and value elements.
fn vital_signs(rest: &str) -> String {
    format!(
        r#"{{"resourceType":"Observation","id":"o",
        "meta":{{"profile":["http://hl7.org/fhir/StructureDefinition/vitalsigns"]}},
        "status":"final","category":[{{"coding":[{{"system":
        "http://terminology.hl7.org/CodeSystem/observation-category","code":"vital-signs"}}]}}],
        "code":{{"coding":[{{"system":"http://loinc.org","code":"8867-4"}}]}},
        "subject":{{"reference":"Patient/p"}},{rest}}}"#
    )
}

const CONFORMING: &str = r#""effectiveDateTime":"2024-01-01",
    "valueQuantity":{"value":60,"unit":"/min","system":"http://unitsofmeasure.org",
    "code":"/min"}"#;

#[test]
fn conforming_vital_signs_has_no_invariant_error() {
    assert_eq!(
        invariant_errors(&vital_signs(CONFORMING)),
        Vec::<String>::new()
    );
}

#[test]
fn vital_signs_without_value_violates_vs_2() {
    let json = vital_signs(r#""effectiveDateTime":"2024-01-01""#);
    assert_eq!(invariant_errors(&json), ["INVARIANT_vs-2 Observation"]);
}

#[test]
fn vital_signs_with_a_year_only_effective_violates_vs_1() {
    let json = vital_signs(&CONFORMING.replace("2024-01-01", "2024"));
    assert_eq!(
        invariant_errors(&json),
        ["INVARIANT_vs-1 Observation.effectiveDateTime"]
    );
}

#[test]
fn vital_signs_component_without_value_violates_vs_3() {
    let component = r#","component":[{"code":{"coding":[{"system":"http://loinc.org",
        "code":"8480-6"}]}}]"#;
    let json = vital_signs(&format!("{CONFORMING}{component}"));
    assert_eq!(
        invariant_errors(&json),
        ["INVARIANT_vs-3 Observation.component[0]"]
    );
}

#[test]
fn inherited_base_constraint_is_reported_once() {
    // obs-6 comes from the base Observation: the profile repeats it, the result must not.
    let reason = r#","dataAbsentReason":{"text":"unknown"}"#;
    let json = vital_signs(&format!("{CONFORMING}{reason}"));
    assert_eq!(invariant_errors(&json), ["INVARIANT_obs-6 Observation"]);
}

#[test]
fn profile_constraint_of_a_bundle_entry_is_reported_at_its_path() {
    let entry = vital_signs(r#""effectiveDateTime":"2024-01-01""#);
    let json = format!(
        r#"{{"resourceType":"Bundle","type":"collection",
        "entry":[{{"fullUrl":"urn:uuid:0c3151bd-1cbf-4d64-b04d-cd9187a4c6e0",
        "resource":{entry}}}]}}"#
    );
    assert_eq!(
        invariant_errors(&json),
        ["INVARIANT_vs-2 Bundle.entry[0].resource"]
    );
}
