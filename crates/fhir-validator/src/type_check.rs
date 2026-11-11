use fhir_parser::{Span, Value};

use crate::diagnostic::Diagnostic;

/// Check one value against a primitive FHIR type, recursing into arrays.
///
/// Emits `TYPE_001` through `TYPE_005` for format and type mismatches, and the
/// `TYPE_006` warning for a control character in a string.
pub(crate) fn check_type(
    value: &Value,
    fhir_type: &str,
    path: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Recurse into arrays.
    if let Value::Array(items) = value {
        for (i, item) in items.iter().enumerate() {
            let item_path = format!("{path}[{i}]");
            check_type(&item.value, fhir_type, &item_path, item.span, diagnostics);
        }
        return;
    }

    let expected = |what: &str| ("TYPE_001", format!("Expected {what} at '{path}'"));
    let string_format = |is_valid: fn(&str) -> bool| match value {
        Value::Str(s) if is_valid(s) => None,
        Value::Str(s) => Some((
            "TYPE_003",
            format!("Invalid {fhir_type} format at '{path}': '{s}'"),
        )),
        _ => Some(expected(&format!("{fhir_type} string"))),
    };

    let error: Option<(&str, String)> = match fhir_type {
        "boolean" if !matches!(value, Value::Bool(_)) => Some(expected("boolean")),
        // R5 integer, unsignedInt and positiveInt are 32-bit (`maxValueInteger` 2147483647).
        "integer" | "unsignedInt" | "positiveInt" => match value {
            Value::Integer(n) => {
                let (min, kind) = match fhir_type {
                    "unsignedInt" => (0, "non-negative "),
                    "positiveInt" => (1, "positive "),
                    _ => (i64::from(i32::MIN), ""),
                };
                (!(min..=i64::from(i32::MAX)).contains(n)).then(|| {
                    (
                        "TYPE_002",
                        format!("Expected {kind}32-bit integer at '{path}', got {n}"),
                    )
                })
            }
            _ => Some(expected("integer")),
        },
        "decimal" if !matches!(value, Value::Decimal(_) | Value::Integer(_)) => {
            Some(expected("decimal number"))
        }
        "date" => string_format(is_valid_date),
        "dateTime" => string_format(is_valid_datetime),
        "instant" => string_format(is_valid_instant),
        "time" => string_format(is_valid_time),
        // Official regex `[0]|[-+]?[1-9][0-9]*`, within i64 range.
        "integer64" => string_format(|s| {
            s.parse::<i64>().is_ok()
                && (s == "0" || !s.trim_start_matches(['+', '-']).starts_with('0'))
        }),
        "code" => match value {
            Value::Str(s) if !is_valid_code(s) => Some((
                "TYPE_004",
                format!("Code at '{path}' must be words separated by single spaces: '{s}'"),
            )),
            Value::Str(_) => None,
            _ => Some(expected("code string")),
        },
        "id" => match value {
            Value::Str(s) if !is_valid_id(s) => {
                Some(("TYPE_005", format!("Invalid id format at '{path}': '{s}'")))
            }
            Value::Str(_) => None,
            _ => Some(expected("id string")),
        },
        "string" | "markdown" | "base64Binary" | "xhtml" | "uri" | "url" | "canonical" | "oid"
        | "uuid"
            if !matches!(value, Value::Str(_)) =>
        {
            Some(expected("string"))
        }
        // Complex and unknown types — skip deep validation here.
        _ => None,
    };

    if let Some((code, message)) = error {
        diagnostics.push(Diagnostic::error(code, message, path.to_owned()).with_location(span));
    }

    // R5 datatypes.html: `string` (and its specialisations) SHOULD NOT hold controls
    // below U+0020 other than tab, CR and LF. XML rejects them at parse time.
    if let ("string" | "markdown" | "code" | "id", Value::Str(s)) = (fhir_type, value) {
        if s.chars().any(fhir_parser::is_disallowed_control) {
            diagnostics.push(
                Diagnostic::warning(
                    "TYPE_006",
                    format!("String at '{path}' contains a control character below U+0020"),
                    path.to_owned(),
                )
                .with_location(span),
            );
        }
    }
}

/// FHIR date: `YYYY`, `YYYY-MM`, or `YYYY-MM-DD` (year 0001-9999, day valid for its month).
fn is_valid_date(s: &str) -> bool {
    parse_date(s).is_some_and(|(_, rest)| rest.is_empty())
}

/// FHIR code: R5 regex `[^\s]+( [^\s]+)*`.
fn is_valid_code(s: &str) -> bool {
    s.split(' ')
        .all(|word| !word.is_empty() && !word.contains(char::is_whitespace))
}

/// FHIR dateTime: a date, a full date with a time and a mandatory zone (as `instant`), or,
/// as the R5 regex allows, a year-month or full date followed by a zone (`2020-01-01Z`).
/// The zone must be complete: the regex's bare `+`/`-` is rejected, as for `instant`.
fn is_valid_datetime(s: &str) -> bool {
    is_valid_date(s)
        || is_valid_instant(s)
        // `YYYY-MM` or `YYYY-MM-DD` then a zone; the regex takes no zone on a bare year.
        || [7, 10].into_iter().any(|len| {
            s.get(..len).is_some_and(is_valid_date) && s.get(len..).is_some_and(is_valid_zone)
        })
}

/// FHIR instant: `YYYY-MM-DDThh:mm:ss(.sss)?(Z|(+|-)hh:mm)`.
fn is_valid_instant(s: &str) -> bool {
    matches!(parse_date(s), Some((true, rest)) if rest
        .strip_prefix('T')
        .and_then(parse_time)
        .is_some_and(is_valid_zone))
}

/// FHIR time: `hh:mm:ss(.f{1,9})?`, no zone.
fn is_valid_time(s: &str) -> bool {
    parse_time(s) == Some("")
}

/// Reads a leading `YYYY(-MM(-DD)?)?`; returns whether the day was present and the rest.
fn parse_date(s: &str) -> Option<(bool, &str)> {
    let (year, rest) = digits(s, 4)?;
    if year == 0 {
        return None;
    }
    let Some(rest) = rest.strip_prefix('-') else {
        return Some((false, rest));
    };
    let (month, rest) = digits(rest, 2)?;
    if !(1..=12).contains(&month) {
        return None;
    }
    let Some(rest) = rest.strip_prefix('-') else {
        return Some((false, rest));
    };
    let (day, rest) = digits(rest, 2)?;
    (1..=days_in_month(year, month))
        .contains(&day)
        .then_some((true, rest))
}

/// Reads a leading `hh:mm:ss(.f{1,9})?` (second 60 allowed, as in the R5 regex); returns the rest.
fn parse_time(s: &str) -> Option<&str> {
    let (hour, rest) = digits(s, 2)?;
    let (minute, rest) = digits(rest.strip_prefix(':')?, 2)?;
    let (second, rest) = digits(rest.strip_prefix(':')?, 2)?;
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    match rest.strip_prefix('.') {
        None => Some(rest),
        Some(fraction) => {
            let len = fraction.bytes().take_while(u8::is_ascii_digit).count();
            (1..=9).contains(&len).then(|| &fraction[len..])
        }
    }
}

/// `Z` or `(+|-)hh:mm` with an offset between 00:00 and 14:00.
fn is_valid_zone(s: &str) -> bool {
    let Some(offset) = s.strip_prefix(['+', '-']) else {
        return s == "Z";
    };
    let Some((hour, rest)) = digits(offset, 2) else {
        return false;
    };
    let Some((minute, "")) = rest.strip_prefix(':').and_then(|r| digits(r, 2)) else {
        return false;
    };
    (hour <= 13 && minute <= 59) || (hour, minute) == (14, 0)
}

/// Reads exactly `count` leading ASCII digits; `None` on anything else (non-ASCII included).
fn digits(s: &str, count: usize) -> Option<(u32, &str)> {
    let head = s.get(..count)?;
    if !head.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((head.parse().ok()?, &s[count..]))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// FHIR id: `[A-Za-z0-9\-\.]{1,64}`.
pub(crate) fn is_valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use fhir_parser::{Node, Span};

    fn check(value: Value, fhir_type: &str) -> Vec<Diagnostic> {
        let mut d = vec![];
        check_type(&value, fhir_type, "X.f", Span::default(), &mut d);
        d
    }

    #[test]
    fn valid_boolean_produces_no_error() {
        let d = check(Value::Bool(true), "boolean");
        assert!(d.is_empty());
    }

    #[test]
    fn string_for_boolean_produces_type_001() {
        let d = check(Value::Str(Arc::from("true")), "boolean");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_001");
    }

    #[test]
    fn valid_date_produces_no_error() {
        for s in &["1990", "1990-01", "1990-01-15"] {
            let d = check(Value::Str(Arc::from(*s)), "date");
            assert!(d.is_empty(), "Expected no error for date '{s}'");
        }
    }

    #[test]
    fn invalid_date_produces_type_003() {
        let d = check(Value::Str(Arc::from("not-a-date")), "date");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_003");
    }

    #[test]
    fn integer_beyond_i64_produces_type_001() {
        for type_code in ["integer", "unsignedInt", "positiveInt"] {
            let source = r#"{"resourceType": "Obs", "n": 12345678901234567890}"#;
            let resource = fhir_parser::parse_json(source).unwrap();
            let d = check(resource.fields["n"].value.clone(), type_code);
            assert_eq!(d.len(), 1, "{type_code}: {d:?}");
            assert_eq!(d[0].code.as_ref(), "TYPE_001");
        }
    }

    #[test]
    fn integer64_requires_a_digit_string() {
        for (value, expected) in [
            (Value::Str(Arc::from("-123")), None),
            (Value::Str(Arc::from("12x")), Some("TYPE_003")),
            (Value::Str(Arc::from("-")), Some("TYPE_003")),
            (Value::Str(Arc::from("+1")), None),
            (Value::Str(Arc::from("0")), None),
            (Value::Str(Arc::from("01")), Some("TYPE_003")),
            (Value::Str(Arc::from("-0")), Some("TYPE_003")),
            (
                Value::Str(Arc::from("99999999999999999999")),
                Some("TYPE_003"),
            ),
            (Value::Integer(5), Some("TYPE_001")),
        ] {
            let d = check(value, "integer64");
            assert_eq!(d.first().map(|x| x.code.as_ref()), expected, "{d:?}");
        }
    }

    #[test]
    fn code_follows_the_r5_regex() {
        for (code, valid) in [
            ("text/plain; charset=utf-8", true),
            ("a b c", true),
            ("a  b", false),
            (" a", false),
            ("a ", false),
            ("a\tb", false),
            ("a\nb", false),
            ("a\u{a0}b", false),
            ("", false),
        ] {
            let d = check(Value::Str(Arc::from(code)), "code");
            assert_eq!(d.is_empty(), valid, "{code:?}: {d:?}");
        }
    }

    #[test]
    fn valid_code_produces_no_error() {
        let d = check(Value::Str(Arc::from("final")), "code");
        assert!(d.is_empty());
    }

    #[test]
    fn negative_unsigned_int_produces_type_002() {
        let d = check(Value::Integer(-1), "unsignedInt");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_002");
    }

    #[test]
    fn zero_positive_int_produces_type_002() {
        let d = check(Value::Integer(0), "positiveInt");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_002");
    }

    #[test]
    fn valid_id_produces_no_error() {
        for s in &["abc", "ABC-123", "a.b.c", "x-1"] {
            let d = check(Value::Str(Arc::from(*s)), "id");
            assert!(d.is_empty(), "Expected no error for id '{s}'");
        }
    }

    #[test]
    fn id_with_invalid_chars_produces_type_005() {
        let d = check(Value::Str(Arc::from("hello world")), "id");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_005");
    }

    #[test]
    fn array_elements_are_each_checked() {
        let items = vec![
            Node {
                value: Value::Str(Arc::from("1990-01-01")),
                span: Span::default(),
            },
            Node {
                value: Value::Str(Arc::from("not-a-date")),
                span: Span::default(),
            },
        ];
        let d = check(Value::Array(items), "date");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code.as_ref(), "TYPE_003");
    }

    #[test]
    fn non_ascii_date_and_time_do_not_panic() {
        for s in [
            "abcé-0",
            "２０２４-01-01",
            "é024-01-01",
            "é:00:00",
            "2024-01-01Té:00:00",
        ] {
            assert!(!is_valid_date(s));
            assert!(!is_valid_datetime(s));
            assert!(!is_valid_time(s));
        }
    }

    #[test]
    fn valid_time_and_datetime() {
        assert!(is_valid_time("12:30:59"));
        assert!(!is_valid_time("24:00:00"));
        assert!(is_valid_datetime("2024-01-01T12:30:59Z"));
        assert!(!is_valid_datetime("2024-01-01T+1:30:59Z"));
        for valid in [
            "2020-01Z",
            "2020-01-01Z",
            "2020-01-01+02:00",
            "2020-01-05:00",
        ] {
            assert!(is_valid_datetime(valid), "{valid}");
        }
        for invalid in [
            "2020Z",
            "2020-01-01+",
            "2020-01-01Zjunk",
            "2020-01-01T10:00:00",
        ] {
            assert!(!is_valid_datetime(invalid), "{invalid}");
        }
    }
}
