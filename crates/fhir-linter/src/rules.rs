use fhir_definitions::Registry;
use fhir_parser::{Resource, Value};
use fhir_validator::Diagnostic;

use crate::Rule;

fn has_element(resource: &Resource, registry: &Registry, name: &str) -> bool {
    let rt = resource.resource_type.as_ref();
    registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{rt}"))
        .is_some_and(|sd| sd.element(&format!("{rt}.{name}")).is_some())
}

/// The resource has no (or an empty) human-readable narrative.
pub struct NarrativeAbsent;

impl Rule for NarrativeAbsent {
    fn id(&self) -> &'static str {
        "narrative-absent"
    }

    fn check(&self, resource: &Resource, registry: &Registry) -> Vec<Diagnostic> {
        if !has_element(resource, registry, "text") {
            return vec![];
        }
        let div_empty = match resource.fields.get("text").map(|n| &n.value) {
            Some(Value::Object(text)) => {
                !matches!(text.get("div").map(|n| &n.value), Some(Value::Str(s)) if !s.trim().is_empty())
            }
            _ => true,
        };
        if !div_empty {
            return vec![];
        }
        let d = Diagnostic::warning(
            "LINT_001",
            "Resource narrative (text.div) is absent or empty",
            format!("{}.text", resource.resource_type),
        );
        vec![match resource.fields.get("text") {
            Some(n) => d.with_location(n.span),
            None => d,
        }]
    }
}

/// The resource type supports `identifier` but none is given.
pub struct MissingIdentifier;

impl Rule for MissingIdentifier {
    fn id(&self) -> &'static str {
        "missing-identifier"
    }

    fn check(&self, resource: &Resource, registry: &Registry) -> Vec<Diagnostic> {
        if resource.fields.contains_key("identifier")
            || !has_element(resource, registry, "identifier")
        {
            return vec![];
        }
        vec![Diagnostic::warning(
            "LINT_002",
            "Resource has no identifier",
            format!("{}.identifier", resource.resource_type),
        )]
    }
}

/// `meta.profile` is not declared.
pub struct MissingMetaProfile;

impl Rule for MissingMetaProfile {
    fn id(&self) -> &'static str {
        "missing-meta-profile"
    }

    fn check(&self, resource: &Resource, _registry: &Registry) -> Vec<Diagnostic> {
        let declared = matches!(
            resource.fields.get("meta").map(|n| &n.value),
            Some(Value::Object(meta)) if meta.contains_key("profile")
        );
        if declared {
            return vec![];
        }
        vec![Diagnostic::warning(
            "LINT_003",
            "Resource does not declare meta.profile",
            format!("{}.meta.profile", resource.resource_type),
        )]
    }
}
