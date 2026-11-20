use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use serde_json::{Value, json};

use crate::{Error, Registry};

static BASE: LazyLock<Arc<Registry>> = LazyLock::new(|| {
    Registry::shared(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5"))
        .unwrap()
});

const PROFILE: &str = "http://example.org/StructureDefinition/my-patient";
const VALUE_SET: &str = "http://example.org/ValueSet/colours";
const CODE_SYSTEM: &str = "http://example.org/CodeSystem/colours";

/// A fresh directory removed on drop; pid- and name-scoped against collisions.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("{}_overlay_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, resource: &Value) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, resource.to_string()).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A constraint on Patient with a one-element snapshot declaring a choice element.
fn profile(url: &str) -> Value {
    json!({
        "resourceType": "StructureDefinition",
        "url": url,
        "name": "MyPatient",
        "kind": "resource",
        "derivation": "constraint",
        "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Patient",
        "snapshot": {"element": [
            {"path": "Patient", "min": 0, "max": "*"},
            {"path": "Patient.colour[x]", "min": 0, "max": "1",
             "type": [{"code": "Ratio"}]},
        ]},
    })
}

fn value_set() -> Value {
    json!({
        "resourceType": "ValueSet",
        "url": VALUE_SET,
        "name": "Colours",
        "compose": {"include": [{"system": CODE_SYSTEM}]},
    })
}

fn code_system() -> Value {
    json!({
        "resourceType": "CodeSystem",
        "url": CODE_SYSTEM,
        "name": "Colours",
        "content": "complete",
        "concept": [{"code": "red"}, {"code": "blue"}],
    })
}

fn bundle(resources: &[Value]) -> Value {
    let entry: Vec<Value> = resources.iter().map(|r| json!({"resource": r})).collect();
    json!({"resourceType": "Bundle", "type": "collection", "entry": entry})
}

#[test]
fn overlay_finds_both_layers() {
    let dir = TempDir::new("both_layers");
    dir.write("profile.json", &profile(PROFILE));
    dir.write("terminology.json", &bundle(&[value_set(), code_system()]));
    let overlay = Registry::with_extra(Arc::clone(&BASE), &[&dir.0]).unwrap();

    assert!(overlay.structure_definition(PROFILE).is_some());
    assert!(overlay.core("Patient").is_some());
    assert_eq!(
        overlay.value_set_contains(VALUE_SET, Some(CODE_SYSTEM), "red"),
        Some(true)
    );
    assert_eq!(
        overlay.value_set_contains(VALUE_SET, Some(CODE_SYSTEM), "green"),
        Some(false)
    );
    assert!(
        overlay
            .value_set("http://hl7.org/fhir/ValueSet/administrative-gender")
            .is_some()
    );
    assert!(overlay.is_choice_variant("colour", "Ratio"));
    assert!(overlay.is_choice_variant("value", "Quantity"));
    assert_eq!(
        overlay.type_ancestors("code").collect::<Vec<_>>(),
        BASE.type_ancestors("code").collect::<Vec<_>>(),
    );
}

#[test]
fn base_is_unchanged_by_an_overlay() {
    let counts = (BASE.structure_definition_count(), BASE.value_set_count());
    let dir = TempDir::new("base_unchanged");
    let file = dir.write("profile.json", &profile(PROFILE));
    let overlay = Registry::with_extra(Arc::clone(&BASE), &[&file]).unwrap();

    assert!(BASE.structure_definition(PROFILE).is_none());
    assert!(!BASE.is_choice_variant("colour", "Ratio"));
    assert_eq!(
        (BASE.structure_definition_count(), BASE.value_set_count()),
        counts
    );
    let patient = BASE.core("Patient").unwrap();
    assert!(std::ptr::eq(overlay.core("Patient").unwrap(), patient));
    let shared =
        Registry::shared(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5"))
            .unwrap();
    assert!(Arc::ptr_eq(&shared, &BASE));
}

#[test]
fn url_already_in_base_is_a_conflict() {
    let dir = TempDir::new("conflict_base");
    let file = dir.write(
        "patient.json",
        &profile("http://hl7.org/fhir/StructureDefinition/Patient"),
    );
    let error = Registry::with_extra(Arc::clone(&BASE), &[&file])
        .err()
        .unwrap();
    assert!(
        matches!(&error, Error::Conflict { url, file }
            if url == "http://hl7.org/fhir/StructureDefinition/Patient"
                && file.ends_with("patient.json")),
        "{error}"
    );
}

#[test]
fn url_twice_among_extras_is_a_conflict() {
    let dir = TempDir::new("conflict_extras");
    let first = dir.write("a.json", &profile(PROFILE));
    let second = dir.write("b.json", &bundle(&[profile(PROFILE)]));
    let error = Registry::with_extra(Arc::clone(&BASE), &[&first, &second])
        .err()
        .unwrap();
    assert!(
        matches!(
            &error,
            Error::Conflict { url, file } if url == PROFILE && file.ends_with("b.json")
        ),
        "{error}"
    );
}

#[test]
fn specialization_is_rejected() {
    let mut custom = profile("http://example.org/StructureDefinition/Custom");
    custom["derivation"] = json!("specialization");
    let dir = TempDir::new("specialization");
    let file = dir.write("custom.json", &custom);
    let error = Registry::with_extra(Arc::clone(&BASE), &[&file])
        .err()
        .unwrap();
    assert!(matches!(error, Error::Unsupported { .. }), "{error}");
}

#[test]
fn snapshotless_definition_is_recorded_not_stored() {
    let mut differential_only = profile(PROFILE);
    differential_only
        .as_object_mut()
        .unwrap()
        .remove("snapshot");
    let dir = TempDir::new("needs_snapshot");
    let file = dir.write("profile.json", &differential_only);
    let overlay = Registry::with_extra(Arc::clone(&BASE), &[&file]).unwrap();

    assert!(overlay.structure_definition(PROFILE).is_none());
    assert!(overlay.needs_snapshot(PROFILE));
    assert!(!BASE.needs_snapshot(PROFILE));
}

#[test]
fn oversize_file_is_rejected_by_name() {
    let dir = TempDir::new("oversize");
    let path = dir.0.join("huge.json");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(crate::MAX_INPUT_BYTES + 1)
        .unwrap();
    let error = Registry::with_extra(Arc::clone(&BASE), &[&path])
        .err()
        .unwrap();
    assert!(
        matches!(&error, Error::Read { file, source }
            if file.ends_with("huge.json") && source.kind() == std::io::ErrorKind::FileTooLarge),
        "{error}"
    );
}

#[test]
fn directory_skips_non_json_files() {
    let dir = TempDir::new("non_json");
    dir.write("profile.json", &profile(PROFILE));
    std::fs::write(dir.0.join("notes.txt"), "not json").unwrap();
    let overlay = Registry::with_extra(Arc::clone(&BASE), &[&dir.0]).unwrap();
    assert!(overlay.structure_definition(PROFILE).is_some());
}

#[cfg(unix)]
#[test]
fn directory_skips_symlinks() {
    let outside = TempDir::new("symlink_target");
    let target = outside.write("profile.json", &profile(PROFILE));
    let dir = TempDir::new("symlink");
    std::os::unix::fs::symlink(&target, dir.0.join("link.json")).unwrap();
    let overlay = Registry::with_extra(Arc::clone(&BASE), &[&dir.0]).unwrap();
    assert!(overlay.structure_definition(PROFILE).is_none());
}

#[test]
fn non_json_file_is_unsupported() {
    let dir = TempDir::new("non_json_file");
    let path = dir.0.join("notes.txt");
    std::fs::write(&path, "x").unwrap();
    let error = Registry::with_extra(Arc::clone(&BASE), &[&path])
        .err()
        .unwrap();
    assert!(matches!(error, Error::Unsupported { .. }), "{error}");
}

#[cfg(unix)]
#[test]
fn fifo_is_unsupported_without_blocking() {
    let dir = TempDir::new("fifo");
    let path = dir.0.join("pipe.json");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let error = Registry::with_extra(Arc::clone(&BASE), &[&path])
        .err()
        .unwrap();
    assert!(matches!(error, Error::Unsupported { .. }), "{error}");
}

#[test]
fn directory_over_file_limit_is_a_limit_error() {
    let dir = TempDir::new("file_limit");
    for index in 0..=10_000 {
        std::fs::File::create(dir.0.join(format!("{index}.json"))).unwrap();
    }
    let error = Registry::with_extra(Arc::clone(&BASE), &[&dir.0])
        .err()
        .unwrap();
    assert!(matches!(error, Error::Limit { .. }), "{error}");
}

#[test]
fn total_size_over_cap_is_a_limit_error() {
    let dir = TempDir::new("total_limit");
    // One file under the per-file cap, given five times, passes 256 MiB. Reads before the
    // cap is hit are parsed, so it is `{}` padded with whitespace.
    let mut content = vec![b' '; 60 * 1024 * 1024];
    content[..2].copy_from_slice(b"{}");
    let file = dir.0.join("padded.json");
    std::fs::write(&file, content).unwrap();
    let error = Registry::with_extra(Arc::clone(&BASE), &[&file; 5])
        .err()
        .unwrap();
    assert!(
        matches!(&error, Error::Limit { file, .. } if file.ends_with("padded.json")),
        "{error}"
    );
}

#[test]
fn bundle_without_entries_loads_nothing() {
    let dir = TempDir::new("empty_bundle");
    let file = dir.write(
        "bundle.json",
        &json!({"resourceType": "Bundle", "type": "collection"}),
    );
    let overlay = Registry::with_extra(Arc::clone(&BASE), &[&file]).unwrap();
    assert!(overlay.structure_definition(PROFILE).is_none());
}

#[test]
fn value_set_and_code_system_in_base_are_conflicts() {
    let dir = TempDir::new("conflict_terminology");
    let mut vs = value_set();
    vs["url"] = json!("http://hl7.org/fhir/ValueSet/administrative-gender");
    let mut cs = code_system();
    cs["url"] = json!("http://hl7.org/fhir/administrative-gender");
    for (name, resource) in [("vs.json", vs), ("cs.json", cs)] {
        let file = dir.write(name, &resource);
        let error = Registry::with_extra(Arc::clone(&BASE), &[&file])
            .err()
            .unwrap();
        assert!(
            matches!(&error, Error::Conflict { file, .. } if file.ends_with(name)),
            "{name}: {error}"
        );
    }
}

#[test]
fn bundle_entry_that_is_not_an_array_is_malformed() {
    let dir = TempDir::new("entry_not_array");
    let file = dir.write(
        "bundle.json",
        &json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": {"resource": profile(PROFILE)},
        }),
    );
    let error = Registry::with_extra(Arc::clone(&BASE), &[&file])
        .err()
        .unwrap();
    assert!(matches!(error, Error::MalformedBundle { .. }), "{error}");
}
