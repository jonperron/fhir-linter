//! Runs the official `fhir-test-cases/validator` manifest (R5, self-contained cases).
//!
//! Cases with supporting profiles/packages are skipped. Cases the manifest
//! marks as out of scope (`supporting5`, `for-publication`) are skipped and
//! reported apart, outside the pass-rate denominator. The expected result
//! comes from the inline `java` entry (`errorCount` or `outcome`) or from the
//! OperationOutcome file it names under `outcomes/`; expected errors are its
//! `error` + `fatal` issues. Each case is judged on "has errors" vs "expected
//! errors". JSON and XML cases are counted apart and each pass rate must not
//! drop below its floor. A case that fails to parse counts as "has errors",
//! so parser regressions lower the rate.
//!
//! The error count (one for a parse failure) is also compared with the expected
//! one: the test prints a mismatch table (largest gap first) and a matched / over /
//! under summary per format, and the matched count has its own floor. The count
//! does not change pass/fail. A passing test hides its output: run
//! `cargo test -p fhir-validator --test manifest_tests -- --nocapture` to see the reports.
use std::path::{Path, PathBuf};

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::{Severity, validate};
use serde_json::Value;

/// Regression floors (rates measured on the pinned fixtures, rounded down);
/// raise them as validator coverage improves.
// 0.66 -> 0.71 (main 75/111 -> 79/111): the R5 regexes catch base64-ws/r5-bad-char,
// r5-bad-length, r5-bad-padding, and an empty fullUrl in bundle-bad-empty.json.
// 0.71 -> 0.70 (79/111 -> 78/111): fhirpath-good.json is a real R5 violation that Java
// accepts: CareTeam.participant.onBehalfOf is Reference(Organization) but holds
// "Practitioner/2", so REFERENCE_005 is a correct error.
// 0.70 -> 0.79 (main 78/111 -> 88/111): validator fixes of PRs #123-#145.
// 0.79 -> 0.81 (main 88/111 -> 91/111): validator fixes of PRs #148-#152.
// 0.81 -> 0.87 (main 91/111 -> 94/108): PRs #158-#165; the denominator drops by the
// three cases the manifest marks out of scope.
const MIN_JSON_PASS_RATE: f64 = 0.87;
// 0.58 -> 0.56: primitive-good.xml and primitive-bad-empty.xml only passed through a false
// TYPE_004 on the valid code "asd asd"; their real errors (decimal regex) are not caught yet.
// 0.56 -> 0.68 (main 64/108 -> 74/108): the XML conformance checks (namespaces, mixed text,
// encoding, malformed XHTML attributes, schema instance namespace) catch 10 more cases.
// 0.68 -> 0.71 (main 78/108 -> 77/108): primitive-bad-empty.xml now passes (its empty
// string is TYPE_003); narrative-binary.xml and narrative-binary-bad.xml now fail because their
// base64Binary placeholder `MEKH....SD/Z` is invalid under the R5 regex (user decision).
// 0.71 -> 0.83 (main 77/108 -> 90/108): validator fixes of PRs #123-#145.
// 0.83 -> 0.85 (main 90/108 -> 92/108): validator fixes of PRs #148-#152.
// 0.85 -> 0.88 (main 92/108 -> 95/107): PRs #158-#165; the denominator drops by the
// case the manifest marks out of scope.
const MIN_XML_PASS_RATE: f64 = 0.88;

/// (judged, skipped without expected result, out of scope) case counts per format on
/// the pinned fixtures, so the rates cannot rise by losing cases.
const EXPECTED_JSON_COUNTS: (u32, u32, u32) = (108, 1, 3);
const EXPECTED_XML_COUNTS: (u32, u32, u32) = (107, 1, 1);

/// (JSON, XML) judged cases whose error count equals the expected one, measured on
/// the pinned fixtures with no slack; raise them as counts converge.
// (78, 78) -> (78, 79): observation-cholesterol-bad-wrongcode.xml, once PROFILE_004 names
// each part that breaks a fixed value.
const MIN_ERROR_COUNT_MATCHES: (u32, u32) = (78, 79);

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

/// Whether the manifest marks a case as outside this validator's scope:
/// `supporting5` names extra definitions we do not load, and
/// `for-publication` (bool or string `"true"`) turns on HL7 publishing rules.
fn out_of_scope(case: &Value) -> bool {
    let publication = &case["for-publication"];
    case.get("supporting5").is_some() || *publication == true || *publication == "true"
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
    /// Cases the manifest marks as out of scope (see `out_of_scope`).
    out_of_scope: u32,
    failing: Vec<String>,
    /// Judged cases whose error count equals the expected one.
    count_matches: u32,
    count_mismatches: Vec<CountMismatch>,
}

/// A judged case whose error count differs from the expected one.
struct CountMismatch {
    file: String,
    expected: u64,
    actual: u64,
    /// The case did not parse; `actual` is then 1 (the parse error).
    parse_failure: bool,
}

impl CountMismatch {
    fn delta(&self) -> i64 {
        self.actual as i64 - self.expected as i64
    }
}

impl Tally {
    fn record_count(&mut self, file: &str, expected: u64, actual: u64, parse_failure: bool) {
        if expected == actual {
            self.count_matches += 1;
        } else {
            self.count_mismatches.push(CountMismatch {
                file: file.to_owned(),
                expected,
                actual,
                parse_failure,
            });
        }
    }

    /// (matched, over-reported, under-reported) error counts.
    fn count_summary(&self) -> (u32, u32, u32) {
        let over = self
            .count_mismatches
            .iter()
            .filter(|m| m.delta() > 0)
            .count() as u32;
        let under = self.count_mismatches.len() as u32 - over;
        (self.count_matches, over, under)
    }

    /// Mismatches, largest gap first, then by file name.
    fn sorted_mismatches(&self) -> Vec<&CountMismatch> {
        let mut sorted: Vec<_> = self.count_mismatches.iter().collect();
        sorted.sort_by(|a, b| (b.delta().abs(), &a.file).cmp(&(a.delta().abs(), &b.file)));
        sorted
    }

    fn report_counts(&self, format: &str) {
        let (matched, over, under) = self.count_summary();
        eprintln!(
            "error counts {format}: matched {matched}, over {over}, under {under}\n\
             | case | format | expected | actual | delta |\n| --- | --- | --- | --- | --- |"
        );
        for m in self.sorted_mismatches() {
            let parse = if m.parse_failure {
                " (parse failure)"
            } else {
                ""
            };
            eprintln!(
                "| {} | {format} | {} | {}{parse} | {:+} |",
                m.file,
                m.expected,
                m.actual,
                m.delta(),
            );
        }
    }

    /// (judged, skipped without expected result, out of scope) case counts.
    fn counts(&self) -> (u32, u32, u32) {
        (self.cases, self.skipped, self.out_of_scope)
    }

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
             parse failures {}, skipped (no expected result) {}, skipped (out of scope) {}\n\
             failing {format}: {:?}",
            self.passed,
            self.cases,
            self.rate() * 100.0,
            self.false_errors,
            self.missed_errors,
            self.parse_failures,
            self.skipped,
            self.out_of_scope,
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
fn error_count_mismatches_are_tallied_and_sorted_by_gap() {
    let mut tally = Tally::default();
    tally.record_count("same.json", 2, 2, false);
    tally.record_count("small-under.json", 3, 2, false);
    tally.record_count("big-under.json", 12, 1, false);
    tally.record_count("over.json", 0, 4, false);
    tally.record_count("parse.json", 5, 1, true);
    assert_eq!(tally.count_summary(), (1, 1, 3));
    let order: Vec<_> = tally
        .sorted_mismatches()
        .iter()
        .map(|m| (m.file.as_str(), m.delta()))
        .collect();
    assert_eq!(
        order,
        [
            ("big-under.json", -11),
            ("over.json", 4),
            ("parse.json", -4),
            ("small-under.json", -1),
        ]
    );
}

#[test]
fn out_of_scope_follows_manifest_flags() {
    use serde_json::json;
    assert!(!out_of_scope(&json!({"name": "c", "file": "c.json"})));
    assert!(out_of_scope(
        &json!({"file": "c.json", "supporting5": ["sd.xml"]})
    ));
    assert!(out_of_scope(
        &json!({"file": "c.xml", "for-publication": true})
    ));
    assert!(out_of_scope(
        &json!({"file": "c.json", "for-publication": "true"})
    ));
    assert!(!out_of_scope(
        &json!({"file": "c.json", "for-publication": false})
    ));
    assert!(!out_of_scope(
        &json!({"file": "c.json", "for-publication": "false"})
    ));
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
        if out_of_scope(case) {
            tally.out_of_scope += 1;
            continue;
        }
        let Some(expected) = expected_errors(&case["java"], &dir) else {
            tally.skipped += 1;
            continue;
        };
        let bytes = std::fs::read(dir.join(file)).expect("manifest case file");
        // A file that is not UTF-8 (e.g. a UTF-16 XML case) counts as a parse failure.
        let parsed = String::from_utf8(bytes)
            .ok()
            .and_then(|text| parse(&text).ok());
        let actual = match &parsed {
            Some(resource) => validate(resource, &registry)
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count() as u64,
            None => {
                tally.parse_failures += 1;
                1
            }
        };
        tally.cases += 1;
        tally.record_count(file, expected, actual, parsed.is_none());
        match (actual > 0, expected > 0) {
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
    json.report_counts("json");
    xml.report_counts("xml");
    // Exact counts, so lost or newly skipped cases cannot shrink the denominator
    // and pass the floors with less coverage.
    assert_eq!(
        (json.counts(), xml.counts()),
        (EXPECTED_JSON_COUNTS, EXPECTED_XML_COUNTS),
        "manifest case counts drifted (fixture bump or filter change): update the counts and floors"
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
    let matches = (json.count_matches, xml.count_matches);
    assert!(
        matches.0 >= MIN_ERROR_COUNT_MATCHES.0 && matches.1 >= MIN_ERROR_COUNT_MATCHES.1,
        "(JSON, XML) error-count matches {matches:?} below floor {MIN_ERROR_COUNT_MATCHES:?}",
    );
}
