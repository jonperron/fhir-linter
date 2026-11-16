//! Invariants that call `resolve()` (enc-2, obs-9) and `memberOf()` (exs-1, opd-3).
//! A reference the validator cannot resolve gives no false error.
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_parser::parse_json;
use fhir_validator::{Severity, validate};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Finding = (Severity, String, String);

/// `(severity, code, path)` of every diagnostic about the constraint `key`, skips included.
fn about(json: &str, key: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    let registry = Registry::from_definitions_dir(dir)?;
    let resource = parse_json(json)?;
    Ok(validate(&resource, &registry)
        .into_iter()
        .filter(|d| {
            d.code.as_ref() == format!("INVARIANT_{key}")
                || (d.code.as_ref() == "INVARIANT_SKIPPED"
                    && d.message.starts_with(&format!("constraint {key} ")))
        })
        .map(|d| (d.severity, d.code.to_string(), d.path))
        .collect())
}

fn encounter(actor: &str, with_type: bool) -> String {
    let participant_type = if with_type {
        r#","type":[{"text":"attender"}]"#
    } else {
        ""
    };
    format!(
        r##"{{"resourceType":"Encounter","status":"completed",
        "contained":[{{"resourceType":"Patient","id":"pat"}},
            {{"resourceType":"Practitioner","id":"doc"}}],
        "subject":{{"reference":"#pat"}},
        "participant":[{{"actor":{{"reference":"{actor}"}}{participant_type}}}]}}"##
    )
}

#[test]
fn enc2_fails_for_a_typed_contained_patient_participant() -> TestResult {
    let findings = about(&encounter("#pat", true), "enc-2")?;
    assert_eq!(
        findings,
        vec![(
            Severity::Error,
            "INVARIANT_enc-2".to_owned(),
            "Encounter.participant[0]".to_owned(),
        )],
    );
    Ok(())
}

#[test]
fn enc2_passes_for_a_typed_practitioner_or_an_untyped_patient() -> TestResult {
    assert_eq!(about(&encounter("#doc", true), "enc-2")?, vec![]);
    assert_eq!(about(&encounter("#pat", false), "enc-2")?, vec![]);
    Ok(())
}

#[test]
fn enc2_gives_no_error_for_a_reference_it_cannot_resolve() -> TestResult {
    for actor in [
        "Patient/elsewhere",
        "#missing",
        "urn:uuid:5b3f6d36-1d4e-4f5c-9b0e-8f7a1c1e0e7d",
    ] {
        let findings = about(&encounter(actor, true), "enc-2")?;
        assert_eq!(
            findings,
            vec![(
                Severity::Information,
                "INVARIANT_SKIPPED".to_owned(),
                "Encounter.participant[0]".to_owned(),
            )],
            "{actor}",
        );
    }
    Ok(())
}

fn observation(group_member: &str) -> String {
    format!(
        r##"{{"resourceType":"Observation","status":"final","code":{{"text":"x"}},
        "contained":[{{"resourceType":"Group","id":"grp","type":"specimen",
                "membership":"enumerated",
                "member":[{{"entity":{{"reference":"{group_member}"}}}}]}},
            {{"resourceType":"Specimen","id":"spec"}},
            {{"resourceType":"Patient","id":"pat"}}],
        "specimen":{{"reference":"#grp"}}}}"##
    )
}

#[test]
fn obs9_checks_the_members_of_a_contained_group() -> TestResult {
    assert_eq!(about(&observation("#spec"), "obs-9")?, vec![]);
    assert_eq!(
        about(&observation("#pat"), "obs-9")?,
        vec![(
            Severity::Error,
            "INVARIANT_obs-9".to_owned(),
            "Observation.specimen".to_owned(),
        )],
    );
    // A member the validator cannot resolve skips the constraint, never fails it.
    assert_eq!(
        about(&observation("Specimen/elsewhere"), "obs-9")?,
        vec![(
            Severity::Information,
            "INVARIANT_SKIPPED".to_owned(),
            "Observation.specimen".to_owned(),
        )],
    );
    Ok(())
}

fn example_scenario(system: &str, code: &str, version: bool) -> String {
    let structure_version = if version {
        r#","structureVersion":"5.0.0""#
    } else {
        ""
    };
    format!(
        r#"{{"resourceType":"ExampleScenario","status":"draft",
        "instance":[{{"key":"i1","title":"one",
            "structureType":{{"system":"{system}","code":"{code}"}}{structure_version}}}]}}"#
    )
}

const FHIR_TYPES: &str = "http://hl7.org/fhir/fhir-types";

#[test]
fn exs1_requires_a_version_for_a_non_fhir_structure() -> TestResult {
    assert_eq!(
        about(
            &example_scenario("http://example.org/types", "Thing", false),
            "exs-1"
        )?,
        vec![(
            Severity::Error,
            "INVARIANT_exs-1".to_owned(),
            "ExampleScenario.instance[0]".to_owned(),
        )],
    );
    Ok(())
}

#[test]
fn exs1_passes_for_a_fhir_resource_type_or_a_versioned_structure() -> TestResult {
    assert_eq!(
        about(&example_scenario(FHIR_TYPES, "Patient", false), "exs-1")?,
        vec![]
    );
    let versioned = example_scenario("http://example.org/types", "Thing", true);
    assert_eq!(about(&versioned, "exs-1")?, vec![]);
    Ok(())
}

fn operation(parameter_type: &str, target_profile: bool) -> String {
    let profile = if target_profile {
        r#","targetProfile":["http://hl7.org/fhir/StructureDefinition/Patient"]"#
    } else {
        ""
    };
    format!(
        r#"{{"resourceType":"OperationDefinition","name":"Op","status":"draft","kind":"operation",
        "code":"op","system":false,"type":false,"instance":false,
        "parameter":[{{"name":"p","use":"in","min":0,"max":"1",
            "type":"{parameter_type}"{profile}}}]}}"#
    )
}

#[test]
fn opd3_allows_a_target_profile_only_on_reference_canonical_or_resource_types() -> TestResult {
    assert_eq!(about(&operation("Patient", true), "opd-3")?, vec![]);
    assert_eq!(about(&operation("string", false), "opd-3")?, vec![]);
    assert_eq!(
        about(&operation("string", true), "opd-3")?,
        vec![(
            Severity::Error,
            "INVARIANT_opd-3".to_owned(),
            "OperationDefinition.parameter[0]".to_owned(),
        )],
    );
    Ok(())
}
