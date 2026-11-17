//! Attachment.size and Attachment.hash against the decoded Attachment.data
//! (datatypes.html: size counts the bytes before base64 encoding, hash is the
//! base64 SHA-1 of those bytes), on the official validator test cases.
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

fn attachment_findings(resource: &Resource) -> Vec<Diagnostic> {
    validate(resource, registry())
        .into_iter()
        .filter(|d| d.code.starts_with("ATTACHMENT_"))
        .collect()
}

fn json(name: &str) -> Vec<Diagnostic> {
    attachment_findings(&parse_json(&fixture(name)).expect("parses"))
}

fn xml(source: &str) -> Vec<Diagnostic> {
    attachment_findings(&parse_xml(source).expect("parses"))
}

fn assert_one_error(found: &[Diagnostic], code: &str, path: &str, message: &str) {
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(&*found[0].code, code);
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(found[0].path, path);
    assert!(found[0].message.contains(message), "{}", found[0].message);
}

#[test]
fn wrong_size_of_line_wrapped_data_is_an_error() {
    let found = json("base64-ws/r5-wrong-size.json");
    let path = "Parameters.parameter[0].valueAttachment";
    assert_one_error(&found, "ATTACHMENT_001", path, "size 999");
    assert!(found[0].message.contains("116"), "{}", found[0].message);
}

#[test]
fn wrong_hash_of_line_wrapped_data_is_an_error() {
    let found = json("base64-ws/r5-wrong-hash.json");
    let path = "Parameters.parameter[0].valueAttachment";
    assert_one_error(
        &found,
        "ATTACHMENT_002",
        path,
        "0TUjD0LsI0HVJR71SRAloD71d+o=",
    );
}

#[test]
fn document_reference_with_wrong_size_and_hash_gets_both_errors() {
    let found = json("dr-bad-att-hash.json");
    let codes: Vec<&str> = found.iter().map(|d| &*d.code).collect();
    assert_eq!(codes, ["ATTACHMENT_001", "ATTACHMENT_002"], "{found:?}");
    assert!(
        found
            .iter()
            .all(|d| d.path == "DocumentReference.content[0].attachment")
    );
    assert!(found[0].message.contains("190") && found[0].message.contains("189"));
    assert!(found[1].message.contains("25WudypM2fY1GUNkG1iaD+5FsQQ="));
}

#[test]
fn matching_size_and_hash_are_accepted() {
    assert!(json("base64-ws/r5-clean.json").is_empty());
    assert!(json("base64-ws/r5-spaces.json").is_empty());
    assert!(xml(&fixture("base64-ws/r5-xml.xml")).is_empty());
}

#[test]
fn xml_wrong_size_and_hash_are_errors() {
    let source = fixture("base64-ws/r5-xml.xml");
    // A lone XML element is not an array, so the path has no index.
    let path = "Parameters.parameter.valueAttachment";
    let wrong_size = source.replace(r#"<size value="116"/>"#, r#"<size value="117"/>"#);
    assert_one_error(&xml(&wrong_size), "ATTACHMENT_001", path, "117");
    let wrong_hash = source.replace(
        "0TUjD0LsI0HVJR71SRAloD71d+o=",
        "LEZwlbCg5nvlH2vQD4DLIRiEbdw=",
    );
    assert_one_error(
        &xml(&wrong_hash),
        "ATTACHMENT_002",
        path,
        "LEZwlbCg5nvlH2vQD4DLIRiEbdw=",
    );
}

#[test]
fn no_data_means_nothing_to_compare() {
    // The content may live at `url`: size and hash then describe bytes the validator lacks.
    let source = r#"{"resourceType": "Parameters", "parameter": [{"name": "a",
        "valueAttachment": {"url": "http://example.org/a", "size": "5",
        "hash": "LEZwlbCg5nvlH2vQD4DLIRiEbdw="}}]}"#;
    assert!(attachment_findings(&parse_json(source).expect("parses")).is_empty());
}

#[test]
fn size_given_as_a_json_number_is_still_compared() {
    // "aGVscCBpJ20gYSBidWc=" is 15 bytes; the number shape itself is a TYPE_001.
    let source = r#"{"resourceType": "Parameters", "parameter": [{"name": "a",
        "valueAttachment": {"data": "aGVscCBpJ20gYSBidWc=", "size": 100}}]}"#;
    let found = attachment_findings(&parse_json(source).expect("parses"));
    let path = "Parameters.parameter[0].valueAttachment";
    assert_one_error(&found, "ATTACHMENT_001", path, "size 100");
}

#[test]
fn undecodable_data_is_left_to_the_format_check() {
    let source = r#"{"resourceType": "Parameters", "parameter": [{"name": "a",
        "valueAttachment": {"data": "a$==", "size": "5"}}]}"#;
    assert!(attachment_findings(&parse_json(source).expect("parses")).is_empty());
}
