//! XML conformance rules from the FHIR R5 `xml.html` page (audit 11.6.4).
use fhir_parser::{ParseError, parse_xml};

const FHIR_NS: &str = "xmlns=\"http://hl7.org/fhir\"";

/// The `XmlError` message for `source`; panics when it parses.
fn xml_error(source: &str) -> String {
    match parse_xml(source) {
        Err(ParseError::XmlError { message, .. }) => message,
        other => panic!("expected an XmlError for {source:?}, got {other:?}"),
    }
}

#[test]
fn dtd_is_rejected() {
    let source = format!("<!DOCTYPE Patient><Patient {FHIR_NS}><active value=\"true\"/></Patient>");
    assert!(xml_error(&source).contains("DTD"));
}

#[test]
fn non_utf8_encoding_declaration_is_rejected() {
    for encoding in ["UTF-16", "ISO-8859-1"] {
        let source = format!("<?xml version=\"1.0\" encoding=\"{encoding}\"?><Patient {FHIR_NS}/>");
        assert!(xml_error(&source).contains("UTF-8"), "{encoding}");
    }
}

#[test]
fn utf8_encoding_declaration_is_accepted_in_any_case() {
    for encoding in ["UTF-8", "utf-8"] {
        let source = format!("<?xml version=\"1.0\" encoding=\"{encoding}\"?><Patient {FHIR_NS}/>");
        assert!(parse_xml(&source).is_ok(), "{encoding}");
    }
}
