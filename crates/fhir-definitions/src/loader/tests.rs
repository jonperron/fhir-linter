use super::*;

/// Temp file removed on drop; name is pid-scoped to avoid cross-run collisions.
struct TempFile(std::path::PathBuf);

impl std::ops::Deref for TempFile {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn write_temp(name: &str, content: &str) -> TempFile {
    let path = std::env::temp_dir().join(format!("{}_{name}", std::process::id()));
    std::fs::write(&path, content).unwrap();
    TempFile(path)
}

#[test]
fn load_bundle_missing_file_returns_io_error() {
    let mut defs = Definitions::default();
    let result = load_bundle(
        std::path::Path::new("/nonexistent/path/file.json"),
        &mut defs,
    );
    assert!(matches!(result, Err(crate::Error::Io(_))));
}

#[test]
fn load_bundle_invalid_json_returns_json_error() {
    let path = write_temp("fhir_def_test_invalid_json.json", "not valid json {{{{");
    let mut defs = Definitions::default();
    let result = load_bundle(&path, &mut defs);
    assert!(matches!(result, Err(crate::Error::Json { .. })));
}

#[test]
fn load_bundle_missing_entry_array_returns_malformed_bundle_error() {
    let path = write_temp(
        "fhir_def_test_no_entry.json",
        r#"{"resourceType": "Bundle"}"#,
    );
    let mut defs = Definitions::default();
    let result = load_bundle(&path, &mut defs);
    assert!(matches!(result, Err(crate::Error::MalformedBundle { .. })));
}

#[test]
fn load_bundle_skips_unknown_resource_types() {
    let content = r#"{"entry": [
        {"resource": {"resourceType": "Observation", "url": "http://example.com"}}
    ]}"#;
    let path = write_temp("fhir_def_test_unknown_type.json", content);
    let mut defs = Definitions::default();
    load_bundle(&path, &mut defs).unwrap();
    assert!(defs.structure_definitions.is_empty());
    assert!(defs.value_sets.is_empty());
    assert!(defs.code_systems.is_empty());
}

#[test]
fn load_bundle_skips_structure_definition_with_missing_url() {
    let content = r#"{"entry": [
        {"resource": {"resourceType": "StructureDefinition", "name": "Foo", "kind": "resource"}}
    ]}"#;
    let path = write_temp("fhir_def_test_sd_no_url.json", content);
    let mut defs = Definitions::default();
    load_bundle(&path, &mut defs).unwrap();
    assert!(defs.structure_definitions.is_empty());
}

#[test]
fn load_bundle_parses_value_set_and_code_system() {
    let content = r#"{
        "entry": [
            {"resource": {
                "resourceType": "ValueSet", "url": "http://example.com/vs", "name": "TestVS"
            }},
            {"resource": {
                "resourceType": "CodeSystem", "url": "http://example.com/cs", "name": "TestCS"
            }}
        ]
    }"#;
    let path = write_temp("fhir_def_test_vs_cs.json", content);
    let mut defs = Definitions::default();
    load_bundle(&path, &mut defs).unwrap();
    assert!(defs.value_sets.contains_key("http://example.com/vs"));
    assert_eq!(
        defs.value_sets["http://example.com/vs"].name.as_ref(),
        "TestVS"
    );
    assert!(defs.code_systems.contains_key("http://example.com/cs"));
    assert_eq!(
        defs.code_systems["http://example.com/cs"].name.as_ref(),
        "TestCS"
    );
}

#[test]
fn load_bundle_parses_structure_definition_with_snapshot() {
    let content = r#"{
        "entry": [{
            "resource": {
                "resourceType": "StructureDefinition",
                "url": "http://example.com/sd",
                "name": "TestSD",
                "kind": "resource",
                "abstract": true,
                "baseDefinition": "http://example.com/parent|1.0",
                "snapshot": {
                    "element": [
                        {"path": "TestSD.id", "min": 0, "max": "1", "type": [{"code": "id"}]}
                    ]
                }
            }
        }]
    }"#;
    let path = write_temp("fhir_def_test_sd_snapshot.json", content);
    let mut defs = Definitions::default();
    load_bundle(&path, &mut defs).unwrap();
    let sd = defs
        .structure_definitions
        .get("http://example.com/sd")
        .expect("SD should be parsed");
    assert_eq!(sd.name.as_ref(), "TestSD");
    assert!(sd.is_abstract);
    assert_eq!(
        sd.base_definition.as_deref(),
        Some("http://example.com/parent")
    );
    assert_eq!(sd.snapshot.len(), 1);
    assert_eq!(sd.snapshot[0].path.as_ref(), "TestSD.id");
    assert_eq!(sd.snapshot[0].min, 0);
    assert_eq!(sd.snapshot[0].max.as_ref(), "1");
    assert_eq!(sd.snapshot[0].types[0].as_ref(), "id");
}

#[test]
fn fhirpath_type_code_resolves_to_fhir_type_extension() {
    let elem = serde_json::json!({
        "path": "Patient.id", "min": 0, "max": "1",
        "type": [{
            "extension": [{
                "url": "http://hl7.org/fhir/StructureDefinition/structuredefinition-fhir-type",
                "valueUrl": "id"
            }],
            "code": "http://hl7.org/fhirpath/System.String"
        }]
    });
    let parsed = parse_element_definition(&elem, 0).expect("element should be parsed");
    assert_eq!(parsed.types[0].as_ref(), "id");
}

#[test]
fn value_regex_is_kept_only_for_primitive_types() -> std::result::Result<(), String> {
    let definition = |kind: &str| {
        serde_json::json!({
            "url": "http://example.com/Thing", "name": "Thing", "kind": kind, "type": "thing",
            "snapshot": {"element": [{
                "path": "thing.value", "min": 0, "max": "1",
                "type": [{
                    "code": "http://hl7.org/fhirpath/System.String",
                    "extension": [
                        {"url": "http://example.com/other", "valueString": "no"},
                        {"url": "http://hl7.org/fhir/StructureDefinition/regex",
                            "valueString": "[a-z]+"}
                    ]
                }]
            }]}
        })
    };
    let parse = |kind: &str| {
        parse_structure_definition(&definition(kind))?.ok_or_else(|| "skipped".to_string())
    };
    assert_eq!(
        parse("primitive-type")?.value_regex.as_deref(),
        Some("[a-z]+")
    );
    assert_eq!(parse("complex-type")?.value_regex, None);
    Ok(())
}

#[test]
fn fhirpath_type_code_without_fhir_type_extension_is_kept() {
    let code = "http://hl7.org/fhirpath/System.String";
    let other_ext = serde_json::json!([{"url": "http://example.com/other", "valueUrl": "id"}]);
    for ty in [
        serde_json::json!({"code": code}),
        serde_json::json!({"code": code, "extension": other_ext}),
    ] {
        let elem = serde_json::json!({"path": "Patient.id", "min": 0, "max": "1", "type": [ty]});
        let parsed = parse_element_definition(&elem, 0).expect("element should be parsed");
        assert_eq!(parsed.types[0].as_ref(), code);
    }
}

#[test]
fn choice_value_needs_the_prefix_then_a_type_name() -> std::result::Result<(), &'static str> {
    let elem = serde_json::json!({
        "_fixedCode": {"id": "x"}, "fixedness": 1, "patternCoding": {"code": "a"}
    });
    assert!(choice_value(&elem, "fixed").is_none());
    let pattern = choice_value(&elem, "pattern").ok_or("patternCoding is a pattern")?;
    assert_eq!(pattern.key.as_ref(), "patternCoding");
    assert_eq!(pattern.value, serde_json::json!({"code": "a"}));
    Ok(())
}

#[test]
fn slicing_keeps_ordered_and_every_discriminator() -> std::result::Result<(), &'static str> {
    let slicing = parse_slicing(&serde_json::json!({
        "rules": "closed", "ordered": true,
        "discriminator": [{"type": "type", "path": "$this"}, {"type": "exists", "path": "x"}]
    }))
    .ok_or("slicing is complete")?;
    assert!(slicing.ordered);
    assert_eq!(slicing.rules.as_ref(), "closed");
    let kinds: Vec<_> = slicing
        .discriminators
        .iter()
        .map(|d| d.kind.as_ref())
        .collect();
    assert_eq!(kinds, ["type", "exists"]);
    Ok(())
}

#[test]
fn additional_binding_keeps_any_and_skips_incomplete_entries()
-> std::result::Result<(), &'static str> {
    let binding = parse_binding(&serde_json::json!({
        "strength": "required",
        "additional": [
            {"purpose": "maximum", "valueSet": "http://x/vs|1.0", "any": true},
            {"purpose": "ui"}
        ]
    }))
    .ok_or("binding has a strength")?;
    let [maximum] = binding.additional.as_slice() else {
        return Err("only the complete entry is kept");
    };
    assert!(maximum.any);
    assert_eq!(maximum.value_set.as_ref(), "http://x/vs");
    Ok(())
}

#[test]
fn load_bundle_rejects_malformed_snapshot_element() {
    for (name, element, expected) in [
        (
            "slicing_no_rules",
            r#"{"path": "TestSD2.id", "min": 0, "max": "1", "slicing": {}}"#,
            "TestSD2.id has a slicing without",
        ),
        (
            "discriminator_no_type",
            r#"{"path": "TestSD2.id", "min": 0, "max": "1",
                "slicing": {"rules": "open", "discriminator": [{"path": "code"}]}}"#,
            "TestSD2.id has a slicing without",
        ),
        (
            "discriminator_no_path",
            r#"{"path": "TestSD2.id", "min": 0, "max": "1",
                "slicing": {"rules": "open", "discriminator": [{"type": "value"}]}}"#,
            "TestSD2.id has a slicing without",
        ),
        (
            "no_path",
            r#"{"min": 0, "max": "1"}"#,
            "element 1 has no 'path'",
        ),
        (
            "no_max",
            r#"{"path": "TestSD2.id", "min": 0}"#,
            "TestSD2.id has no 'max'",
        ),
    ] {
        let content = format!(
            r#"{{"entry": [{{"resource": {{
                "resourceType": "StructureDefinition", "url": "http://example.com/sd2",
                "name": "TestSD2", "kind": "resource",
                "snapshot": {{"element": [
                    {{"path": "TestSD2", "min": 0, "max": "*"}}, {element}]}}
            }}}}]}}"#
        );
        let path = write_temp(&format!("fhir_def_test_sd_elem_{name}.json"), &content);
        let mut defs = Definitions::default();
        let result = load_bundle(&path, &mut defs);
        let Err(crate::Error::MalformedBundle { message, .. }) = result else {
            panic!("{name}: expected MalformedBundle, got {result:?}");
        };
        assert!(
            message.contains("http://example.com/sd2"),
            "{name}: {message}"
        );
        assert!(message.contains(expected), "{name}: {message}");
    }
}

#[test]
fn load_bundle_skips_value_set_with_missing_name() {
    let content = r#"{"entry": [
        {"resource": {"resourceType": "ValueSet", "url": "http://example.com/vs-no-name"}}
    ]}"#;
    let path = write_temp("fhir_def_test_vs_no_name.json", content);
    let mut defs = Definitions::default();
    load_bundle(&path, &mut defs).unwrap();
    assert!(
        defs.value_sets.is_empty(),
        "ValueSet without name should be skipped"
    );
}

#[test]
fn code_system_collects_not_selectable_concepts_at_every_depth() {
    let cs = parse_code_system(&serde_json::json!({
        "url": "http://example.com/cs", "name": "CS", "content": "complete",
        "concept": [
            {"code": "a", "property": [{"code": "notSelectable", "valueBoolean": true}],
             "concept": [
                 {"code": "b", "property": [{"code": "abstract", "valueBoolean": true}]},
                 {"code": "c", "property": [{"code": "notSelectable", "valueBoolean": false}]}
             ]},
            {"code": "d", "property": [{"code": "abstract-type", "valueBoolean": true}]}
        ]
    }))
    .expect("CodeSystem should be parsed");
    assert_eq!(cs.codes.len(), 4);
    let mut flagged: Vec<_> = cs.not_selectable.iter().map(AsRef::as_ref).collect();
    flagged.sort_unstable();
    // `abstract` is not an R5 concept property.
    assert_eq!(flagged, ["a"]);
}

#[test]
fn code_system_finds_not_selectable_by_its_declared_property_uri() {
    let cs = parse_code_system(&serde_json::json!({
        "url": "http://example.com/cs", "name": "CS", "content": "complete",
        "property": [{
            "code": "ns", "type": "boolean",
            "uri": "http://hl7.org/fhir/concept-properties#notSelectable"
        }],
        "concept": [
            {"code": "a", "property": [{"code": "ns", "valueBoolean": true}],
             "concept": [
                 {"code": "c", "property": [{"code": "ns", "valueBoolean": true}]},
                 {"code": "d", "property": [{"code": "ns", "valueBoolean": false}]}
             ]},
            {"code": "b", "property": [{"code": "notSelectable", "valueBoolean": true}]}
        ]
    }))
    .expect("CodeSystem should be parsed");
    let mut flagged: Vec<_> = cs.not_selectable.iter().map(AsRef::as_ref).collect();
    flagged.sort_unstable();
    // The local code `ns` carries the URI; the bare code is only a fallback.
    assert_eq!(flagged, ["a", "c"]);
}

#[test]
fn load_bundle_skips_code_system_with_missing_name() {
    let content = r#"{"entry": [
        {"resource": {"resourceType": "CodeSystem", "url": "http://example.com/cs-no-name"}}
    ]}"#;
    let path = write_temp("fhir_def_test_cs_no_name.json", content);
    let mut defs = Definitions::default();
    load_bundle(&path, &mut defs).unwrap();
    assert!(
        defs.code_systems.is_empty(),
        "CodeSystem without name should be skipped"
    );
}
