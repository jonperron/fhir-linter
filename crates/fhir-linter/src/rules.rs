use fhir_definitions::Registry;
use fhir_parser::{Node, Resource, Value};
use fhir_validator::Diagnostic;

use crate::Rule;

/// A repeating element is given only when its array holds at least one non-null item.
fn is_given(node: Option<&Node>) -> bool {
    match node.map(|n| &n.value) {
        None => false,
        Some(Value::Array(items)) => items.iter().any(|n| !matches!(n.value, Value::Null)),
        Some(_) => true,
    }
}

fn has_element(resource: &Resource, registry: &Registry, name: &str) -> bool {
    let rt = resource.resource_type.as_ref();
    registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{rt}"))
        .is_some_and(|sd| sd.element(&format!("{rt}.{name}")).is_some())
}

/// narrative.html: the div SHALL have some non-whitespace content (text or an image), so
/// it has content when, tags stripped, non-whitespace text or an `<img>` element remains.
/// Entity and character references (`&#160;`) count as text.
// shortcut: a tag ends at its first `>`, so a `>` inside an attribute value or comment leaves
// text behind and silences the rule; use a real XHTML parser if that ever matters.
fn div_has_content(div: &str) -> bool {
    let mut rest = div;
    while let Some(open) = rest.find('<') {
        if !rest[..open].trim().is_empty() {
            return true;
        }
        let tag = &rest[open + 1..];
        let end = tag.find('>').unwrap_or(tag.len());
        let name = tag[..end]
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or_default();
        if name.rsplit(':').next() == Some("img") {
            return true;
        }
        rest = &tag[(end + 1).min(tag.len())..];
    }
    !rest.trim().is_empty()
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
        let narrative_empty = match resource.fields.get("text").map(|n| &n.value) {
            Some(Value::Object(text)) => {
                let string = |key: &str| match text.get(key).map(|n| &n.value) {
                    Some(Value::Str(s)) => Some(s.as_ref()),
                    _ => None,
                };
                // narrative-status `empty`: no human-readable text, whatever the div says.
                string("status") == Some("empty") || !string("div").is_some_and(div_has_content)
            }
            _ => true,
        };
        if !narrative_empty {
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

/// R5 `Bundle.identifier` comment: persistent identity only matters for Bundles of type
/// document, message and collection. Always true for other resource types.
fn bundle_needs_identity(resource: &Resource) -> bool {
    resource.resource_type.as_ref() != "Bundle"
        || matches!(
            resource.fields.get("type").map(|n| &n.value),
            Some(Value::Str(kind)) if matches!(kind.as_ref(), "document" | "message" | "collection")
        )
}

/// Locate a resource-level finding at `meta`, the closest span when it is present.
fn at_meta(resource: &Resource, diagnostic: Diagnostic) -> Diagnostic {
    match resource.fields.get("meta") {
        Some(meta) => diagnostic.with_location(meta.span),
        None => diagnostic,
    }
}

/// The resource type supports `identifier` but none is given.
pub struct MissingIdentifier;

impl Rule for MissingIdentifier {
    fn id(&self) -> &'static str {
        "missing-identifier"
    }

    fn check(&self, resource: &Resource, registry: &Registry) -> Vec<Diagnostic> {
        if is_given(resource.fields.get("identifier"))
            || !has_element(resource, registry, "identifier")
            || !bundle_needs_identity(resource)
        {
            return vec![];
        }
        vec![at_meta(
            resource,
            Diagnostic::warning(
                "LINT_002",
                "Resource has no identifier",
                format!("{}.identifier", resource.resource_type),
            ),
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
            Some(Value::Object(meta)) if is_given(meta.get("profile"))
        );
        if declared {
            return vec![];
        }
        vec![at_meta(
            resource,
            Diagnostic::warning(
                "LINT_003",
                "Resource does not declare meta.profile",
                format!("{}.meta.profile", resource.resource_type),
            ),
        )]
    }
}
