use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use crate::model::{
    AdditionalBinding, Binding, ChoiceValue, CodeSystem, Concept, ConceptMap, ConceptMapElement,
    ConceptMapGroup, ConceptMapTarget, Constraint, Discriminator, ElementDefinition, Include,
    Slicing, StructureDefinition, TypeProfiles, ValueSet,
};
use crate::{Error, Result};

/// The definitions loaded from one or more files, keyed by canonical URL.
#[derive(Default)]
pub struct Definitions {
    pub structure_definitions: HashMap<Arc<str>, StructureDefinition>,
    pub value_sets: HashMap<Arc<str>, ValueSet>,
    pub code_systems: HashMap<Arc<str>, CodeSystem>,
    pub concept_maps: HashMap<Arc<str>, ConceptMap>,
    /// StructureDefinitions without a snapshot: kept out of the map, so no lookup
    /// ever sees an empty snapshot.
    pub needs_snapshot: HashSet<Arc<str>>,
}

/// Parse a FHIR Bundle JSON file and add any `StructureDefinition`, `ValueSet`,
/// `CodeSystem` or `ConceptMap` entries found to `defs`.
pub fn load_bundle(path: &Path, defs: &mut Definitions) -> Result<()> {
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
        load_resource(&entry["resource"], &file_name, defs)?;
    }

    Ok(())
}

/// Add one resource to `defs` if it is a `StructureDefinition`, `ValueSet`,
/// `CodeSystem` or `ConceptMap`, and return its canonical URL. Other resources, and definitions
/// without the fields the registry needs, are skipped (`None`). `file` names the
/// source in errors.
pub fn load_resource(
    resource: &Value,
    file: &str,
    defs: &mut Definitions,
) -> Result<Option<Arc<str>>> {
    let Some(resource_type) = resource["resourceType"].as_str() else {
        return Ok(None);
    };

    let url = match resource_type {
        "StructureDefinition" => {
            let sd =
                parse_structure_definition(resource).map_err(|message| Error::MalformedBundle {
                    file: file.to_owned(),
                    message,
                })?;
            sd.map(|sd| {
                let url = Arc::clone(&sd.url);
                // A reloaded URL keeps only its latest state, never one in each collection.
                if sd.snapshot.is_empty() {
                    defs.structure_definitions.remove(&url);
                    defs.needs_snapshot.insert(Arc::clone(&url));
                } else {
                    defs.needs_snapshot.remove(&url);
                    defs.structure_definitions.insert(Arc::clone(&url), sd);
                }
                url
            })
        }
        "ValueSet" => parse_value_set(resource).map(|vs| {
            let url = Arc::clone(&vs.url);
            defs.value_sets.insert(Arc::clone(&url), vs);
            url
        }),
        "CodeSystem" => parse_code_system(resource).map(|cs| {
            let url = Arc::clone(&cs.url);
            defs.code_systems.insert(Arc::clone(&url), cs);
            url
        }),
        "ConceptMap" => parse_concept_map(resource).map(|cm| {
            let url = Arc::clone(&cm.url);
            defs.concept_maps.insert(Arc::clone(&url), cm);
            url
        }),
        _ => None,
    };
    Ok(url)
}

/// `Ok(None)` skips a definition without url, name or kind; a snapshot element
/// without `path` or `max` is an error, so no element is dropped silently. A
/// missing snapshot gives an empty one, which [`load_resource`] keeps out of the map.
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

    let value_regex = (kind == "primitive-type")
        .then(|| value_regex(resource))
        .flatten();
    Ok(Some(
        StructureDefinition::new(url.into(), name.into(), kind.into(), is_abstract, snapshot)
            .with_value_regex(value_regex)
            .with_version(resource["version"].as_str().map(Arc::from))
            .with_base_definition(resource["baseDefinition"].as_str().map(strip_version))
            .with_implements(implements(resource)),
    ))
}

/// The `valueUri` of each `structuredefinition-implements` extension, version stripped.
fn implements(resource: &Value) -> Vec<Arc<str>> {
    const IMPLEMENTS: &str =
        "http://hl7.org/fhir/StructureDefinition/structuredefinition-implements";
    resource["extension"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|extension| extension["url"] == IMPLEMENTS)
        .filter_map(|extension| extension["valueUri"].as_str().map(strip_version))
        .collect()
}

/// The `regex` extension on a primitive definition's `<type>.value` element type, kept raw.
/// A linear scan of the raw snapshot is fine here: it runs once per primitive definition at
/// load, over its four elements (`<type>`, `.id`, `.extension`, `.value`).
fn value_regex(resource: &Value) -> Option<Arc<str>> {
    let value_path = format!("{}.value", resource["type"].as_str()?);
    resource["snapshot"]["element"]
        .as_array()?
        .iter()
        .find(|elem| elem["path"].as_str() == Some(value_path.as_str()))?["type"]
        .as_array()?
        .iter()
        .filter_map(|type_entry| type_entry["extension"].as_array())
        .flatten()
        .find(|ext| {
            ext["url"].as_str() == Some("http://hl7.org/fhir/StructureDefinition/regex")
        })?["valueString"]
        .as_str()
        .map(Arc::from)
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
        is_modifier: elem["isModifier"].as_bool().unwrap_or(false),
        order_meaning: elem["orderMeaning"].is_string(),
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
    // Kept as written: the validator resolves `|version` and warns on a version mismatch.
    let references = |list: &Value| -> Vec<Arc<str>> {
        list.as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(Arc::from)
            .collect()
    };
    let profiles = references(&t["profile"]);
    let target_profiles = references(&t["targetProfile"]);
    (!profiles.is_empty() || !target_profiles.is_empty()).then(|| TypeProfiles {
        code: code.into(),
        profiles,
        target_profiles,
    })
}

/// A JSON array of canonical URLs, `|version` suffixes kept.
fn canonicals(list: &Value) -> Vec<Arc<str>> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(Arc::from)
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
    let value_set = b["valueSet"].as_str().map(Arc::from);
    // Like a binding without strength, an entry without purpose or valueSet (both 1..1)
    // cannot be applied, so it is skipped rather than failing the whole load.
    let additional = b["additional"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            Some(AdditionalBinding {
                purpose: a["purpose"].as_str()?.into(),
                value_set: a["valueSet"].as_str()?.into(),
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
    let name = resource["name"].as_str().map(Arc::from);
    let compose = &resource["compose"];
    let includes = compose["include"]
        .as_array()
        .map(|list| list.iter().map(parse_include).collect())
        .unwrap_or_default();
    // `inactive = false` drops inactive codes, and no concept status is kept: undecidable.
    let has_exclude = compose["exclude"].as_array().is_some_and(|e| !e.is_empty())
        || compose["inactive"].as_bool() == Some(false);
    Some(ValueSet {
        url,
        name,
        version: resource["version"].as_str().map(Arc::from),
        includes,
        has_exclude,
    })
}

fn parse_include(inc: &Value) -> Include {
    // A `concept` that is not an array is unreadable too.
    let listed = match &inc["concept"] {
        Value::Null => 0,
        Value::Array(list) => list.len(),
        _ => usize::MAX,
    };
    let concepts: Vec<_> = inc["concept"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_concept)
        .collect();
    Include {
        system: inc["system"].as_str().map(Arc::from),
        version: inc["version"].as_str().map(Arc::from),
        // An unreadable concept entry makes the list undecidable, like a filter; it must
        // not shrink to a shorter list, or to "the whole system" when none is readable.
        has_filter: inc["filter"].as_array().is_some_and(|f| !f.is_empty())
            || concepts.len() != listed,
        concepts,
        value_sets: canonicals(&inc["valueSet"]),
    }
}

fn parse_code_system(resource: &Value) -> Option<CodeSystem> {
    let url: Arc<str> = resource["url"].as_str()?.into();
    let name = resource["name"].as_str().map(Arc::from);
    let mut concepts = Vec::new();
    let mut not_selectable = HashSet::new();
    let flag = not_selectable_property_code(resource);
    collect_concepts(
        &resource["concept"],
        flag,
        &mut concepts,
        &mut not_selectable,
    );
    let codes: HashSet<Arc<str>> = concepts.iter().map(|c| Arc::clone(&c.code)).collect();
    // Only a complete, enumerated system can prove a code absent.
    // Case-insensitive systems are never proven absent: lookup is exact-match.
    let enumerated = resource["content"].as_str() == Some("complete") && !codes.is_empty();
    let complete = enumerated && resource["caseSensitive"].as_bool() != Some(false);
    Some(CodeSystem {
        url,
        name,
        version: resource["version"].as_str().map(Arc::from),
        codes,
        concepts,
        not_selectable,
        complete,
        enumerated,
    })
}

/// `None` for a concept without `code`.
fn parse_concept(concept: &Value) -> Option<Concept> {
    Some(Concept {
        code: concept["code"].as_str()?.into(),
        display: concept["display"].as_str().map(Arc::from),
    })
}

/// `None` skips a map without url. Targets that are not plain code mappings, and
/// elements without `code`, are not stored but flagged, so a lookup never takes
/// the rest for the whole answer.
fn parse_concept_map(resource: &Value) -> Option<ConceptMap> {
    fn array(value: &Value) -> impl Iterator<Item = &Value> {
        value.as_array().into_iter().flatten()
    }
    let plain_target = |target: &Value| {
        // `product` only adds outputs; `dependsOn` makes the mapping conditional.
        if target["dependsOn"].is_array() {
            return None;
        }
        Some(ConceptMapTarget {
            code: target["code"].as_str()?.into(),
            relationship: target["relationship"].as_str()?.into(),
        })
    };
    let element = |element: &Value| {
        let parsed: Vec<_> = array(&element["target"]).map(plain_target).collect();
        Some(ConceptMapElement {
            code: element["code"].as_str()?.into(),
            undecided: parsed.iter().any(Option::is_none),
            targets: parsed.into_iter().flatten().collect(),
        })
    };
    let groups = array(&resource["group"])
        .map(|group| {
            let parsed: Vec<_> = array(&group["element"]).map(element).collect();
            // A canonical `url|version` pins a version that translate cannot check or give back.
            let versioned = ["source", "target"]
                .iter()
                .any(|key| group[key].as_str().is_some_and(|c| c.contains('|')));
            ConceptMapGroup {
                versioned,
                source: group["source"].as_str().map(strip_version),
                target: group["target"].as_str().map(strip_version),
                unlisted_undecided: group["unmapped"].is_object()
                    || parsed.iter().any(Option::is_none),
                elements: parsed.into_iter().flatten().collect(),
            }
        })
        .collect();
    Some(ConceptMap {
        url: resource["url"].as_str()?.into(),
        version: resource["version"].as_str().map(Arc::from),
        groups,
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
    out: &mut Vec<Concept>,
    not_selectable: &mut HashSet<Arc<str>>,
) {
    for c in concepts.as_array().into_iter().flatten() {
        if let Some(concept) = parse_concept(c) {
            if is_flagged(c, flag) {
                not_selectable.insert(Arc::clone(&concept.code));
            }
            out.push(concept);
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
mod tests;
