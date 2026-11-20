//! Profiles supplied through `Registry::with_extra` without a snapshot: every place that
//! looks a profile up reports `PROFILE_010` instead of an unknown profile or nothing.
use std::path::PathBuf;
use std::sync::Arc;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};
use serde_json::{Value, json};

const SNAPSHOTLESS: &str = "http://example.org/StructureDefinition/no-snapshot";
const LAYER: &str = "http://example.org/StructureDefinition/patient-layer";

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
}

/// A differential-only constraint on `base_type`: it loads without a snapshot.
fn snapshotless_profile(base_type: &str) -> Value {
    json!({
        "resourceType": "StructureDefinition",
        "url": SNAPSHOTLESS,
        "name": "NoSnapshot",
        "status": "draft",
        "kind": "complex-type",
        "abstract": false,
        "type": base_type,
        "baseDefinition": format!("http://hl7.org/fhir/StructureDefinition/{base_type}"),
        "derivation": "constraint",
        "differential": {"element": [{"id": base_type, "path": base_type}]},
    })
}

/// The core Patient definition as a constraint at `LAYER`, its snapshot changed by `edit`.
fn patient_layer(edit: impl FnOnce(&mut Vec<Value>)) -> Value {
    let text = std::fs::read_to_string(definitions_dir().join("profiles-resources.json")).unwrap();
    let bundle: Value = serde_json::from_str(&text).unwrap();
    let mut patient = bundle["entry"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| &entry["resource"])
        .find(|r| r["url"] == "http://hl7.org/fhir/StructureDefinition/Patient")
        .unwrap()
        .clone();
    patient["url"] = json!(LAYER);
    patient["derivation"] = json!("constraint");
    patient["baseDefinition"] = json!("http://hl7.org/fhir/StructureDefinition/Patient");
    let elements = patient["snapshot"]["element"].as_array_mut().unwrap();
    edit(elements);
    patient
}

/// The registry with `resources` supplied as one Bundle file in a fresh directory.
fn overlay(name: &str, resources: &[Value]) -> Registry {
    let dir = std::env::temp_dir().join(format!("supplied-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("definitions.json");
    let entries: Vec<Value> = resources.iter().map(|r| json!({"resource": r})).collect();
    let bundle = json!({"resourceType": "Bundle", "type": "collection", "entry": entries});
    std::fs::write(&file, bundle.to_string()).unwrap();
    let base = Arc::new(Registry::from_definitions_dir(definitions_dir()).unwrap());
    let registry = Registry::with_extra(base, &[file.as_path()]).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    registry
}

/// `"<severity> <code> <path>"` for the profile diagnostics of `patient`.
fn profile_diagnostics(patient: Value, registry: &Registry) -> Vec<String> {
    let resource = parse_json(&patient.to_string()).unwrap();
    validate(&resource, registry)
        .iter()
        .filter(|d| d.code.starts_with("PROFILE_") || &*d.code == "SLICING_SKIPPED")
        .map(|d| format!("{:?} {} {}", d.severity, d.code, d.path))
        .collect()
}

#[test]
fn meta_profile_without_snapshot_is_profile_010() {
    let registry = overlay("meta", &[snapshotless_profile("Patient")]);
    assert!(registry.needs_snapshot(SNAPSHOTLESS));
    let patient = json!({"resourceType": "Patient", "meta": {"profile": [SNAPSHOTLESS]}});
    assert_eq!(
        profile_diagnostics(patient, &registry),
        ["Error PROFILE_010 Patient.meta.profile[0]"],
    );
}

#[test]
fn meta_profile_without_snapshot_message_names_the_profile() {
    let registry = overlay("message", &[snapshotless_profile("Patient")]);
    let patient = json!({"resourceType": "Patient", "meta": {"profile": [SNAPSHOTLESS]}});
    let resource = parse_json(&patient.to_string()).unwrap();
    let diagnostics = validate(&resource, &registry);
    let found = diagnostics
        .iter()
        .find(|d| &*d.code == "PROFILE_010")
        .unwrap();
    assert_eq!(
        found.message,
        format!("Profile '{SNAPSHOTLESS}' has no snapshot; snapshot required"),
    );
}

#[test]
fn supplied_profile_with_snapshot_is_validated() {
    let layer = patient_layer(|elements| {
        let gender = elements
            .iter_mut()
            .find(|e| e["path"] == "Patient.gender")
            .unwrap();
        gender["min"] = json!(1);
    });
    let registry = overlay("snapshot", &[layer]);
    assert!(!registry.needs_snapshot(LAYER));
    let patient = json!({"resourceType": "Patient", "meta": {"profile": [LAYER]}});
    let resource = parse_json(&patient.to_string()).unwrap();
    let found: Vec<_> = validate(&resource, &registry)
        .iter()
        .filter(|d| d.severity == Severity::Error || d.code.starts_with("PROFILE_"))
        .map(|d| format!("{} {}", d.code, d.path))
        .collect();
    // The profile's own rule applies; it is neither unknown nor snapshotless.
    assert_eq!(found, ["CARDINALITY_001 Patient.gender"]);
}

#[test]
fn unknown_meta_profile_stays_profile_001() {
    let registry = overlay("unknown", &[snapshotless_profile("Patient")]);
    let patient = json!({
        "resourceType": "Patient",
        "meta": {"profile": ["http://example.org/StructureDefinition/absent"]},
    });
    assert_eq!(
        profile_diagnostics(patient, &registry),
        ["Warning PROFILE_001 Patient.meta.profile[0]"],
    );
}

#[test]
fn slice_profile_without_snapshot_is_profile_010() {
    let layer = patient_layer(|elements| {
        let at = elements
            .iter()
            .position(|e| e["path"] == "Patient.identifier")
            .unwrap();
        let mut slice = elements[at].clone();
        elements[at]["slicing"] = json!({
            "discriminator": [{"type": "profile", "path": "$this"}],
            "rules": "open",
        });
        slice["id"] = json!("Patient.identifier:local");
        slice["sliceName"] = json!("local");
        slice["type"] = json!([{"code": "Identifier", "profile": [SNAPSHOTLESS]}]);
        elements.insert(at + 1, slice);
    });
    let registry = overlay("slice", &[snapshotless_profile("Identifier"), layer]);
    let patient = json!({
        "resourceType": "Patient",
        "meta": {"profile": [LAYER]},
        "identifier": [{"value": "1"}, {"value": "2"}],
    });
    // The slicing still cannot be evaluated; the profile is named once, not per item.
    assert_eq!(
        profile_diagnostics(patient, &registry),
        [
            "Information SLICING_SKIPPED Patient.identifier",
            "Error PROFILE_010 Patient.identifier",
        ],
    );
}
