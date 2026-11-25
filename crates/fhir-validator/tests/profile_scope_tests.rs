//! Which instances a `meta.profile` applies to, and what it allows there: a profile on an
//! ancestor type (DomainResource) applies to every subtype and is walked from its own root,
//! a profile on another type is `PROFILE_002` whatever its `abstract` flag, and a choice type
//! the profile removed does not conform.
use std::path::PathBuf;
use std::sync::Arc;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::validate;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PROFILE: &str = "http://example.org/StructureDefinition/scope";

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
}

/// The core definition of `name` as a constraint at `PROFILE`, changed by `edit`.
fn core_constraint(name: &str, edit: impl FnOnce(&mut Value) -> TestResult) -> TestResult<Value> {
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
    edit(&mut sd)?;
    Ok(sd)
}

/// The snapshot element of `sd` with id `id`.
fn element<'a>(sd: &'a mut Value, id: &str) -> TestResult<&'a mut Value> {
    Ok(sd["snapshot"]["element"]
        .as_array_mut()
        .ok_or("definition has no snapshot")?
        .iter_mut()
        .find(|e| e["id"] == id)
        .ok_or("snapshot element not found")?)
}

/// The registry with `profile` supplied as one file in a fresh directory.
fn overlay(name: &str, profile: Value) -> TestResult<Registry> {
    let dir = std::env::temp_dir().join(format!("profile-scope-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("profile.json");
    std::fs::write(&file, profile.to_string())?;
    let base = Arc::new(Registry::from_definitions_dir(definitions_dir())?);
    let registry = Registry::with_extra(base, &[file.as_path()])?;
    std::fs::remove_dir_all(&dir)?;
    Ok(registry)
}

/// `"<code> <path>"` for every diagnostic of `instance`.
fn diagnostics(instance: Value, registry: &Registry) -> TestResult<Vec<String>> {
    let resource = parse_json(&instance.to_string())?;
    Ok(validate(&resource, registry)
        .iter()
        .map(|d| format!("{} {}", d.code, d.path))
        .collect())
}

fn patient() -> Value {
    json!({"resourceType": "Patient", "meta": {"profile": [PROFILE]}})
}

fn observation(value: (&str, Value)) -> Value {
    let mut observation = json!({
        "resourceType": "Observation",
        "meta": {"profile": [PROFILE]},
        "status": "final",
        "code": {"text": "weight"},
    });
    observation[value.0] = value.1;
    observation
}

/// Observation.value[x] limited to Quantity.
fn quantity_only(sd: &mut Value) -> TestResult {
    element(sd, "Observation.value[x]")?["type"] = json!([{"code": "Quantity"}]);
    Ok(())
}

#[test]
fn choice_type_removed_by_the_profile_is_an_error() -> TestResult {
    let registry = overlay(
        "removed-choice",
        core_constraint("Observation", quantity_only)?,
    )?;
    let found = diagnostics(observation(("valueString", json!("70kg"))), &registry)?;
    assert!(
        found.contains(&"STRUCTURE_001 Observation.valueString".to_owned()),
        "{found:?}",
    );
    Ok(())
}

#[test]
fn choice_type_kept_by_the_profile_is_accepted() -> TestResult {
    let registry = overlay(
        "kept-choice",
        core_constraint("Observation", quantity_only)?,
    )?;
    let quantity = json!({"value": 70, "unit": "kg"});
    let found = diagnostics(observation(("valueQuantity", quantity)), &registry)?;
    assert!(
        !found.iter().any(|d| d.starts_with("STRUCTURE_001")),
        "{found:?}",
    );
    Ok(())
}

#[test]
fn abstract_profile_on_another_type_is_profile_002() -> TestResult {
    let profile = core_constraint("Observation", |sd| {
        sd["abstract"] = json!(true);
        Ok(())
    })?;
    let registry = overlay("abstract-other", profile)?;
    let found = diagnostics(patient(), &registry)?;
    assert!(
        found.contains(&"PROFILE_002 Patient.meta.profile[0]".to_owned()),
        "{found:?}",
    );
    // The Observation root invariants never run on the Patient.
    assert!(
        !found.iter().any(|d| d.starts_with("INVARIANT_obs")),
        "{found:?}",
    );
    Ok(())
}

/// DomainResource with `text` required, its `abstract` flag set to `is_abstract`.
fn domain_resource_requiring_text(is_abstract: bool) -> TestResult<Value> {
    core_constraint("DomainResource", |sd| {
        sd["abstract"] = json!(is_abstract);
        element(sd, "DomainResource.text")?["min"] = json!(1);
        Ok(())
    })
}

#[test]
fn concrete_profile_on_an_ancestor_type_applies() -> TestResult {
    let registry = overlay("ancestor-concrete", domain_resource_requiring_text(false)?)?;
    let found = diagnostics(patient(), &registry)?;
    assert!(
        !found.iter().any(|d| d.starts_with("PROFILE_002")),
        "{found:?}",
    );
    assert!(
        found.contains(&"CARDINALITY_001 Patient.text".to_owned()),
        "{found:?}",
    );
    Ok(())
}

#[test]
fn abstract_profile_on_an_ancestor_type_walks_its_elements() -> TestResult {
    let registry = overlay("ancestor-abstract", domain_resource_requiring_text(true)?)?;
    let found = diagnostics(patient(), &registry)?;
    assert!(
        !found.iter().any(|d| d.starts_with("PROFILE_002")),
        "{found:?}",
    );
    assert!(
        found.contains(&"CARDINALITY_001 Patient.text".to_owned()),
        "{found:?}",
    );
    Ok(())
}

#[test]
fn abstract_profile_on_a_type_that_is_not_an_ancestor_is_profile_002() -> TestResult {
    // Bundle derives from Resource, not DomainResource.
    let registry = overlay("not-ancestor", domain_resource_requiring_text(true)?)?;
    let bundle = json!({
        "resourceType": "Bundle",
        "meta": {"profile": [PROFILE]},
        "type": "collection",
    });
    let found = diagnostics(bundle, &registry)?;
    assert!(
        found.contains(&"PROFILE_002 Bundle.meta.profile[0]".to_owned()),
        "{found:?}",
    );
    Ok(())
}
