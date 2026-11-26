//! Profile layer: type profiles, constraint identity and slice conformance cost.
use super::*;

thread_local! {
    /// Conformance checks `check_slices` actually ran (not answered from its memo).
    pub(super) static CONFORMS_RUNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

const NESTED: &str = "http://example.org/StructureDefinition/nested";
const NESTED_LAYER: &str = "http://example.org/StructureDefinition/nested-layer";

fn element(path: &str, min: u32, max: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut e = serde_json::json!({"id": path, "path": path, "min": min, "max": max});
    e.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    e
}

fn profile_sd(
    url: &str,
    ty: &str,
    kind: &str,
    elements: Vec<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "resourceType": "StructureDefinition",
        "url": url,
        "name": "Nested",
        "status": "draft",
        "kind": kind,
        "abstract": false,
        "type": ty,
        "baseDefinition": format!("http://hl7.org/fhir/StructureDefinition/{ty}"),
        "derivation": "constraint",
        "snapshot": {"element": elements},
    })
}

/// `NESTED`: an Extension whose `extension` is sliced by profile into `a` (required) and
/// `b`, both `NESTED` again, so a leaf without `a` fails every level above it and each
/// level tries both slices. `NESTED_LAYER`: Patient with an extension slice of `NESTED`.
fn nested_registry() -> Registry {
    let slicing = serde_json::json!({"slicing": {
        "discriminator": [{"type": "profile", "path": "$this"}], "rules": "open"}});
    let typed = serde_json::json!({"type": [{"code": "Extension", "profile": [NESTED]}]});
    let slice = |name: &str, min: u32| {
        let mut e = element("Extension.extension", min, "*", typed.clone());
        e["id"] = serde_json::json!(format!("Extension.extension:{name}"));
        e["sliceName"] = serde_json::json!(name);
        e
    };
    let nested = profile_sd(
        NESTED,
        "Extension",
        "complex-type",
        vec![
            element("Extension", 0, "*", serde_json::json!({})),
            element("Extension.extension", 0, "*", slicing.clone()),
            slice("a", 1),
            slice("b", 0),
            element("Extension.url", 1, "1", serde_json::json!({})),
            element("Extension.value[x]", 0, "1", serde_json::json!({})),
        ],
    );
    let mut x = element("Patient.extension", 1, "*", typed);
    x["id"] = serde_json::json!("Patient.extension:x");
    x["sliceName"] = serde_json::json!("x");
    let layer = profile_sd(
        NESTED_LAYER,
        "Patient",
        "resource",
        vec![
            element("Patient", 0, "*", serde_json::json!({})),
            element("Patient.extension", 0, "*", slicing),
            x,
        ],
    );
    let bundle = serde_json::json!({"resourceType": "Bundle", "type": "collection",
        "entry": [{"resource": nested}, {"resource": layer}]});
    supplied("nested-slices", &bundle)
}

/// Conformance checks run validating a Patient with extensions nested `depth` deep, and
/// the `(code, path)` of its errors.
fn conforms_runs(registry: &Registry, depth: usize) -> (usize, Vec<(String, String)>) {
    let mut ext = serde_json::json!({"url": NESTED, "valueString": "leaf"});
    for _ in 1..depth {
        ext = serde_json::json!({"url": NESTED, "extension": [ext]});
    }
    let patient = serde_json::json!({"resourceType": "Patient",
        "meta": {"profile": [NESTED_LAYER]}, "extension": [ext]});
    let resource = fhir_parser::parse_json(&patient.to_string()).unwrap();
    CONFORMS_RUNS.with(|runs| runs.set(0));
    let errors = crate::validate(&resource, registry)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.code.to_string(), d.path))
        .collect();
    (CONFORMS_RUNS.with(std::cell::Cell::get), errors)
}

#[test]
fn recursive_profile_slices_cost_linear_conformance_checks() {
    let registry = nested_registry();
    let ((shallow, errors), (deep, _)) =
        (conforms_runs(&registry, 8), conforms_runs(&registry, 16));
    assert!(shallow > 0);
    // The leaf lacks the required slice `a`, so no level conforms and the required slice
    // `x` of the Patient stays empty.
    assert_eq!(
        errors,
        [("PROFILE_006".to_owned(), "Patient.extension".to_owned())]
    );
    // Twice the depth must not cost more than about twice the checks (it was 2^depth).
    assert!(
        deep <= 3 * shallow,
        "depth 8: {shallow} checks, depth 16: {deep}"
    );
}

fn constraint(key: &str, expression: &str) -> Constraint {
    let human = Arc::from("human");
    Constraint::new(
        Arc::from(key),
        Arc::from("error"),
        human,
        Some(Arc::from(expression)),
    )
}

fn definitions() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
}

/// The registry with `sd` supplied as one file.
fn supplied(name: &str, sd: &serde_json::Value) -> Registry {
    let dir = std::env::temp_dir().join(format!("profile-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sd.json");
    std::fs::write(&file, sd.to_string()).unwrap();
    let base = Arc::new(Registry::from_definitions_dir(definitions()).unwrap());
    let registry = Registry::with_extra(base, &[file.as_path()]).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    registry
}

/// `(code, severity, message)` of the diagnostics `Patient.maritalStatus` gets (no value)
/// when its CodeableConcept type names the profile `reference`.
fn marital_status_profiled(registry: &Registry, reference: &str) -> Vec<Diagnostic> {
    let mut elem = registry
        .core("Patient")
        .and_then(|sd| sd.element("Patient.maritalStatus"))
        .unwrap()
        .clone();
    elem.type_profiles = vec![fhir_definitions::TypeProfiles {
        code: Arc::from("CodeableConcept"),
        profiles: vec![Arc::from(reference)],
        target_profiles: vec![],
    }];
    let mut diagnostics = Vec::new();
    check_type_profiles(
        (&IndexMap::new(), &[], Span::default()),
        &elem,
        "CodeableConcept",
        "Patient.maritalStatus",
        registry,
        &Environment::default(),
        0,
        &mut diagnostics,
    );
    diagnostics
}

fn summary(diagnostics: &[Diagnostic]) -> Vec<(&str, Severity, &str)> {
    diagnostics
        .iter()
        .map(|d| (d.code.as_ref(), d.severity, d.message.as_str()))
        .collect()
}

const CC: &str = "http://example.org/StructureDefinition/cc";

/// CodeableConcept with `text` required, at version 1.0, supplied in directory `name`.
fn cc_profile(name: &str) -> Registry {
    let core = Registry::from_definitions_dir(definitions()).unwrap();
    let mut elements: Vec<serde_json::Value> = core
        .core("CodeableConcept")
        .unwrap()
        .snapshot
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": &*e.path,
                "path": &*e.path,
                "min": e.min,
                "max": &*e.max,
            })
        })
        .collect();
    for e in &mut elements {
        if e["path"] == "CodeableConcept.text" {
            e["min"] = serde_json::json!(1);
        }
    }
    let sd = serde_json::json!({
        "resourceType": "StructureDefinition",
        "url": CC,
        "version": "1.0",
        "name": "TextCC",
        "status": "draft",
        "kind": "complex-type",
        "abstract": false,
        "type": "CodeableConcept",
        "baseDefinition": "http://hl7.org/fhir/StructureDefinition/CodeableConcept",
        "derivation": "constraint",
        "snapshot": {"element": elements},
    });
    supplied(name, &sd)
}

#[test]
fn type_profile_without_snapshot_is_profile_010() {
    // No core element names a profile the bundle lacks, so only an edited element
    // reaches this path with a supplied snapshotless profile.
    let url = "http://example.org/StructureDefinition/no-snapshot-cc";
    let sd = serde_json::json!({
        "resourceType": "StructureDefinition",
        "url": url,
        "name": "NoSnapshotCC",
        "status": "draft",
        "kind": "complex-type",
        "abstract": false,
        "type": "CodeableConcept",
        "derivation": "constraint",
        "differential": {"element": [{"path": "CodeableConcept"}]},
    });
    let registry = supplied("010-type", &sd);
    for reference in [url.to_owned(), format!("{url}|1.0")] {
        let diagnostics = marital_status_profiled(&registry, &reference);
        // The diagnostic names the reference as written.
        let message = format!("Profile '{reference}' has no snapshot; snapshot required");
        assert_eq!(
            summary(&diagnostics),
            [("PROFILE_010", Severity::Error, message.as_str())]
        );
    }
}

#[test]
fn versioned_type_profile_of_the_loaded_version_is_applied() {
    let registry = cc_profile("cc-loaded");
    let diagnostics = marital_status_profiled(&registry, &format!("{CC}|1.0"));
    let codes: Vec<_> = diagnostics.iter().map(|d| (&*d.code, &*d.path)).collect();
    assert_eq!(codes, [("CARDINALITY_001", "Patient.maritalStatus.text")]);
}

#[test]
fn versioned_type_profile_of_another_version_is_profile_003() {
    let registry = cc_profile("cc-other");
    let reference = format!("{CC}|2.0");
    let diagnostics = marital_status_profiled(&registry, &reference);
    let codes: Vec<_> = diagnostics.iter().map(|d| (&*d.code, &*d.path)).collect();
    // Validated against the loaded version, as for meta.profile.
    assert_eq!(
        codes,
        [
            ("PROFILE_003", "Patient.maritalStatus"),
            ("CARDINALITY_001", "Patient.maritalStatus.text"),
        ]
    );
    assert!(diagnostics[0].message.contains(&format!("'{reference}'")));
    assert_eq!(diagnostics[0].severity, Severity::Warning);
}

#[test]
fn a_redefined_expression_is_another_constraint() {
    let base = constraint("vs-2", "value.exists()");
    assert!(same_constraint(
        &base,
        &constraint("vs-2", "value.exists()")
    ));
    assert!(!same_constraint(
        &base,
        &constraint("vs-2", "value.exists() and code.exists()")
    ));
    assert!(!same_constraint(
        &base,
        &constraint("vs-3", "value.exists()")
    ));
}
