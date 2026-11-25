//! Audit 2026-11-25: a present element below its `min` (ElementDefinition.min: "minimum
//! number of times this element SHALL appear"), and JSON arrays nested in arrays (R5
//! json.html: no element holds an array as a value), checked against the R5 Registry.
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

/// `"<code> <path>"` for every Error diagnostic of `source`, invariants aside: a finding
/// expected here is thereby checked to be an Error.
fn diagnostics(source: &str) -> Vec<String> {
    let resource = parse_json(source).expect("parses");
    validate(&resource, registry())
        .iter()
        .filter(|d| d.severity == Severity::Error && !d.code.starts_with("INVARIANT_"))
        .map(|d| format!("{} {}", d.code, d.path))
        .collect()
}

fn lipid_report(results: usize) -> String {
    let results: Vec<String> = (0..results)
        .map(|i| format!(r#"{{"reference": "Observation/o{i}"}}"#))
        .collect();
    format!(
        r#"{{"resourceType": "DiagnosticReport",
            "meta": {{"profile": ["http://hl7.org/fhir/StructureDefinition/lipidprofile"]}},
            "status": "final",
            "code": {{"coding": [{{"system": "http://loinc.org", "code": "57698-3"}}]}},
            "result": [{}]}}"#,
        results.join(",")
    )
}

#[test]
fn profile_min_applies_to_a_present_element() {
    let found = diagnostics(&lipid_report(1));
    assert!(
        found.contains(&"CARDINALITY_001 DiagnosticReport.result".to_owned()),
        "{found:?}"
    );
}

#[test]
fn profile_min_met_is_accepted() {
    let found = diagnostics(&lipid_report(3));
    assert!(
        !found.iter().any(|d| d.starts_with("CARDINALITY_001")),
        "{found:?}"
    );
}

#[test]
fn required_element_set_to_null_is_missing() {
    let found = diagnostics(
        r#"{"resourceType": "Observation", "status": null,
            "code": {"text": "x"}}"#,
    );
    assert!(
        found.contains(&"CARDINALITY_001 Observation.status".to_owned()),
        "{found:?}"
    );
}

#[test]
fn required_null_compensated_by_extension_is_present() {
    let found = diagnostics(
        r#"{"resourceType": "Observation", "status": null, "_status": {"id": "a"},
            "code": {"text": "x"}}"#,
    );
    assert!(
        !found.iter().any(|d| d.starts_with("CARDINALITY_001")),
        "{found:?}"
    );
}

#[test]
fn array_of_complex_arrays_is_reported() {
    let found = diagnostics(r#"{"resourceType": "Patient", "name": [[{"family": "x"}]]}"#);
    assert_eq!(found, ["STRUCTURE_002 Patient.name[0]"], "{found:?}");
}

#[test]
fn array_of_primitive_arrays_is_reported() {
    let found = diagnostics(r#"{"resourceType": "Patient", "name": [{"given": [["a"]]}]}"#);
    assert_eq!(
        found,
        ["STRUCTURE_002 Patient.name[0].given[0]"],
        "{found:?}"
    );
}

#[test]
fn null_items_do_not_count_towards_min() {
    let source = lipid_report(3).replace(r#""Observation/o1"}"#, r#""Observation/o1"}, null"#);
    let source = source.replacen(r#"{"reference": "Observation/o2"}"#, "null", 1);
    let found = diagnostics(&source);
    assert!(
        found.contains(&"CARDINALITY_001 DiagnosticReport.result".to_owned()),
        "{found:?}"
    );
}

#[test]
fn required_choice_null_with_primitive_extension_is_present() {
    let found = diagnostics(
        r#"{"resourceType": "Immunization", "status": "completed",
            "vaccineCode": {"text": "x"}, "patient": {"reference": "Patient/p"},
            "occurrenceDateTime": null, "_occurrenceDateTime": {"id": "a"}}"#,
    );
    assert!(
        !found.iter().any(|d| d.starts_with("CARDINALITY_001")),
        "{found:?}"
    );
}

#[test]
fn array_of_contained_arrays_is_reported() {
    let found = diagnostics(
        r#"{"resourceType": "Patient", "contained": [[{"resourceType": "Patient", "id": "p"}]]}"#,
    );
    assert_eq!(found, ["STRUCTURE_002 Patient.contained[0]"], "{found:?}");
}

/// The new diagnostics carry their message and the span of the offending node.
#[test]
fn below_min_and_nested_array_messages_and_locations() {
    let source = lipid_report(1);
    let resource = parse_json(&source).expect("parses");
    let found = validate(&resource, registry());
    let below = found
        .iter()
        .find(|d| &*d.code == "CARDINALITY_001")
        .expect("CARDINALITY_001");
    assert_eq!(below.severity, Severity::Error);
    assert_eq!(
        below.message,
        "Element 'DiagnosticReport.result' has 1 value(s) but minimum is 3"
    );
    let offset = below.location.expect("location").offset as usize;
    assert!(source[offset..].starts_with('['), "{below:?}");

    let source = r#"{"resourceType": "Patient", "name": [[{"family": "x"}]]}"#;
    let resource = parse_json(source).expect("parses");
    let found = validate(&resource, registry());
    let nested = found
        .iter()
        .find(|d| &*d.code == "STRUCTURE_002")
        .expect("STRUCTURE_002");
    assert_eq!(nested.severity, Severity::Error);
    assert_eq!(
        nested.message,
        "Element 'Patient.name[0]' is a JSON array inside an array"
    );
    let offset = nested.location.expect("location").offset as usize;
    assert!(source[offset..].starts_with("[{"), "{nested:?}");
}

#[test]
fn nested_array_items_do_not_count_towards_min() {
    let source = (0..3).fold(lipid_report(3), |source, i| {
        let item = format!(r#"{{"reference": "Observation/o{i}"}}"#);
        source.replace(&item, &format!("[{item}]"))
    });
    let found = diagnostics(&source);
    assert!(
        found.contains(&"CARDINALITY_001 DiagnosticReport.result".to_owned()),
        "{found:?}"
    );
}

#[test]
fn extension_past_the_value_array_does_not_count() {
    let found = diagnostics(
        r#"{"resourceType": "Observation", "status": null, "_status": [null, {"id": "a"}],
            "code": {"text": "x"}}"#,
    );
    assert!(
        found.contains(&"CARDINALITY_001 Observation.status".to_owned()),
        "{found:?}"
    );
}

/// `_name` arrays are walked as `Element`s, not shape-checked: an array item there is
/// already a `TYPE_001` (not an Element object), never silent.
#[test]
fn array_in_a_primitive_extension_array_is_reported() {
    let found = diagnostics(
        r#"{"resourceType": "Patient", "name": [{"given": ["a"], "_given": [[{"id": "x"}]]}]}"#,
    );
    assert!(
        found.contains(&"TYPE_001 Patient.name[0]._given[0]".to_owned()),
        "{found:?}"
    );
}
