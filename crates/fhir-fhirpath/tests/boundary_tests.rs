//! `lowBoundary()`, `highBoundary()` (FHIRPath N2 utility functions) and
//! `comparable()` (N2 comparison), as the official suite expects them.
use std::sync::Arc;

use fhir_fhirpath::{Value, evaluate};

fn eval(expr: &str) -> Vec<Value> {
    evaluate(expr, &[]).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

fn text(s: &str) -> Arc<str> {
    Arc::from(s)
}

#[test]
fn decimal_boundaries_follow_the_input_precision() {
    let cases = [
        ("1.587.lowBoundary()", 1.5865),
        ("1.587.lowBoundary(2)", 1.58),
        ("1.587.lowBoundary(0)", 1.0),
        ("1.587.highBoundary()", 1.5875),
        ("1.587.highBoundary(2)", 1.59),
        ("(-1.587).lowBoundary(2)", -1.59),
        ("(-1.587).highBoundary(2)", -1.58),
        ("(-9.96).lowBoundary(1)", -10.0),
        ("99.96.highBoundary(1)", 100.0),
        ("0.0034.highBoundary(1)", 0.0),
        ("1.lowBoundary(5)", 0.5),
        ("120.lowBoundary(2)", 119.5),
        ("10.lowBoundary(0)", 9.0),
    ];
    for (expr, expected) in cases {
        assert_eq!(eval(expr), [Value::Decimal(expected)], "{expr}");
    }
    // The sign of a zero boundary below a negative value is kept.
    let [Value::Decimal(zero)] = eval("(-0.0034).lowBoundary(1)")[..] else {
        panic!("not a Decimal");
    };
    assert!(zero == 0.0 && zero.is_sign_negative());
}

#[test]
fn precision_out_of_range_is_empty() {
    for expr in [
        "1.587.lowBoundary(-1)",
        "1.587.highBoundary(32)",
        "@2014.lowBoundary(5)",
        "@2014-01-01T08.highBoundary(18)",
        "@T10:30.lowBoundary(10)",
        "@2014.lowBoundary()",
    ] {
        assert_eq!(eval(expr), [], "{expr}");
    }
}

#[test]
fn date_and_time_boundaries_fill_the_missing_parts() {
    let cases = [
        ("@2014.lowBoundary(6)", Value::Date(text("2014-01"))),
        ("@2014.highBoundary(6)", Value::Date(text("2014-12"))),
        ("@2014-02.highBoundary(8)", Value::Date(text("2014-02-28"))),
        ("@2016-02.highBoundary(8)", Value::Date(text("2016-02-29"))),
        (
            "@2014-01-01T08.lowBoundary(17)",
            Value::DateTime(text("2014-01-01T08:00:00.000+14:00")),
        ),
        (
            "@2014-01-01T08.highBoundary(17)",
            Value::DateTime(text("2014-01-01T08:00:59.999-12:00")),
        ),
        (
            "@2014-01-01T08:05-05:00.highBoundary()",
            Value::DateTime(text("2014-01-01T08:05:59.999-05:00")),
        ),
        (
            "@2014-01-01T08.lowBoundary(8)",
            Value::DateTime(text("2014-01-01")),
        ),
        ("@T10:30.lowBoundary(9)", Value::Time(text("10:30:00.000"))),
        ("@T10:30.highBoundary()", Value::Time(text("10:30:59.999"))),
    ];
    for (expr, expected) in cases {
        assert_eq!(eval(expr), [expected], "{expr}");
    }
}

#[test]
fn mixed_precision_boundaries_compare() {
    assert_eq!(
        eval("@2024-01-01T.lowBoundary() <= @2024-01-01T10:00Z.highBoundary()"),
        [Value::Bool(true)],
    );
    assert_eq!(
        eval("@2024-05-01T00:00Z.lowBoundary() <= @2020-01-01T00:00Z.highBoundary()"),
        [Value::Bool(false)],
    );
}

#[test]
fn quantity_boundary_keeps_the_unit() {
    assert_eq!(
        eval("1.587 'cm'.lowBoundary(8)"),
        [Value::Quantity(1.5865, text("cm"))],
    );
    assert_eq!(
        eval("1.587 'm'.highBoundary(8)"),
        [Value::Quantity(1.5875, text("m"))],
    );
}

#[test]
fn comparable_needs_units_the_engine_can_compare() {
    assert_eq!(eval("1 'mg'.comparable(2 'mg')"), [Value::Bool(true)]);
    assert_eq!(eval("1 'kg'.comparable(1 'cm')"), [Value::Bool(false)]);
    assert_eq!(eval("1 'cm'.comparable(1 's')"), [Value::Bool(false)]);
    assert_eq!(eval("1 year.comparable(1 'a')"), [Value::Bool(false)]);
    assert_eq!(eval("{}.comparable(1 'mg')"), []);
    assert_eq!(eval("1 'mg'.comparable({})"), []);
    assert_eq!(eval("1.comparable(1 'mg')"), []);
}

#[test]
fn hour_only_literals_parse_and_short_parts_do_not() {
    assert_eq!(eval("@T10"), [Value::Time(text("10"))]);
    assert_eq!(
        eval("@2014-01-01T08"),
        [Value::DateTime(text("2014-01-01T08"))]
    );
    for expr in ["@T1", "@2014-01-01T8", "@T10:3"] {
        assert!(evaluate(expr, &[]).is_err(), "{expr}");
    }
}
