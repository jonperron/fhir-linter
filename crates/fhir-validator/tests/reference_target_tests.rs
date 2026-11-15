//! Reference target types (`REFERENCE_005`) and uncoded Codings under a required
//! binding (`TERMINOLOGY_001`), against the bundled R5 definitions.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// `(severity, path)` of every diagnostic with `code`.
fn found(json: &str, code: &str) -> Vec<(Severity, String)> {
    let resource = parse_json(json).expect("test JSON parses");
    validate(&resource, registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == code)
        .map(|d| (d.severity, d.path.to_string()))
        .collect()
}

fn patient(field: &str) -> String {
    format!(r#"{{"resourceType":"Patient","id":"p",{field}}}"#)
}

#[test]
fn reference_to_a_type_outside_target_profiles_is_an_error() {
    // Patient.managingOrganization is Reference(Organization).
    for reference in [
        "Practitioner/2",
        "Practitioner/2/_history/1",
        "http://example.org/fhir/Practitioner/2",
    ] {
        let json = patient(&format!(
            r#""managingOrganization":{{"reference":"{reference}"}}"#
        ));
        assert_eq!(
            found(&json, "REFERENCE_005"),
            [(
                Severity::Error,
                "Patient.managingOrganization.reference".to_owned()
            )],
            "{reference}",
        );
    }
}

#[test]
fn reference_to_an_allowed_type_is_clean() {
    for reference in [
        "Organization/2",
        "https://example.org/fhir/Organization/2/_history/3",
    ] {
        let json = patient(&format!(
            r#""managingOrganization":{{"reference":"{reference}"}}"#
        ));
        assert!(found(&json, "REFERENCE_005").is_empty(), "{reference}");
    }
    // Patient.generalPractitioner allows Organization, Practitioner and PractitionerRole.
    let json = patient(r#""generalPractitioner":[{"reference":"PractitionerRole/1"}]"#);
    assert!(found(&json, "REFERENCE_005").is_empty());
}

#[test]
fn nested_references_are_checked_at_every_depth() {
    // Backbone element: Patient.contact.organization is Reference(Organization).
    let json = patient(r#""contact":[{"organization":{"reference":"Patient/1"}}]"#);
    assert_eq!(
        found(&json, "REFERENCE_005"),
        [(
            Severity::Error,
            "Patient.contact[0].organization.reference".to_owned()
        )],
    );
    // Datatype: Identifier.assigner is Reference(Organization).
    let json = patient(r#""identifier":[{"assigner":{"reference":"Device/1"}}]"#);
    assert_eq!(
        found(&json, "REFERENCE_005"),
        [(
            Severity::Error,
            "Patient.identifier[0].assigner.reference".to_owned()
        )],
    );
}

#[test]
fn resource_target_logical_and_urn_references_are_not_type_checked() {
    // Extension.value[x] Reference has no targetProfile; Provenance.target is Reference(Resource).
    let extension = patient(
        r#""extension":[{"url":"http://example.org/x","valueReference":{"reference":"Device/1"}}]"#,
    );
    assert!(found(&extension, "REFERENCE_005").is_empty());
    let provenance = r#"{"resourceType":"Provenance","target":[{"reference":"Device/1"}],
        "recorded":"2024-01-01T00:00:00Z","agent":[{"who":{"reference":"Device/1"}}]}"#;
    assert!(found(provenance, "REFERENCE_005").is_empty());
    for reference in [
        r#"{"reference":"urn:uuid:a6f5d8e2-6a0f-4b4c-9b0a-1d2e3f4a5b6c"}"#,
        r#"{"identifier":{"value":"x"}}"#,
        r#"{"reference":"Practitioner?identifier=x"}"#,
    ] {
        let json = patient(&format!(r#""managingOrganization":{reference}"#));
        assert!(found(&json, "REFERENCE_005").is_empty(), "{reference}");
    }
}

/// A Patient with one contained resource `a` of `contained_type`, linked through
/// `Patient.link.other` (Reference(Patient | RelatedPerson)) with `other`.
fn patient_linking_contained(contained_type: &str, other: &str) -> String {
    patient(&format!(
        r#""contained":[{{"resourceType":"{contained_type}","id":"a"{extra}}}],
        "link":[{{"other":{other},"type":"seealso"}}]"#,
        extra = if contained_type == "Basic" {
            r#","code":{"text":"x"}"#
        } else {
            ""
        },
    ))
}

#[test]
fn contained_reference_to_a_type_outside_target_profiles_is_an_error() {
    let json = patient_linking_contained("Basic", r##"{"reference":"#a"}"##);
    assert_eq!(
        found(&json, "REFERENCE_005"),
        [(
            Severity::Error,
            "Patient.link[0].other.reference".to_owned()
        )],
    );
    let json = patient_linking_contained("Patient", r##"{"reference":"#a"}"##);
    assert!(found(&json, "REFERENCE_005").is_empty());
    // A bare `#` names the container, never a contained resource with an empty id.
    let json = patient(
        r##""contained":[{"resourceType":"Basic","id":"","code":{"text":"x"}}],
        "link":[{"other":{"reference":"#","type":"Patient"},"type":"seealso"}]"##,
    );
    assert!(found(&json, "REFERENCE_005").is_empty());
    assert!(found(&json, "REFERENCE_003").is_empty());
}

#[test]
fn contained_reference_type_must_match_the_contained_resource() {
    let other = r##"{"reference":"#a","type":"Patient"}"##;
    let json = patient_linking_contained("Basic", other);
    assert_eq!(
        found(&json, "REFERENCE_003"),
        [(
            Severity::Error,
            "Patient.link[0].other.reference".to_owned()
        )],
    );
    let json = patient_linking_contained("Patient", other);
    assert!(found(&json, "REFERENCE_003").is_empty());
    // An unknown resourceType (STRUCTURE_000) gives the reference no type to check.
    let json = patient_linking_contained("NotAType", other);
    assert!(found(&json, "REFERENCE_003").is_empty());
    assert!(found(&json, "REFERENCE_005").is_empty());
}

#[test]
fn contained_reference_in_xml_is_type_checked() {
    let codes = |contained: &str| -> Vec<String> {
        let xml = format!(
            r##"<Patient xmlns="http://hl7.org/fhir"><id value="p"/>
            <contained>{contained}</contained>
            <link><other><reference value="#a"/><type value="Patient"/></other>
            <type value="seealso"/></link></Patient>"##
        );
        let resource = parse_xml(&xml).expect("test XML parses");
        validate(&resource, registry())
            .into_iter()
            .map(|d| d.code.to_string())
            .filter(|code| code == "REFERENCE_003" || code == "REFERENCE_005")
            .collect()
    };
    let basic = r#"<Basic><id value="a"/><code><text value="x"/></code></Basic>"#;
    assert_eq!(codes(basic), ["REFERENCE_003", "REFERENCE_005"]);
    assert!(codes(r#"<Patient><id value="a"/></Patient>"#).is_empty());
}

#[test]
fn contained_reference_inside_a_bundle_entry_is_type_checked() {
    let bundle = |contained_type: &str| {
        format!(
            r#"{{"resourceType":"Bundle","type":"collection","entry":[{{
            "fullUrl":"http://example.org/fhir/Patient/p","resource":{}}}]}}"#,
            patient_linking_contained(contained_type, r##"{"reference":"#a","type":"Patient"}"##),
        )
    };
    let paths = |json: &str, code: &str| -> Vec<String> {
        found(json, code)
            .into_iter()
            .map(|(_, path)| path)
            .collect()
    };
    let json = bundle("Basic");
    for code in ["REFERENCE_005", "REFERENCE_003"] {
        let hits = paths(&json, code);
        assert_eq!(hits.len(), 1, "{code}: {hits:?}");
        assert!(
            hits[0].ends_with("link[0].other.reference"),
            "{code}: {hits:?}"
        );
    }
    let json = bundle("Patient");
    assert!(paths(&json, "REFERENCE_005").is_empty());
    assert!(paths(&json, "REFERENCE_003").is_empty());
}

#[test]
fn every_bundled_reference_target_profile_resolves() -> Result<(), Box<dyn std::error::Error>> {
    // So the "unresolvable profile allows any type" branch never hides a core violation.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    let mut unresolved = Vec::new();
    for bundle in [
        "profiles-resources.json",
        "profiles-types.json",
        "profiles-others.json",
    ] {
        let text = std::fs::read_to_string(dir.join(bundle))?;
        let json: serde_json::Value = serde_json::from_str(&text)?;
        let types = json["entry"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|e| e["resource"]["snapshot"]["element"].as_array())
            .flatten()
            .flat_map(|el| el["type"].as_array())
            .flatten();
        for ty in types.filter(|t| t["code"] == "Reference") {
            for url in ty["targetProfile"].as_array().into_iter().flatten() {
                let url = url.as_str().unwrap_or_default();
                let url = url.split('|').next().unwrap_or(url);
                if registry().structure_definition(url).is_none() {
                    unresolved.push(url.to_owned());
                }
            }
        }
    }
    assert!(unresolved.is_empty(), "{unresolved:?}");
    Ok(())
}

#[test]
fn codeable_concept_coding_without_code_under_required_binding_is_reported() {
    let json = r#"{"resourceType":"Condition","subject":{"reference":"Patient/1"},
        "clinicalStatus":{"coding":[
            {"system":"http://terminology.hl7.org/CodeSystem/condition-clinical"}]}}"#;
    assert_eq!(
        found(json, "TERMINOLOGY_001"),
        [(Severity::Error, "Condition.clinicalStatus".to_owned())],
    );
}

#[test]
fn coding_without_code_under_required_binding_is_reported() {
    // Appointment...monthlyTemplate.dayOfWeek is a Coding bound (required) to days-of-week.
    let json = r#"{"resourceType":"Appointment","status":"proposed",
        "recurrenceTemplate":[{"recurrenceType":{"text":"monthly"},"monthlyTemplate":{
        "dayOfWeek":{"system":"http://hl7.org/fhir/days-of-week"},"monthInterval":1}}]}"#;
    assert_eq!(
        found(json, "TERMINOLOGY_001"),
        [(
            Severity::Error,
            "Appointment.recurrenceTemplate[0].monthlyTemplate.dayOfWeek".to_owned()
        )],
    );
    let coded = json.replace(r#"days-of-week""#, r#"days-of-week","code":"mon""#);
    assert!(found(&coded, "TERMINOLOGY_001").is_empty());
}

/// A Patient with one contained Basic per id, all referenced from `Patient.link`.
fn patient_with_contained_ids(ids: &[&str]) -> String {
    let contained: Vec<String> = ids
        .iter()
        .map(|id| format!(r#"{{"resourceType":"Basic","id":"{id}","code":{{"text":"x"}}}}"#))
        .collect();
    let links: Vec<String> = ids
        .iter()
        .map(|id| format!(r##"{{"other":{{"reference":"#{id}"}},"type":"seealso"}}"##))
        .collect();
    patient(&format!(
        r#""contained":[{}],"link":[{}]"#,
        contained.join(","),
        links.join(","),
    ))
}

#[test]
fn duplicate_contained_id_is_an_error() {
    // narrative.html 2.1.4.0.4: ids SHALL be unique within the resource, contained included.
    assert_eq!(
        found(&patient_with_contained_ids(&["a", "a"]), "REFERENCE_006"),
        [(Severity::Error, "Patient.contained[1]".to_owned())],
    );
    assert_eq!(
        found(
            &patient_with_contained_ids(&["a", "b", "a", "a"]),
            "REFERENCE_006"
        ),
        [
            (Severity::Error, "Patient.contained[2]".to_owned()),
            (Severity::Error, "Patient.contained[3]".to_owned()),
        ],
    );
}

#[test]
fn unique_contained_ids_are_clean() {
    assert!(found(&patient_with_contained_ids(&["a", "b"]), "REFERENCE_006").is_empty());
    assert!(found(&patient_with_contained_ids(&["a"]), "REFERENCE_006").is_empty());
}

#[test]
fn duplicate_contained_id_in_xml_is_an_error() {
    let xml = r##"<Patient xmlns="http://hl7.org/fhir"><id value="p"/>
        <contained><Basic><id value="a"/><code><text value="x"/></code></Basic></contained>
        <contained><Basic><id value="a"/><code><text value="y"/></code></Basic></contained>
        <link><other><reference value="#a"/></other><type value="seealso"/></link></Patient>"##;
    let resource = parse_xml(xml).expect("test XML parses");
    let paths: Vec<String> = validate(&resource, registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == "REFERENCE_006")
        .map(|d| d.path.to_string())
        .collect();
    assert_eq!(paths, ["Patient.contained[1]"]);
}

#[test]
fn duplicate_contained_id_inside_a_bundle_entry_is_an_error() {
    let json = format!(
        r#"{{"resourceType":"Bundle","type":"collection","entry":[{{
        "fullUrl":"http://example.org/fhir/Patient/p","resource":{}}}]}}"#,
        patient_with_contained_ids(&["a", "a"]),
    );
    let hits = found(&json, "REFERENCE_006");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].1.ends_with("resource.contained[1]"), "{hits:?}");
}

#[test]
fn official_duplicate_contained_case_reports_the_second_occurrence() {
    // fhir-test-cases validator/patient-duplicate-contained.json: the Java validator
    // reports "Duplicate ID for contained resource: pat1" at Patient.contained[1].
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/validator/patient-duplicate-contained.json");
    let Ok(json) = std::fs::read_to_string(&file) else {
        eprintln!("fixtures submodule not checked out; skipping");
        return;
    };
    assert_eq!(
        found(&json, "REFERENCE_006"),
        [(Severity::Error, "Patient.contained[1]".to_owned())],
    );
}
