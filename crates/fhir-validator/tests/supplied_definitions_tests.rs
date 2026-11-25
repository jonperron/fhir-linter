//! Profiles supplied through `Registry::with_extra` without a snapshot: every place that
//! looks a profile up reports `PROFILE_010` instead of an unknown profile or nothing.
//! Versioned references (`url|version`) to supplied profiles resolve on every lookup path.
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
    core_constraint("profiles-resources.json", "Patient", LAYER, edit)
}

/// The core definition of `name` (in bundle `file`) as a constraint at `url`, its snapshot
/// changed by `edit`.
fn core_constraint(file: &str, name: &str, url: &str, edit: impl FnOnce(&mut Vec<Value>)) -> Value {
    let text = std::fs::read_to_string(definitions_dir().join(file)).unwrap();
    let bundle: Value = serde_json::from_str(&text).unwrap();
    let core = format!("http://hl7.org/fhir/StructureDefinition/{name}");
    let mut sd = bundle["entry"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| &entry["resource"])
        .find(|r| r["url"] == core.as_str())
        .unwrap()
        .clone();
    sd["url"] = json!(url);
    sd["derivation"] = json!("constraint");
    sd["baseDefinition"] = json!(core);
    let elements = sd["snapshot"]["element"].as_array_mut().unwrap();
    edit(elements);
    sd
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

/// Every diagnostic of `resource` as `"<severity> <code> <path> <message>"`.
fn all_diagnostics(resource: &Value, registry: &Registry) -> Vec<String> {
    let resource = parse_json(&resource.to_string()).unwrap();
    validate(&resource, registry)
        .iter()
        .map(|d| format!("{:?} {} {} {}", d.severity, d.code, d.path, d.message))
        .collect()
}

#[test]
fn supplied_core_namespace_url_creates_no_resource_type() {
    let zzz = json!({
        "resourceType": "StructureDefinition",
        "url": "http://hl7.org/fhir/StructureDefinition/Zzz",
        "name": "Zzz",
        "status": "draft",
        "kind": "resource",
        "abstract": false,
        "type": "Zzz",
        "baseDefinition": "http://hl7.org/fhir/StructureDefinition/DomainResource",
        "derivation": "constraint",
        "snapshot": {"element": [{"id": "Zzz", "path": "Zzz", "min": 0, "max": "*"}]},
    });
    let registry = overlay("core-namespace", &[zzz]);
    let base = Registry::from_definitions_dir(definitions_dir()).unwrap();
    let resource = json!({"resourceType": "Zzz"});
    let without_extras = all_diagnostics(&resource, &base);
    assert!(
        without_extras
            .iter()
            .any(|d| d.contains("Unknown resource type 'Zzz'")),
        "{without_extras:?}"
    );
    assert_eq!(all_diagnostics(&resource, &registry), without_extras);
}

const IDENTIFIER: &str = "http://example.org/StructureDefinition/identifier-system";

/// Identifier with `system` required, at version 1.0.
fn identifier_profile() -> Value {
    let mut sd = core_constraint(
        "profiles-types.json",
        "Identifier",
        IDENTIFIER,
        |elements| {
            let system = elements
                .iter_mut()
                .find(|e| e["path"] == "Identifier.system")
                .unwrap();
            system["min"] = json!(1);
        },
    );
    sd["version"] = json!("1.0");
    sd
}

/// `LAYER` with `Patient.identifier` sliced by profile, slice `local` (1..*) typed by the
/// profile `reference`.
fn identifier_sliced_by(reference: &str) -> Value {
    patient_layer(|elements| {
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
        slice["min"] = json!(1);
        slice["type"] = json!([{"code": "Identifier", "profile": [reference]}]);
        elements.insert(at + 1, slice);
    })
}

/// `"<severity> <code> <path>"` for every diagnostic that is not information.
fn findings(patient: Value, registry: &Registry) -> Vec<String> {
    let resource = parse_json(&patient.to_string()).unwrap();
    validate(&resource, registry)
        .iter()
        .filter(|d| d.severity != Severity::Information || &*d.code == "SLICING_SKIPPED")
        // The test patients carry no narrative.
        .filter(|d| &*d.code != "INVARIANT_dom-6")
        .map(|d| format!("{:?} {} {}", d.severity, d.code, d.path))
        .collect()
}

fn patient_with_identifier(identifier: Value) -> Value {
    json!({
        "resourceType": "Patient",
        "meta": {"profile": [LAYER]},
        "identifier": [identifier],
    })
}

#[test]
fn versioned_slice_profile_is_evaluated() {
    let reference = format!("{IDENTIFIER}|1.0");
    let registry = overlay(
        "slice-version",
        &[identifier_profile(), identifier_sliced_by(&reference)],
    );
    // The item conforms to the profile, so it fills the required slice.
    let patient = patient_with_identifier(json!({"system": "urn:s", "value": "1"}));
    assert_eq!(findings(patient, &registry), Vec::<String>::new());
}

#[test]
fn versioned_slice_profile_of_another_version_is_profile_003() {
    let reference = format!("{IDENTIFIER}|2.0");
    let registry = overlay(
        "slice-other-version",
        &[identifier_profile(), identifier_sliced_by(&reference)],
    );
    let patient = json!({
        "resourceType": "Patient",
        "meta": {"profile": [LAYER]},
        "identifier": [{"system": "urn:s", "value": "1"}, {"system": "urn:s", "value": "2"}],
    });
    // Named once for the slicing, not per item.
    assert_eq!(
        findings(patient, &registry),
        ["Warning PROFILE_003 Patient.identifier"],
    );
}

#[test]
fn meta_profile_with_a_fragment_names_a_contained_profile() {
    // references.html: `url|version#id` is the resource `id` contained in that version of
    // `url`, not `url` itself; the Registry holds no contained definitions.
    let registry = overlay("fragment", &[patient_layer(|_| {})]);
    let patient = json!({
        "resourceType": "Patient",
        "meta": {"profile": [format!("{LAYER}|1.0#inner")]},
    });
    assert_eq!(
        profile_diagnostics(patient, &registry),
        ["Warning PROFILE_001 Patient.meta.profile[0]"],
    );
}

#[test]
fn versioned_slice_profile_without_snapshot_is_profile_010() {
    let reference = format!("{SNAPSHOTLESS}|1.0");
    let registry = overlay(
        "slice-snapshotless-version",
        &[
            snapshotless_profile("Identifier"),
            identifier_sliced_by(&reference),
        ],
    );
    let patient = patient_with_identifier(json!({"value": "1"}));
    assert_eq!(
        profile_diagnostics(patient, &registry),
        [
            "Information SLICING_SKIPPED Patient.identifier",
            "Error PROFILE_010 Patient.identifier",
        ],
    );
}

#[test]
fn extension_slice_with_a_versioned_profile_matches_the_bare_url() {
    // Extension.url holds the bare canonical: a `|version` on the slice's profile must not
    // stop the `value` discriminator on `url` from matching.
    let extension = "http://example.org/StructureDefinition/ext";
    let layer = patient_layer(|elements| {
        let at = elements
            .iter()
            .position(|e| e["path"] == "Patient.extension")
            .unwrap();
        let mut slice = elements[at].clone();
        elements[at]["slicing"] = json!({
            "discriminator": [{"type": "value", "path": "url"}],
            "rules": "open",
        });
        slice["id"] = json!("Patient.extension:ext");
        slice["sliceName"] = json!("ext");
        slice["min"] = json!(1);
        slice["type"] = json!([{"code": "Extension", "profile": [format!("{extension}|1.0")]}]);
        elements.insert(at + 1, slice);
    });
    let registry = overlay("extension-version", &[layer]);
    let patient = |url: &str| {
        json!({
            "resourceType": "Patient",
            "meta": {"profile": [LAYER]},
            "extension": [{"url": url, "valueString": "x"}],
        })
    };
    assert_eq!(
        findings(patient(extension), &registry),
        Vec::<String>::new()
    );
    // Another URL leaves the required slice empty.
    assert_eq!(
        findings(patient("http://example.org/other"), &registry),
        ["Error PROFILE_006 Patient.extension"],
    );
}
