//! Narrative invariants txt-1 and txt-2 (`htmlChecks()`, R5 narrative.html#xhtml),
//! each case in JSON and in XML input.
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Severity, validate};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Finding = (Severity, String, String);

const XHTML: &str = "http://www.w3.org/1999/xhtml";

fn registry() -> Result<Registry, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    Ok(Registry::from_definitions_dir(dir)?)
}

/// `(severity, code, path)` of every txt-1 / txt-2 diagnostic, skips included, for an
/// Encounter whose narrative content is `inner`, parsed from JSON and from XML.
fn findings(
    registry: &Registry,
    inner: &str,
) -> Result<[Vec<Finding>; 2], Box<dyn std::error::Error>> {
    let div = format!("<div xmlns=\"{XHTML}\">{inner}</div>");
    let json = format!(
        "{{\"resourceType\":\"Encounter\",\"text\":{{\"status\":\"generated\",\"div\":{}}},\
         \"status\":\"completed\"}}",
        serde_json::to_string(&div)?,
    );
    let xml = format!(
        "<Encounter xmlns=\"http://hl7.org/fhir\"><text><status value=\"generated\"/>{div}\
         </text><status value=\"completed\"/></Encounter>",
    );
    let about = |resource| -> Vec<Finding> {
        validate(&resource, registry)
            .into_iter()
            .filter(|d| {
                ["txt-1", "txt-2"].iter().any(|key| {
                    d.code.as_ref() == format!("INVARIANT_{key}")
                        || (d.code.as_ref() == "INVARIANT_SKIPPED"
                            && d.message.starts_with(&format!("constraint {key} ")))
                })
            })
            .map(|d| (d.severity, d.code.to_string(), d.path))
            .collect()
    };
    Ok([about(parse_json(&json)?), about(parse_xml(&xml)?)])
}

/// txt-1 and txt-2 share the expression `htmlChecks()`, so a narrative that breaks
/// either rule fails both.
fn errors() -> Vec<Finding> {
    ["txt-1", "txt-2"]
        .map(|key| {
            (
                Severity::Error,
                format!("INVARIANT_{key}"),
                "Encounter.text.div".to_owned(),
            )
        })
        .to_vec()
}

#[test]
fn txt1_fails_for_active_or_unlisted_content() -> TestResult {
    let registry = registry()?;
    for inner in [
        "<script>alert(1)</script>",
        "<p onclick=\"x()\">hi</p>",
        "<a href=\"javascript:alert(1)\">x</a>",
        "<a href=\" JavaScript:alert(1)\">x</a>",
        "<img src=\"vbscript:x\" alt=\"x\"/>",
        "<a href=\"data:text/html;base64,PHNjcmlwdD4=\">x</a>",
        "<p><object data=\"x.swf\">x</object></p>",
        "<form><p>x</p></form>",
        "<iframe src=\"https://example.org\">x</iframe>",
        "<p><font color=\"red\">deprecated</font></p>",
        "<p><ins>change marking, chapter 9.4</ins></p>",
        "<p><a xmlns:xlink=\"http://www.w3.org/1999/xlink\" xlink:href=\"x\">x</a></p>",
    ] {
        let [json, xml] = findings(&registry, inner)?;
        assert_eq!(json, errors(), "JSON {inner}");
        assert_eq!(xml, errors(), "XML {inner}");
    }
    Ok(())
}

#[test]
fn txt1_and_txt2_pass_for_basic_formatting() -> TestResult {
    let registry = registry()?;
    for inner in [
        "<p>plain</p>",
        "<p><b>bold</b> and <i>italic</i></p>",
        "<table class=\"grid\" border=\"1\"><tr><th scope=\"col\">a</th></tr>\
         <tr><td colspan=\"2\" style=\"color: red\">b</td></tr></table>",
        "<a name=\"top\">anchor</a> <a href=\"https://example.org/x?a=1\">link</a>",
        "<img src=\"data:image/png;base64,AAAA\" alt=\"pic\"/>",
        "<img src=\"#photo\" alt=\"contained\"/>",
        "<ul><li lang=\"en\" dir=\"ltr\">x</li></ul>",
        "<pre xml:space=\"preserve\">x</pre>",
        "<x:b xmlns:x=\"http://www.w3.org/1999/xhtml\">prefixed</x:b>",
    ] {
        let [json, xml] = findings(&registry, inner)?;
        assert_eq!(json, vec![], "JSON {inner}");
        assert_eq!(xml, vec![], "XML {inner}");
    }
    Ok(())
}

#[test]
fn txt2_fails_for_whitespace_only_content() -> TestResult {
    let registry = registry()?;
    for inner in [" \n\t ", "<p> </p>", ""] {
        let [json, xml] = findings(&registry, inner)?;
        assert_eq!(json, errors(), "JSON {inner:?}");
        assert_eq!(xml, errors(), "XML {inner:?}");
    }
    Ok(())
}

#[test]
fn txt2_passes_for_an_image_alone() -> TestResult {
    let registry = registry()?;
    let [json, xml] = findings(&registry, "<img src=\"#photo\" alt=\"\"/>")?;
    assert_eq!(json, vec![]);
    assert_eq!(xml, vec![]);
    Ok(())
}

/// txt-1 errors on an official fixture, parsed from its extension.
fn fixture_txt1_errors(
    registry: &Registry,
    file: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(file);
    let source = std::fs::read_to_string(path)?;
    let resource = if file.ends_with(".xml") {
        parse_xml(&source)?
    } else {
        parse_json(&source)?
    };
    Ok(validate(&resource, registry)
        .iter()
        .filter(|d| d.code.as_ref() == "INVARIANT_txt-1")
        .count())
}

#[test]
fn official_fixtures_pass_or_fail_txt1_as_expected() -> TestResult {
    let registry = registry()?;
    // ex-list.xml starts with a byte order mark; list-xhtml-correct2 uses a prefixed div.
    for file in [
        "r5/narrative/ex-list.xml",
        "validator/list-xhtml-correct2.xml",
    ] {
        assert_eq!(fixture_txt1_errors(&registry, file)?, 0, "{file}");
    }
    for file in [
        "validator/list-xhtml-attribute.xml",
        "validator/pat-security-bad-xhtml.json",
    ] {
        assert!(fixture_txt1_errors(&registry, file)? > 0, "{file}");
    }
    Ok(())
}

#[test]
fn txt1_runs_on_the_narrative_of_a_contained_resource() -> TestResult {
    let registry = registry()?;
    let json = r#"{"resourceType":"Encounter","status":"completed",
        "contained":[{"resourceType":"Patient","id":"p","text":{"status":"generated",
            "div":"<div xmlns=\"http://www.w3.org/1999/xhtml\"><script>x</script></div>"}}]}"#;
    let paths: Vec<String> = validate(&parse_json(json)?, &registry)
        .into_iter()
        .filter(|d| d.code.as_ref() == "INVARIANT_txt-1")
        .map(|d| d.path)
        .collect();
    assert_eq!(paths, vec!["Encounter.contained[0].text.div".to_owned()]);
    Ok(())
}
