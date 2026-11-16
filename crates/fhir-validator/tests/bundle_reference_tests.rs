//! References inside a Bundle (bundle.html 2.4.5.1, resolving references in Bundles) and the
//! agreement of `Bundle.entry.fullUrl` with the resource id (Bundle.entry.fullUrl: "The
//! fullUrl SHALL NOT disagree with the id in the resource").
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::{Resource, parse_json, parse_xml};
use fhir_validator::validate;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// `(code, path)` of each `REFERENCE_*` diagnostic.
fn findings(resource: &Resource) -> Vec<(String, String)> {
    validate(resource, registry())
        .into_iter()
        .filter(|d| d.code.starts_with("REFERENCE_"))
        .map(|d| (d.code.to_string(), d.path))
        .collect()
}

fn pair(code: &str, path: &str) -> (String, String) {
    (code.to_owned(), path.to_owned())
}

/// An official case; a missing fixture fails (run `git submodule update --init`).
fn official(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/validator")
        .join(name);
    std::fs::read_to_string(path).expect("fixtures submodule checked out")
}

#[test]
fn full_url_disagreeing_with_the_resource_id_is_an_error_in_xml() {
    let resource = parse_xml(&official("entry-resource-bad1.xml")).unwrap();
    assert_eq!(
        findings(&resource),
        vec![pair("REFERENCE_007", "Bundle.entry[0]")]
    );
}

#[test]
fn full_url_disagreeing_with_the_resource_id_is_an_error_in_json() {
    let json = r#"{"resourceType":"Bundle","type":"collection","entry":[
        {"fullUrl":"http://acme.com/Patient/pat1","resource":{"resourceType":"Patient","id":"pat1"}},
        {"fullUrl":"http://acme.com/fhir/Patient/pat1","resource":{"resourceType":"Patient","id":"pat2"}},
        {"fullUrl":"urn:uuid:9e2a3c3e-5d1c-4d3e-9a7c-0b0c7e3b1a01","resource":{"resourceType":"Patient","id":"x"}},
        {"fullUrl":"http://acme.com/Patient/pat3","resource":{"resourceType":"Patient"}}
    ]}"#;
    assert_eq!(
        findings(&parse_json(json).unwrap()),
        vec![pair("REFERENCE_007", "Bundle.entry[1]")]
    );
}

/// An Observation whose `subject` is `reference` (with `type` when given), in a Bundle that
/// also holds a Patient and a Medication under `urn:uuid:` fullUrls.
fn bundle_json(reference: &str, declared: Option<&str>) -> String {
    let declared = declared.map_or(String::new(), |t| format!(r#","type":"{t}""#));
    format!(
        r#"{{"resourceType":"Bundle","type":"collection","entry":[
        {{"fullUrl":"urn:uuid:00000000-0000-4000-8000-000000000001",
          "resource":{{"resourceType":"Patient"}}}},
        {{"fullUrl":"urn:uuid:00000000-0000-4000-8000-000000000002",
          "resource":{{"resourceType":"Basic","code":{{"text":"x"}}}}}},
        {{"fullUrl":"urn:uuid:00000000-0000-4000-8000-000000000003",
          "resource":{{"resourceType":"Observation","status":"final","code":{{"text":"x"}},
            "subject":{{"reference":"{reference}"{declared}}}}}}}
    ]}}"#
    )
}

#[test]
fn urn_uuid_reference_resolves_to_its_entry_in_json() {
    let patient = "urn:uuid:00000000-0000-4000-8000-000000000001";
    let basic = "urn:uuid:00000000-0000-4000-8000-000000000002";
    let path = "Bundle.entry[2].resource.subject.reference";
    let check =
        |reference, declared| findings(&parse_json(&bundle_json(reference, declared)).unwrap());
    assert_eq!(check(patient, None), vec![]);
    assert_eq!(check(patient, Some("Patient")), vec![]);
    assert_eq!(
        check(patient, Some("Group")),
        vec![pair("REFERENCE_003", path)]
    );
    assert_eq!(check(basic, None), vec![pair("REFERENCE_005", path)]);
    // Never report a reference that does not resolve.
    let missing = "urn:uuid:00000000-0000-4000-8000-0000000000ff";
    assert_eq!(check(missing, Some("Group")), vec![]);
}

#[test]
fn urn_uuid_reference_resolves_to_its_entry_in_xml() {
    let xml = r##"<Bundle xmlns="http://hl7.org/fhir">
  <type value="collection"/>
  <entry>
    <fullUrl value="urn:uuid:00000000-0000-4000-8000-000000000002"/>
    <resource><Basic><code><text value="x"/></code></Basic></resource>
  </entry>
  <entry>
    <fullUrl value="urn:uuid:00000000-0000-4000-8000-000000000003"/>
    <resource>
      <Observation>
        <contained>
          <Observation>
            <id value="c1"/>
            <status value="final"/>
            <code><text value="x"/></code>
            <subject><reference value="urn:uuid:00000000-0000-4000-8000-000000000002"/></subject>
          </Observation>
        </contained>
        <status value="final"/>
        <code><text value="x"/></code>
        <subject><reference value="urn:uuid:00000000-0000-4000-8000-000000000002"/></subject>
        <hasMember><reference value="#c1"/></hasMember>
      </Observation>
    </resource>
  </entry>
</Bundle>"##;
    assert_eq!(
        findings(&parse_xml(xml).unwrap()),
        vec![
            pair(
                "REFERENCE_005",
                "Bundle.entry[1].resource.subject.reference"
            ),
            pair(
                "REFERENCE_005",
                "Bundle.entry[1].resource.contained.subject.reference"
            ),
        ]
    );
}

#[test]
fn versioned_full_url_compares_the_id_not_the_version() {
    let json = r#"{"resourceType":"Bundle","type":"history","entry":[
        {"fullUrl":"http://acme.com/Patient/p1/_history/2",
         "resource":{"resourceType":"Patient","id":"p1"}},
        {"fullUrl":"http://acme.com/Patient/p1/_history/2",
         "resource":{"resourceType":"Patient","id":"p2"}}
    ]}"#;
    let found = findings(&parse_json(json).unwrap());
    assert_eq!(found, vec![pair("REFERENCE_007", "Bundle.entry[1]")]);
}

/// A Bundle whose entries `types` all share one fullUrl, plus an Observation whose subject
/// references that fullUrl.
fn shared_full_url_bundle(types: &[&str]) -> String {
    let url = "urn:uuid:00000000-0000-4000-8000-0000000000aa";
    let entries: Vec<String> = types
        .iter()
        .map(|t| format!(r#"{{"fullUrl":"{url}","resource":{{"resourceType":"{t}"}}}}"#))
        .collect();
    format!(
        r#"{{"resourceType":"Bundle","type":"collection","entry":[{},
        {{"resource":{{"resourceType":"Observation","status":"final","code":{{"text":"x"}},
          "subject":{{"reference":"{url}","type":"Group"}}}}}}]}}"#,
        entries.join(",")
    )
}

#[test]
fn shared_full_url_resolves_only_when_the_types_agree() {
    let path = "Bundle.entry[2].resource.subject.reference";
    let check = |types: &[&str]| findings(&parse_json(&shared_full_url_bundle(types)).unwrap());
    assert_eq!(
        check(&["Patient", "Patient"]),
        vec![pair("REFERENCE_003", path)]
    );
    assert_eq!(check(&["Patient", "Basic"]), vec![]);
}

#[test]
fn nested_bundle_references_resolve_in_the_innermost_bundle() {
    let outer = "urn:uuid:00000000-0000-4000-8000-0000000000b1";
    let inner = "urn:uuid:00000000-0000-4000-8000-0000000000b2";
    let json = format!(
        r#"{{"resourceType":"Bundle","type":"collection","entry":[
        {{"fullUrl":"{outer}","resource":{{"resourceType":"Basic","code":{{"text":"x"}}}}}},
        {{"resource":{{"resourceType":"Bundle","type":"collection","entry":[
            {{"fullUrl":"{inner}","resource":{{"resourceType":"Basic","code":{{"text":"x"}}}}}},
            {{"resource":{{"resourceType":"Observation","status":"final","code":{{"text":"x"}},
              "subject":{{"reference":"{inner}"}},
              "focus":[{{"reference":"{outer}","type":"Patient"}}]}}}}]}}}}
    ]}}"#
    );
    assert_eq!(
        findings(&parse_json(&json).unwrap()),
        vec![pair(
            "REFERENCE_005",
            "Bundle.entry[1].resource.entry[1].resource.subject.reference"
        )]
    );
}
