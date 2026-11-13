//! Primitive formats checked with the R5 registry regexes (PLAN 11.2-A4, 11.6.3-V8)
//! and the `string` length limit (11.6.3-V9), against the bundled R5 definitions.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Severity, validate};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn registry() -> TestResult<&'static Registry> {
    static REGISTRY: OnceLock<Result<Registry, String>> = OnceLock::new();
    let loaded = REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).map_err(|e| e.to_string())
    });
    loaded.as_ref().map_err(|e| e.clone().into())
}

/// `(code, path)` of every `TYPE_*` diagnostic for `json`.
fn type_findings(json: &str) -> TestResult<Vec<(String, String)>> {
    Ok(validate(&parse_json(json)?, registry()?)
        .into_iter()
        .filter(|d| d.code.starts_with("TYPE_"))
        .map(|d| (d.code.to_string(), d.path))
        .collect())
}

/// A Patient with one extension whose `key` holds `value` (a JSON literal).
fn extension(key: &str, value: &serde_json::Value) -> String {
    let url = r#""url":"http://x.org/e""#;
    format!(r#"{{"resourceType":"Patient","extension":[{{{url},"{key}":{value}}}]}}"#)
}

/// A Patient with `value` in an element of type `type_code`, and that element's path.
fn patient(type_code: &str, value: &str) -> (String, &'static str) {
    let value = serde_json::Value::String(value.to_owned());
    match type_code {
        "string" => (
            format!(r#"{{"resourceType":"Patient","name":[{{"family":{value}}}]}}"#),
            "Patient.name[0].family",
        ),
        "markdown" => (
            extension("valueMarkdown", &value),
            "Patient.extension[0].valueMarkdown",
        ),
        "base64Binary" => (
            format!(r#"{{"resourceType":"Patient","photo":[{{"data":{value}}}]}}"#),
            "Patient.photo[0].data",
        ),
        "uri" => (
            format!(r#"{{"resourceType":"Patient","implicitRules":{value}}}"#),
            "Patient.implicitRules",
        ),
        "url" => (
            format!(r#"{{"resourceType":"Patient","photo":[{{"url":{value}}}]}}"#),
            "Patient.photo[0].url",
        ),
        "canonical" => (
            format!(r#"{{"resourceType":"Patient","meta":{{"profile":[{value}]}}}}"#),
            "Patient.meta.profile[0]",
        ),
        "oid" => (
            extension("valueOid", &value),
            "Patient.extension[0].valueOid",
        ),
        "uuid" => (
            extension("valueUuid", &value),
            "Patient.extension[0].valueUuid",
        ),
        other => panic!("no fixture for {other}"),
    }
}

fn assert_case(type_code: &str, value: &str, expected: Option<&str>) -> TestResult {
    let (json, path) = patient(type_code, value);
    let found = type_findings(&json)?;
    let wanted: Vec<(String, String)> = expected
        .map(|code| (code.to_owned(), path.to_owned()))
        .into_iter()
        .collect();
    assert_eq!(found, wanted, "{type_code} {value:?}");
    Ok(())
}

#[test]
fn string_follows_the_r5_regex_and_needs_non_whitespace() -> TestResult {
    assert_case("string", "Chalmers", None)?;
    assert_case("string", "é ü", None)?;
    assert_case("string", "", Some("TYPE_003"))?;
    // User decision 11.7-A4 (revised): whitespace-only is the TYPE_008 Warning, since
    // datatypes.html only says strings SHOULD contain non-whitespace content.
    assert_case("string", "   ", Some("TYPE_008"))?;
    assert_case("string", "\n\t", Some("TYPE_008"))
}

#[test]
fn markdown_follows_the_r5_regex_and_needs_non_whitespace() -> TestResult {
    assert_case("markdown", "# Title\n\n*text*", None)?;
    assert_case("markdown", "", Some("TYPE_003"))?;
    assert_case("markdown", "  ", Some("TYPE_008"))
}

#[test]
fn base64_binary_follows_the_r5_regex() -> TestResult {
    assert_case("base64Binary", "aGVsbG8=", None)?;
    // datatypes.html: "reading applications should ignore whitespace characters (per RFC 4648)".
    assert_case("base64Binary", " aGVs\tbG8=\r\n", None)?;
    assert_case("base64Binary", " \n", Some("TYPE_008"))?;
    assert_case("base64Binary", "", Some("TYPE_003"))?;
    assert_case("base64Binary", "!!!", Some("TYPE_003"))?;
    assert_case("base64Binary", "abc", Some("TYPE_003"))
}

#[test]
fn uri_with_whitespace_is_rejected() -> TestResult {
    assert_case("uri", "http://example.org/rules", None)?;
    assert_case("uri", "urn:oid:1.2.3", None)?;
    // 11.6.3-V8: R5 regex `\S*`.
    assert_case("uri", "http://a b", Some("TYPE_003"))?;
    assert_case("uri", "", Some("TYPE_003"))
}

#[test]
fn url_with_whitespace_is_rejected() -> TestResult {
    assert_case("url", "http://example.org/a.png", None)?;
    assert_case("url", "http://a b", Some("TYPE_003"))
}

#[test]
fn canonical_with_whitespace_is_rejected() -> TestResult {
    assert_case(
        "canonical",
        "http://example.org/StructureDefinition/x|1.0",
        None,
    )?;
    assert_case("canonical", "http://example.org/ x", Some("TYPE_003"))
}

#[test]
fn oid_follows_the_r5_regex() -> TestResult {
    assert_case("oid", "urn:oid:1.2.840.10008", None)?;
    assert_case("oid", "1.2.840", Some("TYPE_003"))?;
    assert_case("oid", "urn:oid:3.1", Some("TYPE_003"))
}

#[test]
fn uuid_follows_the_r5_regex() -> TestResult {
    assert_case(
        "uuid",
        "urn:uuid:c757873d-ec9a-4326-a141-556f43239520",
        None,
    )?;
    assert_case(
        "uuid",
        "c757873d-ec9a-4326-a141-556f43239520",
        Some("TYPE_003"),
    )?;
    assert_case(
        "uuid",
        "urn:uuid:C757873D-EC9A-4326-A141-556F43239520",
        Some("TYPE_003"),
    )
}

#[test]
fn string_longer_than_max_length_is_type_002() -> TestResult {
    // 11.6.3-V9: string.value maxLength 1,048,576, counted in characters, not bytes.
    let at_limit = "é".repeat(1_048_576);
    assert_case("string", &at_limit, None)?;
    let over = "a".repeat(1_048_577);
    assert_case("string", &over, Some("TYPE_002"))?;
    assert_case("string", &"é".repeat(1_048_577), Some("TYPE_002"))?;
    // Length first: an over-long whitespace-only string is the TYPE_002 error, not a warning.
    assert_case("string", &" ".repeat(1_048_577), Some("TYPE_002"))
}

#[test]
fn invalid_base64_binary_message_omits_the_payload() -> TestResult {
    // The payload can be large or confidential; the message names the path only.
    let (json, _) = patient("base64Binary", "c2VjcmV0!");
    let messages: Vec<String> = validate(&parse_json(&json)?, registry()?)
        .into_iter()
        .filter(|d| d.code.as_ref() == "TYPE_003")
        .map(|d| d.message.to_string())
        .collect();
    assert_eq!(
        messages,
        ["Invalid base64Binary format at 'Patient.photo[0].data'"],
    );
    Ok(())
}

#[test]
fn regex_is_anchored_at_both_ends() -> TestResult {
    assert_case("oid", "urn:oid:1.2.3 x", Some("TYPE_003"))?;
    assert_case("oid", "x urn:oid:1.2.3", Some("TYPE_003"))
}

#[test]
fn non_string_value_for_a_regex_checked_type_is_type_001() -> TestResult {
    let found = type_findings(r#"{"resourceType":"Patient","implicitRules":5}"#)?;
    assert_eq!(
        found,
        [("TYPE_001".to_owned(), "Patient.implicitRules".to_owned())]
    );
    Ok(())
}

#[test]
fn whitespace_only_string_is_a_warning_inside_a_backbone_element() -> TestResult {
    let json = r#"{"resourceType":"Patient","contact":[{"name":{"family":"  "}}]}"#;
    let found: Vec<_> = validate(&parse_json(json)?, registry()?)
        .into_iter()
        .filter(|d| d.code.starts_with("TYPE_"))
        .map(|d| (d.code.to_string(), d.severity, d.path))
        .collect();
    let path = "Patient.contact[0].name.family".to_owned();
    assert_eq!(found, [("TYPE_008".to_owned(), Severity::Warning, path)]);
    Ok(())
}

#[test]
fn official_whitespace_cases_have_no_error() -> TestResult {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/validator");
    for name in [
        "primitive-good-ws.json",
        "primitive-good-ws.xml",
        "params-ws.json",
    ] {
        let text = std::fs::read_to_string(dir.join(name))?;
        let resource = if name.ends_with(".xml") {
            parse_xml(&text)?
        } else {
            parse_json(&text)?
        };
        let errors: Vec<_> = validate(&resource, registry()?)
            .into_iter()
            .filter(|d| d.severity <= Severity::Error)
            .map(|d| (d.code, d.path))
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
    }
    Ok(())
}
