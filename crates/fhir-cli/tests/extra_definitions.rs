//! Black-box tests for `fhir-lint --extra-definitions`.
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn extra(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/extra")
        .join(name)
}

/// `fhir-lint` on the repository's R5 definitions, linting the fixture patient
/// that declares both supplied profiles.
fn fhir_lint() -> Command {
    let mut cmd = Command::cargo_bin("fhir-lint").unwrap();
    cmd.arg("--definitions")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5"))
        .args(["--format", "json"])
        .arg(extra("patient.json"));
    cmd
}

/// The `(code, path)` of every diagnostic in a JSON report.
fn findings(cmd: &mut Command) -> (Option<i32>, Vec<(String, String)>) {
    let out = cmd.output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let pairs = report
        .as_array()
        .into_iter()
        .flatten()
        .map(|d| {
            (
                d["code"].as_str().unwrap_or_default().to_owned(),
                d["path"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    (out.status.code(), pairs)
}

fn has(pairs: &[(String, String)], code: &str, path: &str) -> bool {
    pairs.iter().any(|(c, p)| c == code && p == path)
}

#[test]
fn supplied_profile_enforces_its_required_element() {
    // Without the flag the profile is unknown: a warning only, so the run passes.
    let (code, pairs) = findings(&mut fhir_lint());
    assert_eq!(code, Some(0), "{pairs:?}");
    assert!(
        has(&pairs, "PROFILE_001", "Patient.meta.profile[0]"),
        "{pairs:?}"
    );
    assert!(
        !has(&pairs, "CARDINALITY_001", "Patient.birthDate"),
        "{pairs:?}"
    );

    let (code, pairs) = findings(fhir_lint().arg("--extra-definitions").arg(extra("profile")));
    assert_eq!(code, Some(1), "{pairs:?}");
    assert!(
        has(&pairs, "CARDINALITY_001", "Patient.birthDate"),
        "{pairs:?}"
    );
    assert!(
        !has(&pairs, "PROFILE_001", "Patient.meta.profile[0]"),
        "{pairs:?}"
    );
    assert!(
        !has(&pairs, "CARDINALITY_001", "Patient.active"),
        "{pairs:?}"
    );
}

#[test]
fn flag_can_be_given_twice() {
    let (code, pairs) = findings(
        fhir_lint()
            .arg("--extra-definitions")
            .arg(extra("profile"))
            .arg("--extra-definitions")
            .arg(extra("second/patient-with-active.json")),
    );
    assert_eq!(code, Some(1), "{pairs:?}");
    assert!(
        has(&pairs, "CARDINALITY_001", "Patient.birthDate"),
        "{pairs:?}"
    );
    assert!(
        has(&pairs, "CARDINALITY_001", "Patient.active"),
        "{pairs:?}"
    );
}

#[test]
fn snapshotless_supplied_profile_is_profile_010() {
    let (code, pairs) = findings(
        fhir_lint()
            .arg("--extra-definitions")
            .arg(extra("snapshotless")),
    );
    assert_eq!(code, Some(1), "{pairs:?}");
    assert!(
        has(&pairs, "PROFILE_010", "Patient.meta.profile[1]"),
        "{pairs:?}"
    );
    assert!(
        !has(&pairs, "PROFILE_001", "Patient.meta.profile[1]"),
        "{pairs:?}"
    );
}

#[test]
fn conflict_with_base_exits_two_naming_the_file() {
    fhir_lint()
        .arg("--extra-definitions")
        .arg(extra("conflict.json"))
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("conflict.json"))
        .stderr(predicate::str::contains("already defined"))
        .stderr(predicate::str::contains("\"snapshot\"").not());
}

#[test]
fn missing_path_exits_two_naming_it() {
    fhir_lint()
        .arg("--extra-definitions")
        .arg(extra("does-not-exist.json"))
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("does-not-exist.json"));
}

#[test]
fn malformed_definition_exits_two_naming_the_file() {
    // Shares the CLI's malformed input fixture (tests/fixtures/malformed.json).
    fhir_lint()
        .arg("--extra-definitions")
        .arg(extra("../malformed.json"))
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("malformed.json"));
}
