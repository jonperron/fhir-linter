//! XML representation rules checked against the R5 Registry (xml.html, audit 11.6.4).
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::parse_xml;
use fhir_validator::validate;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// `"<severity> <code> <path>"` for every diagnostic of a Patient with `body`, invariant
/// findings aside (a bare Patient has no narrative).
fn patient_diagnostics(body: &str) -> Vec<String> {
    let source = format!("<Patient xmlns=\"http://hl7.org/fhir\">{body}</Patient>");
    let resource = parse_xml(&source).expect("parses");
    validate(&resource, registry())
        .iter()
        .filter(|d| !d.code.starts_with("INVARIANT_"))
        .map(|d| format!("{:?} {} {}", d.severity, d.code, d.path))
        .collect()
}

#[test]
fn xml_attribute_elements_given_as_child_elements_are_reported() {
    let cases = [
        (
            "<active><value value=\"true\"/></active>",
            "Error STRUCTURE_001 Patient.active.value",
        ),
        (
            "<gender><id value=\"g1\"/></gender>",
            "Error STRUCTURE_001 Patient._gender.id",
        ),
        (
            "<extension><url value=\"http://e.org\"/><valueBoolean value=\"true\"/></extension>",
            "Error STRUCTURE_001 Patient.extension.url",
        ),
        (
            "<name><id value=\"n1\"/><family value=\"S\"/></name>",
            "Error STRUCTURE_001 Patient.name.id",
        ),
    ];
    for (body, expected) in cases {
        let found = patient_diagnostics(body);
        assert!(found.iter().any(|d| d == expected), "{body}: {found:?}");
    }
}

#[test]
fn child_elements_given_as_attributes_are_reported() {
    let found = patient_diagnostics("<name family=\"Smith\"/>");
    assert!(
        found
            .iter()
            .any(|d| d == "Error STRUCTURE_001 Patient.name.family"),
        "{found:?}"
    );
}

#[test]
fn elements_named_value_url_and_id_that_are_child_elements_pass() {
    let found = patient_diagnostics(concat!(
        "<identifier id=\"i1\"><value value=\"123\"/></identifier>",
        "<extension url=\"http://e.org\">",
        "<valueQuantity><value value=\"1\"/></valueQuantity>",
        "</extension>",
        "<birthDate id=\"b1\" value=\"2000-01-01\"/>",
    ));
    assert!(found.is_empty(), "{found:?}");
}

// xml.html: readers SHOULD trim attribute values; datatypes.html: non-string primitives
// SHALL NOT have leading or trailing whitespace. User decision: trim, and warn.
#[test]
fn attribute_whitespace_is_trimmed_with_a_warning() {
    let found = patient_diagnostics(concat!(
        "<active value=\" true \"/>",
        "<gender id=\" g1\" value=\"male \"/>",
        "<birthDate value=\"\t2000-01-01\">",
        "<extension url=\" http://e.org \"><valueBoolean value=\"true\"/></extension>",
        "</birthDate>",
    ));
    assert_eq!(
        found,
        [
            "Warning TYPE_006 Patient.active",
            "Warning TYPE_006 Patient.gender",
            "Warning TYPE_006 Patient._gender.id",
            "Warning TYPE_006 Patient.birthDate",
            "Warning TYPE_006 Patient._birthDate.extension.url",
        ],
    );
}
