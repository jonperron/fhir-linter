//! Attachment.size and Attachment.hash against the bytes of Attachment.data.
//!
//! datatypes.html, Attachment: `size` is "the number of bytes of data that make up this
//! attachment (before base64 encoding)" and `hash` is "the calculated hash of the data using
//! SHA-1. Represented using base64", calculated "on the data prior to base64 encoding".
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use fhir_parser::{Node, Span, Value};
use indexmap::IndexMap;
use std::sync::Arc;

use crate::diagnostic::Diagnostic;

/// Check the `size` and `hash` of the Attachment object `fields` (instance `path`) against
/// its decoded `data`. Without `data`, or when a value does not decode, there is nothing to
/// compare: the content may be at `url`, and the format checks report a bad value.
pub(crate) fn check(
    fields: &IndexMap<Arc<str>, Node>,
    path: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(data) = string(fields, "data").and_then(decode) else {
        return;
    };
    // A JSON number is the wrong shape for integer64 (TYPE_001), but its value still counts.
    let stated_size = match fields.get("size").map(|n| &n.value) {
        Some(Value::Str(s)) => s.parse::<i64>().ok(),
        Some(Value::Integer(i)) => Some(*i),
        _ => None,
    };
    if let Some(stated) = stated_size {
        let actual = data.len();
        if i64::try_from(actual).ok() != Some(stated) {
            diagnostics.push(
                Diagnostic::error(
                    "ATTACHMENT_001",
                    format!(
                        "Stated Attachment size {stated} does not match the actual size \
                         {actual} of the data at '{path}'"
                    ),
                    path,
                )
                .with_location(span),
            );
        }
    }
    if let Some((stated, hash)) = string(fields, "hash").and_then(|s| Some((s, decode(s)?))) {
        let digest = sha1_smol::Sha1::from(&data).digest().bytes();
        if hash != digest {
            let actual = STANDARD.encode(digest);
            diagnostics.push(
                Diagnostic::error(
                    "ATTACHMENT_002",
                    format!(
                        "The SHA-1 hash of the data at '{path}' does not match the stated \
                         hash (stated: '{stated}', actual: '{actual}')"
                    ),
                    path,
                )
                .with_location(span),
            );
        }
    }
}

/// The string value of child `name`, if it has one.
fn string<'a>(fields: &'a IndexMap<Arc<str>, Node>, name: &str) -> Option<&'a str> {
    match &fields.get(name)?.value {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

/// The bytes of base64Binary `s`, whose whitespace readers ignore (datatypes.html,
/// base64Binary, per RFC 4648).
fn decode(s: &str) -> Option<Vec<u8>> {
    let content: String = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    STANDARD.decode(content).ok()
}
