//! Invariants calling `%terminologies` are answered by the Registry in use: the
//! overlay when definitions are supplied. An answer the Registry cannot decide stays
//! `INVARIANT_SKIPPED`.
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::validate;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const LAYER: &str = "http://example.org/StructureDefinition/patient-terminologies";
const GENDER: &str = "http://hl7.org/fhir/ValueSet/administrative-gender";

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
}

/// The core Patient definition as a constraint at `LAYER` whose root carries `term-1`
/// with `expression`.
fn profile(expression: &str) -> TestResult<Value> {
    let text = std::fs::read_to_string(definitions_dir().join("profiles-resources.json"))?;
    let bundle: Value = serde_json::from_str(&text)?;
    let mut patient = bundle["entry"]
        .as_array()
        .ok_or("profiles-resources.json has no entries")?
        .iter()
        .map(|entry| &entry["resource"])
        .find(|r| r["url"] == "http://hl7.org/fhir/StructureDefinition/Patient")
        .ok_or("core Patient definition not found")?
        .clone();
    patient["url"] = json!(LAYER);
    patient["derivation"] = json!("constraint");
    patient["baseDefinition"] = json!("http://hl7.org/fhir/StructureDefinition/Patient");
    let root = &mut patient["snapshot"]["element"][0];
    assert_eq!(root["path"], "Patient");
    root["constraint"]
        .as_array_mut()
        .ok_or("Patient root has no constraints")?
        .push(json!({
            "key": "term-1",
            "severity": "error",
            "human": "terminology check",
            "expression": expression,
        }));
    Ok(patient)
}

/// The base registry overlaid with the `LAYER` profile carrying `expression`, supplied
/// as one Bundle file in a fresh directory named after `name`.
fn overlay(name: &str, expression: &str) -> TestResult<Registry> {
    let dir = std::env::temp_dir().join(format!("terminologies-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("definitions.json");
    let bundle = json!({
        "resourceType": "Bundle",
        "type": "collection",
        "entry": [{"resource": profile(expression)?}],
    });
    std::fs::write(&file, bundle.to_string())?;
    let base = Arc::new(Registry::from_definitions_dir(definitions_dir())?);
    let registry = Registry::with_extra(base, &[file.as_path()]);
    std::fs::remove_dir_all(&dir)?;
    Ok(registry?)
}

/// `"<code> <path>"` of the `term-1` and skip diagnostics of a male Patient claiming
/// the profile with `expression`.
fn diagnostics(name: &str, expression: &str) -> TestResult<Vec<String>> {
    let registry = overlay(name, expression)?;
    let patient = json!({
        "resourceType": "Patient",
        "meta": {"profile": [LAYER]},
        "gender": "male",
    });
    Ok(validate(&parse_json(&patient.to_string())?, &registry)
        .iter()
        .filter(|d| &*d.code == "INVARIANT_term-1" || &*d.code == "INVARIANT_SKIPPED")
        .map(|d| format!("{} {}", d.code, d.path))
        .collect())
}

/// An expression true when `$validate-code` finds `gender` in the ValueSet at `url`.
fn validate_value_set(url: &str) -> String {
    format!("%terminologies.validateVS('{url}', gender).parameter.where(name = 'result').value")
}

fn expanded_codes() -> String {
    format!("%terminologies.expand('{GENDER}').expansion.contains.code")
}

#[test]
fn code_in_value_set_passes() -> TestResult {
    assert!(diagnostics("vs-in", &validate_value_set(GENDER))?.is_empty());
    Ok(())
}

#[test]
fn code_outside_value_set_fails() -> TestResult {
    let address_use = validate_value_set("http://hl7.org/fhir/ValueSet/address-use");
    assert_eq!(
        diagnostics("vs-out", &address_use)?,
        ["INVARIANT_term-1 Patient"],
    );
    Ok(())
}

#[test]
fn expansion_holding_the_code_passes() -> TestResult {
    let expression = format!("{} contains gender", expanded_codes());
    assert!(diagnostics("expand-in", &expression)?.is_empty());
    Ok(())
}

#[test]
fn expansion_lacking_the_code_fails() -> TestResult {
    let expression = format!("{} contains 'not-a-gender'", expanded_codes());
    assert_eq!(
        diagnostics("expand-out", &expression)?,
        ["INVARIANT_term-1 Patient"],
    );
    Ok(())
}

#[test]
fn versioned_expansion_names_the_loaded_url_and_version() -> TestResult {
    let expression = format!(
        "%terminologies.expand('{GENDER}|5.0.0').select(url = '{GENDER}' and version = '5.0.0')"
    );
    assert!(diagnostics("expand-versioned", &expression)?.is_empty());
    Ok(())
}

#[test]
fn undecided_answer_is_skipped() -> TestResult {
    let filtered = validate_value_set("http://hl7.org/fhir/ValueSet/example-filter");
    assert_eq!(
        diagnostics("undecided", &filtered)?,
        ["INVARIANT_SKIPPED Patient"],
    );
    Ok(())
}
