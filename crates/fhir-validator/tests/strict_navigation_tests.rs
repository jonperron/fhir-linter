//! FHIRPath strict navigation in the validator (N1 §3: an identifier that cannot
//! be resolved is an error): the R5 core invariants still resolve every name, and a
//! profile constraint naming an unknown element is reported as skipped.
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};

type TestResult = Result<(), Box<dyn Error>>;

fn registry() -> Result<Registry, Box<dyn Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    Ok(Registry::from_definitions_dir(dir)?)
}

#[test]
fn unreferenced_contained_resource_violates_dom3_under_strict() -> TestResult {
    // dom-3 asks `reference` of every descendant inside `where()`: a descendant
    // without that element (the Basic's `code`) must not make it unresolvable.
    let json = r#"{"resourceType":"Patient",
        "contained":[{"resourceType":"Basic","id":"b","code":{"text":"x"}}]}"#;
    let findings: Vec<_> = validate(&parse_json(json)?, &registry()?)
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

// The R5 core invariants name only declared elements: under strict navigation none
// of them may fail to resolve a name. A fixed sample of the corpus (every 40th file)
// keeps this cheap; #177 checked the whole corpus (3888 files, 0 differences).
#[test]
fn core_invariants_resolve_every_name_on_a_corpus_sample() -> TestResult {
    let registry = registry()?;
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut files = Vec::new();
    json_files(&corpus, &mut files)?;
    assert!(
        files.len() > 1000,
        "corpus not found: {} files",
        files.len()
    );
    let unresolved = |file: &PathBuf| -> Result<Vec<String>, Box<dyn Error>> {
        let Ok(resource) = parse_json(&std::fs::read_to_string(file)?) else {
            return Ok(Vec::new());
        };
        let name = file.strip_prefix(&corpus)?.display().to_string();
        Ok(validate(&resource, &registry)
            .into_iter()
            .filter(|d| d.message.contains("is not an element of"))
            .map(|d| format!("{name}: {} {} {}", d.code, d.path, d.message))
            .collect())
    };
    let found: Vec<Vec<String>> = files
        .iter()
        .step_by(40)
        .map(unresolved)
        .collect::<Result<_, _>>()?;
    let found = found.concat();
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

const TYPO_PROFILE: &str = "http://example.org/StructureDefinition/typo-patient";

/// The bundled R5 registry, except that `profiles-others.json` holds only a Patient
/// profile with typos: constraint `typo-1` reads `name.familly`, and the slicing of
/// `identifier` discriminates by `systen`.
fn typo_registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let definitions = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("strict-typo-profile");
        std::fs::create_dir_all(&dir).expect("create the definitions directory");
        for file in [
            "profiles-resources.json",
            "profiles-types.json",
            "valuesets.json",
        ] {
            std::fs::copy(definitions.join(file), dir.join(file)).expect("copy a bundle");
        }
        let profile = serde_json::json!({
            "resourceType": "StructureDefinition",
            "url": TYPO_PROFILE,
            "name": "TypoPatient",
            "status": "draft",
            "kind": "resource",
            "abstract": false,
            "type": "Patient",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Patient",
            "derivation": "constraint",
            "snapshot": {"element": [
                {
                    "id": "Patient",
                    "path": "Patient",
                    "min": 0,
                    "max": "*",
                    "constraint": [{
                        "key": "typo-1",
                        "severity": "error",
                        "human": "A name has a family name",
                        "expression": "name.familly.exists()",
                    }],
                },
                {
                    "id": "Patient.identifier",
                    "path": "Patient.identifier",
                    "min": 0,
                    "max": "*",
                    "type": [{"code": "Identifier"}],
                    "slicing": {
                        "discriminator": [{"type": "value", "path": "systen"}],
                        "rules": "open",
                    },
                },
                {
                    "id": "Patient.identifier:mrn",
                    "path": "Patient.identifier",
                    "sliceName": "mrn",
                    "min": 0,
                    "max": "1",
                    "type": [{"code": "Identifier"}],
                },
                {
                    "id": "Patient.identifier:mrn.systen",
                    "path": "Patient.identifier.systen",
                    "min": 0,
                    "max": "1",
                    "fixedUri": "http://example.org/mrn",
                },
            ]},
        });
        let bundle = serde_json::json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": [{"resource": profile}],
        });
        std::fs::write(dir.join("profiles-others.json"), bundle.to_string())
            .expect("write the profile bundle");
        Registry::from_definitions_dir(dir).expect("load the definitions")
    })
}

/// Severity, code and message of a diagnostic.
type Finding = (Severity, String, String);

/// The findings of a Patient on [`TYPO_PROFILE`] whose code or message contains `needle`.
fn typo_findings(needle: &str) -> Result<Vec<Finding>, Box<dyn Error>> {
    let json = format!(
        r#"{{"resourceType":"Patient","meta":{{"profile":["{TYPO_PROFILE}"]}},
            "identifier":[{{"system":"http://example.org/mrn","value":"1"}}],
            "name":[{{"family":"Doe"}}]}}"#
    );
    Ok(validate(&parse_json(&json)?, typo_registry())
        .into_iter()
        .filter(|d| d.message.contains(needle) || d.code.contains(needle))
        .map(|d| (d.severity, d.code.to_string(), d.message))
        .collect())
}

#[test]
fn profile_constraint_with_a_typo_is_skipped_naming_it() -> TestResult {
    let findings = typo_findings("typo-1")?;
    let [(Severity::Information, code, message)] = &findings[..] else {
        panic!("{findings:?}");
    };
    assert_eq!(code, "INVARIANT_SKIPPED");
    assert!(
        message.contains("familly is not an element of"),
        "{message}"
    );
    assert!(message.contains("name.familly.exists()"), "{message}");
    Ok(())
}

#[test]
fn discriminator_with_a_typo_is_skipped_naming_it() -> TestResult {
    let findings = typo_findings("Patient.identifier")?;
    let [(Severity::Information, code, message)] = &findings[..] else {
        panic!("{findings:?}");
    };
    assert_eq!(code, "SLICING_SKIPPED");
    assert!(message.contains("discriminator 'systen'"), "{message}");
    assert!(
        message.contains("systen is not an element of Identifier"),
        "{message}"
    );
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
    let skipped: Vec<String> = validate(&resource, &registry)
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
