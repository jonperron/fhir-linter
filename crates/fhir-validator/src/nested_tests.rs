//! Unit tests of `nested.rs`, kept apart to hold the module under the 800-line limit.

use super::*;

fn elem(path: &str, min: u32) -> ElementDefinition {
    ElementDefinition {
        path: Arc::from(path),
        min,
        max: Arc::from("1"),
        types: vec![],
        constraints: vec![],
        binding: None,
        slice_name: None,
        xml_attr: false,
        content_reference: None,
        type_profiles: vec![],
        fixed: None,
        pattern: None,
        slicing: None,
        max_length: None,
        min_value: None,
        max_value: None,
    }
}

#[test]
fn children_cardinality_checks_direct_children_only() {
    let sd = StructureDefinition::new(
        Arc::from("http://example.org/StructureDefinition/obs-profile"),
        Arc::from("ObsProfile"),
        Arc::from("resource"),
        false,
        vec![
            elem("Observation", 0),
            elem("Observation.subject", 1),
            elem("Observation.value[x]", 1),
            elem("Observation.component.code", 1),
        ],
    );
    let mut found = vec![];
    check_children_cardinality(
        &IndexMap::new(),
        &sd,
        "Observation",
        "Observation",
        &mut found,
    );
    let found: Vec<_> = found
        .iter()
        .map(|d| (d.code.as_ref(), d.path.as_str()))
        .collect();
    // A missing required choice is reported at its placeholder path.
    assert_eq!(
        found,
        [
            ("CARDINALITY_001", "Observation.subject"),
            ("CARDINALITY_001", "Observation.value[x]"),
        ],
    );
}

#[test]
fn children_cardinality_skips_choice_slices() {
    let slice = ElementDefinition {
        slice_name: Some(Arc::from("valueQuantity")),
        ..elem("Observation.value[x]", 1)
    };
    let sd = StructureDefinition::new(
        Arc::from("http://example.org/StructureDefinition/obs-profile"),
        Arc::from("ObsProfile"),
        Arc::from("resource"),
        false,
        vec![
            elem("Observation", 0),
            elem("Observation.value[x]", 0),
            slice,
        ],
    );
    let mut found = vec![];
    check_children_cardinality(
        &IndexMap::new(),
        &sd,
        "Observation",
        "Observation",
        &mut found,
    );
    assert!(found.is_empty(), "{found:?}");
}
