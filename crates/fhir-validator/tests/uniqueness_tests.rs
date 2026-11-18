//! Values unique within a resource: Element.id ("Unique id for the element within a
//! resource", narrative.html 2.1.4.0.4: contained resources included) and the Meta.tag /
//! Meta.security sets ("Uniqueness is based the system/code").
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{Resource, parse_json, parse_xml};
use fhir_validator::{Diagnostic, Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

fn fixture(name: &str) -> Resource {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/validator")
        .join(name);
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    parse_xml(&source).expect("parses")
}

fn findings(resource: &Resource, code: &str) -> Vec<Diagnostic> {
    validate(resource, registry())
        .into_iter()
        .filter(|d| &*d.code == code)
        .collect()
}

fn paths(found: &[Diagnostic]) -> Vec<&str> {
    assert!(
        found.iter().all(|d| d.severity == Severity::Error),
        "{found:?}"
    );
    found.iter().map(|d| d.path.as_str()).collect()
}

#[test]
fn element_ids_must_be_unique_within_the_resource_xml() {
    let found = findings(&fixture("patient-dupl-id-1.xml"), "STRUCTURE_006");
    assert_eq!(paths(&found), ["Patient._active.id"]);
    assert!(found[0].message.contains("pat-good"));
    assert!(findings(&fixture("patient-dupl-id-2.xml"), "STRUCTURE_006").is_empty());
}

#[test]
fn element_ids_are_unique_at_every_depth_and_across_contained_json() {
    let resource = parse_json(
        r##"{"resourceType":"Patient","id":"a",
            "name":[{"id":"a","family":"x"}],
            "contact":[{"name":{"id":"n","family":"y"}}],
            "_gender":{"id":"n"},
            "gender":"male",
            "managingOrganization":{"reference":"#o"},
            "contained":[{"resourceType":"Organization","id":"o","name":"o",
                "contact":[{"id":"a"}]}]}"##,
    )
    .expect("parses");
    // The resource ids `a` and `o` are not element ids.
    assert_eq!(
        paths(&findings(&resource, "STRUCTURE_006")),
        ["Patient._gender.id", "Patient.contained[0].contact[0].id"],
    );
}

/// sdf-16 / sdf-17 scope ElementDefinition ids to the snapshot and to the differential;
/// every R5 definition repeats them across both (the core Patient definition here).
#[test]
fn element_definition_ids_repeat_across_snapshot_and_differential_json() {
    let element = r#"{"id":"Patient","path":"Patient"}"#;
    let resource = parse_json(&format!(
        r#"{{"resourceType":"StructureDefinition","id":"p",
            "url":"http://example.org/StructureDefinition/p","name":"P","status":"draft",
            "kind":"resource","abstract":false,"type":"Patient",
            "baseDefinition":"http://hl7.org/fhir/StructureDefinition/Patient",
            "derivation":"constraint",
            "snapshot":{{"element":[{element}]}},
            "differential":{{"element":[{element}]}},
            "extension":[{{"id":"Patient","url":"http://example.org/e","valueString":"x"}}]}}"#
    ))
    .expect("parses");
    // The ElementDefinition ids stay out of the resource-wide set, so `Patient` is unused.
    assert!(findings(&resource, "STRUCTURE_006").is_empty());
}

/// Below an unknown element (STRUCTURE_001) nothing is an Element, so no Element.id.
#[test]
fn ids_below_unknown_elements_are_not_element_ids_json() {
    let resource = parse_json(
        r#"{"resourceType":"Patient","name":[{"id":"a","family":"x"}],"foo":{"id":"a"}}"#,
    )
    .expect("parses");
    assert!(findings(&resource, "STRUCTURE_006").is_empty());
}

#[test]
fn bundle_entries_are_their_own_id_scope_json() {
    let entry = r#"{"resource":{"resourceType":"Basic","code":{"id":"c","text":"x"}}}"#;
    let resource = parse_json(&format!(
        r#"{{"resourceType":"Bundle","type":"collection","entry":[{entry},{entry}]}}"#
    ))
    .expect("parses");
    assert!(findings(&resource, "STRUCTURE_006").is_empty());
}

#[test]
fn security_labels_are_a_set_on_system_and_code_xml() {
    let found = findings(&fixture("duplicate-security-tags.xml"), "STRUCTURE_007");
    assert_eq!(paths(&found), ["Patient.meta.security[2]"]);
    assert!(found[0].message.contains("http://test.org#x"));
}

#[test]
fn tags_are_a_set_ignoring_version_and_display_json() {
    let resource = parse_json(
        r#"{"resourceType":"Patient","meta":{
            "tag":[{"system":"s","code":"a","display":"A"},
                   {"system":"s","code":"a","version":"2"},
                   {"system":"t","code":"a"},
                   {"code":"a"}],
            "security":[{"system":"s","code":"a"}]}}"#,
    )
    .expect("parses");
    assert_eq!(
        paths(&findings(&resource, "STRUCTURE_007")),
        ["Patient.meta.tag[1]"]
    );
}

#[test]
fn contained_resources_have_their_own_meta_sets_json() {
    let tag = r#"{"system":"s","code":"a"}"#;
    let resource = parse_json(&format!(
        r##"{{"resourceType":"Patient","meta":{{"tag":[{tag}]}},
            "managingOrganization":{{"reference":"#o"}},
            "contained":[{{"resourceType":"Organization","id":"o","name":"o",
                "meta":{{"tag":[{tag},{tag}]}}}}]}}"##
    ))
    .expect("parses");
    assert_eq!(
        paths(&findings(&resource, "STRUCTURE_007")),
        ["Patient.contained[0].meta.tag[1]"]
    );
}
