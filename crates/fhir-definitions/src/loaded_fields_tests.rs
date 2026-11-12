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
fn fixed_and_pattern_values_are_loaded_with_their_key() {
    assert_eq!(count(|e| e.fixed.is_some()), 87);
    assert_eq!(count(|e| e.pattern.is_some()), 33);
    assert_eq!(count(|e| e.fixed.is_some() || e.pattern.is_some()), 120);
    let vital_signs = REGISTRY
        .structure_definition("http://hl7.org/fhir/StructureDefinition/vitalsigns")
        .expect("vitalsigns is loaded");
    let category_code = vital_signs
        .snapshot
        .iter()
        .filter(|e| e.path.as_ref() == "Observation.category.coding.code")
        .find_map(|e| e.fixed.as_deref())
        .expect("the VSCat slice fixes its code");
    assert_eq!(category_code.key.as_ref(), "fixedCode");
    assert_eq!(category_code.value, serde_json::json!("vital-signs"));
    let pattern = REGISTRY
        .snapshot_elements()
        .find_map(|e| e.pattern.as_deref())
        .expect("some element has a pattern");
    assert!(pattern.key.starts_with("pattern"));
}

#[test]
fn slicing_and_slice_names_are_loaded() {
    assert_eq!(count(|e| e.slicing.is_some()), 255);
    assert_eq!(count(|e| e.slice_name.is_some()), 174);
    let vital_signs = REGISTRY
        .structure_definition("http://hl7.org/fhir/StructureDefinition/vitalsigns")
        .expect("vitalsigns is loaded");
    let category = vital_signs
        .element("Observation.category")
        .and_then(|e| e.slicing.as_deref())
        .expect("vitalsigns slices Observation.category");
    let discriminators: Vec<_> = category
        .discriminators
        .iter()
        .map(|d| (d.kind.as_ref(), d.path.as_ref()))
        .collect();
    assert_eq!(
        discriminators,
        [("value", "coding.code"), ("value", "coding.system")],
    );
    assert_eq!(category.rules.as_ref(), "open");
    assert!(!category.ordered);
    let vs_cat = vital_signs
        .descendants("Observation.category")
        .iter()
        .find(|e| e.slice_name.as_deref() == Some("VSCat"))
        .expect("VSCat slice");
    assert_eq!(vs_cat.min, 1);
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
