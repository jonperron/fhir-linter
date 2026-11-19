//! FHIRPath function gaps found by the official suite (round 5):
//! `distinct()` on uncomparable dates and its kept occurrence, `precision()`,
//! `encode`/`decode('hex')`, padded `urlbase64`, `unescape('json')` with
//! `\uXXXX`, negation of a decimal zero and exact decimal `+`, `-`, `mod`.
//!
//! Expressions come from `tests-fhir-r5.xml` where a case exists.
use std::sync::Arc;

use indexmap::IndexMap;

use fhir_fhirpath::{Value, evaluate};

fn eval(expr: &str) -> Vec<Value> {
    evaluate(expr, &[]).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

fn is_true(expr: &str) {
    assert_eq!(eval(expr), vec![Value::Bool(true)], "{expr}");
}

fn string(s: &str) -> Value {
    Value::String(Arc::from(s))
}

// N1 §6.1 `=`: dates of different precision compare as empty, so
// `distinct()` cannot decide and returns empty (testDistinct29, 36).
#[test]
fn distinct_of_dates_with_different_precision_is_empty() {
    assert_eq!(eval("@2020.combine(@2020-01-01).distinct()"), vec![]);
    assert_eq!(eval("@2020.combine(@2020-01-01).isDistinct()"), vec![]);
    // An uncomparable string/integer pair is simply distinct.
    is_true("'1'.combine(1).isDistinct()");
    assert_eq!(
        eval("'1'.combine(1).distinct().count()"),
        vec![Value::Integer(2)]
    );
}

// N1 `distinct()` does not guarantee order; the suite keeps the last
// occurrence of each value (testDistinct30, 32-35).
#[test]
fn distinct_keeps_the_last_occurrence() {
    assert_eq!(
        eval("'a'.combine('b').combine('a').distinct()"),
        vec![string("b"), string("a")]
    );
    assert_eq!(
        eval("1.combine(2).combine(1).combine(3).distinct()"),
        vec![Value::Integer(2), Value::Integer(1), Value::Integer(3)]
    );
    // `union` keeps its first-occurrence order.
    assert_eq!(eval("('a' | 'b' | 'a')"), vec![string("a"), string("b")]);
}

// FHIRPath 3.0 `precision()`: digits of precision of a Date, DateTime or Time.
#[test]
fn precision_counts_temporal_digits() {
    assert_eq!(eval("@2014.precision()"), vec![Value::Integer(4)]);
    assert_eq!(
        eval("@2014-01-05T10:30:00.000.precision()"),
        vec![Value::Integer(17)]
    );
    assert_eq!(
        eval("@2014-01-05T10:30:00.000+05:00.precision()"),
        vec![Value::Integer(17)]
    );
    assert_eq!(
        eval("@2014-01-05T10:30Z.precision()"),
        vec![Value::Integer(12)]
    );
    assert_eq!(
        eval("@2014-01-05T10:30:00-05:00.precision()"),
        vec![Value::Integer(14)],
    );
    assert_eq!(eval("@2014-01-05.precision()"), vec![Value::Integer(8)]);
    assert_eq!(eval("@T10:30.precision()"), vec![Value::Integer(4)]);
    assert_eq!(eval("@T10:30:00.000.precision()"), vec![Value::Integer(9)]);
    assert_eq!(eval("1.587.precision()"), vec![Value::Integer(3)]);
    is_true("{}.precision().empty()");
    assert_eq!(eval("'x'.precision()"), vec![]);
    assert!(evaluate("(@2014 | @2015).precision()", &[]).is_err());
}

// FHIRPath 3.0 `encode`: hex is lowercase base 16; base64 and urlbase64 are
// padded with `=` (testEncodeHex, testDecodeHex, testEncodeUrlBase64).
#[test]
fn hex_and_padded_urlbase64() {
    assert_eq!(eval("'test'.encode('hex')"), vec![string("74657374")]);
    assert_eq!(eval("'\\n'.encode('hex')"), vec![string("0a")]);
    assert_eq!(eval("'74657374'.decode('hex')"), vec![string("test")]);
    assert_eq!(eval("'74657A74'.decode('hex')"), vec![string("tezt")]);
    assert!(evaluate("'746'.decode('hex')", &[]).is_err());
    assert!(evaluate("'zz'.decode('hex')", &[]).is_err());
    assert!(evaluate("'+a'.decode('hex')", &[]).is_err());
    assert_eq!(
        eval("'subjects?_d'.encode('urlbase64')"),
        vec![string("c3ViamVjdHM_X2Q=")]
    );
    assert_eq!(
        eval("'c3ViamVjdHM_X2Q='.decode('urlbase64')"),
        vec![string("subjects?_d")]
    );
}

// FHIRPath 3.0 string escapes include `\uXXXX` (testUnescapeJson3).
#[test]
fn unescape_json_reads_unicode_escapes() {
    assert_eq!(eval(r"'\\u0041B'.unescape('json')"), vec![string("AB")]);
    assert_eq!(eval(r"'\\u00e9'.unescape('json')"), vec![string("é")]);
    // A malformed escape stays as written.
    assert_eq!(eval(r"'\\u00zz'.unescape('json')"), vec![string(r"\u00zz")]);
    assert_eq!(eval(r"'\\u00'.unescape('json')"), vec![string(r"\u00")]);
}

// A Decimal has no negative zero (HighBoundaryDecimal16).
#[test]
fn negating_decimal_zero_gives_zero() {
    assert_eq!(
        eval("(-0.0034.highBoundary(1)).toString()"),
        eval("0.0034.highBoundary(1).toString()")
    );
    is_true("(-0.0).toString() = 0.0.toString()");
}

// N1 Decimal is exact, not binary floating point (testMinus3, testMod4).
#[test]
fn decimal_add_sub_mod_are_exact() {
    is_true("1.8 - 1.2 = 0.6");
    is_true("2.2 mod 1.8 = 0.4");
    is_true("0.1 + 0.2 = 0.3");
    is_true("5.5 mod 0.7 = 0.6");
    is_true("1.8 - 1 = 0.8");
}

// The scale rounding must not touch values it cannot represent exactly,
// nor leave a negative zero.
#[test]
fn decimal_rounding_keeps_large_values_and_no_negative_zero() {
    is_true("123456789.123456789 + 0.0 = 123456789.123456789");
    is_true("((-0.5) mod 0.5).toString() = 0.0.toString()");
}

// A short `\u` escape consumes only its hex digits.
#[test]
fn unescape_json_short_unicode_escape_keeps_following_escape() {
    assert_eq!(
        eval(r"'\\u12\\n'.unescape('json')"),
        vec![string("\\u12\n")]
    );
}

// Complex values have no FHIRPath `=` result here; they compare structurally.
#[test]
fn distinct_compares_complex_values_structurally() {
    let object = |family: &str| {
        let mut fields: IndexMap<Arc<str>, Vec<Value>> = IndexMap::new();
        fields.insert(Arc::from("family"), vec![string(family)]);
        Value::Object(Arc::new(fields), None)
    };
    let input = [object("Doe"), object("Roe"), object("Doe")];
    let distinct = evaluate("distinct()", &input).unwrap();
    assert_eq!(distinct, vec![object("Roe"), object("Doe")]);
    assert_eq!(
        evaluate("isDistinct()", &input).unwrap(),
        vec![Value::Bool(false)],
    );
}

// Scaled operands keep a remainder at an exact multiple from rounding up to
// the divisor (`1.2 % 0.1` is just below 0.1 in f64).
#[test]
fn decimal_mod_at_an_exact_multiple_is_zero() {
    is_true("1.2 mod 0.1 = 0.0");
    is_true("0.3 mod 0.1 = 0.0");
    is_true("(-1.2) mod 0.5 = -0.2");
}

// A JSON surrogate pair is one character; a lone surrogate stays as written.
#[test]
fn unescape_json_joins_surrogate_pairs() {
    assert_eq!(
        eval(r"'\\uD83D\\uDE00!'.unescape('json')"),
        vec![string("\u{1F600}!")],
    );
    assert_eq!(
        eval(r"'\\uD83Dx'.unescape('json')"),
        vec![string(r"\uD83Dx")],
    );
    assert_eq!(
        eval(r"'\\uD83D\\u0041'.unescape('json')"),
        vec![string(r"\uD83DA")],
    );
}

// A repeated value makes `isDistinct()` false even next to uncomparable dates.
#[test]
fn is_distinct_false_on_a_repeat_beside_uncomparable_dates() {
    assert_eq!(
        eval("1.combine(1).combine(@2020).combine(@2020-01-01).isDistinct()"),
        vec![Value::Bool(false)],
    );
    is_true("(1 | 2 | 3).isDistinct()");
}
