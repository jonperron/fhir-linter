//! Runs the official `fhir-test-cases/validator` manifest (R5, JSON, self-contained cases).
//!
//! Only cases with an inline expected `errorCount` and no supporting
//! profiles/packages are run. Each is judged on "has errors" vs "expected
//! errors"; the pass rate must not drop below `MIN_PASS_RATE`. A case that
//! fails to parse counts as "has errors", so parser regressions lower the rate.
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};

/// Regression floor; raise it as validator coverage improves.
const MIN_PASS_RATE: f64 = 0.6;

fn fixtures_dir() -> PathBuf {
    std::env::var_os("FHIR_FIXTURES_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"),
        PathBuf::from,
    )
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
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let registry = Registry::from_definitions_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5"),
    )
    .unwrap();

    let (mut run, mut passed) = (0u32, 0u32);
    let mut failures = Vec::new();
    for case in manifest["test-cases"].as_array().unwrap() {
        let is_r5 = matches!(case["version"].as_str(), None | Some("5.0.0" | "5.0"));
        let standalone = ["supporting", "profile", "profiles", "packages", "logical"]
            .iter()
            .all(|k| case.get(k).is_none());
        let (Some(file), Some(expected)) = (
            case["file"].as_str().filter(|f| f.ends_with(".json")),
            case["java"]["errorCount"].as_u64(),
        ) else {
            continue;
        };
        if !is_r5 || !standalone {
            continue;
        }
        let has_errors = match parse_json(&std::fs::read_to_string(dir.join(file)).unwrap()) {
            Ok(resource) => validate(&resource, &registry)
                .iter()
                .any(|d| d.severity == Severity::Error),
            Err(_) => true,
        };
        run += 1;
        if has_errors == (expected > 0) {
            passed += 1;
        } else {
            failures.push(file.to_owned());
        }
    }

    let rate = if run == 0 {
        1.0
    } else {
        f64::from(passed) / f64::from(run)
    };
    eprintln!(
        "manifest: {passed}/{run} passed ({:.0}%)\nfailing: {failures:?}",
        rate * 100.0
    );
    assert!(rate >= MIN_PASS_RATE, "pass rate {rate:.2} below floor");
}
