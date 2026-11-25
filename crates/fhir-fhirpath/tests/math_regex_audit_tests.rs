//! Audit 2026-11-25: math functions on several items, Integer power(),
//! matchesFull() anchoring and `\u` escapes.

use fhir_fhirpath::{Value, evaluate};

fn eval(expr: &str) -> Result<Vec<Value>, String> {
    evaluate(expr, &[]).map_err(|e| e.to_string())
}

// N1 §5.7: more than one input item signals an error.
#[test]
fn math_functions_reject_several_input_items() {
    for expr in [
        "(1 | -4).abs()",
        "(1.5 | 4.5).ceiling()",
        "(1.5 | 4.5).floor()",
        "(1.5 | 4.5).round()",
        "(1 | 4).sqrt()",
        "(1 | 4).exp()",
        "(1 | 4).ln()",
        "(1 | 4).log(10)",
        "(1 | 4).power(2)",
        "(1.5 | 4.5).truncate()",
    ] {
        assert!(eval(expr).is_err(), "{expr} should be an error");
    }
}

#[test]
fn math_arguments_reject_several_items() {
    for expr in ["2.power(2 | 3)", "100.log(10 | 2)"] {
        assert!(eval(expr).is_err(), "{expr} should be an error");
    }
}

#[test]
fn math_on_empty_input_stays_empty() {
    assert_eq!(eval("{}.sqrt()"), Ok(vec![]));
    assert_eq!(eval("{}.power(2)"), Ok(vec![]));
    assert_eq!(eval("2.power({})"), Ok(vec![]));
}

// N1 §5.7.8: "If this function is used with Integers, the result is an Integer."
#[test]
fn power_of_integers_is_an_integer() {
    assert_eq!(eval("2.power(3)"), Ok(vec![Value::Integer(8)]));
    assert_eq!(eval("2.power(3) is Integer"), Ok(vec![Value::Bool(true)]));
    assert_eq!(eval("(-2).power(3)"), Ok(vec![Value::Integer(-8)]));
    assert_eq!(eval("5.power(0)"), Ok(vec![Value::Integer(1)]));
}

#[test]
fn power_of_integers_overflow_is_an_error() {
    assert!(eval("2.power(64)").is_err());
    assert!(eval("2.power(9999999999)").is_err());
}

#[test]
fn power_of_huge_integer_exponent_keeps_parity() {
    assert_eq!(eval("1.power(9999999999)"), Ok(vec![Value::Integer(1)]));
    assert_eq!(eval("0.power(9999999999)"), Ok(vec![Value::Integer(0)]));
    assert_eq!(eval("(-1).power(9999999999)"), Ok(vec![Value::Integer(-1)]));
    assert_eq!(eval("(-1).power(9999999998)"), Ok(vec![Value::Integer(1)]));
}

#[test]
fn power_with_decimal_or_negative_exponent_stays_decimal() {
    assert_eq!(eval("2.power(-1) is Decimal"), Ok(vec![Value::Bool(true)]));
    assert_eq!(eval("2.power(0.5) is Decimal"), Ok(vec![Value::Bool(true)]));
    assert_eq!(eval("2.5.power(2) is Decimal"), Ok(vec![Value::Bool(true)]));
    assert_eq!(eval("(-1).power(0.5)"), Ok(vec![]));
}

// The pattern must not close the anchoring group and escape `\A..\z`.
#[test]
fn matches_full_pattern_cannot_escape_its_anchors() {
    assert!(eval("'zzzb'.matchesFull('a)|(?:b')").is_err());
    assert!(eval("'zzzb'.matches('a)|(?:b')").is_err());
    assert_eq!(
        eval("'ab'.matchesFull('a|b')"),
        Ok(vec![Value::Bool(false)])
    );
    assert_eq!(eval("'b'.matchesFull('a|b')"), Ok(vec![Value::Bool(true)]));
}

// FHIRPath grammar: UNICODE is 'u' HEX HEX HEX HEX; `+` is not a hex digit.
#[test]
fn unicode_escape_requires_four_hex_digits() {
    assert!(eval(r"'\u+041'").is_err());
    assert!(eval(r"`\u+041`").is_err());
    assert_eq!(eval(r"'A'"), Ok(vec![Value::String("A".into())]));
}
