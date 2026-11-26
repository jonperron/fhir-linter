//! Report rendering: text, JSON and SARIF 2.1.0.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use fhir_parser::Span;
use fhir_validator::{Diagnostic, Severity};
use serde_json::{Value, json};

use crate::Format;

const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// A diagnostic together with the file it was found in.
pub struct Finding {
    /// Lossy display name, for the human-readable text and JSON reports.
    pub file: String,
    /// The exact path, so the SARIF URI keeps every byte of the name.
    pub path: PathBuf,
    pub diagnostic: Diagnostic,
    /// The location's column in UTF-16 code units, as SARIF expects (see [`Utf16Columns`]).
    pub utf16_col: Option<u32>,
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
/// control characters, so untrusted text (resource content, file names) cannot
/// inject terminal sequences.
pub fn terminal_safe(text: &str) -> String {
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
            let mut location = json!({ "artifactLocation": { "uri": file_uri(&finding.path) } });
            if let Some(span) = d.location {
                // SARIF regions are 1-based; never emit 0.
                location["region"] = json!({
                    "startLine": span.line.max(1),
                    "startColumn": finding.utf16_col.unwrap_or(span.col).max(1),
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
            "columnKind": "utf16CodeUnits",
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

/// Bytes per checkpoint in [`Utf16Columns`]: memory is 1/8 of the text, a
/// lookup scans at most this many bytes.
const CHECKPOINT_BYTES: usize = 64;

/// Converts a file's 1-based UTF-8 byte columns into the 1-based UTF-16
/// columns SARIF expects. Built once per file, so many findings on one long
/// line stay cheap: each lookup is a binary search plus a short scan.
pub struct Utf16Columns<'a> {
    text: &'a [u8],
    /// Byte offset where each line starts (lines end at `\n`, as in the
    /// parser). Empty for ASCII text, where bytes are units.
    line_starts: Vec<u32>,
    /// UTF-8 bytes minus UTF-16 units of the text before each multiple of
    /// `CHECKPOINT_BYTES`.
    checkpoints: Vec<usize>,
    /// Bytes of a leading BOM that line-1 spans count but `text` omits.
    bom_bytes: u32,
}

impl<'a> Utf16Columns<'a> {
    /// Index `text`. A leading BOM is never counted (editors hide it);
    /// `spans_count_bom` says whether the parser's line-1 columns include its
    /// bytes (the JSON parser does, the XML parser skips it).
    pub fn new(text: &'a str, spans_count_bom: bool) -> Self {
        let stripped = text.strip_prefix('\u{FEFF}');
        let bom_bytes = match stripped {
            Some(_) if spans_count_bom => 3,
            _ => 0,
        };
        let bytes = stripped.unwrap_or(text).as_bytes();
        if bytes.is_ascii() {
            return Utf16Columns {
                text: bytes,
                line_starts: Vec::new(),
                checkpoints: Vec::new(),
                bom_bytes,
            };
        }
        let line_starts = std::iter::once(0)
            .chain(
                bytes
                    .iter()
                    .enumerate()
                    .filter(|&(_, &byte)| byte == b'\n')
                    .map(|(offset, _)| u32::try_from(offset + 1).unwrap_or(u32::MAX)),
            )
            .collect();
        let checkpoints = std::iter::once(0)
            .chain(bytes.chunks(CHECKPOINT_BYTES).scan(0, |saved, chunk| {
                *saved += savings(chunk);
                Some(*saved)
            }))
            .collect();
        Utf16Columns {
            text: bytes,
            line_starts,
            checkpoints,
            bom_bytes,
        }
    }

    /// The UTF-16 column of `span`; past the end of its line, one unit per byte.
    pub fn column(&self, span: Span) -> u32 {
        let line = usize::try_from(span.line.max(1)).unwrap_or(usize::MAX);
        let col = match line {
            1 => span.col.saturating_sub(self.bom_bytes),
            _ => span.col,
        }
        .max(1);
        let Some(start) = self.line_start(line - 1) else {
            // ASCII text, or a line the text does not have: bytes are units.
            return col;
        };
        let line_end = self
            .line_start(line)
            .map_or(self.text.len(), |next| next - 1);
        let byte_col = usize::try_from(col - 1).unwrap_or(usize::MAX);
        let end = start.saturating_add(byte_col).min(line_end);
        let saved = self.saved_before(end) - self.saved_before(start);
        col.saturating_sub(u32::try_from(saved).unwrap_or(u32::MAX))
            .max(1)
    }

    fn line_start(&self, index: usize) -> Option<usize> {
        let start = *self.line_starts.get(index)?;
        usize::try_from(start).ok()
    }

    /// Bytes minus units of the chars that start before `offset`.
    fn saved_before(&self, offset: usize) -> usize {
        let block = offset / CHECKPOINT_BYTES;
        self.checkpoints[block] + savings(&self.text[block * CHECKPOINT_BYTES..offset])
    }
}

/// UTF-8 bytes minus UTF-16 units of the chars whose lead byte is in `bytes`:
/// a 2-byte char is 1 unit, a 3-byte char 1 unit, a 4-byte char 2 units.
fn savings(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .map(|&byte| match byte {
            0xC0..=0xDF => 1,
            0xE0..=0xFF => 2,
            _ => 0,
        })
        .sum()
}

/// SARIF wants a URI: relative paths stay relative, absolute ones become `file://`.
fn file_uri(path: &Path) -> String {
    // Unix names are bytes, not always UTF-8: encode those, not a lossy rendering.
    #[cfg(unix)]
    let slashed = Cow::Borrowed(std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str()));
    // `\` is a separator only on Windows; elsewhere it is a filename character.
    #[cfg(not(unix))]
    let slashed: Cow<[u8]> = {
        let file = path.to_string_lossy();
        let file = if cfg!(windows) {
            file.replace('\\', "/")
        } else {
            file.into_owned()
        };
        Cow::Owned(file.into_bytes())
    };
    let mut uri = String::new();
    let mut rest: &[u8] = &slashed;
    if path.is_absolute() {
        uri.push_str("file://");
        if !rest.starts_with(b"/") {
            // Windows drive path: `C:/x` -> `file:///C:/x`, drive colon kept as is.
            let (drive, tail) = rest.split_at(2);
            uri.push('/');
            uri.extend(drive.iter().copied().map(char::from));
            rest = tail;
        }
    }
    for &byte in rest {
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
        assert_eq!(file_uri(Path::new("dir/a b.json")), "dir/a%20b.json");
    }

    #[test]
    fn file_uri_encodes_colon_in_relative_paths() {
        assert_eq!(
            file_uri(Path::new("javascript:x.json")),
            "javascript%3Ax.json"
        );
    }

    #[cfg(unix)]
    #[test]
    fn file_uri_prefixes_absolute_paths() {
        assert_eq!(file_uri(Path::new("/tmp/x.json")), "file:///tmp/x.json");
    }

    #[cfg(unix)]
    #[test]
    fn file_uri_keeps_backslash_as_a_unix_filename_character() {
        assert_eq!(file_uri(Path::new("a\\b.json")), "a%5Cb.json");
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
            path: PathBuf::from("f.json"),
            diagnostic,
            utf16_col: None,
        }]);
        let region = &log["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 1);
        assert_eq!(region["startColumn"], 1);
    }

    fn column(text: &str, line: u32, col: u32) -> u32 {
        Utf16Columns::new(text, true).column(Span {
            line,
            col,
            offset: 0,
        })
    }

    #[test]
    fn utf16_columns_count_non_bmp_chars_as_two_units() {
        // U+1F600 is 4 bytes, 2 units: byte column 7 on line 2 is unit column 5.
        assert_eq!(column("ab\n\u{1F600}xyz", 2, 7), 5);
    }

    #[test]
    fn utf16_columns_cross_checkpoints() {
        // 63 ASCII bytes put U+1F600 across the first 64-byte checkpoint.
        let text = format!("{}\u{1F600}{}x", "a".repeat(63), "\u{E9}".repeat(100));
        assert_eq!(column(&text, 1, 63 + 4 + 200 + 1), 63 + 2 + 100 + 1);
        // A column at the end of a text exactly one checkpoint long.
        assert_eq!(column(&"\u{E9}".repeat(32), 1, 65), 33);
    }

    #[test]
    fn utf16_columns_skip_a_bom_whether_or_not_spans_count_it() {
        let span = |col| Span {
            line: 1,
            col,
            offset: 0,
        };
        // `x` is the second char after the BOM: unit column 2.
        assert_eq!(
            Utf16Columns::new("\u{FEFF}\u{E9}x", true).column(span(6)),
            2
        );
        assert_eq!(
            Utf16Columns::new("\u{FEFF}\u{E9}x", false).column(span(3)),
            2
        );
        assert_eq!(Utf16Columns::new("\u{FEFF}ax", true).column(span(5)), 2);
    }

    #[test]
    fn utf16_columns_ignore_other_lines() {
        assert_eq!(column("\u{E9}\u{E9}\nab\n\u{E9}\u{E9}x", 3, 5), 3);
        assert_eq!(column("\u{E9}\u{E9}\nab\n\u{E9}", 2, 2), 2);
    }

    #[test]
    fn utf16_columns_count_bytes_past_the_line_end() {
        assert_eq!(column("\u{E9}", 1, 5), 4);
        // Not the chars of the next line.
        assert_eq!(column("\u{E9}\n\u{1F600}", 1, 6), 5);
    }

    #[test]
    fn utf16_columns_keep_byte_columns_for_ascii_or_missing_lines() {
        assert_eq!(column("abc", 1, 3), 3);
        assert_eq!(column("\u{E9}", 4, 3), 3);
    }

    #[test]
    fn utf16_columns_are_never_zero() {
        assert_eq!(column("", 0, 0), 1);
        assert_eq!(column("\u{E9}", 0, 0), 1);
    }

    #[cfg(unix)]
    #[test]
    fn file_uri_percent_encodes_non_utf8_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let path = Path::new(std::ffi::OsStr::from_bytes(b"/d/\xFF.json"));
        assert_eq!(file_uri(path), "file:///d/%FF.json");
    }
}
