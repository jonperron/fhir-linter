use fhir_definitions::Registry;
use fhir_parser::{Node, Resource, Value};
use fhir_validator::Diagnostic;
use quick_xml::Reader;
use quick_xml::events::Event;

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

/// Read a primitive's value: a string, or (XML input) the `value` of a primitive that
/// carries an extension. JSON objects are ignored: JSON puts extensions in `_name`.
fn primitive<'a>(resource: &Resource, node: Option<&'a Node>) -> Option<&'a str> {
    match node.map(|n| &n.value) {
        Some(Value::Str(s)) => Some(s),
        Some(Value::Object(object)) if resource.from_xml => match object.get("value") {
            Some(Node {
                value: Value::Str(s),
                ..
            }) => Some(s),
            _ => None,
        },
        _ => None,
    }
}

/// XML whitespace (space, tab, CR, LF); a non-breaking space is text.
fn has_text(text: &str) -> bool {
    text.chars().any(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
}

/// narrative.html: the div SHALL have some non-whitespace content (text or an image).
/// JSON gives the div as XHTML markup; the XML parser gives its decoded text only.
// shortcut: the XML parser drops `<img>`, so an image-only XML div reads as empty; keep the
// div markup in the parser if that false positive matters.
fn div_has_content(div: &str, from_xml: bool) -> bool {
    if from_xml {
        return has_text(div);
    }
    let mut reader = Reader::from_str(div);
    loop {
        match reader.read_event() {
            Ok(Event::Text(text)) if text.decode().is_ok_and(|t| has_text(&t)) => return true,
            Ok(Event::CData(data)) if has_text(&String::from_utf8_lossy(&data)) => return true,
            Ok(Event::GeneralRef(reference)) => match reference.resolve_char_ref() {
                Ok(Some(c)) if !has_text(c.encode_utf8(&mut [0; 4])) => {}
                // Named entities and non-whitespace characters are text.
                _ => return true,
            },
            Ok(Event::Start(tag) | Event::Empty(tag)) if tag.local_name().as_ref() == b"img" => {
                return true;
            }
            Ok(Event::Eof) => return false,
            // Malformed XHTML is the validator's finding, not an empty narrative.
            Err(_) => return true,
            Ok(_) => {}
        }
    }
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
                // narrative-status `empty`: no human-readable text, whatever the div says.
                primitive(resource, text.get("status")) == Some("empty")
                    || !primitive(resource, text.get("div"))
                        .is_some_and(|div| div_has_content(div, resource.from_xml))
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
            primitive(resource, resource.fields.get("type")),
            Some("document" | "message" | "collection")
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
