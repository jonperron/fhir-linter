//! The root `<id>` the XML parser lifts into `Resource::id`: its place in the element
//! order (`STRUCTURE_004`) and the location of its `TYPE_007` trim warning.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{Span, parse_xml};
use fhir_validator::{Diagnostic, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

fn diagnostics(source: &str, code: &str) -> Vec<Diagnostic> {
    let resource = parse_xml(source).expect("parses");
    validate(&resource, registry())
        .into_iter()
        .filter(|d| &*d.code == code)
        .collect()
}

/// The span of the first `<id` in `source`, on line 1.
fn id_span(source: &str) -> Span {
    let offset = source.find("<id").expect("has an <id>");
    Span {
        line: 1,
        col: u32::try_from(offset + 1).expect("short"),
        offset: u32::try_from(offset).expect("short"),
    }
}

#[test]
fn root_id_after_a_later_sibling_is_out_of_order() {
    let source =
        r#"<Patient xmlns="http://hl7.org/fhir"><active value="true"/><id value="p"/></Patient>"#;
    let found = diagnostics(source, "STRUCTURE_004");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path, "Patient.id");
    assert_eq!(found[0].location, Some(id_span(source)));
}

#[test]
fn root_id_with_an_extension_is_reported_once_when_out_of_order() {
    let source = concat!(
        r#"<Patient xmlns="http://hl7.org/fhir"><active value="true"/><id value="p">"#,
        r#"<extension url="http://e.org"><valueBoolean value="true"/></extension></id>"#,
        "</Patient>",
    );
    let found = diagnostics(source, "STRUCTURE_004");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].location, Some(id_span(source)));
}

#[test]
fn root_id_first_is_in_order() {
    let source =
        r#"<Patient xmlns="http://hl7.org/fhir"><id value="p"/><active value="true"/></Patient>"#;
    assert!(diagnostics(source, "STRUCTURE_004").is_empty());
}

#[test]
fn trimmed_resource_id_warning_is_located_at_the_id_element() {
    let source = r#"<Patient xmlns="http://hl7.org/fhir"><id value=" p "/></Patient>"#;
    let found = diagnostics(source, "TYPE_007");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path, "Patient.id");
    assert_eq!(found[0].location, Some(id_span(source)));
}
