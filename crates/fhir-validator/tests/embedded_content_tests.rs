//! contentReference elements (PLAN 11.6.3-V6), contained and Bundle entry resources (the
//! A3 gap) and their XML form (11.6.4-P1), against the bundled R5 definitions.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Diagnostic, Severity, validate};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn registry() -> TestResult<&'static Registry> {
    static REGISTRY: OnceLock<Result<Registry, String>> = OnceLock::new();
    let loaded = REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).map_err(|e| e.to_string())
    });
    loaded.as_ref().map_err(|e| e.clone().into())
}

/// `(code, path)` of every diagnostic but `INVARIANT_SKIPPED`.
fn pairs(found: Vec<Diagnostic>) -> Vec<(String, String)> {
    found
        .into_iter()
        .filter(|d| d.code.as_ref() != "INVARIANT_SKIPPED")
        .map(|d| (d.code.to_string(), d.path))
        .collect()
}

fn json(source: &str) -> TestResult<Vec<(String, String)>> {
    Ok(pairs(validate(&parse_json(source)?, registry()?)))
}

fn xml(source: &str) -> TestResult<Vec<(String, String)>> {
    Ok(pairs(validate(&parse_xml(source)?, registry()?)))
}

/// Errors only, as `(code, path)`.
fn errors(found: Vec<Diagnostic>) -> Vec<(String, String)> {
    pairs(
        found
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect(),
    )
}

fn has(found: &[(String, String)], code: &str, path: &str) -> bool {
    found.iter().any(|(c, p)| c == code && p == path)
}

// ── contentReference ──────────────────────────────────────────────────────────

#[test]
fn content_reference_item_is_walked() -> TestResult {
    let found = json(
        r#"{"resourceType":"Questionnaire","status":"draft","item":[{"linkId":"1",
        "type":"group","item":[{"type":"string","bogus":1}]}]}"#,
    )?;
    let path = "Questionnaire.item[0].item[0]";
    assert!(
        has(&found, "STRUCTURE_001", &format!("{path}.bogus")),
        "{found:?}"
    );
    assert!(
        has(&found, "CARDINALITY_001", &format!("{path}.linkId")),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn content_reference_item_gets_types_and_invariants_at_every_depth() -> TestResult {
    let found = json(
        r#"{"resourceType":"Questionnaire","status":"draft","item":[{"linkId":"1",
        "type":"group","item":[{"linkId":"2","type":"group","item":[{"linkId":"3",
        "type":"display","required":"yes"}]}]}]}"#,
    )?;
    let path = "Questionnaire.item[0].item[0].item[0]";
    assert!(
        has(&found, "TYPE_001", &format!("{path}.required")),
        "{found:?}"
    );
    // que-6: display items cannot be required (Questionnaire.item, through the reference).
    assert!(has(&found, "INVARIANT_que-6", path), "{found:?}");
    Ok(())
}

#[test]
fn valid_nested_items_stay_clean() -> TestResult {
    let found = errors(validate(
        &parse_json(
            r#"{"resourceType":"Questionnaire","status":"draft","item":[{"linkId":"1",
            "type":"group","item":[{"linkId":"2","type":"string","required":true}]}]}"#,
        )?,
        registry()?,
    ));
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}

#[test]
fn xml_content_reference_item_is_reshaped_and_walked() -> TestResult {
    let found = xml(
        r#"<Questionnaire xmlns="http://hl7.org/fhir"><status value="draft"/><item>
        <linkId value="1"/><type value="group"/><item><linkId value="2"/>
        <type value="string"/><required value="true"/><bogus value="x"/></item></item>
        </Questionnaire>"#,
    )?;
    let path = "Questionnaire.item.item";
    assert!(
        has(&found, "STRUCTURE_001", &format!("{path}.bogus")),
        "{found:?}"
    );
    assert!(
        !found.iter().any(|(c, _)| c.starts_with("TYPE_")),
        "{found:?}"
    );
    Ok(())
}

// ── contained resources ───────────────────────────────────────────────────────

#[test]
fn contained_resource_is_validated_against_its_own_type() -> TestResult {
    let found = json(
        r##"{"resourceType":"Patient","contained":[{"resourceType":"Practitioner","id":"p1",
        "bogus":1,"active":"yes"}],"generalPractitioner":[{"reference":"#p1"}]}"##,
    )?;
    assert!(
        has(&found, "STRUCTURE_001", "Patient.contained[0].bogus"),
        "{found:?}"
    );
    assert!(
        has(&found, "TYPE_001", "Patient.contained[0].active"),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn contained_resource_id_and_type_are_checked() -> TestResult {
    let found = json(
        r#"{"resourceType":"Patient","contained":[{"resourceType":"Practitioner",
        "id":"bad id!"},{"resourceType":"Nope","id":"n"},{"id":"x"},
        {"resourceType":"DomainResource","id":"d"}]}"#,
    )?;
    assert!(
        has(&found, "TYPE_005", "Patient.contained[0].id"),
        "{found:?}"
    );
    for i in 1..=3 {
        let path = format!("Patient.contained[{i}]");
        assert!(has(&found, "STRUCTURE_000", &path), "{path}: {found:?}");
    }
    Ok(())
}

#[test]
fn contained_resource_references_resolve_against_the_container() -> TestResult {
    // `#` is the container and `#o1` a sibling (references.html, ref-1).
    let found = json(
        r##"{"resourceType":"Patient","contained":[{"resourceType":"Organization","id":"o1"},
        {"resourceType":"Organization","id":"o2","partOf":{"reference":"#o1"},
        "endpoint":[{"reference":"#"}]},
        {"resourceType":"Organization","id":"o3","partOf":{"reference":"#nope"}}],
        "managingOrganization":{"reference":"#o2"},
        "generalPractitioner":[{"reference":"#o3"},{"reference":"#o1"}]}"##,
    )?;
    let refs: Vec<_> = found
        .iter()
        .filter(|(c, _)| c.starts_with("REFERENCE_"))
        .collect();
    assert_eq!(
        refs,
        [&(
            "REFERENCE_002".to_owned(),
            "Patient.contained[2].partOf.reference".to_owned(),
        )],
        "{found:?}",
    );
    Ok(())
}

#[test]
fn contained_non_object_null_and_non_string_type_items_are_reported() -> TestResult {
    let found = json(
        r#"{"resourceType":"Patient","contained":[1,null,{"resourceType":5},
        {"resourceType":"Practitioner","id":"ok"}]}"#,
    )?;
    assert!(has(&found, "TYPE_001", "Patient.contained[0]"), "{found:?}");
    assert!(
        has(&found, "STRUCTURE_000", "Patient.contained[2]"),
        "{found:?}"
    );
    assert!(!found.iter().any(|(_, p)| p == "Patient.contained[1]"));
    assert!(!has(&found, "STRUCTURE_000", "Patient.contained[3]"));
    Ok(())
}

#[test]
fn xml_contained_array_and_unknown_wrapper_are_handled() -> TestResult {
    let found = xml(
        r#"<Patient xmlns="http://hl7.org/fhir"><contained><Practitioner><active value="yes"/>
        </Practitioner></contained><contained><Nope><x value="1"/></Nope></contained></Patient>"#,
    )?;
    assert!(
        has(&found, "TYPE_001", "Patient.contained[0].active"),
        "{found:?}"
    );
    // An unknown wrapper is kept as is, so it has no resourceType.
    assert!(
        has(&found, "STRUCTURE_000", "Patient.contained[1]"),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn contained_resource_runs_its_root_invariants() -> TestResult {
    // org-1 on the contained Organization's own root; dom-2 on the container, and the
    // nested contained resource it reports is still validated.
    let found = json(
        r#"{"resourceType":"Patient","contained":[{"resourceType":"Organization","id":"a",
        "contained":[{"resourceType":"Organization","id":"b","name":"B","bogus":1}]}]}"#,
    )?;
    assert!(
        has(&found, "INVARIANT_org-1", "Patient.contained[0]"),
        "{found:?}"
    );
    assert!(has(&found, "INVARIANT_dom-2", "Patient"), "{found:?}");
    let nested = "Patient.contained[0].contained[0].bogus";
    assert!(has(&found, "STRUCTURE_001", nested), "{found:?}");
    assert!(!has(
        &found,
        "INVARIANT_org-1",
        "Patient.contained[0].contained[0]"
    ));
    Ok(())
}

#[test]
fn xml_contained_organization_reference_is_clean_like_json() -> TestResult {
    let body = r##"<contained><Organization><id value="o"/><name value="Acme"/>
        </Organization></contained><managingOrganization><reference value="#o"/>
        </managingOrganization>"##;
    let found = errors(validate(
        &parse_xml(&format!(
            r#"<Patient xmlns="http://hl7.org/fhir">{body}</Patient>"#
        ))?,
        registry()?,
    ));
    let from_json = errors(validate(
        &parse_json(
            r##"{"resourceType":"Patient","contained":[{"resourceType":"Organization",
            "id":"o","name":"Acme"}],"managingOrganization":{"reference":"#o"}}"##,
        )?,
        registry()?,
    ));
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(found, from_json);
    Ok(())
}

#[test]
fn xml_contained_resource_with_a_bad_field_is_reported() -> TestResult {
    let found = xml(
        r#"<Patient xmlns="http://hl7.org/fhir"><contained><Practitioner><id value="p"/>
        <active value="yes"/><bogus value="1"/></Practitioner></contained></Patient>"#,
    )?;
    assert!(
        has(&found, "STRUCTURE_001", "Patient.contained.bogus"),
        "{found:?}"
    );
    assert!(
        has(&found, "TYPE_001", "Patient.contained.active"),
        "{found:?}"
    );
    Ok(())
}

// ── Bundle entries and Parameters ─────────────────────────────────────────────

#[test]
fn bundle_entry_resource_is_validated() -> TestResult {
    let found = json(
        r#"{"resourceType":"Bundle","type":"collection","entry":[{"fullUrl":
        "urn:uuid:3f1c7e2a-0d5b-4c8e-9a6f-2b7d1e4c5a90","resource":{"resourceType":
        "Patient","active":"yes"}}]}"#,
    )?;
    assert!(
        has(&found, "TYPE_001", "Bundle.entry[0].resource.active"),
        "{found:?}"
    );
    // The entry is its own resource: its root invariants run at its path (dom-6).
    assert!(
        has(&found, "INVARIANT_dom-6", "Bundle.entry[0].resource"),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn bundle_entry_hash_reference_is_its_own_root() -> TestResult {
    // A Bundle entry is not contained: `#` alone has no container.
    let found = json(
        r##"{"resourceType":"Bundle","type":"collection","entry":[{"resource":
        {"resourceType":"Patient","managingOrganization":{"reference":"#"}}}]}"##,
    )?;
    let path = "Bundle.entry[0].resource.managingOrganization.reference";
    assert!(has(&found, "REFERENCE_002", path), "{found:?}");
    Ok(())
}

#[test]
fn bundle_entry_meta_profile_is_applied() -> TestResult {
    let found = json(
        r#"{"resourceType":"Bundle","type":"collection","entry":[{"resource":
        {"resourceType":"Observation","meta":{"profile":
        ["http://hl7.org/fhir/StructureDefinition/vitalsigns"]},"status":"final",
        "code":{"text":"x"}}}]}"#,
    )?;
    let path = "Bundle.entry[0].resource.subject";
    assert!(has(&found, "CARDINALITY_001", path), "{found:?}");
    Ok(())
}

#[test]
fn parameters_resource_is_validated() -> TestResult {
    let found = json(
        r#"{"resourceType":"Parameters","parameter":[{"name":"p","resource":
        {"resourceType":"Patient","bogus":1}}]}"#,
    )?;
    let path = "Parameters.parameter[0].resource.bogus";
    assert!(has(&found, "STRUCTURE_001", path), "{found:?}");
    Ok(())
}

#[test]
fn xml_message_bundle_has_no_false_bdl_12() -> TestResult {
    let found = xml(
        r#"<Bundle xmlns="http://hl7.org/fhir"><identifier><system value="urn:x"/>
        <value value="1"/></identifier><type value="message"/>
        <timestamp value="2024-01-01T00:00:00Z"/><entry>
        <fullUrl value="urn:uuid:3f1c7e2a-0d5b-4c8e-9a6f-2b7d1e4c5a90"/><resource>
        <MessageHeader><eventCanonical value="http://example.org/e"/><source>
        <endpointUrl value="http://example.org/s"/></source></MessageHeader></resource>
        </entry></Bundle>"#,
    )?;
    assert!(!has(&found, "INVARIANT_bdl-12", "Bundle"), "{found:?}");
    Ok(())
}

#[test]
fn xml_bundle_entry_resource_is_validated() -> TestResult {
    let found = xml(
        r#"<Bundle xmlns="http://hl7.org/fhir"><type value="collection"/><entry><resource>
        <Patient><active value="yes"/></Patient></resource></entry></Bundle>"#,
    )?;
    assert!(
        has(&found, "TYPE_001", "Bundle.entry.resource.active"),
        "{found:?}"
    );
    Ok(())
}
