//! Runs the official `fhir-test-cases/validator` manifest (R5, self-contained cases).
//!
//! Cases with supporting profiles/packages are skipped. The expected result
//! comes from the inline `java` entry (`errorCount` or `outcome`) or from the
//! OperationOutcome file it names under `outcomes/`; expected errors are its
//! `error` + `fatal` issues. Each case is judged on "has errors" vs "expected
//! errors". JSON and XML cases are counted apart and each pass rate must not
//! drop below its floor. A case that fails to parse counts as "has errors",
//! so parser regressions lower the rate.
use std::path::{Path, PathBuf};

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Severity, validate};
use serde_json::Value;

/// Regression floors (rates measured on the pinned fixtures, rounded down);
/// raise them as validator coverage improves.
const MIN_JSON_PASS_RATE: f64 = 0.66;
// 0.58 -> 0.56: primitive-good.xml and primitive-bad-empty.xml only passed through a false
// TYPE_004 on the valid code "asd asd"; their real errors (decimal regex) are not caught yet.
// 0.56 -> 0.63: the XML conformance checks (namespaces, mixed text, encoding) catch 8 more
// invalid cases (70/108); graph-definition-validation also passes, but only because it
// declares the schema instance namespace, so the floor leaves it out (69/108).
const MIN_XML_PASS_RATE: f64 = 0.63;

fn fixtures_dir() -> PathBuf {
    std::env::var_os("FHIR_FIXTURES_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"),
        PathBuf::from,
    )
}

/// Number of `error` + `fatal` issues in an OperationOutcome.
fn outcome_error_count(outcome: &Value) -> Option<u64> {
    let issues = outcome["issue"].as_array().map_or(&[][..], Vec::as_slice);
    let count = issues
        .iter()
        .filter(|issue| matches!(issue["severity"].as_str(), Some("error" | "fatal")))
        .count();
    (outcome["resourceType"] == "OperationOutcome").then_some(count as u64)
}

/// Expected error count of a case, or `None` when the manifest gives none or
/// names an outcome file the fixtures do not ship (counted as skipped).
/// Panics when a named outcome file exists but is not a valid OperationOutcome,
/// so a corrupt fixture cannot silently drop out of the pass rate.
fn expected_errors(java: &Value, dir: &Path) -> Option<u64> {
    if let Some(file) = java.as_str() {
        let path = dir.join("outcomes").join(file);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Err(e) => panic!("outcome file {}: {e}", path.display()),
        };
        let outcome = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("outcome file {}: {e}", path.display()));
        return Some(
            outcome_error_count(&outcome)
                .unwrap_or_else(|| panic!("{} is not an OperationOutcome", path.display())),
        );
    }
    if let Some(count) = java["errorCount"].as_u64() {
        return Some(count + java["fatalCount"].as_u64().unwrap_or(0));
    }
    outcome_error_count(&java["outcome"])
}

#[derive(Default)]
struct Tally {
    cases: u32,
    passed: u32,
    false_errors: u32,
    missed_errors: u32,
    parse_failures: u32,
    /// Cases with no expected result (no outcome file).
    skipped: u32,
    failing: Vec<String>,
}

impl Tally {
    fn rate(&self) -> f64 {
        if self.cases == 0 {
            1.0
        } else {
            f64::from(self.passed) / f64::from(self.cases)
        }
    }

    fn report(&self, format: &str) {
        eprintln!(
            "manifest {format}: {}/{} passed ({:.1}%), false errors {}, missed errors {}, \
             parse failures {}, skipped (no expected result) {}\nfailing {format}: {:?}",
            self.passed,
            self.cases,
            self.rate() * 100.0,
            self.false_errors,
            self.missed_errors,
            self.parse_failures,
            self.skipped,
            self.failing,
        );
    }
}

#[test]
fn outcome_error_count_counts_error_and_fatal_only() {
    let outcome = serde_json::json!({
        "resourceType": "OperationOutcome",
        "issue": [
            {"severity": "fatal"},
            {"severity": "error"},
            {"severity": "warning"},
            {"severity": "information"},
        ],
    });
    assert_eq!(outcome_error_count(&outcome), Some(2));
    assert_eq!(
        outcome_error_count(&serde_json::json!({"resourceType": "OperationOutcome"})),
        Some(0)
    );
    assert_eq!(outcome_error_count(&Value::Null), None);
}

#[test]
fn absent_outcome_file_is_skipped_but_corrupt_one_fails() {
    let dir = std::env::temp_dir().join(format!("manifest-outcomes-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("outcomes")).unwrap();
    std::fs::write(dir.join("outcomes/bad.json"), "{not json").unwrap();
    assert_eq!(
        expected_errors(&serde_json::json!("absent.json"), &dir),
        None
    );
    let corrupt = std::panic::catch_unwind(|| {
        expected_errors(&serde_json::json!("bad.json"), &dir);
    });
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(corrupt.is_err());
}

#[test]
fn validator_manifest_pass_rate() {
    let dir = fixtures_dir().join("validator");
    let Ok(manifest) = std::fs::read_to_string(dir.join("manifest.json")) else {
        assert!(
            std::env::var_os("CI").is_none(),
            "fixtures submodule missing in CI: run `git submodule update --init`"
        );
        eprintln!("fixtures submodule not checked out; skipping");
        return;
    };
    let manifest: Value = serde_json::from_str(&manifest).unwrap();
    let registry = Registry::from_definitions_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5"),
    )
    .unwrap();

    let (mut json, mut xml) = (Tally::default(), Tally::default());
    for case in manifest["test-cases"].as_array().unwrap() {
        let is_r5 = matches!(case["version"].as_str(), None | Some("5.0.0" | "5.0"));
        let standalone = ["supporting", "profile", "profiles", "packages", "logical"]
            .iter()
            .all(|k| case.get(k).is_none());
        let Some(file) = case["file"].as_str() else {
            continue;
        };
        if !is_r5 || !standalone {
            continue;
        }
        let (tally, parse): (_, fn(&str) -> Result<_, _>) = if file.ends_with(".json") {
            (&mut json, parse_json)
        } else if file.ends_with(".xml") {
            (&mut xml, parse_xml)
        } else {
            continue;
        };
        let Some(expected) = expected_errors(&case["java"], &dir) else {
            tally.skipped += 1;
            continue;
        };
        let bytes = std::fs::read(dir.join(file)).expect("manifest case file");
        // A file that is not UTF-8 (e.g. a UTF-16 XML case) counts as a parse failure.
        let parsed = String::from_utf8(bytes)
            .ok()
            .and_then(|text| parse(&text).ok());
        let has_errors = match parsed {
            Some(resource) => validate(&resource, &registry)
                .iter()
                .any(|d| d.severity == Severity::Error),
            None => {
                tally.parse_failures += 1;
                true
            }
        };
        tally.cases += 1;
        match (has_errors, expected > 0) {
            (true, true) | (false, false) => {
                tally.passed += 1;
                continue;
            }
            (true, false) => tally.false_errors += 1,
            (false, true) => tally.missed_errors += 1,
        }
        tally.failing.push(file.to_owned());
    }

    json.report("json");
    xml.report("xml");
    assert!(
        json.cases > 0 && xml.cases > 0,
        "no eligible manifest cases (json {}, xml {}): floors would pass vacuously",
        json.cases,
        xml.cases,
    );
    assert!(
        json.rate() >= MIN_JSON_PASS_RATE,
        "JSON pass rate {:.3} below floor {MIN_JSON_PASS_RATE}",
        json.rate(),
    );
    assert!(
        xml.rate() >= MIN_XML_PASS_RATE,
        "XML pass rate {:.3} below floor {MIN_XML_PASS_RATE}",
        xml.rate(),
    );
}
