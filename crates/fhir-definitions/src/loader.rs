use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use crate::model::{
    AdditionalBinding, Binding, ChoiceValue, CodeSystem, Constraint, Discriminator,
    ElementDefinition, Include, Slicing, StructureDefinition, TypeProfiles, ValueSet,
};
use crate::{Error, Result};

/// Parse a FHIR Bundle JSON file and populate the given maps with any
/// `StructureDefinition`, `ValueSet`, or `CodeSystem` entries found.
pub fn load_bundle(
    path: &Path,
    structure_definitions: &mut HashMap<Arc<str>, StructureDefinition>,
    value_sets: &mut HashMap<Arc<str>, ValueSet>,
    code_systems: &mut HashMap<Arc<str>, CodeSystem>,
) -> Result<()> {
    let file_name = path.to_string_lossy().into_owned();
    let content = std::fs::read_to_string(path).map_err(Error::Io)?;
    let bundle: Value = serde_json::from_str(&content).map_err(|source| Error::Json {
        file: file_name.clone(),
        source,
    })?;

    let entries = bundle["entry"]
        .as_array()
        .ok_or_else(|| Error::MalformedBundle {
            file: file_name.clone(),
            message: "missing 'entry' array".to_string(),
        })?;

    for entry in entries {
        let resource = &entry["resource"];
        let Some(resource_type) = resource["resourceType"].as_str() else {
            continue;
        };

        match resource_type {
            "StructureDefinition" => {
                let sd = parse_structure_definition(resource).map_err(|message| {
                    Error::MalformedBundle {
                        file: file_name.clone(),
                        message,
                    }
                })?;
                if let Some(sd) = sd {
                    structure_definitions.insert(Arc::clone(&sd.url), sd);
                }
            }
            "ValueSet" => {
                if let Some(vs) = parse_value_set(resource) {
                    value_sets.insert(Arc::clone(&vs.url), vs);
                }
            }
            "CodeSystem" => {
                if let Some(cs) = parse_code_system(resource) {
                    code_systems.insert(Arc::clone(&cs.url), cs);
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// `Ok(None)` skips a definition without url, name or kind; a snapshot element
/// without `path` or `max` is an error, so no element is dropped silently.
fn parse_structure_definition(
    resource: &Value,
) -> std::result::Result<Option<StructureDefinition>, String> {
    let (Some(url), Some(name), Some(kind)) = (
        resource["url"].as_str(),
        resource["name"].as_str(),
        resource["kind"].as_str(),
    ) else {
        return Ok(None);
    };
    let is_abstract = resource["abstract"].as_bool().unwrap_or(false);

    let snapshot = resource["snapshot"]["element"]
        .as_array()
        .map(|elems| {
            elems
                .iter()
                .enumerate()
                .map(|(index, elem)| parse_element_definition(elem, index))
                .collect::<std::result::Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|message| format!("StructureDefinition {url}: {message}"))?
        .unwrap_or_default();

    Ok(Some(StructureDefinition::new(
        url.into(),
        name.into(),
        kind.into(),
        is_abstract,
        snapshot,
    )))
}

/// `index` is the element's position in the snapshot, used to name it when `path` is missing.
fn parse_element_definition(
    elem: &Value,
    index: usize,
) -> std::result::Result<ElementDefinition, String> {
    let path: Arc<str> = elem["path"]
        .as_str()
        .ok_or_else(|| format!("snapshot element {index} has no 'path'"))?
        .into();
    let min = u32::try_from(elem["min"].as_u64().unwrap_or(0)).unwrap_or(0);
    let max: Arc<str> = elem["max"]
        .as_str()
        .ok_or_else(|| format!("snapshot element {path} has no 'max'"))?
        .into();
    let types = elem["type"]
        .as_array()
        .map(|type_list| {
            type_list
                .iter()
                .filter_map(|t| type_code(t).map(Arc::from))
                .collect()
        })
        .unwrap_or_default();

    let constraints = elem["constraint"]
        .as_array()
        .map(|list| list.iter().filter_map(parse_constraint).collect())
        .unwrap_or_default();

    let binding = parse_binding(&elem["binding"]);
    let slice_name = elem["sliceName"].as_str().map(Arc::from);
    let xml_attr = elem["representation"]
        .as_array()
        .is_some_and(|r| r.iter().any(|v| v == "xmlAttr"));
    let content_reference = elem["contentReference"]
        .as_str()
        .map(|r| Arc::from(r.strip_prefix('#').unwrap_or(r)));
    let type_profiles = elem["type"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_type_profiles)
        .collect();
    let slicing = match &elem["slicing"] {
        Value::Null => None,
        slicing => Some(Box::new(parse_slicing(slicing).ok_or_else(|| {
            format!(
                "snapshot element {path} has a slicing without rules or discriminator type/path"
            )
        })?)),
    };

    Ok(ElementDefinition {
        path,
        min,
        max,
        types,
        constraints,
        binding,
        slice_name,
        xml_attr,
        content_reference,
        type_profiles,
        fixed: choice_value(elem, "fixed"),
        pattern: choice_value(elem, "pattern"),
        slicing,
        max_length: elem["maxLength"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok()),
        min_value: choice_value(elem, "minValue"),
        max_value: choice_value(elem, "maxValue"),
    })
}

/// `None` when a required part (`rules`, a discriminator's `type` or `path`) is missing.
fn parse_slicing(slicing: &Value) -> Option<Slicing> {
    let discriminators = slicing["discriminator"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|d| {
            Some(Discriminator {
                kind: d["type"].as_str()?.into(),
                path: d["path"].as_str()?.into(),
            })
        })
        .collect::<Option<_>>()?;
    Some(Slicing {
        discriminators,
        rules: slicing["rules"].as_str()?.into(),
        ordered: slicing["ordered"].as_bool().unwrap_or(false),
    })
}

/// The `<prefix>[x]` property of `elem` (`fixedCode` for `fixed`), if any.
fn choice_value(elem: &Value, prefix: &str) -> Option<Box<ChoiceValue>> {
    let (key, value) = elem.as_object()?.iter().find(|(key, _)| {
        key.strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with(|c: char| c.is_ascii_uppercase()))
    })?;
    Some(Box::new(ChoiceValue {
        key: key.as_str().into(),
        value: value.clone(),
    }))
}

/// The profiles of one `ElementDefinition.type`; `None` when it declares none.
fn parse_type_profiles(t: &Value) -> Option<TypeProfiles> {
    let code = type_code(t)?;
    let profiles = canonicals(&t["profile"]);
    let target_profiles = canonicals(&t["targetProfile"]);
    (!profiles.is_empty() || !target_profiles.is_empty()).then(|| TypeProfiles {
        code: code.into(),
        profiles,
        target_profiles,
    })
}

/// A JSON array of canonical URLs, version suffixes stripped.
fn canonicals(list: &Value) -> Vec<Arc<str>> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(strip_version)
        .collect()
}

/// The FHIR type code of an `ElementDefinition.type`. FHIRPath system types
/// (`http://hl7.org/fhirpath/System.String`) carry the real FHIR type in the
/// `structuredefinition-fhir-type` extension.
fn type_code(t: &Value) -> Option<&str> {
    let code = t["code"].as_str()?;
    if !code.starts_with("http://hl7.org/fhirpath/") {
        return Some(code);
    }
    let fhir_type = t["extension"].as_array().and_then(|exts| {
        exts.iter().find(|e| {
            e["url"].as_str()
                == Some("http://hl7.org/fhir/StructureDefinition/structuredefinition-fhir-type")
        })?["valueUrl"]
            .as_str()
    });
    Some(fhir_type.unwrap_or(code))
}

fn parse_constraint(c: &Value) -> Option<Constraint> {
    let key: Arc<str> = c["key"].as_str()?.into();
    let severity: Arc<str> = c["severity"].as_str()?.into();
    let human: Arc<str> = c["human"].as_str()?.into();
    let expression = c["expression"].as_str().map(Arc::from);
    Some(Constraint::new(key, severity, human, expression))
}

/// Strip a canonical version suffix ("http://x/foo|5.0.0" becomes "http://x/foo").
fn strip_version(s: &str) -> Arc<str> {
    Arc::from(s.split_once('|').map_or(s, |(url, _)| url))
}

fn parse_binding(b: &Value) -> Option<Binding> {
    let strength: Arc<str> = b["strength"].as_str()?.into();
    let value_set = b["valueSet"].as_str().map(strip_version);
    let additional = b["additional"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            Some(AdditionalBinding {
                purpose: a["purpose"].as_str()?.into(),
                value_set: strip_version(a["valueSet"].as_str()?),
                any: a["any"].as_bool().unwrap_or(false),
            })
        })
        .collect();
    Some(Binding {
        strength,
        value_set,
        additional,
    })
}

fn parse_value_set(resource: &Value) -> Option<ValueSet> {
    let url: Arc<str> = resource["url"].as_str()?.into();
    let name: Arc<str> = resource["name"].as_str()?.into();
    let compose = &resource["compose"];
    let includes = compose["include"]
        .as_array()
        .map(|list| list.iter().map(parse_include).collect())
        .unwrap_or_default();
    let has_exclude = compose["exclude"].as_array().is_some_and(|e| !e.is_empty());
    Some(ValueSet {
        url,
        name,
        includes,
        has_exclude,
    })
}

fn parse_include(inc: &Value) -> Include {
    let strings = |v: &Value, key: &str| -> Vec<Arc<str>> {
        v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x[key].as_str().map(Arc::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    Include {
        system: inc["system"].as_str().map(Arc::from),
        version: inc["version"].as_str().map(Arc::from),
        codes: strings(&inc["concept"], "code"),
        has_filter: inc["filter"].as_array().is_some_and(|f| !f.is_empty()),
        value_sets: canonicals(&inc["valueSet"]),
    }
}

fn parse_code_system(resource: &Value) -> Option<CodeSystem> {
    let url: Arc<str> = resource["url"].as_str()?.into();
    let name: Arc<str> = resource["name"].as_str()?.into();
    let mut codes = HashSet::new();
    let mut not_selectable = HashSet::new();
    let flag = not_selectable_property_code(resource);
    collect_concepts(&resource["concept"], flag, &mut codes, &mut not_selectable);
    // Only a complete, enumerated system can prove a code absent.
    // Case-insensitive systems are never proven absent: lookup is exact-match.
    let complete = resource["content"].as_str() == Some("complete")
        && resource["caseSensitive"].as_bool() != Some(false)
        && !codes.is_empty();
    Some(CodeSystem {
        url,
        name,
        version: resource["version"].as_str().map(Arc::from),
        codes,
        not_selectable,
        complete,
    })
}

/// The local property code a CodeSystem declares for the R5 `notSelectable` URI,
/// or `notSelectable` itself when no declared property carries that URI.
fn not_selectable_property_code(resource: &Value) -> &str {
    const URI: &str = "http://hl7.org/fhir/concept-properties#notSelectable";
    resource["property"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|p| {
            p["code"]
                .as_str()
                .filter(|_| p["uri"].as_str() == Some(URI))
        })
        .unwrap_or("notSelectable")
}

fn collect_concepts(
    concepts: &Value,
    flag: &str,
    out: &mut HashSet<Arc<str>>,
    not_selectable: &mut HashSet<Arc<str>>,
) {
    for c in concepts.as_array().into_iter().flatten() {
        if let Some(code) = c["code"].as_str() {
            out.insert(Arc::from(code));
            if is_flagged(c, flag) {
                not_selectable.insert(Arc::from(code));
            }
        }
        collect_concepts(&c["concept"], flag, out, not_selectable);
    }
}

/// A concept whose property `flag` is `valueBoolean: true`.
fn is_flagged(concept: &Value, flag: &str) -> bool {
    concept["property"].as_array().is_some_and(|props| {
        props
            .iter()
            .any(|p| p["code"].as_str() == Some(flag) && p["valueBoolean"].as_bool() == Some(true))
    })
}

#[cfg(test)]
mod tests {
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
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        let result = load_bundle(
            std::path::Path::new("/nonexistent/path/file.json"),
            &mut sds,
            &mut vss,
            &mut css,
        );
        assert!(matches!(result, Err(crate::Error::Io(_))));
    }

    #[test]
    fn load_bundle_invalid_json_returns_json_error() {
        let path = write_temp("fhir_def_test_invalid_json.json", "not valid json {{{{");
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        let result = load_bundle(&path, &mut sds, &mut vss, &mut css);
        assert!(matches!(result, Err(crate::Error::Json { .. })));
    }

    #[test]
    fn load_bundle_missing_entry_array_returns_malformed_bundle_error() {
        let path = write_temp(
            "fhir_def_test_no_entry.json",
            r#"{"resourceType": "Bundle"}"#,
        );
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        let result = load_bundle(&path, &mut sds, &mut vss, &mut css);
        assert!(matches!(result, Err(crate::Error::MalformedBundle { .. })));
    }

    #[test]
    fn load_bundle_skips_unknown_resource_types() {
        let content = r#"{"entry": [{"resource": {"resourceType": "Observation", "url": "http://example.com"}}]}"#;
        let path = write_temp("fhir_def_test_unknown_type.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(sds.is_empty());
        assert!(vss.is_empty());
        assert!(css.is_empty());
    }

    #[test]
    fn load_bundle_skips_structure_definition_with_missing_url() {
        let content = r#"{"entry": [{"resource": {"resourceType": "StructureDefinition", "name": "Foo", "kind": "resource"}}]}"#;
        let path = write_temp("fhir_def_test_sd_no_url.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(sds.is_empty());
    }

    #[test]
    fn load_bundle_parses_value_set_and_code_system() {
        let content = r#"{
            "entry": [
                {"resource": {"resourceType": "ValueSet", "url": "http://example.com/vs", "name": "TestVS"}},
                {"resource": {"resourceType": "CodeSystem", "url": "http://example.com/cs", "name": "TestCS"}}
            ]
        }"#;
        let path = write_temp("fhir_def_test_vs_cs.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(vss.contains_key("http://example.com/vs"));
        assert_eq!(vss["http://example.com/vs"].name.as_ref(), "TestVS");
        assert!(css.contains_key("http://example.com/cs"));
        assert_eq!(css["http://example.com/cs"].name.as_ref(), "TestCS");
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
                    "snapshot": {
                        "element": [
                            {"path": "TestSD.id", "min": 0, "max": "1", "type": [{"code": "id"}]}
                        ]
                    }
                }
            }]
        }"#;
        let path = write_temp("fhir_def_test_sd_snapshot.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        let sd = sds
            .get("http://example.com/sd")
            .expect("SD should be parsed");
        assert_eq!(sd.name.as_ref(), "TestSD");
        assert!(sd.is_abstract);
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
    fn fhirpath_type_code_without_fhir_type_extension_is_kept() {
        let code = "http://hl7.org/fhirpath/System.String";
        let other_ext = serde_json::json!([{"url": "http://example.com/other", "valueUrl": "id"}]);
        for ty in [
            serde_json::json!({"code": code}),
            serde_json::json!({"code": code, "extension": other_ext}),
        ] {
            let elem =
                serde_json::json!({"path": "Patient.id", "min": 0, "max": "1", "type": [ty]});
            let parsed = parse_element_definition(&elem, 0).expect("element should be parsed");
            assert_eq!(parsed.types[0].as_ref(), code);
        }
    }

    #[test]
    fn choice_value_needs_the_prefix_then_a_type_name() {
        let elem = serde_json::json!({
            "_fixedCode": {"id": "x"}, "fixedness": 1, "patternCoding": {"code": "a"}
        });
        assert!(choice_value(&elem, "fixed").is_none());
        let pattern = choice_value(&elem, "pattern").expect("patternCoding is a pattern");
        assert_eq!(pattern.key.as_ref(), "patternCoding");
        assert_eq!(pattern.value, serde_json::json!({"code": "a"}));
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
            let mut sds = HashMap::new();
            let mut vss = HashMap::new();
            let mut css = HashMap::new();
            let result = load_bundle(&path, &mut sds, &mut vss, &mut css);
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
        let content = r#"{"entry": [{"resource": {"resourceType": "ValueSet", "url": "http://example.com/vs-no-name"}}]}"#;
        let path = write_temp("fhir_def_test_vs_no_name.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(vss.is_empty(), "ValueSet without name should be skipped");
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
        let content = r#"{"entry": [{"resource": {"resourceType": "CodeSystem", "url": "http://example.com/cs-no-name"}}]}"#;
        let path = write_temp("fhir_def_test_cs_no_name.json", content);
        let mut sds = HashMap::new();
        let mut vss = HashMap::new();
        let mut css = HashMap::new();
        load_bundle(&path, &mut sds, &mut vss, &mut css).unwrap();
        assert!(css.is_empty(), "CodeSystem without name should be skipped");
    }
}
