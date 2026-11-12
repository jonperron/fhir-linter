#![forbid(unsafe_code)]
//! FHIR R5 resource validator.
//!
//! Applies validation layers in order: structural, cardinality and type
//! (one recursive walk), terminology, invariants, references, and profile checks.
//! Emits `Diagnostic` values for every detected issue.

mod diagnostic;
mod invariants;
mod nested;
mod profile;
mod references;
mod terminology;
mod type_check;
mod xml_shape;

pub use diagnostic::{Diagnostic, Severity};

use fhir_definitions::Registry;
use fhir_parser::Resource;

/// Validate a parsed FHIR resource against the definitions in the registry.
///
/// Applies validation layers in order and returns all findings.
///
/// Returns a single `STRUCTURE_000` error when the resource type is not a
/// concrete resource known to the registry (abstract resources such as
/// `DomainResource` and datatypes are rejected).
pub fn validate(resource: &Resource, registry: &Registry) -> Vec<Diagnostic> {
    let resource_type = resource.resource_type.as_ref();
    let url = format!("http://hl7.org/fhir/StructureDefinition/{resource_type}");

    let definition = registry.structure_definition(&url);
    let Some(sd) = definition.filter(|sd| sd.kind.as_ref() == "resource" && !sd.is_abstract) else {
        let message = match definition {
            Some(_) => format!("'{resource_type}' is not a concrete resource type"),
            None => format!("Unknown resource type '{resource_type}'"),
        };
        return vec![Diagnostic::error(
            "STRUCTURE_000",
            message,
            resource_type.to_owned(),
        )];
    };

    // XML input is reshaped once, so every layer sees the JSON form (typed primitives, `_name`).
    // The reshape keeps XML grouping, so only JSON input has a meaningful array shape.
    let json = !resource.from_xml;
    let reshaped;
    let mut diagnostics = Vec::new();
    let resource = if resource.from_xml {
        let (shaped, xml_diagnostics) = xml_shape::to_json_shape(resource, sd, registry);
        reshaped = shaped;
        diagnostics = xml_diagnostics;
        &reshaped
    } else {
        resource
    };

    nested::validate(resource, sd, registry, json, &mut diagnostics);
    terminology::validate(resource, sd, registry, &mut diagnostics);
    invariants::validate(
        resource,
        sd,
        &|base, suffix| registry.is_choice_variant(base, suffix),
        &|parent, name| registry.child_element(parent, name),
        &mut diagnostics,
    );
    references::validate(resource, sd, registry, &mut diagnostics);
    profile::validate(resource, sd, registry, &mut diagnostics);

    diagnostics
}
