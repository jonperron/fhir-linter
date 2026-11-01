use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::{Resource, Value};

use crate::cardinality;
use crate::diagnostic::Diagnostic;

/// Validate the profiles declared in `meta.profile`.
///
/// Profiles known to the registry are checked for resource-type match
/// (`PROFILE_002`) and their cardinality rules are applied. Unknown profiles
/// produce a `PROFILE_001` warning because they cannot be validated against.
pub fn validate(
    resource: &Resource,
    base: &StructureDefinition,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resource_type = resource.resource_type.as_ref();
    let Some(Value::Object(meta)) = resource.fields.get("meta").map(|n| &n.value) else {
        return;
    };
    let Some(Value::Array(profiles)) = meta.get("profile").map(|n| &n.value) else {
        return;
    };

    for (i, node) in profiles.iter().enumerate() {
        let Value::Str(url) = &node.value else {
            continue;
        };
        let path = format!("{resource_type}.meta.profile[{i}]");
        // Strip the optional `|version` suffix.
        let canonical = url.split('|').next().unwrap_or(url);
        let Some(sd) = registry.structure_definition(canonical) else {
            diagnostics.push(
                Diagnostic::warning("PROFILE_001", format!("Profile '{url}' is not known"), path)
                    .with_location(node.span),
            );
            continue;
        };
        let profile_type = sd.snapshot.first().map(|e| e.path.as_ref());
        // Profiles on abstract parents (e.g. DomainResource) apply to every subtype.
        if profile_type != Some(resource_type) && !sd.is_abstract {
            diagnostics.push(
                Diagnostic::error(
                    "PROFILE_002",
                    format!("Profile '{url}' does not apply to resource type '{resource_type}'"),
                    path,
                )
                .with_location(node.span),
            );
        } else if sd.url != base.url {
            // Keep only findings the base check did not already report.
            let mut found = Vec::new();
            cardinality::validate(resource, sd, &mut found);
            found.retain(|f| {
                !diagnostics
                    .iter()
                    .any(|d| d.code == f.code && d.path == f.path)
            });
            diagnostics.extend(found);
        }
    }
}
