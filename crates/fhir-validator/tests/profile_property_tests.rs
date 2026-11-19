//! ElementDefinition properties a constraint profile may not introduce or change, from the
//! comments of the R5 ElementDefinition definition (contentReference, orderMeaning,
//! isModifier), on the official validator test cases and on hand-built definitions.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{Resource, parse_json, parse_xml};
use fhir_validator::{Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// Error paths with `code`.
fn paths(resource: &Resource, code: &str) -> Vec<String> {
    validate(resource, registry())
        .into_iter()
        .filter(|d| &*d.code == code && d.severity == Severity::Error)
        .map(|d| d.path)
        .collect()
}

fn fixture(name: &str) -> Resource {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/validator")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    parse_xml(&text).expect("fixture parses")
}

/// A definition of `type_name` with `derivation` whose differential holds the root and
/// `element` (a JSON object, without its braces).
fn definition(derivation: &str, type_name: &str, kind: &str, element: &str) -> Resource {
    let base = if derivation == "constraint" {
        type_name
    } else {
        "DomainResource"
    };
    let source = format!(
        r#"{{"resourceType":"StructureDefinition",
        "url":"http://example.org/StructureDefinition/test","name":"Test","status":"draft",
        "kind":"{kind}","abstract":false,"type":"{type_name}",
        "baseDefinition":"http://hl7.org/fhir/StructureDefinition/{base}",
        "derivation":"{derivation}",
        "differential":{{"element":[{{"id":"{type_name}","path":"{type_name}"}},{{{element}}}]}}}}"#
    );
    parse_json(&source).expect("definition parses")
}

#[test]
fn profile_may_not_set_a_content_reference_on_the_root() {
    assert_eq!(
        paths(&fixture("sd-root-profile.xml"), "DEFINITION_006"),
        ["StructureDefinition.differential.element[0].contentReference"]
    );
}

#[test]
fn profile_may_not_change_a_content_reference() {
    let changed = definition(
        "constraint",
        "Questionnaire",
        "resource",
        r##""id":"Questionnaire.item.item","path":"Questionnaire.item.item",
        "contentReference":"#Questionnaire""##,
    );
    assert_eq!(
        paths(&changed, "DEFINITION_006"),
        ["StructureDefinition.differential.element[1].contentReference"]
    );
}

#[test]
fn profile_may_repeat_the_base_content_reference() {
    let same = definition(
        "constraint",
        "Questionnaire",
        "resource",
        r##""id":"Questionnaire.item.item","path":"Questionnaire.item.item",
        "contentReference":"#Questionnaire.item""##,
    );
    assert!(paths(&same, "DEFINITION_006").is_empty());
    // The same target written as a canonical URL into the core definition, as derived
    // snapshots such as vitalsigns do.
    let canonical = definition(
        "constraint",
        "Observation",
        "resource",
        r#""id":"Observation.component.referenceRange",
        "path":"Observation.component.referenceRange",
        "contentReference":"http://hl7.org/fhir/StructureDefinition/Observation#Observation.referenceRange""#,
    );
    assert!(paths(&canonical, "DEFINITION_006").is_empty());
}

#[test]
fn specialization_may_define_a_content_reference() {
    let specialization = definition(
        "specialization",
        "Questionnaire",
        "resource",
        r##""id":"Questionnaire.item.item","path":"Questionnaire.item.item",
        "contentReference":"#Questionnaire""##,
    );
    assert!(paths(&specialization, "DEFINITION_006").is_empty());
}
