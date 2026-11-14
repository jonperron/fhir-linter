//! `fixed[x]` (`PROFILE_004`), `pattern[x]` (`PROFILE_005`), value-discriminated slice
//! cardinality (`PROFILE_006`) and type profiles (`SimpleQuantity` on `Claim.item.quantity`),
//! against the bundled R5 definitions.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Diagnostic, Severity, validate};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

fn diagnostics(json: &str) -> Vec<Diagnostic> {
    validate(&parse_json(json).expect("test JSON parses"), registry())
}

/// Paths of every diagnostic with `code`.
fn paths(json: &str, code: &str) -> Vec<String> {
    diagnostics(json)
        .into_iter()
        .filter(|d| d.code.as_ref() == code)
        .map(|d| d.path.to_string())
        .collect()
}

fn errors(json: &str) -> Vec<String> {
    diagnostics(json)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| format!("{} {} {}", d.code, d.path, d.message))
        .collect()
}

fn vital_signs(category: &str) -> String {
    format!(
        r#"{{"resourceType":"Observation","id":"o",
        "meta":{{"profile":["http://hl7.org/fhir/StructureDefinition/vitalsigns"]}},
        "status":"final","category":[{category}],
        "code":{{"coding":[{{"system":"http://loinc.org","code":"8867-4"}}]}},
        "subject":{{"reference":"Patient/p"}},"effectiveDateTime":"2024-01-01",
        "valueQuantity":{{"value":60,"unit":"/min","system":"http://unitsofmeasure.org",
        "code":"/min"}}}}"#
    )
}

const VITAL_SIGNS_CATEGORY: &str = r#"{"coding":[{"system":
    "http://terminology.hl7.org/CodeSystem/observation-category","code":"vital-signs"}]}"#;

#[test]
fn vital_signs_with_the_wrong_category_coding_is_an_error() {
    let laboratory = r#"{"coding":[{"system":
        "http://terminology.hl7.org/CodeSystem/observation-category","code":"laboratory"}]}"#;
    assert_eq!(
        paths(&vital_signs(laboratory), "PROFILE_006"),
        ["Observation.category"]
    );
}

#[test]
fn vital_signs_with_the_vital_signs_category_is_clean() {
    assert_eq!(
        errors(&vital_signs(VITAL_SIGNS_CATEGORY)),
        Vec::<String>::new()
    );
    // Another category beside it is allowed: the slicing is open.
    let both = format!(r#"{VITAL_SIGNS_CATEGORY},{{"text":"other"}}"#);
    assert_eq!(errors(&vital_signs(&both)), Vec::<String>::new());
}

#[test]
fn vital_signs_with_the_category_twice_exceeds_the_slice_max() {
    let twice = format!("{VITAL_SIGNS_CATEGORY},{VITAL_SIGNS_CATEGORY}");
    assert_eq!(
        paths(&vital_signs(&twice), "PROFILE_006"),
        ["Observation.category"]
    );
}

fn bundle(profile: &str, ty: &str) -> String {
    format!(
        r#"{{"resourceType":"Bundle","id":"b",
        "meta":{{"profile":["http://hl7.org/fhir/StructureDefinition/{profile}"]}},
        "type":"{ty}"}}"#
    )
}

#[test]
fn fixed_code_must_be_exactly_the_value() {
    assert_eq!(
        paths(&bundle("document-bundle", "collection"), "PROFILE_004"),
        ["Bundle.type"]
    );
    assert!(paths(&bundle("document-bundle", "document"), "PROFILE_004").is_empty());
}

#[test]
fn pattern_code_must_equal_the_value() {
    assert_eq!(
        paths(&bundle("history-bundle", "searchset"), "PROFILE_005"),
        ["Bundle.type"]
    );
    assert!(paths(&bundle("history-bundle", "history"), "PROFILE_005").is_empty());
}

#[test]
fn fixed_code_is_checked_in_xml_input() {
    let xml = r#"<Bundle xmlns="http://hl7.org/fhir"><id value="b"/>
        <meta><profile value="http://hl7.org/fhir/StructureDefinition/document-bundle"/></meta>
        <type value="collection"/></Bundle>"#;
    let found: Vec<_> = validate(&parse_xml(xml).expect("parses"), registry())
        .into_iter()
        .filter(|d| d.code.as_ref() == "PROFILE_004")
        .map(|d| d.path.to_string())
        .collect();
    assert_eq!(found, ["Bundle.type"]);
}

fn library(ty: &str) -> String {
    format!(
        r#"{{"resourceType":"Library","id":"l",
        "meta":{{"profile":["http://hl7.org/fhir/StructureDefinition/logiclibrary"]}},
        "status":"active","type":{ty}}}"#
    )
}

#[test]
fn pattern_codeable_concept_is_a_subset_match() {
    // logiclibrary's R5 pattern is library-type#module-definition (sic).
    // The pattern's coding must match one instance coding; extra codings and text are allowed.
    let extra = r#"{"text":"logic","coding":[{"system":"http://example.org","code":"x"},
        {"system":"http://terminology.hl7.org/CodeSystem/library-type",
        "code":"module-definition","display":"Module Definition"}]}"#;
    assert!(paths(&library(extra), "PROFILE_005").is_empty());
    let wrong = r#"{"coding":[{"system":"http://terminology.hl7.org/CodeSystem/library-type",
        "code":"logic-library"}]}"#;
    assert_eq!(paths(&library(wrong), "PROFILE_005"), ["Library.type"]);
}

fn ldl(high: &str) -> String {
    format!(
        r#"{{"resourceType":"Observation","id":"o",
        "meta":{{"profile":["http://hl7.org/fhir/StructureDefinition/ldlcholesterol"]}},
        "status":"final","code":{{"text":"LDL"}},
        "referenceRange":[{{"high":{high}}}]}}"#
    )
}

#[test]
fn fixed_quantity_at_depth_compares_numbers_as_decimals() {
    // ldlcholesterol fixes Observation.referenceRange.high to {"value": 3.0}.
    for equal in [r#"{"value":3}"#, r#"{"value":3.0}"#, r#"{"value":3.00}"#] {
        assert!(paths(&ldl(equal), "PROFILE_004").is_empty(), "{equal}");
    }
    for different in [r#"{"value":3.1}"#, r#"{"value":3.0,"unit":"mmol/L"}"#] {
        assert_eq!(
            paths(&ldl(different), "PROFILE_004"),
            ["Observation.referenceRange[0].high"],
            "{different}"
        );
    }
}

fn claim(quantity: &str) -> String {
    format!(
        r#"{{"resourceType":"Claim","id":"c","status":"active","use":"claim",
        "type":{{"text":"t"}},"patient":{{"reference":"Patient/p"}},"created":"2024-01-01",
        "insurance":[{{"sequence":1,"focal":true,"coverage":{{"reference":"Coverage/c"}}}}],
        "item":[{{"sequence":1,"quantity":{quantity}}}]}}"#
    )
}

#[test]
fn simple_quantity_type_profile_forbids_the_comparator() {
    let json = claim(r#"{"value":1,"comparator":"<"}"#);
    assert_eq!(
        paths(&json, "CARDINALITY_002"),
        ["Claim.item[0].quantity.comparator"]
    );
    assert_eq!(paths(&json, "INVARIANT_sqty-1"), ["Claim.item[0].quantity"]);
}

#[test]
fn simple_quantity_without_comparator_is_clean() {
    assert_eq!(errors(&claim(r#"{"value":1}"#)), Vec::<String>::new());
}

#[test]
fn fixed_values_inside_slices_do_not_apply_to_other_repetitions() {
    // bp fixes component:SystolicBP.code.coding:SBPCode to loinc#8480-6; neither the other
    // codings of that component nor the diastolic component are bound by it.
    let json = r#"{"resourceType":"Observation","id":"bp",
        "meta":{"profile":["http://hl7.org/fhir/StructureDefinition/bp"]},
        "status":"final","category":[{"coding":[{"system":
        "http://terminology.hl7.org/CodeSystem/observation-category","code":"vital-signs"}]}],
        "code":{"coding":[{"system":"http://loinc.org","code":"85354-9"}]},
        "subject":{"reference":"Patient/p"},"effectiveDateTime":"2024-01-01",
        "component":[
        {"code":{"coding":[{"system":"http://loinc.org","code":"8480-6"},
            {"system":"http://snomed.info/sct","code":"271649006"}]},
         "valueQuantity":{"value":107,"unit":"mmHg","system":"http://unitsofmeasure.org",
            "code":"mm[Hg]"}},
        {"code":{"coding":[{"system":"http://loinc.org","code":"8462-4"}]},
         "valueQuantity":{"value":60,"unit":"mmHg","system":"http://unitsofmeasure.org",
            "code":"mm[Hg]"}}]}"#;
    assert_eq!(errors(json), Vec::<String>::new());
}

#[test]
fn fixed_codeable_concept_matches_a_lone_xml_coding() {
    let xml = r#"<Observation xmlns="http://hl7.org/fhir"><id value="o"/>
        <meta><profile value="http://hl7.org/fhir/StructureDefinition/cholesterol"/></meta>
        <status value="final"/>
        <code><coding><system value="http://loinc.org"/><code value="35200-5"/>
        <display value="Cholesterol [Moles/&#x200B;volume] in Serum or Plasma"/></coding></code>
        <referenceRange><high><value value="4.5"/></high></referenceRange></Observation>"#;
    let found: Vec<_> = validate(&parse_xml(xml).expect("parses"), registry())
        .into_iter()
        .filter(|d| d.code.starts_with("PROFILE_"))
        .map(|d| format!("{} {}", d.code, d.path))
        .collect();
    assert_eq!(found, Vec::<String>::new());
}
