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
fn url_of_a_base_definition_of_another_type_is_a_conflict() {
    let dir = TempDir::new("conflict_base_cross_type");
    // (base url and its type, supplied resource of another type)
    let cases = [
        (
            "http://hl7.org/fhir/StructureDefinition/Patient",
            value_set(),
        ),
        (
            "http://hl7.org/fhir/ValueSet/administrative-gender",
            code_system(),
        ),
        (
            "http://hl7.org/fhir/administrative-gender",
            concept_map(json!([])),
        ),
        (
            "http://hl7.org/fhir/ConceptMap/cm-address-use-v2",
            profile(PROFILE),
        ),
    ];
    for (base_url, mut resource) in cases {
        resource["url"] = json!(base_url);
        let path = dir.write("extra.json", &resource);
        let error = Registry::with_extra(Arc::clone(&BASE), &[&path])
            .err()
            .unwrap();
        assert!(
            matches!(&error, Error::Conflict { url, file }
                if url == base_url && file.ends_with("extra.json")),
            "{base_url}: {error}"
        );
    }
}

#[test]
fn url_shared_by_two_extras_of_different_types_is_a_conflict() {
    let dir = TempDir::new("conflict_extras_cross_type");
    let first = dir.write("a.json", &code_system());
    let mut vs = value_set();
    vs["url"] = json!(CODE_SYSTEM);
    let second = dir.write("b.json", &vs);
    let error = Registry::with_extra(Arc::clone(&BASE), &[&first, &second])
        .err()
        .unwrap();
    assert!(
        matches!(
            &error,
            Error::Conflict { url, file } if url == CODE_SYSTEM && file.ends_with("b.json")
        ),
        "{error}"
    );
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

const CONCEPT_MAP: &str = "http://example.org/ConceptMap/colours";

/// A ConceptMap from the colours CodeSystem with the given groups.
fn concept_map(groups: Value) -> Value {
    json!({
        "resourceType": "ConceptMap",
        "url": CONCEPT_MAP,
        "version": "1",
        "group": groups,
    })
}

fn overlay_with(name: &str, resources: &[Value]) -> Registry {
    let dir = TempDir::new(name);
    let file = dir.write("bundle.json", &bundle(resources));
    Registry::with_extra(Arc::clone(&BASE), &[&file]).unwrap()
}

#[test]
fn overlay_concept_map_is_translated() {
    let overlay = overlay_with(
        "concept_map",
        &[concept_map(json!([{
            "source": CODE_SYSTEM,
            "target": "http://example.org/hex",
            "element": [
                {"code": "red", "target": [
                    {"code": "#f00", "relationship": "equivalent"},
                    {"code": "#e00", "relationship": "source-is-narrower-than-target"},
                ]},
                {"code": "blue", "noMap": true},
            ],
        }]))],
    );
    assert_eq!(
        overlay.translate(CONCEPT_MAP, Some(CODE_SYSTEM), "red"),
        Some(vec![
            ("http://example.org/hex", "#f00", "equivalent"),
            (
                "http://example.org/hex",
                "#e00",
                "source-is-narrower-than-target"
            ),
        ])
    );
    assert_eq!(overlay.translate(CONCEPT_MAP, None, "blue"), Some(vec![]));
    // The base maps stay reachable through the overlay.
    assert!(
        overlay
            .translate(
                "http://hl7.org/fhir/ConceptMap/cm-address-use-v2",
                None,
                "home"
            )
            .is_some()
    );
    assert_eq!(BASE.translate(CONCEPT_MAP, None, "red"), None);
}

#[test]
fn translate_without_a_sure_answer_is_none() {
    let group = |source: &str, extra: Value| {
        let mut group = json!({
            "source": source,
            "target": "http://example.org/hex",
            "element": [{"code": "red", "target": [
                {"code": "#f00", "relationship": "equivalent"},
            ]}],
        });
        group
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        group
    };
    // Unmapped codes follow `group.unmapped`, which is not applied.
    let unmapped = overlay_with(
        "cm_unmapped",
        &[concept_map(json!([group(
            CODE_SYSTEM,
            json!({"unmapped": {"mode": "use-source-code"}})
        )]))],
    );
    assert_eq!(unmapped.translate(CONCEPT_MAP, None, "green"), None);
    assert!(unmapped.translate(CONCEPT_MAP, None, "red").is_some());
    // A systemless code is ambiguous across two source systems.
    let two_sources = overlay_with(
        "cm_two_sources",
        &[concept_map(json!([
            group(CODE_SYSTEM, json!({})),
            group("http://example.org/other", json!({})),
        ]))],
    );
    assert_eq!(two_sources.translate(CONCEPT_MAP, None, "red"), None);
    assert_eq!(
        two_sources.translate(CONCEPT_MAP, Some(CODE_SYSTEM), "red"),
        Some(vec![("http://example.org/hex", "#f00", "equivalent")])
    );
    // A target conditional on `dependsOn` cannot be decided from the code alone.
    let mut conditional = group(CODE_SYSTEM, json!({}));
    conditional["element"][0]["target"][0]["dependsOn"] =
        json!([{"attribute": "x", "valueCode": "y"}]);
    let depends = overlay_with("cm_depends_on", &[concept_map(json!([conditional]))]);
    assert_eq!(depends.translate(CONCEPT_MAP, None, "red"), None);
    // A target without relationship hides part of the answer.
    let mut partial = group(CODE_SYSTEM, json!({}));
    partial["element"][0]["target"]
        .as_array_mut()
        .unwrap()
        .push(json!({"code": "#e00"}));
    let partial = overlay_with("cm_partial", &[concept_map(json!([partial]))]);
    assert_eq!(partial.translate(CONCEPT_MAP, None, "red"), None);
    // An element keyed by valueSet may cover an unlisted code.
    let mut by_value_set = group(CODE_SYSTEM, json!({}));
    by_value_set["element"]
        .as_array_mut()
        .unwrap()
        .push(json!({"valueSet": VALUE_SET, "target": []}));
    let by_value_set = overlay_with("cm_value_set", &[concept_map(json!([by_value_set]))]);
    assert_eq!(by_value_set.translate(CONCEPT_MAP, None, "green"), None);
    assert!(by_value_set.translate(CONCEPT_MAP, None, "red").is_some());
    // A match cannot be named without the group's target system.
    let mut no_target = group(CODE_SYSTEM, json!({}));
    no_target.as_object_mut().unwrap().remove("target");
    let no_target = overlay_with("cm_no_target", &[concept_map(json!([no_target]))]);
    assert_eq!(no_target.translate(CONCEPT_MAP, None, "red"), None);
    assert_eq!(
        no_target.translate(CONCEPT_MAP, None, "green"),
        Some(vec![])
    );
}

#[test]
fn concept_map_in_base_is_a_conflict() {
    let dir = TempDir::new("conflict_concept_map");
    let mut map = concept_map(json!([]));
    map["url"] = json!("http://hl7.org/fhir/ConceptMap/cm-address-use-v2");
    let file = dir.write("cm.json", &map);
    let error = Registry::with_extra(Arc::clone(&BASE), &[&file])
        .err()
        .unwrap();
    assert!(matches!(error, Error::Conflict { .. }), "{error}");
}

#[test]
fn expand_follows_imports_and_takes_displays_from_the_compose_or_system() {
    let colours_vs = |url: &str, include: Value| {
        json!({
            "resourceType": "ValueSet",
            "url": url,
            "name": "V",
            "compose": {"include": include},
        })
    };
    let overlay = overlay_with(
        "expand_imports",
        &[
            json!({
                "resourceType": "CodeSystem",
                "url": CODE_SYSTEM,
                "name": "Colours",
                "content": "complete",
                "concept": [
                    {"code": "red", "display": "Red"},
                    {"code": "blue", "concept": [{"code": "navy", "display": "Navy"}]},
                ],
            }),
            colours_vs("http://x/all", json!([{"system": CODE_SYSTEM}])),
            colours_vs(
                "http://x/listed",
                json!([{"system": CODE_SYSTEM, "concept": [
                    {"code": "navy"},
                    {"code": "red", "display": "Rouge"},
                ]}]),
            ),
            // Intersection of two imports, plus a duplicate include.
            colours_vs(
                "http://x/both",
                json!([
                    {"valueSet": ["http://x/all", "http://x/listed"]},
                    {"system": CODE_SYSTEM, "concept": [{"code": "red"}]},
                ]),
            ),
            colours_vs("http://x/cycle", json!([{"valueSet": ["http://x/cycle"]}])),
        ],
    );
    assert_eq!(
        overlay.expand("http://x/all"),
        Some(vec![
            (CODE_SYSTEM, "red", Some("Red")),
            (CODE_SYSTEM, "blue", None),
            (CODE_SYSTEM, "navy", Some("Navy")),
        ])
    );
    assert_eq!(
        overlay.expand("http://x/listed"),
        Some(vec![
            (CODE_SYSTEM, "navy", Some("Navy")),
            (CODE_SYSTEM, "red", Some("Rouge")),
        ])
    );
    assert_eq!(
        overlay.expand("http://x/both"),
        Some(vec![
            (CODE_SYSTEM, "red", Some("Red")),
            (CODE_SYSTEM, "navy", Some("Navy")),
        ])
    );
    assert_eq!(overlay.expand("http://x/cycle"), None);
}

#[test]
fn translate_with_a_system_over_a_sourceless_group_is_none() {
    let overlay = overlay_with(
        "sourceless_group",
        &[concept_map(json!([{
            "target": "http://example.org/hex",
            "element": [{"code": "red", "target": [
                {"code": "#f00", "relationship": "equivalent"},
            ]}],
        }]))],
    );
    assert_eq!(
        overlay.translate(CONCEPT_MAP, Some(CODE_SYSTEM), "red"),
        None
    );
    assert!(overlay.translate(CONCEPT_MAP, None, "red").is_some());
}

#[test]
fn expand_reads_include_concepts_and_case_insensitive_systems_exactly() {
    let mut insensitive = code_system();
    insensitive["caseSensitive"] = json!(false);
    let mut malformed = value_set();
    malformed["url"] = json!("http://example.org/ValueSet/malformed");
    malformed["compose"]["include"][0]["concept"] = json!([{"display": "no code"}]);
    let overlay = overlay_with("expand_edges", &[insensitive, value_set(), malformed]);
    // An enumerated, complete system lists exactly even when its codes match case-blind.
    assert_eq!(
        overlay.expand(VALUE_SET),
        Some(vec![
            (CODE_SYSTEM, "red", None),
            (CODE_SYSTEM, "blue", None)
        ])
    );
    // A concept list that cannot be read is not "the whole system".
    let malformed_url = "http://example.org/ValueSet/malformed";
    assert_eq!(overlay.expand(malformed_url), None);
    assert_eq!(
        overlay.value_set_contains(malformed_url, Some(CODE_SYSTEM), "red"),
        None
    );
}

#[test]
fn compose_inactive_false_is_undecided_but_true_expands() {
    let mut active_only = value_set();
    active_only["compose"]["inactive"] = json!(false);
    let mut all_codes = value_set();
    all_codes["url"] = json!("http://example.org/ValueSet/all-codes");
    all_codes["compose"]["inactive"] = json!(true);
    let overlay = overlay_with("compose_inactive", &[code_system(), active_only, all_codes]);
    // R5 compose.inactive=false drops inactive codes; the Registry reads no concept status.
    assert_eq!(overlay.expand(VALUE_SET), None);
    assert_eq!(
        overlay.value_set_contains(VALUE_SET, Some(CODE_SYSTEM), "red"),
        None
    );
    assert!(
        overlay
            .expand("http://example.org/ValueSet/all-codes")
            .is_some()
    );
}

#[test]
fn translate_declines_a_group_pinned_to_a_version() {
    for (name, key) in [("cm_source_ver", "source"), ("cm_target_ver", "target")] {
        let mut group = json!({
            "source": CODE_SYSTEM,
            "target": "http://example.org/hex",
            "element": [{"code": "red", "target": [
                {"code": "#f00", "relationship": "equivalent"},
            ]}],
        });
        group[key] = json!(format!("{}|2", group[key].as_str().unwrap()));
        let overlay = overlay_with(name, &[concept_map(json!([group]))]);
        assert_eq!(overlay.translate(CONCEPT_MAP, None, "red"), None, "{key}");
    }
}
