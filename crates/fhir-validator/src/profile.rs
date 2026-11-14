use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::{Resource, Value};

use crate::diagnostic::Diagnostic;
use crate::nested::check_children_cardinality;

/// Validate the profiles declared in `meta.profile`.
///
/// Profiles known to the registry are checked for resource-type match
/// (`PROFILE_002`) and their cardinality rules are applied. Unknown profiles
/// produce a `PROFILE_001` warning because they cannot be validated against. A `|version`
/// other than the loaded definition's gives a `PROFILE_003` warning.
/// `resource_path` is the resource's instance path.
pub fn validate(
    resource: &Resource,
    base: &StructureDefinition,
    registry: &Registry,
    resource_path: &str,
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
        let path = format!("{resource_path}.meta.profile[{i}]");
        // references.html canonical form: `url|version#fragment`.
        let (canonical, version) = match url.split_once('|') {
            Some((canonical, rest)) => (canonical, rest.split('#').next()),
            None => (url.as_ref(), None),
        };
        let Some(sd) = registry.structure_definition(canonical) else {
            diagnostics.push(
                Diagnostic::warning("PROFILE_001", format!("Profile '{url}' is not known"), path)
                    .with_location(node.span),
            );
            continue;
        };
        // The spec is silent on a version mismatch: warn, then validate against the loaded
        // version (user decision, PLAN 11.7 V10).
        if let Some(version) = version.filter(|v| sd.version.as_deref() != Some(*v)) {
            let loaded = sd.version.as_deref().unwrap_or("none");
            diagnostics.push(
                Diagnostic::warning(
                    "PROFILE_003",
                    format!(
                        "Profile '{url}' asks for version '{version}' but version '{loaded}' is \
                         loaded; validated against the loaded version"
                    ),
                    path.clone(),
                )
                .with_location(node.span),
            );
        }
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
            check_children_cardinality(
                &resource.fields,
                sd,
                resource_type,
                resource_path,
                &mut found,
            );
            found.retain(|f| {
                !diagnostics
                    .iter()
                    .any(|d| d.code == f.code && d.path == f.path)
            });
            diagnostics.extend(found);
        }
    }
}
