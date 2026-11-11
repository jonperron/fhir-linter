use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_linter::{Config, RuleRegistry};
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::Severity;

fn registry() -> Registry {
    Registry::from_definitions_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5"),
    )
    .expect("definitions")
}

fn run(json: &str, config: &str) -> Result<Vec<(String, Severity)>, String> {
    let r = parse_json(json).unwrap();
    RuleRegistry::default()
        .run(&r, &registry(), &Config::from_toml(config).unwrap())
        .map(|ds| {
            ds.into_iter()
                .map(|d| (d.rule.unwrap().to_string(), d.severity))
                .collect()
        })
}

const BARE: &str = r#"{"resourceType":"Patient","id":"p"}"#;
const FULL: &str = r#"{"resourceType":"Patient","meta":{"profile":["http://x"]},
    "identifier":[{"value":"1"}],"text":{"status":"generated","div":"<div>hi</div>"}}"#;

#[test]
fn bare_resource_triggers_all_rules() {
    let ids: Vec<_> = run(BARE, "")
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        ids,
        [
            "narrative-absent",
            "missing-identifier",
            "missing-meta-profile"
        ]
    );
}

#[test]
fn complete_resource_is_clean() {
    assert!(run(FULL, "").unwrap().is_empty());
}

#[test]
fn empty_div_counts_as_absent() {
    let json = r#"{"resourceType":"Patient","text":{"status":"generated","div":" "}}"#;
    assert!(
        run(json, "")
            .unwrap()
            .iter()
            .any(|(id, _)| id == "narrative-absent")
    );
}

fn fires(json: &str, rule: &str) -> bool {
    run(json, "").unwrap().iter().any(|(id, _)| id == rule)
}

fn patient_with_div(div: &str) -> String {
    format!(r#"{{"resourceType":"Patient","text":{{"status":"generated","div":"{div}"}}}}"#)
}

// narrative.html: the div SHALL have some non-whitespace content (text or an image).
#[test]
fn div_with_only_markup_and_whitespace_counts_as_absent() {
    for div in [
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\">   </div>"#,
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\"><p> </p><br/></div>"#,
    ] {
        assert!(fires(&patient_with_div(div), "narrative-absent"), "{div}");
    }
}

#[test]
fn div_with_text_or_image_has_content() {
    for div in [
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\"><p>hi</p></div>"#,
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\"><img src=\"x.png\"/></div>"#,
    ] {
        assert!(!fires(&patient_with_div(div), "narrative-absent"), "{div}");
    }
}

// narrative-status `empty`: "No human-readable text provided in this case".
#[test]
fn status_empty_counts_as_absent() {
    let json = r#"{"resourceType":"Patient","text":{"status":"empty",
        "div":"<div xmlns=\"http://www.w3.org/1999/xhtml\">No narrative</div>"}}"#;
    assert!(fires(json, "narrative-absent"));
}

// Bundle.identifier comment: identity only matters for document, message and collection.
#[test]
fn bundle_identifier_expected_only_for_document_message_collection() {
    let bundle = |kind: &str| format!(r#"{{"resourceType":"Bundle","type":"{kind}"}}"#);
    for kind in ["document", "message", "collection"] {
        assert!(fires(&bundle(kind), "missing-identifier"), "{kind}");
    }
    for kind in [
        "searchset",
        "history",
        "batch",
        "transaction",
        "batch-response",
    ] {
        assert!(!fires(&bundle(kind), "missing-identifier"), "{kind}");
    }
}

#[test]
fn empty_identifier_or_profile_array_counts_as_missing() {
    for json in [
        r#"{"resourceType":"Patient","identifier":[],"meta":{"profile":[]}}"#,
        r#"{"resourceType":"Patient","identifier":[null],"meta":{"profile":[null]}}"#,
    ] {
        assert!(fires(json, "missing-identifier"), "{json}");
        assert!(fires(json, "missing-meta-profile"), "{json}");
    }
}

#[test]
fn identifier_and_profile_findings_carry_the_meta_span() {
    let json = r#"{"resourceType":"Patient","meta":{"versionId":"1"}}"#;
    let resource = parse_json(json).unwrap();
    let meta_span = resource.fields.get("meta").unwrap().span;
    let found = RuleRegistry::default()
        .run(&resource, &registry(), &Config::default())
        .unwrap();
    for code in ["LINT_002", "LINT_003"] {
        let d = found.iter().find(|d| d.code.as_ref() == code).unwrap();
        assert_eq!(d.location, Some(meta_span), "{code}");
    }
}

fn fires_xml(xml: &str, rule: &str) -> bool {
    RuleRegistry::default()
        .run(&parse_xml(xml).unwrap(), &registry(), &Config::default())
        .unwrap()
        .iter()
        .any(|d| d.rule.as_deref() == Some(rule))
}

#[test]
fn markup_in_attributes_or_comments_is_not_content() {
    for div in [
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\"><p title=\"a>b\"></p></div>"#,
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\"><!-- a > b --></div>"#,
    ] {
        assert!(fires(&patient_with_div(div), "narrative-absent"), "{div}");
    }
}

// XML whitespace is space, tab, CR and LF only: a non-breaking space is text.
#[test]
fn cdata_space_image_and_malformed_markup_are_content() {
    for div in [
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\"><![CDATA[hi]]></div>"#,
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\">&#160;</div>"#,
        "<div xmlns=\\\"http://www.w3.org/1999/xhtml\\\">\u{a0}</div>",
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\"><p><h:img xmlns:h=\"urn:h\"/></p></div>"#,
        // Malformed XHTML is the validator's finding, not an empty narrative.
        r#"<div xmlns=\"http://www.w3.org/1999/xhtml\"><p></div>"#,
    ] {
        assert!(!fires(&patient_with_div(div), "narrative-absent"), "{div}");
    }
}

const XHTML: &str = r#"xmlns="http://www.w3.org/1999/xhtml""#;

// The XML parser hands the div over as decoded text, not markup.
#[test]
fn xml_div_text_that_looks_like_markup_is_content() {
    let xml = format!(
        r#"<Patient xmlns="http://hl7.org/fhir"><text><status value="generated"/>
        <div {XHTML}>&lt;br/&gt;</div></text></Patient>"#
    );
    assert!(!fires_xml(&xml, "narrative-absent"));
}

// In XML a primitive with an extension is an object holding `value`.
#[test]
fn xml_primitives_with_extensions_are_read() {
    let ext = r#"<extension url="http://x"><valueString value="y"/></extension>"#;
    let patient = format!(
        r#"<Patient xmlns="http://hl7.org/fhir"><text><status value="empty">{ext}</status>
        <div {XHTML}>hi</div></text></Patient>"#
    );
    assert!(fires_xml(&patient, "narrative-absent"));
    let bundle = format!(
        r#"<Bundle xmlns="http://hl7.org/fhir"><type value="document">{ext}</type></Bundle>"#
    );
    assert!(fires_xml(&bundle, "missing-identifier"));
}

#[test]
fn resource_without_text_or_identifier_element_is_not_flagged() {
    let ids: Vec<_> = run(r#"{"resourceType":"Parameters"}"#, "")
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(ids, ["missing-meta-profile"]);
}

#[test]
fn config_disables_and_overrides_severity() {
    let cfg = "[rules.missing-identifier]\nenabled = false\n[rules.narrative-absent]\nseverity = \"error\"";
    let found = run(BARE, cfg).unwrap();
    assert!(!found.iter().any(|(id, _)| id == "missing-identifier"));
    assert!(found.contains(&("narrative-absent".into(), Severity::Error)));
}

#[test]
fn unknown_rule_or_severity_is_an_error() {
    assert!(run(BARE, "[rules.nope]\nenabled = false").is_err());
    assert!(run(BARE, "[rules.narrative-absent]\nseverity = \"loud\"").is_err());
}

/// Configured severities are FHIR IssueSeverity codes (`success` is not a
/// diagnostic level); `hint` is a config alias output as `information`.
#[test]
fn config_severity_maps_to_issue_severity() {
    for (configured, output) in [
        ("fatal", "fatal"),
        ("error", "error"),
        ("warning", "warning"),
        ("information", "information"),
        ("hint", "information"),
    ] {
        let cfg = format!("[rules.narrative-absent]\nseverity = \"{configured}\"");
        let found = run(BARE, &cfg).unwrap_or_else(|e| panic!("{configured}: {e}"));
        let (_, severity) = found
            .iter()
            .find(|(id, _)| id == "narrative-absent")
            .unwrap();
        assert_eq!(severity.as_str(), output, "configured {configured}");
    }
}

#[test]
fn config_severity_outside_issue_severity_is_an_error() {
    for configured in ["success", "Error"] {
        let cfg = format!("[rules.narrative-absent]\nseverity = \"{configured}\"");
        assert!(run(BARE, &cfg).is_err(), "configured {configured}");
    }
}
