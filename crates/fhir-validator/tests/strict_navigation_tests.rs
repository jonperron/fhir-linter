//! FHIRPath strict navigation in the validator (`validate_strict`, test-only):
//! the R5 core invariants must give the same result as without it.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate_strict};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn registry() -> Result<Registry, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    Ok(Registry::from_definitions_dir(dir)?)
}

#[test]
fn unreferenced_contained_resource_violates_dom3_under_strict() -> TestResult {
    // dom-3 asks `reference` of every descendant inside `where()`: a descendant
    // without that element (the Basic's `code`) must not make it unresolvable.
    let json = r#"{"resourceType":"Patient",
        "contained":[{"resourceType":"Basic","id":"b","code":{"text":"x"}}]}"#;
    let findings: Vec<_> = validate_strict(&parse_json(json)?, &registry()?)
        .into_iter()
        .filter(|d| d.message.contains("dom-3") || d.code.as_ref() == "INVARIANT_dom-3")
        .map(|d| (d.severity, d.code.to_string(), d.message))
        .collect();
    assert!(
        matches!(&findings[..], [(Severity::Error, code, _)] if code == "INVARIANT_dom-3"),
        "{findings:?}",
    );
    Ok(())
}

/// Every `*.json` file below `dir`, in a stable order.
fn json_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            json_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            files.push(path);
        }
    }
    Ok(())
}

// Strict navigation is safe to turn on in the validator only if it changes no
// diagnostic on the corpus. Slow in a debug build: run it with `--ignored`.
#[test]
#[ignore = "validates the whole fixture corpus twice"]
fn corpus_diagnostics_are_identical_with_strict_navigation() -> TestResult {
    let registry = registry()?;
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut files = Vec::new();
    json_files(&corpus, &mut files)?;
    let render = |diagnostics: Vec<fhir_validator::Diagnostic>| -> Vec<String> {
        diagnostics
            .into_iter()
            .map(|d| format!("{:?} {} {} {}", d.severity, d.code, d.path, d.message))
            .collect()
    };
    let mut differences = Vec::new();
    for file in &files {
        let Ok(resource) = parse_json(&std::fs::read_to_string(file)?) else {
            continue;
        };
        let lenient = render(fhir_validator::validate(&resource, &registry));
        let strict = render(validate_strict(&resource, &registry));
        if lenient != strict {
            let name = file.strip_prefix(&corpus)?.display();
            // Same diagnostics in another order or count still differ.
            let counts = format!("{} vs {} diagnostics", lenient.len(), strict.len());
            differences.push(format!("{name}: {counts}"));
            differences.extend(
                lenient
                    .iter()
                    .filter(|d| !strict.contains(d))
                    .map(|d| format!("{name}: - {d}")),
            );
            differences.extend(
                strict
                    .iter()
                    .filter(|d| !lenient.contains(d))
                    .map(|d| format!("{name}: + {d}")),
            );
        }
    }
    assert!(
        files.len() > 1000,
        "corpus not found: {} files",
        files.len()
    );
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    Ok(())
}

/// A List with five contained Patients, the first four referenced once (the last
/// violates dom-3), then `references` references in total.
fn list_with_many_references(references: usize) -> String {
    let contained: Vec<String> = (0..5)
        .map(|i| format!(r#"{{"resourceType":"Patient","id":"p{i}"}}"#))
        .collect();
    let entries: Vec<String> = (0..references)
        .map(|i| match i {
            0..4 => format!(r##"{{"item":{{"reference":"#p{i}"}}}}"##),
            _ => format!(r#"{{"item":{{"reference":"Patient/x{i}"}}}}"#),
        })
        .collect();
    format!(
        r#"{{"resourceType":"List","status":"current","mode":"working",
            "contained":[{}],"entry":[{}]}}"#,
        contained.join(","),
        entries.join(","),
    )
}

// Strict overhead must stay linear: the instance-path index used to be rebuilt for
// every evaluation (each element-level invariant), which ran past 13 minutes
// here; it now takes about as long as without strict (13 s in a debug build).
#[test]
fn strict_dom3_over_many_references_is_linear() -> TestResult {
    let registry = registry()?;
    let references = 32_000;
    let resource = parse_json(&list_with_many_references(references))?;
    let started = Instant::now();
    let skipped: Vec<String> = validate_strict(&resource, &registry)
        .into_iter()
        .filter(|d| d.code.as_ref() == "INVARIANT_dom-3" || d.code.as_ref() == "INVARIANT_SKIPPED")
        .map(|d| format!("{} {}", d.code, d.path))
        .collect();
    let elapsed = started.elapsed();
    eprintln!("strict validate of {references} references took {elapsed:?}");
    // dom-3 ran: it reports the unreferenced Patient, and nothing is skipped.
    assert_eq!(skipped, ["INVARIANT_dom-3 List"], "{skipped:?}");
    assert!(elapsed < Duration::from_secs(120), "took {elapsed:?}");
    Ok(())
}
