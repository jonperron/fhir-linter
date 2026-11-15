//! per-1 (Period start <= end), rng-2 (Range low <= high) and ratrng-2 call
//! the FHIRPath `lowBoundary()`, `highBoundary()` and `comparable()`
//! functions; they run against the real R5 `Registry` instead of being
//! skipped.
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Diagnostic, validate};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn registry() -> TestResult<&'static Registry> {
    static REGISTRY: OnceLock<Result<Registry, String>> = OnceLock::new();
    let loaded = REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).map_err(|e| e.to_string())
    });
    loaded.as_ref().map_err(|e| e.clone().into())
}

/// Diagnostics that name the constraint `key`: its violation or its skip.
fn about(json: &str, key: &str) -> TestResult<Vec<Diagnostic>> {
    Ok(validate(&parse_json(json)?, registry()?)
        .into_iter()
        .filter(|d| d.code.ends_with(key) || d.message.contains(key))
        .collect())
}

fn encounter(start: &str, end: &str) -> String {
    format!(
        r#"{{"resourceType":"Encounter","status":"planned",
        "actualPeriod":{{"start":"{start}","end":"{end}"}}}}"#
    )
}

fn observation_range(low: &str, high: &str) -> String {
    format!(
        r#"{{"resourceType":"Observation","status":"final","code":{{"text":"x"}},
        "valueRange":{{"low":{low},"high":{high}}}}}"#
    )
}

fn quantity(value: &str, code: &str) -> String {
    format!(r#"{{"value":{value},"system":"http://unitsofmeasure.org","code":"{code}"}}"#)
}

#[test]
fn reversed_period_violates_per_1() -> TestResult {
    let found = about(&encounter("2024-05-01", "2020-01-01"), "per-1")?;
    let codes: Vec<(&str, &str)> = found
        .iter()
        .map(|d| (d.code.as_ref(), d.path.as_str()))
        .collect();
    assert_eq!(codes, [("INVARIANT_per-1", "Encounter.actualPeriod")]);
    Ok(())
}

#[test]
fn equal_and_ordered_periods_satisfy_per_1() -> TestResult {
    for (start, end) in [
        ("2024-05-01", "2024-05-01"),
        ("2020-01-01", "2024-05-01"),
        ("2024-05-01T10:00:00Z", "2024-05-01T10:00:00Z"),
        ("2024-05-01T10:00:00+02:00", "2024-05-01T09:00:00Z"),
    ] {
        let found = about(&encounter(start, end), "per-1")?;
        assert!(found.is_empty(), "{start}..{end}: {found:?}");
    }
    Ok(())
}

#[test]
fn mixed_precision_period_compares_its_boundaries() -> TestResult {
    // 2024 starts at 2024-01-01T00:00+14:00, before the end.
    let ordered = about(&encounter("2024", "2024-01-01T10:00:00Z"), "per-1")?;
    assert!(ordered.is_empty(), "{ordered:?}");
    // 2023 ends at 2023-12-31T23:59:59.999-12:00, still before the start.
    let reversed = about(&encounter("2024-01-02T10:00:00Z", "2023"), "per-1")?;
    assert_eq!(reversed.len(), 1, "{reversed:?}");
    assert_eq!(reversed[0].code.as_ref(), "INVARIANT_per-1");
    Ok(())
}

#[test]
fn range_order_is_checked_by_rng_2() -> TestResult {
    let reversed = observation_range(&quantity("5", "mg"), &quantity("3", "mg"));
    let found = about(&reversed, "rng-2")?;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code.as_ref(), "INVARIANT_rng-2");
    assert_eq!(found[0].path, "Observation.valueRange");

    let ordered = observation_range(&quantity("3", "mg"), &quantity("5", "mg"));
    let found = about(&ordered, "rng-2")?;
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}

#[test]
fn range_with_incomparable_units_satisfies_rng_2() -> TestResult {
    let mixed = observation_range(&quantity("5", "kg"), &quantity("3", "cm"));
    let found = about(&mixed, "rng-2")?;
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}

fn ingredient_ratio_range(low: &str, high: &str) -> String {
    format!(
        r#"{{"resourceType":"Ingredient","status":"active","role":{{"text":"x"}},
        "substance":{{"code":{{"concept":{{"text":"x"}}}},"strength":[{{
        "presentationRatioRange":{{"lowNumerator":{low},"highNumerator":{high}}}}}]}}}}"#
    )
}

#[test]
fn ratio_range_order_is_checked_by_ratrng_2() -> TestResult {
    let reversed = ingredient_ratio_range(&quantity("5", "mg"), &quantity("3", "mg"));
    let found = about(&reversed, "ratrng-2")?;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code.as_ref(), "INVARIANT_ratrng-2");
    assert_eq!(
        found[0].path,
        "Ingredient.substance.strength[0].presentationRatioRange"
    );

    let ordered = ingredient_ratio_range(&quantity("3", "mg"), &quantity("5", "mg"));
    let found = about(&ordered, "ratrng-2")?;
    assert!(found.is_empty(), "{found:?}");
    Ok(())
}
