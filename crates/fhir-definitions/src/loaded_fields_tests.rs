//! Fields the loader keeps from the bundled R5 snapshots, checked on the real `Registry`.
//! Counts come from python scans of `definitions/r5` (PLAN.md 11.6.2).

use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use crate::Registry;
use crate::model::ElementDefinition;

static REGISTRY: LazyLock<Arc<Registry>> = LazyLock::new(|| {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    Registry::shared(dir).unwrap()
});

fn element(sd: &str, path: &str) -> &'static ElementDefinition {
    REGISTRY
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{sd}"))
        .and_then(|sd| sd.element(path))
        .unwrap_or_else(|| panic!("{sd} has no element {path}"))
}

fn count(predicate: impl Fn(&ElementDefinition) -> bool) -> usize {
    REGISTRY
        .snapshot_elements()
        .filter(|e| predicate(e))
        .count()
}

#[test]
fn type_profiles_are_loaded_per_type() {
    let has_profile =
        |e: &ElementDefinition| e.type_profiles.iter().any(|t| !t.profiles.is_empty());
    assert_eq!(count(has_profile), 188);
    let quantity = element("Claim", "Claim.item.quantity");
    let [simple] = quantity.type_profiles.as_slice() else {
        panic!("one profiled type expected: {:?}", quantity.type_profiles);
    };
    assert_eq!(simple.code.as_ref(), "Quantity");
    assert_eq!(
        simple.profiles,
        [Arc::from(
            "http://hl7.org/fhir/StructureDefinition/SimpleQuantity"
        )],
    );
    // Types without a profile take no entry.
    assert!(
        element("Patient", "Patient.birthDate")
            .type_profiles
            .is_empty()
    );
}

#[test]
fn target_profiles_are_loaded_per_type() {
    let targeted: usize = REGISTRY
        .snapshot_elements()
        .flat_map(|e| &e.type_profiles)
        .filter(|t| !t.target_profiles.is_empty())
        .count();
    assert_eq!(targeted, 1543);
    let subject = element("Observation", "Observation.subject");
    let [reference] = subject.type_profiles.as_slice() else {
        panic!("one targeted type expected: {:?}", subject.type_profiles);
    };
    assert_eq!(reference.code.as_ref(), "Reference");
    assert!(reference.profiles.is_empty());
    let targets: Vec<_> = reference
        .target_profiles
        .iter()
        .map(AsRef::as_ref)
        .collect();
    assert!(targets.contains(&"http://hl7.org/fhir/StructureDefinition/Patient"));
    assert!(targets.contains(&"http://hl7.org/fhir/StructureDefinition/Group"));
}

#[test]
fn content_reference_is_loaded_without_the_local_hash() {
    assert_eq!(count(|e| e.content_reference.is_some()), 153);
    assert_eq!(
        element("Questionnaire", "Questionnaire.item.item")
            .content_reference
            .as_deref(),
        Some("Questionnaire.item"),
    );
    // A derived profile points into its base definition by canonical URL; keep it whole.
    assert_eq!(
        element("vitalsigns", "Observation.component.referenceRange")
            .content_reference
            .as_deref(),
        Some("http://hl7.org/fhir/StructureDefinition/Observation#Observation.referenceRange"),
    );
}
