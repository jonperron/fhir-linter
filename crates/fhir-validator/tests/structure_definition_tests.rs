//! Checks on StructureDefinition instances: properties a profile may not set
//! (elementdefinition.html, ElementDefinition.meaningWhenMissing: implicit meanings
//! "can only be specified on a resource, data type, or extension definition, and never
//! in a profile that applies to one of these"), on the official validator test cases.
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

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/validator")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn definition_findings(resource: &Resource) -> Vec<Diagnostic> {
    validate(resource, registry())
        .into_iter()
        .filter(|d| d.code.starts_with("DEFINITION_"))
        .collect()
}

fn xml(name: &str) -> Vec<Diagnostic> {
    definition_findings(&parse_xml(&fixture(name)).expect("parses"))
}

fn json(source: &str) -> Vec<Diagnostic> {
    definition_findings(&parse_json(source).expect("parses"))
}

fn assert_meaning_when_missing(found: &[Diagnostic], path: &str, element: &str) {
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(&*found[0].code, "DEFINITION_001");
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(found[0].path, path);
    assert!(
        found[0].message.contains("meaningWhenMissing") && found[0].message.contains(element),
        "{}",
        found[0].message
    );
}

#[test]
fn profile_on_a_resource_may_not_set_meaning_when_missing_xml() {
    assert_meaning_when_missing(
        &xml("profile-illegal-elements.xml"),
        "StructureDefinition.differential.element[0].meaningWhenMissing",
        "Patient.identifier",
    );
}

#[test]
fn profile_on_a_data_type_may_not_set_meaning_when_missing_xml() {
    // The fixture breaks other root rules too (tests/profile_property_tests.rs).
    let found: Vec<_> = xml("sd-root-profile.xml")
        .into_iter()
        .filter(|d| &*d.code == "DEFINITION_001")
        .collect();
    assert_meaning_when_missing(
        &found,
        "StructureDefinition.differential.element[0].meaningWhenMissing",
        "CodeableConcept",
    );
}

#[test]
fn specializations_may_set_meaning_when_missing_xml() {
    assert!(xml("sd-root-logical.xml").is_empty());
    assert!(xml("sd-root-specialization.xml").is_empty());
}

fn structure_definition(derivation: &str, type_name: &str, base: &str) -> String {
    format!(
        r#"{{
            "resourceType": "StructureDefinition",
            "url": "http://example.org/StructureDefinition/test",
            "name": "Test",
            "status": "draft",
            "kind": "complex-type",
            "abstract": false,
            "type": "{type_name}",
            "baseDefinition": "{base}",
            "derivation": "{derivation}",
            "differential": {{
                "element": [
                    {{"id": "{type_name}", "path": "{type_name}"}},
                    {{
                        "id": "{type_name}.id",
                        "path": "{type_name}.id",
                        "meaningWhenMissing": "none"
                    }}
                ]
            }}
        }}"#
    )
}

#[test]
fn profile_may_not_set_meaning_when_missing_json() {
    let source = structure_definition(
        "constraint",
        "Identifier",
        "http://hl7.org/fhir/StructureDefinition/Identifier",
    );
    assert_meaning_when_missing(
        &json(&source),
        "StructureDefinition.differential.element[1].meaningWhenMissing",
        "Identifier.id",
    );
}

#[test]
fn extension_definition_may_set_meaning_when_missing_json() {
    let source = structure_definition(
        "constraint",
        "Extension",
        "http://hl7.org/fhir/StructureDefinition/Extension|5.0.0",
    );
    assert!(json(&source).is_empty());
}

#[test]
fn profile_on_an_extension_definition_may_not_set_meaning_when_missing_json() {
    // A profile on an extension "applies to" the extension definition, so the rule holds.
    // The base need not be loaded: only the core Extension type exempts a definition.
    let base = "http://example.org/StructureDefinition/some-extension";
    let source = structure_definition("constraint", "Extension", base);
    assert_meaning_when_missing(
        &json(&source),
        "StructureDefinition.differential.element[1].meaningWhenMissing",
        "Extension.id",
    );
}

#[test]
fn core_structure_definitions_set_meaning_when_missing_without_findings() {
    let source = structure_definition(
        "specialization",
        "Identifier",
        "http://hl7.org/fhir/StructureDefinition/DataType",
    );
    assert!(json(&source).is_empty());
}

/// The test profile on Identifier, edited by `edit` before validation.
fn edited_profile(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<Diagnostic> {
    let source = structure_definition(
        "constraint",
        "Identifier",
        "http://hl7.org/fhir/StructureDefinition/Identifier",
    );
    let mut value: serde_json::Value = serde_json::from_str(&source).expect("valid JSON");
    edit(&mut value);
    json(&value.to_string())
}

#[test]
fn profile_without_meaning_when_missing_has_no_findings_json() {
    let found = edited_profile(|sd| {
        sd["differential"]["element"][1]
            .as_object_mut()
            .expect("element")
            .remove("meaningWhenMissing");
    });
    assert!(found.is_empty(), "{found:?}");
    let found = edited_profile(|sd| {
        sd.as_object_mut().expect("object").remove("differential");
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn profile_without_type_is_left_to_the_required_field_check_json() {
    // Without a type the extension exemption cannot be decided.
    let found = edited_profile(|sd| {
        sd.as_object_mut().expect("object").remove("type");
    });
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn profile_without_base_definition_may_not_set_meaning_when_missing_json() {
    let found = edited_profile(|sd| {
        sd.as_object_mut().expect("object").remove("baseDefinition");
    });
    assert_meaning_when_missing(
        &found,
        "StructureDefinition.differential.element[1].meaningWhenMissing",
        "Identifier.id",
    );
}

#[test]
fn profile_in_a_bundle_entry_may_not_set_meaning_when_missing_json() {
    let profile = structure_definition(
        "constraint",
        "Identifier",
        "http://hl7.org/fhir/StructureDefinition/Identifier",
    );
    let source = format!(
        r#"{{"resourceType": "Bundle", "type": "collection", "entry": [
            {{"fullUrl": "http://example.org/StructureDefinition/test", "resource": {profile}}}
        ]}}"#
    );
    assert_meaning_when_missing(
        &json(&source),
        "Bundle.entry[0].resource.differential.element[1].meaningWhenMissing",
        "Identifier.id",
    );
}

// Slice cardinality (profiling.html, Slice Cardinality): "The maximum cardinality on each
// slice cannot exceed n" and "The sum of the minimum cardinalities must be less or equal to
// n", where n is the sliced element's maximum. eld-2: "Min <= Max".

/// A profile on Observation whose differential is `elements`.
fn observation_profile(elements: serde_json::Value) -> Vec<Diagnostic> {
    let profile = serde_json::json!({
        "resourceType": "StructureDefinition",
        "url": "http://example.org/StructureDefinition/obs",
        "name": "Obs",
        "status": "draft",
        "kind": "resource",
        "abstract": false,
        "type": "Observation",
        "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Observation",
        "derivation": "constraint",
        "differential": { "element": elements },
    });
    json(&profile.to_string())
}

fn assert_single(found: &[Diagnostic], code: &str, path: &str, words: &[&str]) {
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(&*found[0].code, code);
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(found[0].path, path);
    for word in words {
        assert!(found[0].message.contains(word), "{}", found[0].message);
    }
}

#[test]
fn slice_minimums_may_not_exceed_the_slicer_maximum_json() {
    assert_single(
        &json(&fixture("sd-card-slice.json")),
        "DEFINITION_002",
        "StructureDefinition.differential.element[0]",
        &["Observation.component", "at least 2", "at most 1"],
    );
}

#[test]
fn slice_child_minimum_may_not_exceed_the_slicer_child_maximum_json() {
    assert_single(
        &json(&fixture("sd-card-child.json")),
        "DEFINITION_003",
        "StructureDefinition.differential.element[4].min",
        &[
            "Observation.component:a.value[x]",
            "minimum 1 is",
            "maximum 0",
        ],
    );
}

#[test]
fn slice_maximum_may_not_exceed_the_slicer_maximum_json() {
    let found = observation_profile(serde_json::json!([
        { "id": "Observation.component", "path": "Observation.component",
          "slicing": { "rules": "open" }, "max": "2" },
        { "id": "Observation.component:a", "path": "Observation.component",
          "sliceName": "a", "max": "*" },
    ]));
    assert_single(
        &found,
        "DEFINITION_002",
        "StructureDefinition.differential.element[1].max",
        &["Observation.component:a", "*", "2"],
    );
}

#[test]
fn slices_within_the_slicer_maximum_have_no_findings_json() {
    let found = observation_profile(serde_json::json!([
        { "id": "Observation.component", "path": "Observation.component",
          "slicing": { "rules": "open" }, "max": "2" },
        { "id": "Observation.component.value[x]", "path": "Observation.component.value[x]",
          "max": "1" },
        { "id": "Observation.component:a", "path": "Observation.component",
          "sliceName": "a", "min": 1, "max": "2" },
        { "id": "Observation.component:b", "path": "Observation.component",
          "sliceName": "b", "min": 1, "max": "1" },
        { "id": "Observation.component:b.value[x]", "path": "Observation.component.value[x]",
          "min": 1 },
    ]));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn slicer_maximum_comes_from_the_base_definition_when_unstated_json() {
    // Observation.value[x] is 0..1 in the base Observation.
    let found = observation_profile(serde_json::json!([
        { "id": "Observation.value[x]", "path": "Observation.value[x]",
          "slicing": { "discriminator": [{ "type": "type", "path": "$this" }],
                       "rules": "open" } },
        { "id": "Observation.value[x]:valueQuantity", "path": "Observation.value[x]",
          "sliceName": "valueQuantity", "min": 1 },
        { "id": "Observation.value[x]:valueString", "path": "Observation.value[x]",
          "sliceName": "valueString", "min": 1 },
    ]));
    assert_single(
        &found,
        "DEFINITION_002",
        "StructureDefinition.differential.element[0]",
        &["Observation.value[x]", "at least 2", "at most 1"],
    );
}

#[test]
fn unbounded_base_slicer_allows_any_slice_minimums_json() {
    let found = observation_profile(serde_json::json!([
        { "id": "Observation.component", "path": "Observation.component",
          "slicing": { "rules": "open" } },
        { "id": "Observation.component:a", "path": "Observation.component",
          "sliceName": "a", "min": 5 },
    ]));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn slice_maximum_over_a_nested_slicer_maximum_xml() {
    // Parameters.parameter:address.part is 3..5; its slice recipientHealthcareService is 1..10.
    assert_single(
        &xml("profile-parameters-complex.profile.xml"),
        "DEFINITION_002",
        "StructureDefinition.differential.element[18].max",
        &[
            "Parameters.parameter:address.part:recipientHealthcareService",
            "10",
            "5",
        ],
    );
}

#[test]
fn huge_slice_minimums_do_not_overflow_json() {
    let huge = i64::MAX;
    let found = observation_profile(serde_json::json!([
        { "id": "Observation.component", "path": "Observation.component",
          "slicing": { "rules": "open" }, "max": "1" },
        { "id": "Observation.component:a", "path": "Observation.component",
          "sliceName": "a", "min": huge },
        { "id": "Observation.component:b", "path": "Observation.component",
          "sliceName": "b", "min": huge },
        { "id": "Observation.component:c", "path": "Observation.component",
          "sliceName": "c", "min": huge },
    ]));
    assert_single(
        &found,
        "DEFINITION_002",
        "StructureDefinition.differential.element[0]",
        &["Observation.component", "at most 1"],
    );
}

#[test]
fn slice_minimums_overflowing_a_u64_maximum_are_reported_json() {
    let huge = i64::MAX;
    let found = observation_profile(serde_json::json!([
        { "id": "Observation.component", "path": "Observation.component",
          "slicing": { "rules": "open" }, "max": u64::MAX.to_string() },
        { "id": "Observation.component:a", "path": "Observation.component",
          "sliceName": "a", "min": huge },
        { "id": "Observation.component:b", "path": "Observation.component",
          "sliceName": "b", "min": huge },
        { "id": "Observation.component:c", "path": "Observation.component",
          "sliceName": "c", "min": huge },
    ]));
    assert_single(
        &found,
        "DEFINITION_002",
        "StructureDefinition.differential.element[0]",
        &["Observation.component"],
    );
}

#[test]
fn differential_without_ids_has_no_slice_findings_json() {
    let found = observation_profile(serde_json::json!([
        { "path": "Observation.component", "slicing": { "rules": "open" }, "max": "1" },
        { "path": "Observation.component", "sliceName": "a", "min": 2 },
    ]));
    assert!(found.is_empty(), "{found:?}");
}

// references.html, Canonical URLs: only resources with canonical URLs "are allowed to be the
// target of references to a canonical URLs".
#[test]
fn canonical_may_not_target_a_non_canonical_resource_json() {
    let found = json(&fixture("sd-canonical-implements-wrong-target.json"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(&*found[0].code, "DEFINITION_004");
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(
        found[0].path,
        "StructureDefinition.differential.element[0].type[0].targetProfile[0]"
    );
    assert!(found[0].message.contains("Patient"), "{}", found[0].message);
}

#[test]
fn canonical_may_not_target_a_non_canonical_resource_xml() {
    let found = xml_source(
        r#"<StructureDefinition xmlns="http://hl7.org/fhir">
  <url value="http://example.org/StructureDefinition/lib"/>
  <name value="Lib"/>
  <status value="draft"/>
  <kind value="resource"/>
  <abstract value="false"/>
  <type value="Library"/>
  <baseDefinition value="http://hl7.org/fhir/StructureDefinition/Library"/>
  <derivation value="constraint"/>
  <differential>
    <element id="Library.relatedArtifact.resource">
      <path value="Library.relatedArtifact.resource"/>
      <type>
        <code value="canonical"/>
        <targetProfile value="http://hl7.org/fhir/StructureDefinition/Patient"/>
      </type>
    </element>
  </differential>
</StructureDefinition>"#,
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(&*found[0].code, "DEFINITION_004");
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(
        found[0].path,
        "StructureDefinition.differential.element[0].type[0].targetProfile[0]"
    );
}

#[test]
fn canonical_may_not_target_a_non_canonical_resource_on_a_direct_child_json() {
    let found = json(
        &serde_json::json!({
            "resourceType": "StructureDefinition",
            "url": "http://example.org/StructureDefinition/activity",
            "name": "Activity",
            "status": "draft",
            "kind": "resource",
            "abstract": false,
            "type": "ActivityDefinition",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/ActivityDefinition",
            "derivation": "constraint",
            "differential": { "element": [{
                "id": "ActivityDefinition.library",
                "path": "ActivityDefinition.library",
                "type": [{
                    "code": "canonical",
                    "targetProfile": ["http://hl7.org/fhir/StructureDefinition/Patient"],
                }],
            }] },
        })
        .to_string(),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(&*found[0].code, "DEFINITION_004");
    assert_eq!(found[0].severity, Severity::Error);
}

#[test]
fn canonical_may_target_canonical_resources_and_unknown_definitions_json() {
    // ViewDefinition comes from a package the Registry does not hold: no finding.
    assert!(json(&fixture("sd-canonical-implements.json")).is_empty());
    let found = library_resource_targets(&[
        "http://hl7.org/fhir/StructureDefinition/Library",
        "http://hl7.org/fhir/StructureDefinition/ValueSet|5.0.0",
        "http://hl7.org/fhir/StructureDefinition/CanonicalResource",
        "http://hl7.org/fhir/StructureDefinition/MetadataResource",
        "http://hl7.org/fhir/StructureDefinition/Resource",
        "http://hl7.org/fhir/StructureDefinition/shareablevalueset",
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn canonical_may_not_target_a_profile_on_a_non_canonical_resource_json() {
    let found = library_resource_targets(&[
        "http://hl7.org/fhir/StructureDefinition/Library",
        "http://hl7.org/fhir/StructureDefinition/vitalsigns",
        "http://hl7.org/fhir/StructureDefinition/Patient|5.0.0",
    ]);
    assert!(
        found
            .iter()
            .all(|d| &*d.code == "DEFINITION_004" && d.severity == Severity::Error)
    );
    let paths: Vec<&str> = found.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "StructureDefinition.differential.element[0].type[0].targetProfile[1]",
            "StructureDefinition.differential.element[0].type[0].targetProfile[2]",
        ]
    );
}

// The R5 core specialization NutritionIntake targets the non-canonical ClinicalUseDefinition.
#[test]
fn specializations_are_not_checked_for_canonical_targets_json() {
    let found = library_targets(
        "specialization",
        "canonical",
        &["http://hl7.org/fhir/StructureDefinition/Patient"],
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn reference_targets_are_not_canonical_targets_json() {
    let found = library_targets(
        "constraint",
        "Reference",
        &["http://hl7.org/fhir/StructureDefinition/Patient"],
    );
    assert!(found.is_empty(), "{found:?}");
}

// R5 extensibility: an `element` context is the id of an element of the base specification.
#[test]
fn extension_context_must_name_a_base_element_json() {
    for name in [
        "ext-stds-status-normative.json",
        "ext-stds-status-deprecated.json",
    ] {
        let found = json(&fixture(name));
        assert_eq!(found.len(), 1, "{name}: {found:?}");
        assert_eq!(&*found[0].code, "DEFINITION_005");
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(found[0].path, "StructureDefinition.context[6].expression");
        assert!(found[0].message.contains("ElementDefinition.targetProfile"));
    }
}

#[test]
fn extension_contexts_naming_base_elements_pass_json() {
    let contexts = [
        ("element", "Element"),
        ("element", "CanonicalResource"),
        ("element", "Observation.value[x]"),
        ("element", "ElementDefinition.type.targetProfile"),
        ("element", "Patient.contact.name"),
        // A slice id from a core profile (vitalsigns), not resolvable by path.
        ("element", "Observation.category:VSCat"),
        ("fhirpath", "Patient.notAnElement"),
        ("extension", "http://example.org/StructureDefinition/other"),
    ];
    let context: Vec<_> = contexts
        .iter()
        .map(|(kind, expression)| serde_json::json!({ "type": kind, "expression": expression }))
        .collect();
    let found = json(
        &serde_json::json!({
            "resourceType": "StructureDefinition",
            "url": "http://example.org/StructureDefinition/ext",
            "name": "Ext",
            "status": "draft",
            "kind": "complex-type",
            "abstract": false,
            "context": context,
            "type": "Extension",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Extension",
            "derivation": "constraint",
        })
        .to_string(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn extension_context_naming_no_element_fails_xml() {
    let found = xml_source(
        r#"<StructureDefinition xmlns="http://hl7.org/fhir">
            <url value="http://example.org/StructureDefinition/ext"/>
            <name value="Ext"/>
            <status value="draft"/>
            <kind value="complex-type"/>
            <abstract value="false"/>
            <context><type value="element"/><expression value="Patient.name"/></context>
            <context><type value="element"/><expression value="Patient.nope"/></context>
            <type value="Extension"/>
            <baseDefinition value="http://hl7.org/fhir/StructureDefinition/Extension"/>
            <derivation value="constraint"/>
        </StructureDefinition>"#,
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(&*found[0].code, "DEFINITION_005");
    assert_eq!(found[0].path, "StructureDefinition.context[1].expression");
}

fn xml_source(source: &str) -> Vec<Diagnostic> {
    definition_findings(&parse_xml(source).expect("parses"))
}

fn library_resource_targets(targets: &[&str]) -> Vec<Diagnostic> {
    library_targets("constraint", "canonical", targets)
}

fn library_targets(derivation: &str, code: &str, targets: &[&str]) -> Vec<Diagnostic> {
    json(
        &serde_json::json!({
            "resourceType": "StructureDefinition",
            "url": "http://example.org/StructureDefinition/lib",
            "name": "Lib",
            "status": "draft",
            "kind": "resource",
            "abstract": false,
            "type": "Library",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Library",
            "derivation": derivation,
            "differential": { "element": [{
                "id": "Library.relatedArtifact.resource",
                "path": "Library.relatedArtifact.resource",
                "type": [{ "code": code, "targetProfile": targets }],
            }] },
        })
        .to_string(),
    )
}
