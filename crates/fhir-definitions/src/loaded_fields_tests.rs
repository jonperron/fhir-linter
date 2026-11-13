//! Fields the loader keeps from the bundled R5 snapshots, checked on the real `Registry`.
//! Counts come from python scans of `definitions/r5` (PLAN.md 11.6.2).

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use crate::Registry;
use crate::model::{ElementDefinition, StructureDefinition};

type TestResult = Result<(), Box<dyn Error>>;

/// The process-wide shared registry (`Registry::shared` loads it once).
fn registry() -> crate::Result<Arc<Registry>> {
    Registry::shared(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5"))
}

fn definition<'a>(registry: &'a Registry, sd: &str) -> Result<&'a StructureDefinition, String> {
    registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{sd}"))
        .ok_or_else(|| format!("{sd} is not loaded"))
}

fn element<'a>(
    registry: &'a Registry,
    sd: &str,
    path: &str,
) -> Result<&'a ElementDefinition, String> {
    definition(registry, sd)?
        .element(path)
        .ok_or_else(|| format!("{sd} has no element {path}"))
}

fn count(registry: &Registry, predicate: impl Fn(&ElementDefinition) -> bool) -> usize {
    registry
        .snapshot_elements()
        .filter(|e| predicate(e))
        .count()
}

#[test]
fn type_profiles_are_loaded_per_type() -> TestResult {
    let registry = registry()?;
    let has_profile =
        |e: &ElementDefinition| e.type_profiles.iter().any(|t| !t.profiles.is_empty());
    assert_eq!(count(&registry, has_profile), 188);
    let quantity = element(&registry, "Claim", "Claim.item.quantity")?;
    let [simple] = quantity.type_profiles.as_slice() else {
        return Err(format!("one profiled type expected: {:?}", quantity.type_profiles).into());
    };
    assert_eq!(simple.code.as_ref(), "Quantity");
    assert_eq!(
        simple.profiles,
        [Arc::from(
            "http://hl7.org/fhir/StructureDefinition/SimpleQuantity"
        )],
    );
    // Types without a profile take no entry.
    let birth_date = element(&registry, "Patient", "Patient.birthDate")?;
    assert!(birth_date.type_profiles.is_empty());
    Ok(())
}

#[test]
fn target_profiles_are_loaded_per_type() -> TestResult {
    let registry = registry()?;
    let targeted = registry
        .snapshot_elements()
        .flat_map(|e| &e.type_profiles)
        .filter(|t| !t.target_profiles.is_empty())
        .count();
    assert_eq!(targeted, 1543);
    let subject = element(&registry, "Observation", "Observation.subject")?;
    let [reference] = subject.type_profiles.as_slice() else {
        return Err(format!("one targeted type expected: {:?}", subject.type_profiles).into());
    };
    assert_eq!(reference.code.as_ref(), "Reference");
    assert!(reference.profiles.is_empty());
    let targets: Vec<_> = reference
        .target_profiles
        .iter()
        .map(AsRef::as_ref)
        .collect();
    assert!(targets.contains(&"http://hl7.org/fhir/StructureDefinition/Patient"));
    assert!(targets.contains(&"http://hl7.org/fhir/StructureDefinition/Group"));
    Ok(())
}

#[test]
fn fixed_and_pattern_values_are_loaded_with_their_key() -> TestResult {
    let registry = registry()?;
    assert_eq!(count(&registry, |e| e.fixed.is_some()), 87);
    assert_eq!(count(&registry, |e| e.pattern.is_some()), 33);
    assert_eq!(
        count(&registry, |e| e.fixed.is_some() || e.pattern.is_some()),
        120
    );
    let category_code = definition(&registry, "vitalsigns")?
        .snapshot
        .iter()
        .filter(|e| e.path.as_ref() == "Observation.category.coding.code")
        .find_map(|e| e.fixed.as_deref())
        .ok_or("the VSCat slice fixes its code")?;
    assert_eq!(category_code.key.as_ref(), "fixedCode");
    assert_eq!(category_code.value, serde_json::json!("vital-signs"));
    let pattern = registry
        .snapshot_elements()
        .find_map(|e| e.pattern.as_deref())
        .ok_or("some element has a pattern")?;
    assert!(pattern.key.starts_with("pattern"));
    Ok(())
}

#[test]
fn slicing_and_slice_names_are_loaded() -> TestResult {
    let registry = registry()?;
    assert_eq!(count(&registry, |e| e.slicing.is_some()), 255);
    assert_eq!(count(&registry, |e| e.slice_name.is_some()), 174);
    let vital_signs = definition(&registry, "vitalsigns")?;
    let category = vital_signs
        .element("Observation.category")
        .and_then(|e| e.slicing.as_deref())
        .ok_or("vitalsigns slices Observation.category")?;
    let discriminators: Vec<_> = category
        .discriminators
        .iter()
        .map(|d| (d.kind.as_ref(), d.path.as_ref()))
        .collect();
    assert_eq!(
        discriminators,
        [("value", "coding.code"), ("value", "coding.system")],
    );
    assert_eq!(category.rules.as_ref(), "open");
    assert!(!category.ordered);
    let vs_cat = vital_signs
        .descendants("Observation.category")
        .iter()
        .find(|e| e.slice_name.as_deref() == Some("VSCat"))
        .ok_or("VSCat slice")?;
    assert_eq!(vs_cat.min, 1);
    Ok(())
}

#[test]
fn additional_bindings_are_loaded() -> TestResult {
    let registry = registry()?;
    let additional = |e: &ElementDefinition| e.binding.as_ref().map_or(0, |b| b.additional.len());
    assert_eq!(count(&registry, |e| additional(e) > 0), 267);
    let maximum = registry
        .snapshot_elements()
        .flat_map(|e| e.binding.iter().flat_map(|b| &b.additional))
        .filter(|a| a.purpose.as_ref() == "maximum")
        .count();
    assert_eq!(maximum, 4);
    let language = element(&registry, "Expression", "Expression.language")?;
    let binding = language
        .binding
        .as_ref()
        .ok_or("Expression.language is bound")?;
    let [mimetypes] = binding.additional.as_slice() else {
        return Err("one additional binding expected".into());
    };
    assert_eq!(mimetypes.purpose.as_ref(), "maximum");
    assert_eq!(
        mimetypes.value_set.as_ref(),
        "http://hl7.org/fhir/ValueSet/mimetypes"
    );
    assert!(!mimetypes.any);
    Ok(())
}

#[test]
fn max_length_and_value_limits_are_loaded() -> TestResult {
    let registry = registry()?;
    assert_eq!(count(&registry, |e| e.max_length.is_some()), 1);
    let string = element(&registry, "string", "string.value")?;
    assert_eq!(string.max_length, Some(1_048_576));
    assert_eq!(count(&registry, |e| e.min_value.is_some()), 2);
    assert_eq!(count(&registry, |e| e.max_value.is_some()), 2);
    let integer = element(&registry, "integer", "integer.value")?;
    let min = integer
        .min_value
        .as_deref()
        .ok_or("integer has a minimum")?;
    let max = integer
        .max_value
        .as_deref()
        .ok_or("integer has a maximum")?;
    assert_eq!(min.key.as_ref(), "minValueInteger");
    assert_eq!(min.value, serde_json::json!(-2_147_483_648_i64));
    assert_eq!(max.key.as_ref(), "maxValueInteger");
    assert_eq!(max.value, serde_json::json!(2_147_483_647));
    Ok(())
}

#[test]
fn content_reference_is_loaded_without_the_local_hash() -> TestResult {
    let registry = registry()?;
    assert_eq!(count(&registry, |e| e.content_reference.is_some()), 153);
    let item = element(&registry, "Questionnaire", "Questionnaire.item.item")?;
    assert_eq!(
        item.content_reference.as_deref(),
        Some("Questionnaire.item")
    );
    // A derived profile points into its base definition by canonical URL; keep it whole.
    let range = element(
        &registry,
        "vitalsigns",
        "Observation.component.referenceRange",
    )?;
    assert_eq!(
        range.content_reference.as_deref(),
        Some("http://hl7.org/fhir/StructureDefinition/Observation#Observation.referenceRange"),
    );
    Ok(())
}

#[test]
fn primitive_regex_is_loaded_raw_from_the_value_element() -> TestResult {
    let registry = registry()?;
    assert_eq!(registry.primitive_regex("uri"), Some(r"\S*"));
    assert_eq!(registry.primitive_regex("string"), Some(r"^[\s\S]+$"));
    assert_eq!(
        registry.primitive_regex("id"),
        Some(r"[A-Za-z0-9\-\.]{1,64}")
    );
    assert_eq!(
        registry.primitive_regex("oid"),
        Some(r"urn:oid:[0-2](\.(0|[1-9][0-9]*))+"),
    );
    // Kept as published, typo included: `{1,9}}` (PLAN.md 11.2-A4).
    assert_eq!(
        registry.primitive_regex("decimal"),
        Some(r"-?(0|[1-9][0-9]{0,17})(\.[0-9]{1,17})?([eE][+-]?[0-9]{1,9}})?"),
    );
    assert_eq!(registry.primitive_regex("xhtml"), None);
    assert_eq!(registry.primitive_regex("HumanName"), None);
    assert_eq!(registry.primitive_regex("Patient"), None);
    assert_eq!(registry.primitive_regex("no-such-type"), None);

    // Every R5 primitive but xhtml publishes one.
    let primitives: Vec<_> = registry
        .structure_definitions()
        .filter(|sd| sd.kind.as_ref() == "primitive-type")
        .collect();
    assert_eq!(primitives.len(), 21);
    let with_regex = primitives
        .iter()
        .filter(|sd| registry.primitive_regex(&sd.name).is_some())
        .count();
    assert_eq!(with_regex, 20);
    Ok(())
}
