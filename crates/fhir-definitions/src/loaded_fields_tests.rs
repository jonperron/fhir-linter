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
