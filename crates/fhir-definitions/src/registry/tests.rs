use super::*;
use std::sync::LazyLock;

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
}

// Shared with the `shared_*` tests so the test binary loads the definitions once.
static REGISTRY: LazyLock<Arc<Registry>> =
    LazyLock::new(|| Registry::shared(definitions_dir()).unwrap());

#[test]
fn registry_structure_definition_lookup_is_case_sensitive() {
    let registry = &*REGISTRY;
    assert!(
        registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/patient")
            .is_none()
    );
}

#[test]
fn choice_variants_come_from_declared_choice_types() {
    assert!(REGISTRY.is_choice_variant("value", "Quantity"));
    assert!(REGISTRY.is_choice_variant("value", "DateTime"));
    assert!(REGISTRY.is_choice_variant("age", "Age"));
    // `modifierExtension` is a plain element, not a variant of `modifier[x]`.
    assert!(!REGISTRY.is_choice_variant("modifier", "Extension"));
    assert!(!REGISTRY.is_choice_variant("value", "Element"));
    // Suffixes are case-sensitive: `valuequantity` is not a variant.
    assert!(!REGISTRY.is_choice_variant("value", "quantity"));
}

#[test]
fn has_element_names_declared_elements_without_choice_suffix() {
    assert_eq!(REGISTRY.has_element("Patient", "name"), Some(true));
    assert_eq!(REGISTRY.has_element("Patient", "id"), Some(true));
    // `resourceType` is a JSON property, not an element of the model.
    assert_eq!(REGISTRY.has_element("Patient", "resourceType"), Some(false));
    assert_eq!(REGISTRY.has_element("HumanName", "given1"), Some(false));
    // A choice element is named without its `[x]` and never with a type suffix.
    assert_eq!(REGISTRY.has_element("Observation", "value"), Some(true));
    assert_eq!(
        REGISTRY.has_element("Observation", "valueQuantity"),
        Some(false)
    );
    assert_eq!(REGISTRY.has_element("Observation", "value[x]"), Some(false));
    // A constraint on a type lists its elements under the base type's path.
    assert_eq!(REGISTRY.has_element("SimpleQuantity", "unit"), Some(true));
    // Backbone children are defined at the owning path, not under the type.
    assert_eq!(REGISTRY.has_element("BackboneElement", "name"), None);
    assert_eq!(REGISTRY.has_element("NoSuchType", "name"), None);
}

#[test]
fn has_element_resolves_backbone_children_by_instance_path() {
    let has = |path: &str, name: &str| REGISTRY.has_element(path, name);
    assert_eq!(has("Patient.contact", "name"), Some(true));
    assert_eq!(has("Patient.contact", "nmae"), Some(false));
    // A backbone inside a complex type is defined under that type.
    assert_eq!(
        has(
            "MedicationRequest.dosageInstruction.timing.repeat",
            "frequency"
        ),
        Some(true),
    );
    // A contentReference element has the children of its target, at any depth.
    assert_eq!(has("Questionnaire.item.item", "linkId"), Some(true));
    assert_eq!(has("Questionnaire.item.item.item", "lnkId"), Some(false));
    // No choice suffix, as for types.
    assert_eq!(has("Observation.component", "value"), Some(true));
    assert_eq!(has("Observation.component", "valueString"), Some(false));
    // A path that names no element cannot tell.
    assert_eq!(has("Patient.nosuch", "name"), None);
}

#[test]
fn child_element_gives_definition_path_and_type() {
    let child = |parent: &str, name: &str| {
        REGISTRY
            .child_element(parent, name)
            .map(|(definition, ty)| (definition.to_string(), ty.to_string()))
    };
    let pair = |definition: &str, ty: &str| Some((definition.to_owned(), ty.to_owned()));
    assert_eq!(child("Patient", "birthDate"), pair("date", "date"));
    assert_eq!(
        child("Patient", "contact"),
        pair("Patient.contact", "BackboneElement")
    );
    assert_eq!(
        child("Patient.contact", "name"),
        pair("HumanName", "HumanName")
    );
    assert_eq!(child("Period", "start"), pair("dateTime", "dateTime"));
    assert_eq!(child("Appointment", "start"), pair("instant", "instant"));
    // A choice variant takes the type its suffix names.
    assert_eq!(child("Observation", "valueTime"), pair("time", "time"));
    assert_eq!(child("Observation", "valueBogus"), None);
    // A contentReference element takes its target's definition path and type.
    assert_eq!(
        child("Questionnaire.item", "item"),
        pair("Questionnaire.item", "BackboneElement")
    );
    assert_eq!(child("Patient", "unknown"), None);
    assert_eq!(child("NoSuchType", "id"), None);
}

#[test]
fn registry_loads_core_profiles_from_profiles_others() {
    assert!(
        REGISTRY
            .structure_definition("http://hl7.org/fhir/StructureDefinition/vitalsigns")
            .is_some()
    );
}

#[test]
fn registry_loads_patient_structure_definition() {
    let registry = &*REGISTRY;
    let sd = registry
        .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
        .expect("Patient StructureDefinition should be present");
    assert_eq!(sd.name.as_ref(), "Patient");
    assert_eq!(sd.kind.as_ref(), "resource");
    assert!(!sd.snapshot.is_empty(), "snapshot should have elements");
}

#[test]
fn core_looks_up_by_name_under_the_core_prefix() {
    let registry = &*REGISTRY;
    let sd = registry
        .core("Patient")
        .expect("Patient is a core definition");
    assert_eq!(
        sd.url.as_ref(),
        "http://hl7.org/fhir/StructureDefinition/Patient"
    );
    assert!(registry.core("NotAType").is_none());
    assert!(registry.core("").is_none());
}

#[test]
fn registry_resolves_valueset_by_url() {
    let registry = &*REGISTRY;
    let vs = registry
        .value_set("http://hl7.org/fhir/ValueSet/administrative-gender")
        .expect("administrative-gender ValueSet should be present");
    assert_eq!(
        vs.url.as_ref(),
        "http://hl7.org/fhir/ValueSet/administrative-gender"
    );
}

#[test]
fn registry_loads_multiple_structure_definitions() {
    let registry = &*REGISTRY;
    assert!(
        registry.structure_definition_count() > 100,
        "should load more than 100 StructureDefinitions, got {}",
        registry.structure_definition_count()
    );
}

#[test]
fn registry_loads_code_systems() {
    let registry = &*REGISTRY;
    assert!(
        registry.code_system_count() > 0,
        "should load at least one CodeSystem"
    );
}

#[test]
fn patient_snapshot_contains_identifier_element() {
    let registry = &*REGISTRY;
    let sd = registry
        .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
        .unwrap();
    let identifier = sd
        .snapshot
        .iter()
        .find(|e| e.path.as_ref() == "Patient.identifier");
    assert!(
        identifier.is_some(),
        "snapshot should contain Patient.identifier"
    );
    let identifier = identifier.unwrap();
    assert_eq!(identifier.min, 0);
    assert_eq!(identifier.max.as_ref(), "*");
}

#[test]
fn registry_returns_none_for_unknown_url() {
    let registry = &*REGISTRY;
    assert!(
        registry
            .structure_definition("http://example.com/unknown")
            .is_none()
    );
    assert!(registry.value_set("http://example.com/unknown").is_none());
    assert!(registry.code_system("http://example.com/unknown").is_none());
}

#[test]
fn registry_value_set_count_is_positive() {
    let registry = &*REGISTRY;
    assert!(
        registry.value_set_count() > 0,
        "should load at least one ValueSet, got {}",
        registry.value_set_count()
    );
}

#[test]
fn registry_code_system_has_url_and_name() {
    let registry = &*REGISTRY;
    let cs = registry
        .code_system("http://hl7.org/fhir/administrative-gender")
        .expect("administrative-gender CodeSystem should be present");
    assert_eq!(cs.url.as_ref(), "http://hl7.org/fhir/administrative-gender");
    assert!(!cs.name.is_empty(), "CodeSystem name should not be empty");
}

#[test]
fn registry_value_set_has_name() {
    let registry = &*REGISTRY;
    let vs = registry
        .value_set("http://hl7.org/fhir/ValueSet/administrative-gender")
        .expect("administrative-gender ValueSet should be present");
    assert!(!vs.name.is_empty(), "ValueSet name should not be empty");
}

#[test]
fn element_definition_types_are_populated() {
    let registry = &*REGISTRY;
    let sd = registry
        .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
        .unwrap();
    let id_elem = sd
        .snapshot
        .iter()
        .find(|e| e.path.as_ref() == "Patient.identifier")
        .expect("Patient.identifier element should exist");
    assert!(
        !id_elem.types.is_empty(),
        "Patient.identifier should have at least one type"
    );
    assert!(
        id_elem.types.iter().any(|t| t.as_ref() == "Identifier"),
        "Patient.identifier should have type 'Identifier', got {:?}",
        id_elem.types
    );
}

#[test]
fn structure_definition_is_abstract_for_resource_type() {
    let registry = &*REGISTRY;
    let sd = registry
        .structure_definition("http://hl7.org/fhir/StructureDefinition/Resource")
        .expect("Resource StructureDefinition should be present");
    assert!(
        sd.is_abstract,
        "Resource StructureDefinition should be abstract"
    );
}

#[test]
fn structure_definition_is_not_abstract_for_patient() {
    let registry = &*REGISTRY;
    let sd = registry
        .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
        .unwrap();
    assert!(
        !sd.is_abstract,
        "Patient StructureDefinition should not be abstract"
    );
}

#[test]
fn registry_from_nonexistent_dir_returns_error() {
    let result = Registry::from_definitions_dir("/nonexistent/path/that/does/not/exist");
    assert!(
        result.is_err(),
        "loading from a nonexistent dir should fail"
    );
}

#[test]
fn shared_returns_same_registry_for_equivalent_paths() {
    let dir = definitions_dir();
    let first = Registry::shared(&dir).unwrap();
    let second = Registry::shared(dir.join(".")).unwrap();
    let third = Registry::shared(fs::canonicalize(&dir).unwrap()).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(Arc::ptr_eq(&first, &third));
}

#[test]
fn shared_from_nonexistent_dir_returns_error() {
    assert!(Registry::shared("/nonexistent/path/that/does/not/exist").is_err());
}

#[test]
fn value_set_contains_applies_include_version_and_exclude() {
    let dir =
        std::env::temp_dir().join(format!("fhir_def_synthetic_compose_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for file in [
        "profiles-resources.json",
        "profiles-types.json",
        "profiles-others.json",
        "conceptmaps.json",
    ] {
        fs::write(dir.join(file), r#"{"entry": []}"#).unwrap();
    }
    let vs = |name: &str, version: &str, extra: &str| {
        format!(
            r#"{{"resource": {{"resourceType": "ValueSet", "url": "http://x/vs/{name}",
            "name": "{name}", "compose": {{"include": [
                {{"system": "http://x/cs", "version": "{version}"{extra}}}]}}}}}}"#
        )
    };
    let bundle = format!(
        r#"{{"entry": [{}, {}, {}, {}, {}, {{"resource": {{"resourceType": "CodeSystem",
        "url": "http://x/cs", "name": "CS", "version": "2", "content": "complete",
        "concept": [{{"code": "a"}}]}}}}]}}"#,
        vs("same", "2", ""),
        vs("any", "*", ""),
        vs("other", "1", ""),
        // Enumerated codes decide alone, whatever the CodeSystem version.
        vs("other-listed", "1", r#", "concept": [{"code": "a"}]"#),
        // A decidable include plus a sibling compose.exclude (extra closes the include).
        vs(
            "excluded",
            "2",
            r#"}], "exclude": [{"system": "http://x/cs", "concept": [{"code": "a"}]"#,
        ),
    );
    fs::write(dir.join("valuesets.json"), bundle).unwrap();
    let registry = Registry::from_definitions_dir(&dir);
    fs::remove_dir_all(&dir).unwrap();
    let registry = registry.unwrap();
    for (vs, in_code, out_code) in [
        ("same", Some(true), Some(false)),
        ("any", Some(true), Some(false)),
        ("other", None, None),
        ("other-listed", Some(true), Some(false)),
        ("excluded", None, Some(false)),
    ] {
        let url = format!("http://x/vs/{vs}");
        assert_eq!(
            registry.value_set_contains(&url, None, "a"),
            in_code,
            "{vs} a"
        );
        assert_eq!(
            registry.value_set_contains(&url, None, "b"),
            out_code,
            "{vs} b"
        );
    }
}

#[test]
fn value_set_contains_without_system_is_undecided_across_several_systems() {
    let dir = std::env::temp_dir().join(format!("fhir_def_multi_system_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for file in [
        "profiles-resources.json",
        "profiles-types.json",
        "profiles-others.json",
        "conceptmaps.json",
    ] {
        fs::write(dir.join(file), r#"{"entry": []}"#).unwrap();
    }
    let vs = |name: &str, includes: &str| {
        format!(
            r#"{{"resource": {{"resourceType": "ValueSet", "url": "http://x/vs/{name}",
            "name": "{name}", "compose": {{"include": [{includes}]}}}}}}"#
        )
    };
    let listed = |system: &str| {
        format!(r#"{{"system": "http://x/{system}", "concept": [{{"code": "a"}}]}}"#)
    };
    let bundle = format!(
        r#"{{"entry": [{}, {}, {}]}}"#,
        vs("one", &listed("cs1")),
        vs("two", &format!("{}, {}", listed("cs1"), listed("cs2"))),
        vs("nested", r#"{"valueSet": ["http://x/vs/two"]}"#),
    );
    fs::write(dir.join("valuesets.json"), bundle).unwrap();
    let registry = Registry::from_definitions_dir(&dir);
    fs::remove_dir_all(&dir).unwrap();
    let registry = registry.unwrap();
    let contains = |vs: &str, system, code| {
        registry.value_set_contains(&format!("http://x/vs/{vs}"), system, code)
    };
    assert_eq!(contains("one", None, "a"), Some(true));
    assert_eq!(contains("two", None, "a"), None);
    assert_eq!(contains("nested", None, "a"), None);
    assert_eq!(contains("two", None, "b"), Some(false));
    assert_eq!(contains("two", Some("http://x/cs2"), "a"), Some(true));
}

#[test]
fn value_set_contains_decides_membership_from_the_bundled_definitions() {
    const VS: &str = "http://hl7.org/fhir/ValueSet/";
    let gender = Some("http://hl7.org/fhir/administrative-gender");
    // (ValueSet, system, code, expected)
    let cases: &[(&str, Option<&str>, &str, Option<bool>)] = &[
        // Whole complete CodeSystem: in, out, other system.
        ("administrative-gender", gender, "male", Some(true)),
        ("administrative-gender", None, "male", Some(true)),
        ("administrative-gender", gender, "bogus", Some(false)),
        (
            "administrative-gender",
            Some("http://example.org/cs"),
            "male",
            Some(false),
        ),
        // Enumerated concepts.
        // Several code systems: a systemless "N" could be from another one.
        ("security-label-data-examples", None, "N", None),
        ("security-label-data-examples", None, "V", Some(false)),
        // include.filter is never evaluated.
        ("example-filter", None, "anything", None),
        // include.valueSet: decided by the imported ValueSet (fhir-types).
        // (several code systems, so a systemless hit is undecided)
        ("elementdefinition-types", None, "Patient", None),
        ("elementdefinition-types", None, "NotAType", Some(false)),
        // compose.exclude (with a filtered include): undecidable.
        ("encounter-participant-type", None, "ADM", None),
        // CodeSystem with content `example`: present codes only.
        (
            "biologicallyderived-product-property-type-codes",
            None,
            "collectiontype",
            Some(true),
        ),
        (
            "biologicallyderived-product-property-type-codes",
            None,
            "bogus",
            None,
        ),
        // notSelectable concept (item-type#question): undecidable.
        ("item-type", None, "question", None),
        ("item-type", None, "group", Some(true)),
        // include.version 20210721 pins a CodeSystem version other than the loaded 5.0.0.
        ("example-metadata", None, "A", None),
        ("example-metadata", None, "Z", None),
        // Unknown ValueSet.
        ("no-such-value-set", None, "male", None),
    ];
    for &(vs, system, code, expected) in cases {
        assert_eq!(
            REGISTRY.value_set_contains(&format!("{VS}{vs}"), system, code),
            expected,
            "{vs} {system:?} {code}",
        );
    }
}

#[test]
fn expand_lists_a_whole_complete_code_system_in_document_order() {
    const GENDER: &str = "http://hl7.org/fhir/administrative-gender";
    assert_eq!(
        REGISTRY.expand("http://hl7.org/fhir/ValueSet/administrative-gender"),
        Some(vec![
            (GENDER, "male", Some("Male")),
            (GENDER, "female", Some("Female")),
            (GENDER, "other", Some("Other")),
            (GENDER, "unknown", Some("Unknown")),
        ])
    );
}

#[test]
fn expand_lists_explicit_concepts_across_systems() {
    let codes: Vec<_> = REGISTRY
        .expand("http://hl7.org/fhir/ValueSet/security-label-data-examples")
        .unwrap()
        .into_iter()
        .map(|(_, code, _)| code)
        .collect();
    assert_eq!(codes, ["N", "R", "ETH", "PSY", "STD"]);
}

#[test]
fn expand_gives_none_when_the_compose_cannot_be_enumerated_exactly() {
    for vs in [
        // include.filter
        "example-filter",
        // compose.exclude
        "encounter-participant-type",
        // CodeSystem with content `example`
        "biologicallyderived-product-property-type-codes",
        // notSelectable concept (item-type#question)
        "item-type",
        // include.version pins a CodeSystem version other than the loaded one
        "example-metadata",
        "no-such-value-set",
    ] {
        assert_eq!(
            REGISTRY.expand(&format!("http://hl7.org/fhir/ValueSet/{vs}")),
            None,
            "{vs}"
        );
    }
}

#[test]
fn translate_follows_the_r5_concept_map_groups() {
    const CM: &str = "http://hl7.org/fhir/ConceptMap/cm-address-use-v2";
    const V2: &str = "http://terminology.hl7.org/CodeSystem/v2-0190";
    let home = Some(vec![(V2, "H", "equivalent")]);
    assert_eq!(REGISTRY.translate(CM, None, "home"), home);
    assert_eq!(
        REGISTRY.translate(CM, Some("http://hl7.org/fhir/address-use"), "home"),
        home
    );
    assert_eq!(
        REGISTRY.translate(CM, None, "old"),
        Some(vec![(V2, "BA", "source-is-broader-than-target")])
    );
    // A code or system the map does not cover has no match.
    assert_eq!(REGISTRY.translate(CM, None, "bogus"), Some(vec![]));
    assert_eq!(
        REGISTRY.translate(CM, Some("http://example.org/cs"), "home"),
        Some(vec![])
    );
    assert_eq!(
        REGISTRY.translate("http://example.org/no-map", None, "home"),
        None
    );
}

#[test]
fn missing_concept_map_bundle_is_an_io_error() {
    let dir = std::env::temp_dir().join(format!("fhir_def_no_conceptmaps_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for file in [
        "profiles-resources.json",
        "profiles-types.json",
        "profiles-others.json",
        "valuesets.json",
    ] {
        fs::write(dir.join(file), r#"{"entry": []}"#).unwrap();
    }
    let result = Registry::from_definitions_dir(&dir);
    fs::remove_dir_all(&dir).unwrap();
    assert!(matches!(result, Err(crate::Error::Io(_))));
}
