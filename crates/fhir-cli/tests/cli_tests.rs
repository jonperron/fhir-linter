//! Black-box tests for the `fhir-lint` binary.
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> PathBuf {
    manifest_dir().join("tests/fixtures").join(name)
}

/// `fhir-lint` pointed at the repository's R5 definitions.
fn fhir_lint() -> Command {
    let mut cmd = Command::cargo_bin("fhir-lint").unwrap();
    cmd.arg("--definitions")
        .arg(manifest_dir().join("../../definitions/r5"));
    cmd
}

fn stdout_json(cmd: &mut Command) -> serde_json::Value {
    let out = cmd.output().unwrap();
    serde_json::from_slice(&out.stdout).expect("stdout is not valid JSON")
}

#[test]
fn valid_directory_exits_zero() {
    fhir_lint().arg(fixture("valid")).assert().code(0);
}

#[test]
fn directory_recursion_finds_json_and_xml() {
    let diags = stdout_json(fhir_lint().args(["--format", "json"]).arg(fixture("tree")));
    let mut files: Vec<&str> = diags
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["file"].as_str().unwrap())
        .collect();
    files.dedup();
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(files[0].ends_with("bare.json"), "{files:?}");
    assert!(files[1].ends_with("bare.xml"), "{files:?}");
}

#[test]
fn xml_file_is_validated() {
    fhir_lint()
        .arg(fixture("valid/nested/patient.xml"))
        .assert()
        .code(0);
}

#[test]
fn invalid_resource_exits_one_with_diagnostic() {
    fhir_lint()
        .arg(fixture("invalid.json"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("invalid.json:"))
        .stdout(predicate::str::contains("error"))
        .stdout(predicate::str::contains("notAFhirElement"));
}

#[test]
fn malformed_resource_is_a_diagnostic_not_a_tool_error() {
    fhir_lint()
        .arg(fixture("malformed.json"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("malformed.json:"))
        .stdout(predicate::str::contains("PARSE_001"));
}

#[test]
fn warnings_alone_exit_zero() {
    fhir_lint()
        .arg(fixture("warnings.json"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("LINT_003"));
}

#[test]
fn fatal_severity_fails_the_run_and_maps_to_sarif_error() {
    fhir_lint()
        .arg("--config")
        .arg(fixture("fatal-meta-profile.toml"))
        .arg(fixture("warnings.json"))
        .assert()
        .code(1);
    let log = stdout_json(
        fhir_lint()
            .args(["--format", "sarif", "--config"])
            .arg(fixture("fatal-meta-profile.toml"))
            .arg(fixture("warnings.json")),
    );
    let results = log["runs"][0]["results"].as_array().unwrap();
    let lint = results.iter().find(|r| r["ruleId"] == "LINT_003").unwrap();
    assert_eq!(lint["level"], "error");
}

#[test]
fn strict_turns_warnings_into_failure() {
    fhir_lint()
        .arg("--strict")
        .arg(fixture("warnings.json"))
        .assert()
        .code(1);
}

#[test]
fn unknown_fhir_version_is_tool_error() {
    fhir_lint()
        .args(["--fhir-version", "r4"])
        .arg(fixture("valid"))
        .assert()
        .code(2);
}

#[test]
fn r5_fhir_version_is_accepted() {
    fhir_lint()
        .args(["--fhir-version", "r5"])
        .arg(fixture("valid"))
        .assert()
        .code(0);
}

#[test]
fn missing_path_is_tool_error() {
    fhir_lint()
        .arg(fixture("does-not-exist.json"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("does-not-exist.json"));
}

#[test]
fn no_path_is_tool_error() {
    fhir_lint().assert().code(2);
}

#[test]
fn missing_definitions_is_tool_error() {
    Command::cargo_bin("fhir-lint")
        .unwrap()
        .args(["--definitions", "/nonexistent/definitions"])
        .arg(fixture("valid"))
        .assert()
        .code(2);
}

#[test]
fn json_format_is_array_of_diagnostics_with_file() {
    let diags = stdout_json(
        fhir_lint()
            .args(["--format", "json"])
            .arg(fixture("warnings.json")),
    );
    let diags = diags.as_array().expect("top level is an array");
    let lint = diags
        .iter()
        .find(|d| d["code"] == "LINT_003")
        .expect("LINT_003 present");
    assert!(lint["file"].as_str().unwrap().ends_with("warnings.json"));
    assert_eq!(lint["severity"], "warning");
    assert_eq!(lint["rule"], "missing-meta-profile");
}

#[test]
fn sarif_format_is_minimal_valid_log() {
    let log = stdout_json(
        fhir_lint()
            .args(["--format", "sarif"])
            .arg(fixture("invalid.json")),
    );
    assert_eq!(log["version"], "2.1.0");
    let run = &log["runs"][0];
    assert_eq!(run["tool"]["driver"]["name"], "fhir-lint");
    let results = run["results"].as_array().expect("runs[0].results");
    let error = results
        .iter()
        .find(|r| r["level"] == "error")
        .expect("an error result");
    let rule_id = error["ruleId"].as_str().unwrap();
    assert!(
        run["tool"]["driver"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == rule_id)
    );
    assert!(error["message"]["text"].is_string());
    let uri = error["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
        .as_str()
        .unwrap();
    assert!(uri.ends_with("invalid.json"));
}

#[test]
fn sarif_for_valid_resources_has_no_error_or_warning_results() {
    let log = stdout_json(
        fhir_lint()
            .args(["--format", "sarif"])
            .arg(fixture("valid")),
    );
    let results = log["runs"][0]["results"]
        .as_array()
        .expect("runs[0].results");
    // Only `note` results (e.g. skipped invariants) may remain for clean resources.
    assert!(results.iter().all(|r| r["level"] == "note"), "{results:?}");
}

#[test]
fn text_format_prints_one_line_per_finding() {
    let out = fhir_lint()
        .args(["--format", "text"])
        .arg(fixture("warnings.json"))
        .output()
        .unwrap();
    let json = stdout_json(
        fhir_lint()
            .args(["--format", "json"])
            .arg(fixture("warnings.json")),
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout.lines().count(),
        json.as_array().unwrap().len(),
        "{stdout}"
    );
}

#[test]
fn config_disabling_rule_removes_its_diagnostic() {
    fhir_lint()
        .arg("--config")
        .arg(fixture("no-meta-profile.toml"))
        .arg(fixture("warnings.json"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("LINT_003").not())
        .stdout(predicate::str::contains("LINT_002"));
}

#[test]
fn config_with_unknown_rule_is_tool_error() {
    fhir_lint()
        .arg("--config")
        .arg(fixture("unknown-rule.toml"))
        .arg(fixture("warnings.json"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no-such-rule"));
}

#[test]
fn hint_severity_is_reported_as_information_in_every_format() {
    let with_hint = |format: &str| {
        let mut cmd = fhir_lint();
        cmd.args(["--format", format, "--config"])
            .arg(fixture("hint-meta-profile.toml"))
            .arg(fixture("warnings.json"));
        cmd
    };
    with_hint("text")
        .assert()
        .code(0)
        .stdout(predicate::str::contains(": information LINT_003 "));
    let diagnostics = stdout_json(&mut with_hint("json"));
    let lint = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "LINT_003")
        .expect("LINT_003 present");
    assert_eq!(lint["severity"], "information");
    let log = stdout_json(&mut with_hint("sarif"));
    let results = log["runs"][0]["results"].as_array().unwrap();
    let lint = results.iter().find(|r| r["ruleId"] == "LINT_003").unwrap();
    assert_eq!(lint["level"], "note");
}

#[test]
fn strict_does_not_fail_on_information() {
    fhir_lint()
        .arg("--strict")
        .arg("--config")
        .arg(fixture("hint-meta-profile.toml"))
        .arg(fixture("no-profile.json"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains(": information LINT_003 "));
}

#[test]
fn config_with_unknown_severity_is_tool_error() {
    fhir_lint()
        .arg("--config")
        .arg(fixture("unknown-severity.toml"))
        .arg(fixture("warnings.json"))
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("invalid severity 'critical'"));
}

/// A path in the system temp dir, unique to this test process.
fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("fhir-lint-{}-{name}", std::process::id()))
}

/// A sparse file of `len` bytes: cheap to create whatever its size.
fn sparse_file(name: &str, len: u64) -> PathBuf {
    let path = temp_path(name);
    std::fs::File::create(&path).unwrap().set_len(len).unwrap();
    path
}

const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

#[test]
fn input_over_size_limit_is_tool_error() {
    let path = sparse_file("too-big.json", MAX_INPUT_BYTES + 1);
    let result = fhir_lint().arg(&path).assert();
    let _ = std::fs::remove_file(&path);
    result
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("too-big.json"))
        .stderr(predicate::str::contains("64 MiB"));
}

#[test]
fn input_at_size_limit_is_linted() {
    // Valid JSON only if read in full: the closing brace is the very last byte.
    let head = br#"{"resourceType":"Patient""#;
    let padding = MAX_INPUT_BYTES as usize - head.len() - 1;
    let content = [&head[..], &vec![b' '; padding], b"}"].concat();
    let path = temp_path("at-limit.json");
    std::fs::write(&path, &content).unwrap();
    let result = fhir_lint().arg(&path).assert();
    let _ = std::fs::remove_file(&path);
    result
        .code(0)
        .stdout(predicate::str::contains("PARSE_001").not());
}

// Windows file names cannot hold control characters.
#[cfg(unix)]
#[test]
fn control_characters_in_paths_are_escaped_on_stderr() {
    let path = temp_path("evil\x1b[2J\x07.json");
    fhir_lint()
        .arg(&path)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("\x1b").not())
        .stderr(predicate::str::contains("\x07").not())
        .stderr(predicate::str::contains("evil\\u{1b}[2J\\u{7}.json"));
}

#[test]
fn repeated_input_is_linted_once() {
    let once = stdout_json(
        fhir_lint()
            .args(["--format", "json"])
            .arg(fixture("invalid.json")),
    );
    // Same file through a different spelling of the path.
    let twice = stdout_json(
        fhir_lint()
            .args(["--format", "json"])
            .arg(fixture("invalid.json"))
            .arg(fixture("../fixtures/invalid.json")),
    );
    assert_eq!(
        once.as_array().unwrap().len(),
        twice.as_array().unwrap().len()
    );
}

#[test]
fn config_error_wins_over_missing_path() {
    fhir_lint()
        .arg("--config")
        .arg(fixture("unknown-rule.toml"))
        .arg(fixture("does-not-exist.json"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no-such-rule"));
}

#[cfg(unix)]
#[test]
fn fifo_input_is_tool_error_not_a_hang() {
    let fifo = std::env::temp_dir().join(format!("fhir-lint-fifo-{}", std::process::id()));
    let _ = std::fs::remove_file(&fifo);
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert!(made.success());
    let result = fhir_lint()
        .timeout(std::time::Duration::from_secs(20))
        .arg(&fifo)
        .assert();
    let _ = std::fs::remove_file(&fifo);
    result
        .code(2)
        .stderr(predicate::str::contains("not a regular file"));
}

#[test]
fn closed_stdout_does_not_panic() {
    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("fhir-lint"))
        .arg("--definitions")
        .arg(manifest_dir().join("../../definitions/r5"))
        .arg(fixture("invalid.json"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    // Exit 1 (the resource is invalid), not the panic status 101.
    assert_eq!(status.code(), Some(1), "{status:?}");
}
