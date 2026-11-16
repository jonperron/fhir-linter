//! Contained resources referenced from the narrative, for dom-3.
//!
//! R5 references.html §2.1.3.0.10 lets a resource contain another only if "there is a
//! reference from the resource to the contained resource", and narrative.html
//! §2.1.4.0.3 lets `img/@src` name a contained Binary by `#id`. The dom-3 expression
//! only collects structured references (Reference, canonical, uri, url), so the
//! reference validator replaces it by hand and counts `#id` in narrative links and
//! images (official cases narrative-binary.xml / narrative-binary-bad.xml).
use std::collections::HashSet;
use std::sync::Arc;

use fhir_fhirpath::Value;
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// The root with every `contained` entry whose `id` some narrative of the resource names
/// by `#id` left out; `None` when no entry is left out. dom-3 evaluated on it, with the
/// whole resource as `%resource`, then counts narrative references.
pub(crate) fn without_narrative_referenced(root: &Value) -> Option<Value> {
    let Value::Object(fields) = root else {
        return None;
    };
    let contained = fields.get("contained")?;
    let mut referenced = HashSet::new();
    collect(root, &mut referenced);
    let kept: Vec<Value> = contained
        .iter()
        .filter(|resource| !contained_id(resource).is_some_and(|id| referenced.contains(&*id)))
        .cloned()
        .collect();
    if kept.len() == contained.len() {
        return None;
    }
    let mut fields = (**fields).clone();
    fields.insert(Arc::from("contained"), kept);
    Some(Value::Object(Arc::new(fields)))
}

/// The `id` of a contained resource.
fn contained_id(resource: &Value) -> Option<Arc<str>> {
    let Value::Object(fields) = resource else {
        return None;
    };
    fields.get("id")?.first()?.primitive_value().as_string()
}

/// Add the `#id` targets of every xhtml value under `value` to `referenced`.
fn collect(value: &Value, referenced: &mut HashSet<String>) {
    match value {
        Value::Object(fields) => fields
            .values()
            .flatten()
            .for_each(|child| collect(child, referenced)),
        Value::Primitive(inner, children, ty) => {
            if ty.as_deref() == Some("xhtml")
                && let Some(markup) = inner.as_string()
            {
                fragment_references(&markup, referenced);
            }
            children
                .values()
                .flatten()
                .for_each(|child| collect(child, referenced));
        }
        _ => {}
    }
}

/// Add the ids named by `a/@href="#id"` and `img/@src="#id"` in `markup` to `referenced`.
/// Malformed markup stops the scan: txt-1 (`htmlChecks()`) reports it.
fn fragment_references(markup: &str, referenced: &mut HashSet<String>) {
    let mut reader = Reader::from_str(markup);
    loop {
        let element = match reader.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => e,
            Ok(Event::Eof) | Err(_) => return,
            Ok(_) => continue,
        };
        let attribute: &[u8] = match element.local_name().as_ref() {
            b"a" => b"href",
            b"img" => b"src",
            _ => continue,
        };
        let target = element
            .attributes()
            .flatten()
            .find(|attr| attr.key.as_ref() == attribute)
            .and_then(|attr| attr.normalized_value(XmlVersion::Implicit1_0).ok());
        if let Some(id) = target.as_deref().and_then(|url| url.strip_prefix('#')) {
            referenced.insert(id.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn references(markup: &str) -> HashSet<String> {
        let mut referenced = HashSet::new();
        fragment_references(markup, &mut referenced);
        referenced
    }

    #[test]
    fn finds_link_and_image_fragments_only() {
        let markup = r##"<div xmlns="http://www.w3.org/1999/xhtml"><a href="#a">x</a>
            <img src="#i"/><p id="p" title="#t">#text</p><a href="http://x.org/#e">e</a>
            <img alt="#alt" src="pic.png"/><a name="n">n</a></div>"##;
        let expected: HashSet<String> = ["a", "i"].map(String::from).into();
        assert_eq!(references(markup), expected);
    }

    #[test]
    fn decodes_entities_and_stops_at_malformed_markup() {
        assert_eq!(
            references(r##"<a href="#a&amp;b">x</a>"##),
            ["a&b".to_owned()].into()
        );
        assert_eq!(
            references(r##"<p><a href="#a">x</a></q><img src="#b"/>"##),
            ["a".to_owned()].into()
        );
    }
}
