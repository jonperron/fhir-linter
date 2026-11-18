//! dom-3 counts references from the narrative: R5 narrative.html §2.1.4.0.3 lets
//! `img/@src` name a contained Binary by `#id`, and the reference validator (official
//! cases narrative-binary.xml / narrative-binary-bad.xml) reports no dom-3 for it.
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_parser::{Resource, parse_json, parse_xml};
use fhir_validator::validate;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Paths of every `INVARIANT_dom-3` (or dom-3 skip) diagnostic.
fn dom3(resource: &Resource) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    let registry = Registry::from_definitions_dir(dir)?;
    Ok(validate(resource, &registry)
        .into_iter()
        .filter(|d| {
            d.code.as_ref() == "INVARIANT_dom-3"
                || (d.code.as_ref() == "INVARIANT_SKIPPED"
                    && d.message.starts_with("constraint dom-3 "))
        })
        .map(|d| d.path)
        .collect())
}

/// A JSON Patient with `div` as its narrative and `contained` as its contained array.
fn patient(div: &str, contained: &str) -> String {
    format!(
        r#"{{"resourceType":"Patient",
            "text":{{"status":"generated",
                "div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">{div}</div>"}},
            "contained":[{contained}]}}"#
    )
}

const BASIC_B: &str = r#"{"resourceType":"Basic","id":"b","code":{"text":"x"}}"#;
const BASIC_C: &str = r#"{"resourceType":"Basic","id":"c","code":{"text":"x"}}"#;

#[test]
fn image_in_the_narrative_references_a_contained_binary_json() -> TestResult {
    let json = patient(
        r##"<p><img src=\"#pic1\"/></p>"##,
        r#"{"resourceType":"Binary","id":"pic1","contentType":"image/gif","data":"R0lGODlh"}"#,
    );
    assert_eq!(dom3(&parse_json(&json)?)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn link_in_the_narrative_references_a_contained_resource_json() -> TestResult {
    let json = patient(r##"<p><a href=\"#b\">b</a></p>"##, BASIC_B);
    assert_eq!(dom3(&parse_json(&json)?)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn image_in_the_narrative_references_a_contained_binary_xml() -> TestResult {
    let xml = r##"<Patient xmlns="http://hl7.org/fhir">
  <text>
    <status value="generated"/>
    <div xmlns="http://www.w3.org/1999/xhtml"><p><img src="#pic1"/></p></div>
  </text>
  <contained>
    <Binary><id value="pic1"/><contentType value="image/gif"/><data value="R0lGODlh"/></Binary>
  </contained>
</Patient>"##;
    assert_eq!(dom3(&parse_xml(xml)?)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn link_in_the_narrative_references_a_contained_resource_xml() -> TestResult {
    let xml = r##"<Patient xmlns="http://hl7.org/fhir">
  <text>
    <status value="generated"/>
    <div xmlns="http://www.w3.org/1999/xhtml"><p><a href="#b">b</a></p></div>
  </text>
  <contained><Basic><id value="b"/><code><text value="x"/></code></Basic></contained>
</Patient>"##;
    assert_eq!(dom3(&parse_xml(xml)?)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn contained_resource_not_referenced_from_the_narrative_still_violates_dom3() -> TestResult {
    // Text, an `id`, another fragment or an external link do not reference `b`.
    let json = patient(
        r##"<p id=\"b\">#b <a href=\"#c\">c</a> <a href=\"http://x.org/#b\">x</a></p>"##,
        &format!("{BASIC_B},{BASIC_C}"),
    );
    assert_eq!(dom3(&parse_json(&json)?)?, vec!["Patient".to_owned()]);
    Ok(())
}

#[test]
fn references_from_a_narrative_referenced_contained_resource_still_count() -> TestResult {
    // `o` is named only by the narrative; `b` only by `o`.
    let observation = r##"{"resourceType":"Observation","id":"o","status":"final",
        "code":{"text":"x"},"focus":[{"reference":"#b"}]}"##;
    let json = patient(
        r##"<p><a href=\"#o\">o</a></p>"##,
        &format!("{observation},{BASIC_B}"),
    );
    assert_eq!(dom3(&parse_json(&json)?)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn contained_resource_without_a_narrative_still_violates_dom3() -> TestResult {
    let json = format!(r#"{{"resourceType":"Patient","contained":[{BASIC_B}]}}"#);
    assert_eq!(dom3(&parse_json(&json)?)?, vec!["Patient".to_owned()]);
    Ok(())
}

/// R5 publishes the ValueSet root without dom-2..dom-6 (only cnl-0 and its own keys), yet
/// ValueSet is a DomainResource and inherits them (official case contained-canonical-bad).
#[test]
fn unreferenced_contained_resource_in_a_value_set_violates_dom3() -> TestResult {
    let json = format!(r#"{{"resourceType":"ValueSet","status":"draft","contained":[{BASIC_B}]}}"#);
    assert_eq!(dom3(&parse_json(&json)?)?, vec!["ValueSet".to_owned()]);
    Ok(())
}

/// The DomainResource invariants apply (dom-6: a narrative is advised), but not Base's ele-1,
/// an element rule rather than a resource invariant.
#[test]
fn value_set_inherits_domain_resource_but_not_base_root_constraints() -> TestResult {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    let registry = Registry::from_definitions_dir(dir)?;
    let resource = parse_json(r#"{"resourceType":"ValueSet","status":"draft"}"#)?;
    let codes: Vec<_> = validate(&resource, &registry)
        .into_iter()
        .filter(|d| d.code.starts_with("INVARIANT_"))
        .map(|d| d.code.to_string())
        .collect();
    assert_eq!(codes, ["INVARIANT_dom-6"]);
    Ok(())
}
