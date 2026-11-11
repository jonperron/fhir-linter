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
fn control_character_in_root_id_gets_type_005_and_type_006() {
    // The id regex rejects it (TYPE_005), and `id` is a string type (TYPE_006).
    let registry = load_registry();
    let xml_1_1 = patient_xml(r#"<?xml version="1.1"?>"#, r#"<id value="a&#1;"/>"#);
    for resource in [
        parse_json(r#"{"resourceType":"Patient","id":"a\u0001"}"#).unwrap(),
        parse_xml(&xml_1_1).unwrap(),
    ] {
        let codes: Vec<_> = validate(&resource, &registry)
            .into_iter()
            .filter(|d| d.path == "Patient.id")
            .map(|d| (d.code.to_string(), d.severity))
            .collect();
        assert_eq!(
            codes,
            [
                ("TYPE_005".to_owned(), Severity::Error),
                ("TYPE_006".to_owned(), Severity::Warning),
            ]
        );
    }
}

#[test]
fn xml_null_reference_in_narrative_is_rejected() {
    // `&#0;` is never a legal XML character, in 1.0 or 1.1.
    let decl = r#"<?xml version="1.1"?>"#;
    let narrative = |text: &str| {
        let div = format!(r#"<div xmlns="http://www.w3.org/1999/xhtml">{text}</div>"#);
        patient_xml(decl, &format!("<text>{div}</text>"))
    };
    assert!(parse_xml(&narrative("a&#1;")).is_ok());
    let err = parse_xml(&narrative("a&#0;")).unwrap_err();
    assert!(
        err.to_string().contains("invalid entity reference"),
        "{err}"
    );
}

#[test]
fn json_negative_zero_integer_is_type_001() {
    // R5 integer regex `[0]|[-+]?[1-9][0-9]*` rejects `-0` (PLAN 11.6.4-P11).
    let json = r#"{"resourceType":"Patient","multipleBirthInteger":-0}"#;
    let codes: Vec<_> = validate(&parse_json(json).unwrap(), &load_registry())
        .into_iter()
        .filter(|d| d.path == "Patient.multipleBirthInteger")
        .map(|d| d.code.to_string())
        .collect();
    assert_eq!(codes, ["TYPE_001"]);
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
fn xml_literal_control_character_is_not_well_formed_anywhere() {
    // A literal C0 control is illegal in XML 1.0 and, as a RestrictedChar, in XML 1.1.
    for decl in ["", r#"<?xml version="1.1" encoding="UTF-8"?>"#] {
        for body in [
            "<name><family value=\"a\u{1}b\"/></name>",
            "<!-- \u{1} --><active value=\"true\"/>",
            "\u{1}<active value=\"true\"/>",
        ] {
            let err = parse_xml(&patient_xml(decl, body)).unwrap_err();
            assert!(
                err.to_string().contains("not well-formed"),
                "{body:?}: {err}"
            );
        }
    }
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
