//! Invariants that depend on the FHIR data model mapping into FHIRPath (R5
//! fhirpath.html §2.1.9.1): typed temporals, the primitive split
//! representation, `hasValue()`, `children()` and null placeholders.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::validate;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// Diagnostics about constraint `key`: its violation, or a skip naming it.
fn about(resource: &fhir_parser::Resource, key: &str) -> Vec<String> {
    let skipped = format!("constraint {key} not checked");
    validate(resource, registry())
        .iter()
        .filter(|d| {
            d.code.strip_prefix("INVARIANT_") == Some(key) || d.message.starts_with(&skipped)
        })
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

fn json_about(json: &str, key: &str) -> Vec<String> {
    about(&parse_json(json).unwrap(), key)
}

fn xml_about(xml: &str, key: &str) -> Vec<String> {
    about(&parse_xml(xml).unwrap(), key)
}

fn appointment(start: &str, end: &str) -> String {
    format!(
        r#"{{"resourceType":"Appointment","status":"booked",
        "participant":[{{"status":"accepted","actor":{{"display":"a"}}}}],
        "start":"{start}","end":"{end}"}}"#
    )
}

#[test]
fn app_5_compares_instants_in_utc_not_as_text() {
    // 10:00+02:00 is 08:00Z, before 09:30Z; as text "10" > "09".
    let fired = json_about(
        &appointment("2020-01-01T10:00:00+02:00", "2020-01-01T09:30:00Z"),
        "app-5",
    );
    assert!(fired.is_empty(), "{fired:?}");
    // 10:30+02:00 is 08:30Z, before 10:00Z; as text "10:30" > "10:00".
    let fired = json_about(
        &appointment("2020-01-01T10:00:00Z", "2020-01-01T10:30:00+02:00"),
        "app-5",
    );
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert!(fired[0].starts_with("INVARIANT_app-5 "), "{fired:?}");
}

#[test]
fn task_inv_1_compares_date_times_in_utc() {
    let task = |last_modified: &str, authored_on: &str| {
        format!(
            r#"{{"resourceType":"Task","status":"draft","intent":"order",
            "lastModified":"{last_modified}","authoredOn":"{authored_on}"}}"#
        )
    };
    // Same instant written with two offsets: equal, so `>=` holds.
    let fired = json_about(
        &task("2020-01-01T12:00:00+01:00", "2020-01-01T11:00:00Z"),
        "inv-1",
    );
    assert!(fired.is_empty(), "{fired:?}");
    // 12:00+01:00 is 11:00Z, before 11:30Z, though "12" > "11" as text.
    let fired = json_about(
        &task("2020-01-01T12:00:00+01:00", "2020-01-01T11:30:00Z"),
        "inv-1",
    );
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert!(fired[0].starts_with("INVARIANT_inv-1 "), "{fired:?}");
}

#[test]
fn xml_appointment_compares_instants_too() {
    let xml = r#"<Appointment xmlns="http://hl7.org/fhir">
        <status value="booked"/>
        <start value="2020-01-01T10:00:00+02:00"/>
        <end value="2020-01-01T09:30:00Z"/>
        <participant><actor><display value="a"/></actor><status value="accepted"/></participant>
        </Appointment>"#;
    let fired = xml_about(xml, "app-5");
    assert!(fired.is_empty(), "{fired:?}");
}

#[test]
fn extensions_on_a_primitive_without_value_make_it_present() {
    // `_start` alone (say a data-absent-reason) is a start element with no
    // value: it exists, and `start <= end` is empty, so app-5 is not true.
    let json = r#"{"resourceType":"Appointment","status":"booked",
        "participant":[{"status":"accepted","actor":{"display":"a"}}],
        "_start":{"extension":[{"url":"http://hl7.org/fhir/StructureDefinition/data-absent-reason",
        "valueCode":"unknown"}]},"end":"2020-01-01T09:30:00Z"}"#;
    let fired = json_about(json, "app-5");
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert!(fired[0].starts_with("INVARIANT_app-5 "), "{fired:?}");
}

#[test]
fn bdl_10_needs_a_timestamp_value_not_only_extensions() {
    let bundle =
        |timestamp: &str| format!(r#"{{"resourceType":"Bundle","type":"document"{timestamp}}}"#);
    let with_value = bundle(r#","timestamp":"2020-01-01T10:00:00Z""#);
    assert!(json_about(&with_value, "bdl-10").is_empty());
    let extension_only = bundle(
        r#","_timestamp":{"extension":[{"url":"http://hl7.org/fhir/StructureDefinition/data-absent-reason",
        "valueCode":"unknown"}]}"#,
    );
    let fired = json_about(&extension_only, "bdl-10");
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert!(fired[0].starts_with("INVARIANT_bdl-10 "), "{fired:?}");
}
