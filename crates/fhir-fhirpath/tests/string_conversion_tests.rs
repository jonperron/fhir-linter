/// FHIRPath N1 string and conversion function edge cases (PLAN §11.3 F8, F9).
///
/// Expressions come from `tests-fhir-r5.xml` where a case exists.
use std::sync::Arc;

use fhir_fhirpath::{Value, evaluate};

fn eval(expr: &str) -> Vec<Value> {
    evaluate(expr, &[]).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

fn string(s: &str) -> Vec<Value> {
    vec![Value::String(Arc::from(s))]
}

// ── F8: string functions ─────────────────────────────────────────────────────

// N1 §5.6: an empty input or an empty argument gives an empty result.
#[test]
fn string_functions_on_empty_input_or_argument_yield_empty() {
    let cases = [
        "{}.startsWith('1')", // testStartsWith8
        "{}.startsWith('')",  // testStartsWith9
        "'abc'.startsWith({})",
        "{}.endsWith('')", // testEndsWith9
        "'abc'.endsWith({})",
        "{}.contains('')", // testContainsString9
        "'abc'.contains({})",
        "{}.indexOf('-')",                   // testIndexOf4
        "'LogicalModel-Person'.indexOf({})", // testIndexOf5
        "{}.indexOf({})",                    // testIndexOf6
        "{}.lastIndexOf('-')",
        "'a-b'.lastIndexOf({})",
        "{}.matches('FHIR')", // testMatchesEmpty2
        "'FHIR'.matches({})", // testMatchesEmpty
        "{}.matches({})",     // testMatchesEmpty3
        "{}.matchesFull('.*')",
        "'FHIR'.matchesFull({})",
        "'123'.replace({}, 'X')",        // testReplace5
        "'123'.replace('2', {})",        // testReplace6
        "'123'.replaceMatches({}, 'X')", // testReplaceMatches5
        "'123'.replaceMatches('2', {})", // testReplaceMatches6
        "'string'.substring({})",        // testSubstring12
        "{}.substring({})",
        "{}.join(',')", // testJoinEmpty
    ];
    for expr in cases {
        assert_eq!(eval(expr), vec![], "{expr}");
    }
}

// N1 §5.6.3: start outside the string gives empty, not ''.
#[test]
fn substring_start_outside_string_yields_empty() {
    assert_eq!(eval("'12345'.substring(25)"), vec![]); // testSubstring4
    assert_eq!(eval("'12345'.substring(5)"), vec![]);
    assert_eq!(eval("'12345'.substring(5, 1)"), vec![]);
    assert_eq!(eval("'12345'.substring(4)"), string("5"));
}

// N1 §5.6.3: an empty length behaves as if no length was given.
#[test]
fn substring_empty_length_takes_rest_of_string() {
    assert_eq!(eval("'abcdefg'.substring(3, {})"), string("defg")); // testSubstring17
    assert_eq!(eval("'abcdefg'.substring(0, {})"), string("abcdefg")); // testSubstring18
}

#[test]
fn string_functions_keep_results_on_non_empty_input() {
    assert_eq!(eval("'abc'.startsWith('')"), vec![Value::Bool(true)]);
    assert_eq!(eval("'abc'.indexOf('')"), vec![Value::Integer(0)]);
    assert_eq!(eval("'a,b'.join(',')"), string("a,b"));
    assert_eq!(eval("'abc'.matches('b')"), vec![Value::Bool(true)]);
    assert_eq!(eval("'abc'.replace('b', '')"), string("ac"));
}

#[test]
fn string_functions_reject_non_string_arguments() {
    for expr in [
        "'abc'.startsWith(1)",
        "'abc'.indexOf(1)",
        "'abc'.replace(1, 'x')",
    ] {
        assert!(evaluate(expr, &[]).is_err(), "{expr}");
    }
}

// ── F9: conversion functions ─────────────────────────────────────────────────

fn boolean(b: bool) -> Vec<Value> {
    vec![Value::Bool(b)]
}

// N1 §5.5.2: toBoolean accepts these strings, case-insensitively, and
// Integer or Decimal 1 and 0.
#[test]
fn to_boolean_follows_n1_table() {
    for expr in [
        "'true'", "'t'", "'yes'", "'y'", "'1'", "'1.0'", "'TRUE'", "'Y'", "1", "1.0",
    ] {
        assert_eq!(
            eval(&format!("{expr}.toBoolean()")),
            boolean(true),
            "{expr}"
        );
        assert_eq!(
            eval(&format!("{expr}.convertsToBoolean()")),
            boolean(true),
            "{expr}"
        );
    }
    for expr in [
        "'false'", "'f'", "'no'", "'n'", "'0'", "'0.0'", "'False'", "0", "0.0",
    ] {
        assert_eq!(
            eval(&format!("{expr}.toBoolean()")),
            boolean(false),
            "{expr}"
        );
        assert_eq!(
            eval(&format!("{expr}.convertsToBoolean()")),
            boolean(true),
            "{expr}"
        );
    }
}

// N1: a value that is not convertible gives empty, never an error.
#[test]
fn to_boolean_on_unconvertible_value_yields_empty() {
    // "2" is testIntegerLiteralToBooleanEmpty.
    for expr in ["2", "(-1)", "1.5", "'maybe'", "@2015-02-04"] {
        assert_eq!(eval(&format!("{expr}.toBoolean()")), vec![], "{expr}");
        assert_eq!(
            eval(&format!("{expr}.convertsToBoolean()")),
            boolean(false),
            "{expr}"
        );
    }
}

// N1 §5.5.4: a String converts to Decimal only in the form (\+|-)?\d+(\.\d+)?.
#[test]
fn to_decimal_accepts_only_the_n1_format() {
    assert_eq!(eval("'+1.5'.toDecimal()"), vec![Value::Decimal(1.5)]);
    assert_eq!(eval("'-2'.toDecimal()"), vec![Value::Decimal(-2.0)]);
    for text in [
        "NaN",
        "inf",
        "-infinity",
        "1e5",
        "1.",
        ".5",
        "1.a",
        " 1",
        "",
    ] {
        let expr = format!("'{text}'.toDecimal()");
        assert_eq!(eval(&expr), vec![], "{expr}");
        let expr = format!("'{text}'.convertsToDecimal()");
        assert_eq!(eval(&expr), boolean(false), "{expr}");
    }
    assert_eq!(eval("@2015.toDecimal()"), vec![]);
}

// N1 §5.5.3: a Decimal is not convertible to Integer: empty, not an error.
#[test]
fn to_integer_on_decimal_yields_empty() {
    assert_eq!(eval("1.5.toInteger()"), vec![]);
    assert_eq!(eval("1.0.convertsToInteger()"), boolean(false));
    assert_eq!(eval("'+3'.toInteger()"), vec![Value::Integer(3)]);
    assert_eq!(eval("' 3'.toInteger()"), vec![]);
}

// A String that is not a real calendar date is not convertible (year
// 0001-9999, day valid for its month, leap years included).
#[test]
fn date_conversions_reject_impossible_calendar_dates() {
    for text in [
        "0000",
        "2015-02-29",
        "2015-02-30",
        "2015-04-31",
        "2015-02-29T10:00",
    ] {
        assert_eq!(eval(&format!("'{text}'.toDateTime()")), vec![], "{text}");
        assert_eq!(
            eval(&format!("'{text}'.convertsToDateTime()")),
            boolean(false),
            "{text}"
        );
    }
    for text in ["0000", "2015-02-29", "2100-02-29"] {
        assert_eq!(eval(&format!("'{text}'.toDate()")), vec![], "{text}");
    }
    for text in ["2016-02-29", "2000-02-29", "0001", "2015-01-31"] {
        assert_eq!(
            eval(&format!("'{text}'.convertsToDate()")),
            boolean(true),
            "{text}"
        );
    }
    // A time after a partial date is not read as month or day.
    for text in [
        "2015T11:31",
        "2015T00:45",
        "2015-02T23:59",
        "2016-02-29T10:00",
    ] {
        assert_eq!(
            eval(&format!("'{text}'.convertsToDateTime()")),
            boolean(true),
            "{text}"
        );
    }
}

// N1 §5.5.6: Strings in the Date format convert, partial dates included.
#[test]
fn to_date_converts_strings_and_datetimes() {
    let date = |s: &str| vec![Value::Date(Arc::from(s))];
    assert_eq!(eval("'2015'.toDate()"), date("2015"));
    assert_eq!(eval("'2015-02'.toDate()"), date("2015-02"));
    assert_eq!(eval("'2015-02-04'.toDate()"), date("2015-02-04"));
    assert_eq!(eval("@2015-02-04T14:34:28Z.toDate()"), date("2015-02-04"));
    assert_eq!(eval("'2015-02-04'.convertsToDate()"), boolean(true)); // testStringDayConvertsToDate
    for text in [
        "15",
        "2015-2",
        "2015-13",
        "2015-02-32",
        "2015-02-04T14",
        "x",
    ] {
        let expr = format!("'{text}'.toDate()");
        assert_eq!(eval(&expr), vec![], "{expr}");
        let expr = format!("'{text}'.convertsToDate()");
        assert_eq!(eval(&expr), boolean(false), "{expr}");
    }
}

// N1 §5.5.7: Strings in the DateTime format convert, partial values included.
#[test]
fn to_datetime_converts_strings_and_dates() {
    let datetime = |s: &str| vec![Value::DateTime(Arc::from(s))];
    for text in [
        "2015",
        "2015-02-04",
        "2015-02-04T14",
        "2015-02-04T14:34",
        "2015-02-04T14:34:28.123",
        "2015-02-04T14:34:28Z",
        "2015-02-04T14:34:28+10:00",
    ] {
        assert_eq!(
            eval(&format!("'{text}'.toDateTime()")),
            datetime(text),
            "{text}"
        );
        assert_eq!(
            eval(&format!("'{text}'.convertsToDateTime()")),
            boolean(true),
            "{text}"
        );
    }
    // testLiteralUTCToDateTime
    assert_eq!(
        eval("'2015-02-04T14:34:28Z'.toDateTime() = @2015-02-04T14:34:28Z"),
        boolean(true)
    );
    assert_eq!(eval("@2015-02-04.toDateTime()"), datetime("2015-02-04"));
    for text in ["2015-02-04T25", "2015-02-04T14:60", "2015Z", "x"] {
        assert_eq!(eval(&format!("'{text}'.toDateTime()")), vec![], "{text}");
    }
}

// N1 §5.5.9: Strings in the Time format convert, partial times included.
#[test]
fn to_time_converts_strings() {
    let time = |s: &str| vec![Value::Time(Arc::from(s))];
    for text in ["14", "14:34", "14:34:28", "14:34:28.123"] {
        assert_eq!(eval(&format!("'{text}'.toTime()")), time(text), "{text}");
        assert_eq!(
            eval(&format!("'{text}'.convertsToTime()")),
            boolean(true),
            "{text}"
        );
    }
    for text in ["24", "14:3", "14:34:28Z", "T14", "x"] {
        assert_eq!(eval(&format!("'{text}'.toTime()")), vec![], "{text}");
    }
}

// N1 §5.5: conversions on more than one item are an error; on none, empty.
#[test]
fn conversions_on_empty_or_multiple_items() {
    for function in [
        "toBoolean",
        "toInteger",
        "toDecimal",
        "toDate",
        "toDateTime",
        "toTime",
        "convertsToBoolean",
        "convertsToInteger",
        "convertsToDecimal",
        "convertsToDate",
        "convertsToDateTime",
        "convertsToTime",
    ] {
        let expr = format!("('1' | '0').{function}()");
        assert!(evaluate(&expr, &[]).is_err(), "{expr}");
        // N1 §5.5: "If the input collection is empty, the result is empty."
        let expr = format!("{{}}.{function}()");
        assert_eq!(eval(&expr), vec![], "{expr}");
    }
}

// A lone JSON null placeholder is empty for singleton evaluation (N1 §4.5).
#[test]
fn conversions_on_lone_null_yield_empty() {
    for expr in ["convertsToInteger()", "toInteger()", "convertsToDate()"] {
        let result = evaluate(expr, &[Value::Null]).unwrap_or_else(|e| panic!("{expr}: {e}"));
        assert_eq!(result, vec![], "{expr}");
    }
}

// N1 time zones go up to +14:00, as in the FHIR dateTime format.
#[test]
fn to_datetime_bounds_the_time_zone_offset() {
    for text in ["2015-02-04T14:34+14:00", "2015-02-04T14:34-13:59"] {
        assert_eq!(
            eval(&format!("'{text}'.convertsToDateTime()")),
            boolean(true),
            "{text}"
        );
    }
    for text in ["2015-02-04T14:34+14:01", "2015-02-04T14:34-15:00"] {
        assert_eq!(
            eval(&format!("'{text}'.convertsToDateTime()")),
            boolean(false),
            "{text}"
        );
    }
}
