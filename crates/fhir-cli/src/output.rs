//! Report rendering: text, JSON and SARIF 2.1.0.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use fhir_validator::{Diagnostic, Severity};
use serde_json::{Value, json};

use crate::Format;

const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// A diagnostic together with the file it was found in.
pub struct Finding {
    pub file: String,
    pub diagnostic: Diagnostic,
}

pub fn render(findings: &[Finding], format: Format) -> String {
    match format {
        Format::Text => text(findings),
        Format::Json => json_report(findings).to_string(),
        Format::Sarif => sarif(findings).to_string(),
    }
}

/// One line per finding: `file:line:col: severity CODE path: message`.
fn text(findings: &[Finding]) -> String {
    let lines: Vec<String> = findings
        .iter()
        .map(|finding| {
            let d = &finding.diagnostic;
            let mut line = terminal_safe(&finding.file);
            if let Some(span) = d.location {
                let _ = write!(line, ":{}:{}", span.line, span.col);
            }
            let _ = write!(line, ": {} {}", d.severity.as_str(), d.code);
            if !d.path.is_empty() {
                let _ = write!(line, " {}", terminal_safe(&d.path));
            }
            let _ = write!(line, ": {}", terminal_safe(&d.message));
            line
        })
        .collect();
    lines.join("\n")
}

/// Fold multi-line text (e.g. FHIRPath parse errors) onto one line and escape
/// control characters, so resource content cannot inject terminal sequences.
fn terminal_safe(text: &str) -> String {
    let folded: Vec<&str> = text.lines().map(str::trim).collect();
    folded
        .join(" ")
        .chars()
        .map(|c| {
            // Bidi controls are escaped too: they can reorder what the terminal shows.
            let bidi = matches!(
                c,
                '\u{61C}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2066}'..='\u{2069}'
                    | '\u{206A}'..='\u{206F}'
            );
            if c.is_control() || bidi {
                c.escape_debug().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

fn json_report(findings: &[Finding]) -> Value {
    findings
        .iter()
        .map(|finding| {
            let d = &finding.diagnostic;
            json!({
                "file": finding.file,
                "severity": d.severity.as_str(),
                "code": d.code.as_ref(),
                "message": d.message,
                "path": d.path,
                "line": d.location.map(|s| s.line),
                "col": d.location.map(|s| s.col),
                "rule": d.rule.as_deref(),
            })
        })
        .collect()
}

fn sarif(findings: &[Finding]) -> Value {
    let rules: BTreeSet<&str> = findings
        .iter()
        .map(|f| f.diagnostic.code.as_ref())
        .collect();
    let rules: Vec<Value> = rules.into_iter().map(|id| json!({ "id": id })).collect();
    let results: Vec<Value> = findings
        .iter()
        .map(|finding| {
            let d = &finding.diagnostic;
            let mut location = json!({ "artifactLocation": { "uri": file_uri(&finding.file) } });
            if let Some(span) = d.location {
                // SARIF regions are 1-based; never emit 0.
                location["region"] = json!({
                    "startLine": span.line.max(1),
                    "startColumn": span.col.max(1),
                });
            }
            json!({
                "ruleId": d.code.as_ref(),
                "level": sarif_level(d.severity),
                "message": { "text": d.message },
                "locations": [{ "physicalLocation": location }],
            })
        })
        .collect();
    json!({
        "$schema": SARIF_SCHEMA,
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "fhir-lint",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules,
                },
            },
            "results": results,
        }],
    })
}

fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Fatal | Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Information => "note",
    }
}

/// SARIF wants a URI: relative paths stay relative, absolute ones become `file://`.
fn file_uri(file: &str) -> String {
    // `\` is a separator only on Windows; elsewhere it is a filename character.
    let slashed = if cfg!(windows) {
        file.replace('\\', "/")
    } else {
        file.to_owned()
    };
    let mut uri = String::new();
    let mut rest = slashed.as_str();
    if Path::new(file).is_absolute() {
        uri.push_str("file://");
        if !rest.starts_with('/') {
            // Windows drive path: `C:/x` -> `file:///C:/x`, drive colon kept as is.
            let (drive, tail) = rest.split_at(2);
            uri.push('/');
            uri.push_str(drive);
            rest = tail;
        }
    }
    for byte in rest.bytes() {
        // `:` is encoded so a relative `a:b.json` cannot read as a URI scheme.
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uri_keeps_relative_paths_relative() {
        assert_eq!(file_uri("dir/a b.json"), "dir/a%20b.json");
    }

    #[test]
    fn file_uri_encodes_colon_in_relative_paths() {
        assert_eq!(file_uri("javascript:x.json"), "javascript%3Ax.json");
    }

    #[cfg(unix)]
    #[test]
    fn file_uri_prefixes_absolute_paths() {
        assert_eq!(file_uri("/tmp/x.json"), "file:///tmp/x.json");
    }

    #[cfg(unix)]
    #[test]
    fn file_uri_keeps_backslash_as_a_unix_filename_character() {
        assert_eq!(file_uri("a\\b.json"), "a%5Cb.json");
    }

    #[test]
    fn severities_map_to_sarif_levels() {
        assert_eq!(sarif_level(Severity::Fatal), "error");
        assert_eq!(sarif_level(Severity::Error), "error");
        assert_eq!(sarif_level(Severity::Warning), "warning");
        assert_eq!(sarif_level(Severity::Information), "note");
    }

    #[test]
    fn terminal_safe_escapes_control_characters() {
        assert_eq!(terminal_safe("a\u{1b}[31mb\rc"), "a\\u{1b}[31mb\\rc");
    }

    #[test]
    fn terminal_safe_escapes_bidi_controls() {
        assert_eq!(terminal_safe("a\u{202E}b"), "a\\u{202e}b");
    }

    #[test]
    fn terminal_safe_escapes_arabic_letter_mark_and_deprecated_bidi_controls() {
        assert_eq!(
            terminal_safe("a\u{61C}b\u{206A}c\u{206F}"),
            "a\\u{61c}b\\u{206a}c\\u{206f}"
        );
    }

    #[test]
    fn terminal_safe_folds_lines() {
        assert_eq!(terminal_safe("one\n  two\r\nthree"), "one two three");
    }

    #[test]
    fn sarif_region_is_never_zero() {
        let diagnostic = Diagnostic::error("X", "m", "").with_location(fhir_parser::Span {
            line: 0,
            col: 0,
            offset: 0,
        });
        let log = sarif(&[Finding {
            file: "f.json".to_owned(),
            diagnostic,
        }]);
        let region = &log["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 1);
        assert_eq!(region["startColumn"], 1);
    }
}
