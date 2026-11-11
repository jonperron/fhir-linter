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
