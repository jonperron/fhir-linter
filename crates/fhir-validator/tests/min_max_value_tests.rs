//! `minValue[x]` and `maxValue[x]` of profile elements (`PROFILE_011`): inclusive bounds on
//! the instance value, reported only where the comparison is decidable.
use std::path::PathBuf;
use std::sync::Arc;

use fhir_definitions::Registry;
use fhir_parser::{parse_json, parse_xml};
use fhir_validator::validate;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PROFILE: &str = "http://example.org/StructureDefinition/bounds";

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
}

/// The core definition of `name` as a constraint at `PROFILE`, with `bounds` merged into
/// the snapshot element `id`.
fn profile(name: &str, id: &str, bounds: Value) -> TestResult<Value> {
    let text = std::fs::read_to_string(definitions_dir().join("profiles-resources.json"))?;
    let bundle: Value = serde_json::from_str(&text)?;
    let core = format!("http://hl7.org/fhir/StructureDefinition/{name}");
    let mut sd = bundle["entry"]
        .as_array()
        .ok_or("core bundle has no entries")?
        .iter()
        .map(|entry| &entry["resource"])
        .find(|r| r["url"] == core.as_str())
        .ok_or("core definition not found")?
        .clone();
    sd["url"] = json!(PROFILE);
    sd["derivation"] = json!("constraint");
    sd["baseDefinition"] = json!(core);
    let element = sd["snapshot"]["element"]
        .as_array_mut()
        .ok_or("definition has no snapshot")?
        .iter_mut()
        .find(|e| e["id"] == id)
        .ok_or("snapshot element not found")?;
    for (key, value) in bounds.as_object().ok_or("bounds must be an object")? {
        element[key] = value.clone();
    }
    Ok(sd)
}

/// The registry with `profile` supplied as one file in a fresh directory.
fn overlay(name: &str, profile: Value) -> TestResult<Registry> {
    let dir = std::env::temp_dir().join(format!("min-max-value-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("profile.json");
    std::fs::write(&file, profile.to_string())?;
    let base = Arc::new(Registry::from_definitions_dir(definitions_dir())?);
    let registry = Registry::with_extra(base, &[file.as_path()])?;
    std::fs::remove_dir_all(&dir)?;
    Ok(registry)
}

/// Paths of every `PROFILE_011` of `instance`.
fn out_of_range(instance: &Value, registry: &Registry) -> TestResult<Vec<String>> {
    let resource = parse_json(&instance.to_string())?;
    Ok(validate(&resource, registry)
        .into_iter()
        .filter(|d| &*d.code == "PROFILE_011")
        .map(|d| d.path)
        .collect())
}

fn observation(value: (&str, Value)) -> Value {
    let mut observation = json!({
        "resourceType": "Observation",
        "meta": {"profile": [PROFILE]},
        "status": "final",
        "code": {"text": "count"},
    });
    observation[value.0] = value.1;
    observation
}

fn patient_born(date: &str) -> Value {
    json!({"resourceType": "Patient", "meta": {"profile": [PROFILE]}, "birthDate": date})
}

#[test]
fn integer_below_min_value_is_an_error() -> TestResult {
    let sd = profile(
        "Observation",
        "Observation.value[x]",
        json!({"minValueInteger": 1}),
    )?;
    let registry = overlay("integer-min", sd)?;
    let below = observation(("valueInteger", json!(0)));
    assert_eq!(
        out_of_range(&below, &registry)?,
        ["Observation.valueInteger"]
    );
    let equal = observation(("valueInteger", json!(1)));
    assert_eq!(out_of_range(&equal, &registry)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn decimal_above_max_value_is_an_error_at_depth() -> TestResult {
    let id = "RiskAssessment.prediction.relativeRisk";
    let sd = profile("RiskAssessment", id, json!({"maxValueDecimal": 2.5}))?;
    let registry = overlay("decimal-max", sd)?;
    let risk = |value: Value| {
        json!({
            "resourceType": "RiskAssessment",
            "meta": {"profile": [PROFILE]},
            "status": "final",
            "subject": {"reference": "Patient/p"},
            "prediction": [{"relativeRisk": value}],
        })
    };
    let above = risk(json!(2.51));
    assert_eq!(
        out_of_range(&above, &registry)?,
        ["RiskAssessment.prediction[0].relativeRisk"]
    );
    // Inclusive, and decimals compare by value: 2.50 is the bound 2.5.
    let equal = risk(serde_json::from_str("2.50")?);
    assert_eq!(out_of_range(&equal, &registry)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn date_outside_the_range_is_an_error() -> TestResult {
    let bounds = json!({"minValueDate": "2000-06-15", "maxValueDate": "2010-01-01"});
    let registry = overlay(
        "date-range",
        profile("Patient", "Patient.birthDate", bounds)?,
    )?;
    for date in ["2000-06-14", "2000-05", "1999", "2010-01-02", "2011"] {
        assert_eq!(
            out_of_range(&patient_born(date), &registry)?,
            ["Patient.birthDate"],
            "{date}",
        );
    }
    for date in ["2000-06-15", "2010-01-01", "2005-03"] {
        assert_eq!(
            out_of_range(&patient_born(date), &registry)?,
            Vec::<String>::new(),
            "{date}",
        );
    }
    Ok(())
}

#[test]
fn date_whose_precision_overlaps_the_bound_is_undecided() -> TestResult {
    let bounds = json!({"minValueDate": "2000-06-15", "maxValueDate": "2010-01-01"});
    let registry = overlay(
        "date-precision",
        profile("Patient", "Patient.birthDate", bounds)?,
    )?;
    for date in ["2000", "2000-06", "2010", "2010-01"] {
        assert_eq!(
            out_of_range(&patient_born(date), &registry)?,
            Vec::<String>::new(),
            "{date}",
        );
    }
    Ok(())
}

#[test]
fn date_time_and_time_bounds_are_enforced() -> TestResult {
    let sd = profile(
        "Observation",
        "Observation.effective[x]",
        json!({"maxValueDateTime": "2020-01-01T00:00:00Z"}),
    )?;
    let registry = overlay("date-time-max", sd)?;
    // 01:00+01:00 is the bound itself once both are in UTC.
    let equal = observation(("effectiveDateTime", json!("2020-01-01T01:00:00+01:00")));
    assert_eq!(out_of_range(&equal, &registry)?, Vec::<String>::new());
    let later = observation(("effectiveDateTime", json!("2020-01-01T02:00:00Z")));
    assert_eq!(
        out_of_range(&later, &registry)?,
        ["Observation.effectiveDateTime"]
    );

    let sd = profile(
        "Observation",
        "Observation.value[x]",
        json!({"minValueTime": "08:00:00"}),
    )?;
    let registry = overlay("time-min", sd)?;
    let early = observation(("valueTime", json!("07:59:59")));
    assert_eq!(out_of_range(&early, &registry)?, ["Observation.valueTime"]);
    let on_time = observation(("valueTime", json!("08:00:00")));
    assert_eq!(out_of_range(&on_time, &registry)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn quantity_bound_applies_only_with_the_same_unit() -> TestResult {
    let bound = json!({"value": 10, "system": "http://unitsofmeasure.org", "code": "kg"});
    let sd = profile(
        "Observation",
        "Observation.value[x]",
        json!({"minValueQuantity": bound}),
    )?;
    let registry = overlay("quantity-min", sd)?;
    let ucum = "http://unitsofmeasure.org";
    let quantity = |value: i64, code: &str| json!({"value": value, "system": ucum, "code": code});
    let light = observation(("valueQuantity", quantity(9, "kg")));
    assert_eq!(
        out_of_range(&light, &registry)?,
        ["Observation.valueQuantity"]
    );
    let equal = observation(("valueQuantity", quantity(10, "kg")));
    assert_eq!(out_of_range(&equal, &registry)?, Vec::<String>::new());
    // 9000 g is 9 kg, but deciding it needs UCUM conversion: no finding.
    let grams = observation(("valueQuantity", quantity(9000, "g")));
    assert_eq!(out_of_range(&grams, &registry)?, Vec::<String>::new());
    // "< 9 kg" is a range, not a value.
    let mut below = quantity(9, "kg");
    below["comparator"] = json!("<");
    let below = observation(("valueQuantity", below));
    assert_eq!(out_of_range(&below, &registry)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn quantity_without_a_code_is_undecided() -> TestResult {
    let bound = json!({"value": 10, "system": "http://unitsofmeasure.org", "code": "kg"});
    let sd = profile(
        "Observation",
        "Observation.value[x]",
        json!({"minValueQuantity": bound}),
    )?;
    let registry = overlay("quantity-no-code", sd)?;
    let no_code = observation(("valueQuantity", json!({"value": 9, "unit": "kg"})));
    assert_eq!(out_of_range(&no_code, &registry)?, Vec::<String>::new());
    let other_system = json!({"value": 9, "system": "http://example.org/units", "code": "kg"});
    let other_system = observation(("valueQuantity", other_system));
    assert_eq!(
        out_of_range(&other_system, &registry)?,
        Vec::<String>::new()
    );
    Ok(())
}

#[test]
fn integer64_above_max_value_is_an_error() -> TestResult {
    let id = "SubscriptionStatus.eventsSinceSubscriptionStart";
    let sd = profile(
        "SubscriptionStatus",
        id,
        json!({"maxValueInteger64": "100"}),
    )?;
    let registry = overlay("integer64-max", sd)?;
    let status = |events: &str| {
        json!({
            "resourceType": "SubscriptionStatus",
            "meta": {"profile": [PROFILE]},
            "type": "heartbeat",
            "subscription": {"reference": "Subscription/s"},
            "eventsSinceSubscriptionStart": events,
        })
    };
    assert_eq!(out_of_range(&status("101"), &registry)?, [id]);
    assert_eq!(
        out_of_range(&status("100"), &registry)?,
        Vec::<String>::new()
    );
    Ok(())
}

#[test]
fn instant_below_min_value_is_an_error() -> TestResult {
    let bound = json!({"minValueInstant": "2020-01-01T00:00:00Z"});
    let registry = overlay(
        "instant-min",
        profile("Observation", "Observation.issued", bound)?,
    )?;
    let early = observation(("issued", json!("2019-12-31T23:59:59.999Z")));
    assert_eq!(out_of_range(&early, &registry)?, ["Observation.issued"]);
    let equal = observation(("issued", json!("2020-01-01T00:00:00.000Z")));
    assert_eq!(out_of_range(&equal, &registry)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn xml_integer_below_min_value_is_an_error() -> TestResult {
    let sd = profile(
        "Observation",
        "Observation.value[x]",
        json!({"minValueInteger": 1}),
    )?;
    let registry = overlay("xml-integer-min", sd)?;
    let xml = format!(
        r#"<Observation xmlns="http://hl7.org/fhir"><meta><profile value="{PROFILE}"/></meta>
        <status value="final"/><code><text value="count"/></code>
        <valueInteger value="0"/></Observation>"#
    );
    let found: Vec<String> = validate(&parse_xml(&xml)?, &registry)
        .into_iter()
        .filter(|d| &*d.code == "PROFILE_011")
        .map(|d| d.path)
        .collect();
    assert_eq!(found, ["Observation.valueInteger"]);
    Ok(())
}
