//! dom-3 on a resource with many references: the constraint unions every
//! reference of the resource for each contained resource, so a quadratic
//! FHIRPath union made a 1.3 MB List with 32k references take over a minute.
use std::path::PathBuf;
use std::time::{Duration, Instant};

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::validate;

const REFERENCES: usize = 32_000;

/// A List with five contained Patients, each referenced once, then
/// `REFERENCES` references in total.
fn list_with_many_references() -> String {
    let contained: Vec<String> = (0..5)
        .map(|i| format!(r#"{{"resourceType":"Patient","id":"p{i}"}}"#))
        .collect();
    let entries: Vec<String> = (0..REFERENCES)
        .map(|i| match i {
            0..5 => format!(r##"{{"item":{{"reference":"#p{i}"}}}}"##),
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

// In a debug build the old union took 327 s here and the hash path takes
// about 12 s; the bound leaves room for a slow runner.
#[test]
fn dom3_over_many_references_is_linear() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    let registry = Registry::from_definitions_dir(dir)?;
    let resource = parse_json(&list_with_many_references())?;
    let started = Instant::now();
    let dom3: Vec<String> = validate(&resource, &registry)
        .into_iter()
        .filter(|d| {
            d.code.as_ref() == "INVARIANT_dom-3"
                || (d.code.as_ref() == "INVARIANT_SKIPPED"
                    && d.message.starts_with("constraint dom-3 "))
        })
        .map(|d| format!("{} {}", d.code, d.path))
        .collect();
    let elapsed = started.elapsed();
    assert!(dom3.is_empty(), "{dom3:?}");
    assert!(elapsed < Duration::from_secs(120), "took {elapsed:?}");
    Ok(())
}
