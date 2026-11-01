use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_linter::{Config, RuleRegistry};
use fhir_parser::parse_json;
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
