//! `htmlChecks()` (R5 fhirpath.html §2.1.9.1), the expression of the Narrative
//! invariants txt-1 and txt-2. The rules come from R5 narrative.html#xhtml:
//!
//! - only the basic formatting elements of HTML 4.0 chapters 7-11 (except 9.4) and 15,
//!   `<a>` (name or href), images and internal `style` attributes;
//! - no head, body, external stylesheet references, deprecated elements, scripts,
//!   forms, base/link/xlink, frames, iframes, objects or event attributes, so that
//!   the narrative holds no active content;
//! - some non-whitespace content (text or an image).
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

const XHTML_NS: &[u8] = b"http://www.w3.org/1999/xhtml";

/// HTML 4.0 elements of chapters 7 (div, span, headings, address), 8 (bdo), 9 except
/// section 4 (ins, del), 10 and 11, and 15, without the deprecated ones (dir, menu,
/// strike, s, u, font, basefont, center); plus `a` and the image elements of 13.
const ELEMENTS: &[&str] = &[
    "div",
    "span",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "address",
    "bdo",
    "em",
    "strong",
    "dfn",
    "code",
    "samp",
    "kbd",
    "var",
    "cite",
    "abbr",
    "acronym",
    "blockquote",
    "q",
    "sub",
    "sup",
    "p",
    "br",
    "pre",
    "ul",
    "ol",
    "li",
    "dl",
    "dt",
    "dd",
    "table",
    "caption",
    "thead",
    "tfoot",
    "tbody",
    "colgroup",
    "col",
    "tr",
    "th",
    "td",
    "tt",
    "i",
    "b",
    "big",
    "small",
    "hr",
    "a",
    "img",
    "map",
    "area",
];

/// The HTML 4.0 attributes of [`ELEMENTS`] other than event attributes (`on*`):
/// %coreattrs and %i18n, plus the element attributes of those chapters.
// shortcut: one list for every element, not per element; upgrade if a wrong
// attribute-element pair must be reported (none of these is active content).
const ATTRIBUTES: &[&str] = &[
    "id",
    "class",
    "style",
    "title",
    "lang",
    "dir",
    "xml:lang",
    "xml:id",
    "xml:space",
    "cite",
    "type",
    "start",
    "value",
    "compact",
    "summary",
    "width",
    "height",
    "border",
    "frame",
    "rules",
    "cellspacing",
    "cellpadding",
    "align",
    "bgcolor",
    "span",
    "char",
    "charoff",
    "valign",
    "abbr",
    "axis",
    "headers",
    "scope",
    "rowspan",
    "colspan",
    "nowrap",
    "noshade",
    "size",
    "clear",
    "name",
    "href",
    "hreflang",
    "rel",
    "rev",
    "charset",
    "shape",
    "coords",
    "accesskey",
    "tabindex",
    "target",
    "src",
    "alt",
    "longdesc",
    "usemap",
    "ismap",
    "hspace",
    "vspace",
    "nohref",
];

/// Attributes whose value is a URL a browser may follow or load.
// shortcut: `style` values are not parsed (narrative.html allows internal styles and
// lists no forbidden property), so `url(...)` in a style passes; upgrade if the spec
// or the reference validator restricts style content.
const URL_ATTRIBUTES: &[&str] = &["href", "src", "cite", "longdesc", "usemap"];

/// Whether the xhtml markup (a narrative `div`, or a fragment as in the official
/// HTMLChecks tests) has only allowed elements and attributes, no active URL, and some
/// non-whitespace content. Malformed markup fails.
pub(super) fn html_checks(markup: &str) -> bool {
    let mut reader = NsReader::from_str(markup);
    let mut depth = 0usize;
    let mut has_content = false;
    loop {
        let (namespace, event) = match reader.read_resolved_event() {
            Ok(resolved) => resolved,
            Err(_) => return false,
        };
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                if !element_allowed(&namespace, e) {
                    return false;
                }
                has_content |= e.local_name().as_ref() == b"img";
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            Event::End(_) => {
                let Some(outer) = depth.checked_sub(1) else {
                    return false;
                };
                depth = outer;
            }
            Event::Text(ref t) => {
                let Ok(text) = t.decode() else { return false };
                has_content |= text.chars().any(|c| !c.is_whitespace());
            }
            // `&#32;` is whitespace; a named entity (`&amp;`) is content.
            Event::GeneralRef(ref r) => {
                has_content |= !matches!(r.resolve_char_ref(), Ok(Some(c)) if c.is_whitespace());
            }
            // An HTML parser ends a comment or a CDATA section at the first `>`, so markup
            // inside one would turn live once the narrative is rendered as text/html.
            Event::Comment(ref t) if has_markup(t) => return false,
            Event::CData(ref t) if has_markup(t) => return false,
            Event::CData(ref t) => has_content |= t.iter().any(|b| !b.is_ascii_whitespace()),
            Event::Comment(_) => {}
            // A declaration, a processing instruction (`xml-stylesheet`) or a DTD is not
            // narrative content.
            Event::Decl(_) | Event::PI(_) | Event::DocType(_) => return false,
            Event::Eof => return depth == 0 && has_content,
        }
    }
}

/// Whether raw comment or CDATA bytes hold `<` or `>`.
fn has_markup(bytes: &[u8]) -> bool {
    bytes.iter().any(|b| matches!(b, b'<' | b'>'))
}

/// Whether the element is an allowed XHTML element (a prefix bound to the XHTML
/// namespace, or none), each attribute is allowed, and no URL is active.
fn element_allowed(namespace: &ResolveResult<'_>, e: &BytesStart<'_>) -> bool {
    match namespace {
        ResolveResult::Bound(ns) if ns.as_ref() == XHTML_NS => {}
        ResolveResult::Unbound => {}
        _ => return false,
    }
    let local = e.local_name();
    let Ok(name) = std::str::from_utf8(local.as_ref()) else {
        return false;
    };
    if !ELEMENTS.contains(&name) {
        return false;
    }
    e.attributes().all(|attr| {
        let Ok(attr) = attr else { return false };
        let Ok(key) = std::str::from_utf8(attr.key.as_ref()) else {
            return false;
        };
        // Namespace declarations bind no content; `xlink:*` attributes are not allowed.
        if key == "xmlns" || key.starts_with("xmlns:") {
            return true;
        }
        if !ATTRIBUTES.contains(&key) {
            return false;
        }
        let Ok(value) = attr.normalized_value(XmlVersion::Implicit1_0) else {
            return false;
        };
        !URL_ATTRIBUTES.contains(&key) || !is_active_url(&value)
    })
}

/// Whether a URL runs script or carries a document: a `javascript:` or `vbscript:`
/// scheme, or a `data:` URL that is not an image. Browsers ignore ASCII whitespace and
/// controls in the scheme and match it in any case, so this does too.
fn is_active_url(url: &str) -> bool {
    let compact: String = url
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && !c.is_ascii_control())
        .take(16)
        .collect::<String>()
        .to_ascii_lowercase();
    compact.starts_with("javascript:")
        || compact.starts_with("vbscript:")
        || compact.starts_with("data:") && !compact.starts_with("data:image/")
}

#[cfg(test)]
mod tests {
    use super::{html_checks, is_active_url};

    fn div(inner: &str) -> String {
        format!("<div xmlns=\"http://www.w3.org/1999/xhtml\">{inner}</div>")
    }

    #[test]
    fn accepts_basic_formatting() {
        assert!(html_checks(&div("<p>a &amp; <b>b</b></p>")));
        assert!(html_checks(&div("<img src=\"#pic\" alt=\"\"/>")));
        assert!(html_checks(&div("<!-- note --><p>x</p>")));
    }

    #[test]
    fn rejects_active_or_unknown_content() {
        // An HTML parser ends these comment and CDATA sections at the first `>`.
        assert!(!html_checks(&div(
            "<!--><img src=x onerror=alert(1)>--><p>x</p>"
        )));
        assert!(!html_checks(&div(
            "<![CDATA[ > <img src=x onerror=alert(1)> ]]>"
        )));
        assert!(!html_checks(&div("<script>x</script>")));
        assert!(!html_checks(&div("<p onClick=\"x()\">x</p>")));
        assert!(!html_checks(&div("<a href=\"&#106;avascript:x\">x</a>")));
        assert!(!html_checks(&div("<p STYLE=\"x\">x</p>")));
        assert!(!html_checks(&div(
            "<?xml-stylesheet href=\"x.css\"?><p>x</p>"
        )));
    }

    #[test]
    fn accepts_fragments_and_prefixed_xhtml() {
        // The official HTMLChecks test htmlTest02 checks a fragment with no root div.
        assert!(html_checks("<b>This</b> <i>is</i> valid <code>HTML</code>"));
        assert!(html_checks(
            "<n:div xmlns:n=\"http://www.w3.org/1999/xhtml\"><n:p>x</n:p></n:div>"
        ));
    }

    #[test]
    fn rejects_bad_structure() {
        assert!(!html_checks("<b>This</b> is <button>HTML</button>"));
        assert!(!html_checks("<o:p xmlns:o=\"urn:other\">x</o:p>"));
        assert!(!html_checks("<n:p>unbound prefix</n:p>"));
        assert!(!html_checks("</p>x"));
        assert!(!html_checks(&format!(
            "<?xml version=\"1.0\"?>{}",
            div("a")
        )));
        assert!(!html_checks(&div("<p>unclosed")));
        assert!(!html_checks(&div("&#32;")));
        assert!(!html_checks(
            "<div xmlns=\"http://www.w3.org/1999/xhtml\"/>"
        ));
        assert!(!html_checks(""));
    }

    #[test]
    fn finds_active_urls() {
        assert!(is_active_url(" Java\tScript:alert(1)"));
        assert!(is_active_url("data:text/html,x"));
        assert!(!is_active_url("data:image/png;base64,AAAA"));
        assert!(!is_active_url("https://example.org/javascript:"));
        assert!(!is_active_url("#id"));
    }
}
