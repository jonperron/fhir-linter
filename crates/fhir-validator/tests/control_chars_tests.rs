//! R5 datatypes.html: strings SHOULD NOT contain code points below U+0020 other than
//! tab, CR and LF (PLAN 11.6.4-P12). JSON: Warning `TYPE_006`; XML: not well-formed.
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Diagnostic, Severity, validate};

fn load_registry() -> Registry {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
}

fn type_006(json: &str, registry: &Registry) -> Vec<Diagnostic> {
    validate(&parse_json(json).unwrap(), registry)
        .into_iter()
        .filter(|d| d.code.as_ref() == "TYPE_006")
        .collect()
}

#[test]
fn json_control_character_in_string_is_a_warning() {
    let registry = load_registry();
    for (json, path) in [
        (
            r#"{"resourceType":"Patient","name":[{"family":"a\u0001b"}]}"#,
            "Patient.name[0].family",
        ),
        (
            r#"{"resourceType":"Patient","gender":"male\u001f"}"#,
            "Patient.gender",
        ),
        (
            r#"{"resourceType":"Patient","name":[{"given":["ok","\u0000"]}]}"#,
            "Patient.name[0].given[1]",
        ),
    ] {
        let found = type_006(json, &registry);
        assert_eq!(found.len(), 1, "{json}: {found:?}");
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(found[0].path, path);
        assert!(found[0].location.is_some());
    }
}

#[test]
fn official_control_char_fixtures_match_the_reference_validator() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/validator");
    let (Ok(json), Ok(xml)) = (
        std::fs::read_to_string(dir.join("unicode-problem.json")),
        std::fs::read_to_string(dir.join("unicode-problem.xml")),
    ) else {
        assert!(
            std::env::var_os("CI").is_none(),
            "fixtures submodule missing in CI: run `git submodule update --init`"
        );
        return;
    };
    // Reference outcomes: a warning for JSON, fatal for XML.
    assert_eq!(type_006(&json, &load_registry()).len(), 1);
    assert!(parse_xml(&xml).is_err());
}

#[test]
fn json_tab_cr_lf_in_string_are_allowed() {
    let json = r#"{"resourceType":"Patient","name":[{"family":"a\tb\r\nc"}]}"#;
    assert!(type_006(json, &load_registry()).is_empty());
}

fn patient_xml(decl: &str, body: &str) -> String {
    format!(r#"{decl}<Patient xmlns="http://hl7.org/fhir">{body}</Patient>"#)
}

#[test]
fn xml_1_0_control_character_is_not_well_formed() {
    let decl_1_0 = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
    for decl in ["", decl_1_0] {
        for body in [
            r#"<name><family value="a&#1;b"/></name>"#,
            r#"<name><family value="a&#x1F;b"/></name>"#,
            "<name><family value=\"a\u{1}b\"/></name>",
            r#"<id value="a&#2;"/>"#,
            r#"<text><div xmlns="http://www.w3.org/1999/xhtml">a&#3;</div></text>"#,
        ] {
            let err = parse_xml(&patient_xml(decl, body)).unwrap_err();
            assert!(err.to_string().contains("not well-formed"), "{body}: {err}");
        }
    }
    let ok = r#"<name><family value="a&#9;b&#10;c&#13;"/></name>"#;
    assert!(parse_xml(&patient_xml(decl_1_0, ok)).is_ok());
}

#[test]
fn xml_1_1_control_reference_is_a_warning() {
    // XML 1.1 allows `&#x1D;` as a reference; the reference validator warns
    // (fixtures `xml_v11.xml`).
    let decl = r#"<?xml version="1.1" encoding="UTF-8"?>"#;
    let xml = patient_xml(decl, r#"<name><family value="a&#x1D;b"/></name>"#);
    let found: Vec<_> = validate(&parse_xml(&xml).unwrap(), &load_registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == "TYPE_006")
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].path, "Patient.name.family");
}
