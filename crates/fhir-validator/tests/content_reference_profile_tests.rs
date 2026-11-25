//! The `contentReferenceProfile` extension on profile elements (DEFINITION_009), on the
//! official `content-reference-profile/sd-*` cases: its value must be a canonical URL for
//! a profile, followed by `#` and the id of an element of that profile.
use std::path::PathBuf;
use std::sync::Arc;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};

fn case_dir() -> PathBuf {
    std::env::var_os("FHIR_FIXTURES_DIR")
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"),
            PathBuf::from,
        )
        .join("validator/content-reference-profile")
}

/// Whether the official fixtures are present; the tests that need them skip otherwise.
fn have_fixtures() -> bool {
    case_dir().is_dir()
}

/// Error paths of DEFINITION_009 on `StructureDefinition-<name>.json`, validated against
/// the R5 registry overlaid with the extension definition and the `supplied` profiles.
fn paths(name: &str, supplied: &[&str]) -> Vec<String> {
    let dir = case_dir();
    let files: Vec<PathBuf> = ["contentReferenceProfile"]
        .iter()
        .chain(supplied)
        .map(|f| dir.join(format!("StructureDefinition-{f}.json")))
        .collect();
    let files: Vec<&std::path::Path> = files.iter().map(PathBuf::as_path).collect();
    let registry = Registry::with_extra(Arc::clone(&base()), &files).unwrap();
    let text =
        std::fs::read_to_string(dir.join(format!("StructureDefinition-{name}.json"))).unwrap();
    errors(&text, &registry)
}

fn base() -> Arc<Registry> {
    Registry::shared(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5"))
        .unwrap()
}

/// Error paths of DEFINITION_009 on the definition `text`.
fn errors(text: &str, registry: &Registry) -> Vec<String> {
    validate(&parse_json(text).unwrap(), registry)
        .into_iter()
        .filter(|d| &*d.code == "DEFINITION_009" && d.severity == Severity::Error)
        .map(|d| d.path)
        .collect()
}

const DIFFERENTIAL: &str = "StructureDefinition.differential.element[0].extension[0]";

#[test]
fn unknown_profile_is_reported() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    assert_eq!(paths("unresolvable-profile", &[]), [DIFFERENTIAL]);
}

#[test]
fn unknown_element_is_reported() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    assert_eq!(paths("unresolvable-element", &[]), [DIFFERENTIAL]);
}

#[test]
fn value_without_hash_is_reported() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    assert_eq!(paths("unresolvable-value", &[]), [DIFFERENTIAL]);
}

#[test]
fn value_without_canonical_is_reported() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    assert_eq!(paths("unresolvable-fragment", &[]), [DIFFERENTIAL]);
}

#[test]
fn snapshot_reference_differing_from_the_differential_is_reported() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    assert_eq!(
        paths("differing-references", &[]),
        ["StructureDefinition.snapshot.element[68].extension[0]"]
    );
}

#[test]
fn snapshot_repeating_the_differential_reference_is_reported_once() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    assert_eq!(
        paths("repeated-reference", &[]),
        ["StructureDefinition.differential.element[1].extension[0]"]
    );
}

#[test]
fn snapshot_only_reference_is_reported() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    assert_eq!(
        paths("snapshot-only-reference", &[]),
        ["StructureDefinition.snapshot.element[68].extension[0]"]
    );
}

#[test]
fn reference_to_the_profile_itself_resolves() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    assert!(paths("reusable-section", &[]).is_empty());
}

#[test]
fn reference_to_a_supplied_profile_resolves() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    // reusable-section has no snapshot: the overlay holds it unresolved, so the check
    // stays silent rather than guess at its elements.
    assert!(paths("guide", &["reusable-section"]).is_empty());
}

/// DEFINITION_009 paths on unresolvable-element with its value replaced by `value`.
fn paths_for_value(value: &str) -> Vec<String> {
    let text =
        std::fs::read_to_string(case_dir().join("StructureDefinition-unresolvable-element.json"))
            .unwrap()
            .replace(&format!("{SELF}#Composition.missing"), value);
    errors(&text, &base())
}

const SELF: &str = "http://example.org/recursive-sections/StructureDefinition/unresolvable-element";

#[test]
fn self_reference_resolves_through_the_base() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    // Composition.title is stated only by the base Composition, not by the differential.
    assert!(paths_for_value(&format!("{SELF}#Composition.title")).is_empty());
    assert!(paths_for_value(&format!("{SELF}|0.1.0#Composition.title")).is_empty());
}

#[test]
fn slice_ids_stay_silent() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    let core = "http://hl7.org/fhir/StructureDefinition/Composition";
    assert!(paths_for_value(&format!("{core}#Composition.section:any")).is_empty());
}

#[test]
fn reference_into_a_registry_profile_is_checked_against_its_elements() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    let core = "http://hl7.org/fhir/StructureDefinition/Composition";
    assert!(paths_for_value(&format!("{core}#Composition.section")).is_empty());
    assert!(paths_for_value(&format!("{core}|5.0.0#Composition.section")).is_empty());
    assert_eq!(
        paths_for_value(&format!("{core}#Composition.missing")),
        [DIFFERENTIAL]
    );
}

#[test]
fn empty_version_is_reported() {
    if !have_fixtures() {
        eprintln!("skipped: fixtures absent");
        return;
    }
    let core = "http://hl7.org/fhir/StructureDefinition/Composition";
    assert_eq!(
        paths_for_value(&format!("{core}|#Composition.section")),
        [DIFFERENTIAL]
    );
}

#[test]
fn self_reference_to_a_missing_id_without_a_base_is_reported() {
    let url = "http://example.org/StructureDefinition/standalone";
    let text = format!(
        r#"{{"resourceType":"StructureDefinition","url":"{url}","name":"standalone",
        "status":"draft","kind":"logical","abstract":false,"type":"standalone",
        "differential":{{"element":[{{"id":"standalone","path":"standalone",
        "extension":[{{"url":"http://hl7.org/fhir/StructureDefinition/contentReferenceProfile",
        "valueUri":"{url}#standalone.missing"}}]}}]}}}}"#
    );
    assert_eq!(
        errors(&text, &base()),
        ["StructureDefinition.differential.element[0].extension[0]"]
    );
}
