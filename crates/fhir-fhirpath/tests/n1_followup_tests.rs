/// FHIRPath N1 follow-ups (PLAN §11.3): skip/take arguments, the string
/// function singleton rule, conversions, date literals, `$total` and `$index`.
///
/// Expressions come from `tests-fhir-r5.xml` where a case exists.
use fhir_fhirpath::{Value, evaluate};

fn eval(expr: &str) -> Vec<Value> {
    evaluate(expr, &[]).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

fn integers(values: &[i64]) -> Vec<Value> {
    values.iter().map(|n| Value::Integer(*n)).collect()
}

// ── skip / take ──────────────────────────────────────────────────────────────

// N1 §4.4.1: a single-input function passed an empty argument gives empty.
#[test]
fn skip_and_take_with_an_empty_count_yield_empty() {
    for expr in [
        "(0 | 1 | 2).skip({})", // testSkip7
        "{}.skip({})",          // testSkip8
        "(0 | 1 | 2).take({})", // testTake8
        "{}.take({})",          // testTake9
    ] {
        assert_eq!(eval(expr), vec![], "{expr}");
    }
    assert_eq!(eval("(0 | 1 | 2).skip(1)"), integers(&[1, 2]));
    assert_eq!(eval("(0 | 1 | 2).take(2)"), integers(&[0, 1]));
}

// N1 §5: an argument with several items is an error, not its first item.
#[test]
fn skip_and_take_with_several_counts_are_errors() {
    for expr in [
        "(0 | 1 | 2).skip(1 | 2)",
        "(0 | 1 | 2).take(1 | 2)",
        "(0 | 1 | 2).skip('1')",
    ] {
        assert!(evaluate(expr, &[]).is_err(), "{expr}");
    }
}

// ── String functions: singleton input ────────────────────────────────────────

/// Every N1 §5.6 string function, called with a valid argument list.
const STRING_CALLS: [&str; 20] = [
    "indexOf('a')",
    "lastIndexOf('a')",
    "substring(0)",
    "startsWith('a')",
    "endsWith('a')",
    "contains('a')",
    "upper()",
    "lower()",
    "replace('a', 'b')",
    "matches('a')",
    "matchesFull('a')",
    "replaceMatches('a', 'b')",
    "length()",
    "trim()",
    "split(',')",
    "encode('base64')",
    "decode('base64')",
    "escape('html')",
    "unescape('html')",
    "substring(0, 1)",
];

// N1 §5.6: "If there is more than one item, or an item that is not a
// String, the evaluation of the expression will end and signal an error".
#[test]
fn string_functions_on_several_items_are_errors() {
    for call in STRING_CALLS {
        let expr = format!("('YQ==' | 'Yg==').{call}");
        assert!(evaluate(&expr, &[]).is_err(), "{expr}");
    }
}

#[test]
fn string_functions_on_a_non_string_item_are_errors() {
    for call in STRING_CALLS {
        for input in ["1", "true", "@2014-01-01"] {
            let expr = format!("{input}.{call}");
            assert!(evaluate(&expr, &[]).is_err(), "{expr}");
        }
    }
}

#[test]
fn string_functions_on_one_string_still_work() {
    for call in STRING_CALLS {
        let expr = format!("'YQ=='.{call}");
        assert!(evaluate(&expr, &[]).is_ok(), "{expr}");
    }
    assert_eq!(eval("'abc'.upper()"), [Value::String("ABC".into())]);
    assert_eq!(eval("'a,b'.split(',').count()"), integers(&[2]));
}

// ── Conversions ──────────────────────────────────────────────────────────────

fn quantity(value: f64, unit: &str) -> Vec<Value> {
    vec![Value::Quantity(value, unit.into())]
}

fn boolean(b: bool) -> Vec<Value> {
    vec![Value::Bool(b)]
}

fn string(s: &str) -> Vec<Value> {
    vec![Value::String(s.into())]
}

// N1 §5.5.7 toQuantity: Integer, Decimal, Quantity, Boolean, and Strings in
// the form `(\+|-)?\d+(\.\d+)?\s*('unit'|calendar keyword)?`.
#[test]
fn to_quantity_converts_numbers_booleans_and_quantity_strings() {
    assert_eq!(eval("1.toQuantity()"), quantity(1.0, "1"));
    assert_eq!(eval("1.5.toQuantity()"), quantity(1.5, "1"));
    assert_eq!(eval("true.toQuantity()"), quantity(1.0, "1"));
    assert_eq!(eval("false.toQuantity()"), quantity(0.0, "1"));
    assert_eq!(eval("'1'.toQuantity()"), quantity(1.0, "1")); // testStringIntegerLiteralToQuantity
    assert_eq!(eval("'-1.5'.toQuantity()"), quantity(-1.5, "1"));
    assert_eq!(eval("'4 days'.toQuantity()"), quantity(4.0, "day"));
    assert_eq!(
        eval(r"'10 \'mg[Hg]\''.toQuantity()"),
        quantity(10.0, "mg[Hg]")
    );
    assert_eq!(eval("4 'mg'.toQuantity()"), quantity(4.0, "mg"));
    assert_eq!(eval("'1.0'.toQuantity() ~ 1 '1'"), boolean(true)); // testStringDecimalLiteralToQuantity
    for expr in [
        "'abc'.toQuantity()",
        "'1 foo'.toQuantity()",
        "'1.'.toQuantity()",
        "@2014.toQuantity()",
        "{}.toQuantity()",
    ] {
        assert_eq!(eval(expr), vec![], "{expr}");
    }
    assert!(evaluate("(1 | 2).toQuantity()", &[]).is_err());
}

// N1 §5.5.7: without UCUM conversion, a different unit argument gives empty
// (toQuantity) or false (convertsToQuantity).
#[test]
fn to_quantity_with_a_unit_keeps_only_the_same_unit() {
    assert_eq!(eval("4 'mg'.toQuantity('mg')"), quantity(4.0, "mg"));
    assert_eq!(eval("'4 days'.toQuantity('day')"), quantity(4.0, "day"));
    assert_eq!(eval("'4 days'.toQuantity('days')"), quantity(4.0, "day"));
    assert_eq!(eval("4 'mg'.toQuantity('g')"), vec![]);
    assert_eq!(eval("4 'mg'.convertsToQuantity('mg')"), boolean(true));
    assert_eq!(eval("4 'mg'.convertsToQuantity('g')"), boolean(false));
}

#[test]
fn converts_to_quantity_follows_to_quantity() {
    for expr in [
        "1.convertsToQuantity()",           // testIntegerLiteralConvertsToQuantity
        "1.0.convertsToQuantity()",         // testDecimalLiteralConvertsToQuantity
        "'1'.convertsToQuantity()",         // testStringIntegerLiteralConvertsToQuantity
        "'1 day'.convertsToQuantity()",     // testStringQuantityLiteralConvertsToQuantity
        r"'1 \'wk\''.convertsToQuantity()", // testStringQuantityWeekConvertsToQuantity
        "true.convertsToQuantity()",        // testBooleanLiteralConvertsToQuantity
    ] {
        assert_eq!(eval(expr), boolean(true), "{expr}");
    }
    assert_eq!(eval("'abc'.convertsToQuantity()"), boolean(false));
    assert_eq!(eval("{}.convertsToQuantity()"), vec![]);
    assert!(evaluate("(1 | 2).convertsToQuantity()", &[]).is_err());
}

// N1 grammar: `unit : dateTimePrecision | pluralDateTimePrecision | STRING`;
// the calendar unit is represented as its singular keyword.
#[test]
fn calendar_duration_quantity_literals_parse() {
    assert_eq!(eval("4 days"), quantity(4.0, "day"));
    assert_eq!(eval("1 year"), quantity(1.0, "year"));
    assert_eq!(eval("2.5 milliseconds"), quantity(2.5, "millisecond"));
    assert_eq!(eval("'1 day'.toQuantity() = 1 day"), boolean(true)); // testStringQuantityLiteralToQuantity
    assert_eq!(eval("'1 month'.toQuantity() ~ 1 month"), boolean(true));
    assert!(evaluate("1 fortnight", &[]).is_err());
    assert!(evaluate("1 dayz", &[]).is_err());
}

// N1 §6.1.1 / §6.1.2: calendar durations equal definite durations only for
// seconds and below; they are equivalent to all of them.
#[test]
fn calendar_durations_against_ucum_units() {
    assert_eq!(eval("1 second = 1 's'"), boolean(true));
    assert_eq!(eval("1 millisecond = 1 'ms'"), boolean(true));
    assert_eq!(eval("1 year = 1 'a'"), boolean(false));
    assert_eq!(eval("1 year ~ 1 'a'"), boolean(true));
    assert_eq!(eval("1 week ~ 1 'wk'"), boolean(true));
    assert_eq!(eval("1 week ~ 1 'd'"), boolean(false));
    assert_eq!(eval("1 'g' ~ 1 'mg'"), boolean(false));
}

// toLong is not in N1 (FHIRPath 3.0 STU); it follows the N1 §5.5 conversion
// pattern of toInteger: singleton input, empty when not convertible.
#[test]
fn to_long_follows_the_conversion_pattern() {
    assert_eq!(eval("'12'.toLong()"), vec![Value::Long(12)]);
    assert_eq!(eval("1.toLong()"), vec![Value::Long(1)]);
    assert_eq!(eval("true.toLong()"), vec![Value::Long(1)]);
    assert_eq!(eval("1.5.toLong()"), vec![]);
    assert_eq!(eval("'1.5'.toLong()"), vec![]);
    assert_eq!(eval("1.5.convertsToLong()"), boolean(false));
    assert_eq!(eval("'12'.convertsToLong()"), boolean(true));
    assert_eq!(eval("{}.convertsToLong()"), vec![]);
    assert!(evaluate("(1 | 2).toLong()", &[]).is_err());
    assert!(evaluate("(1 | 2).convertsToLong()", &[]).is_err());
}

// N1 §5.5.8 toString: Decimal renders as `(\+|-)?\d+(.\d+)?`, kept distinct
// from Integer; Quantity as `value 'unit'`; several items are an error.
#[test]
fn to_string_renders_per_the_n1_table() {
    assert_eq!(eval("1.0.toString()"), string("1.0")); // testDecimalLiteralToString
    assert_eq!(eval("0.0.toString()"), string("0.0")); // testToString4
    assert_eq!(eval("1.5.toString()"), string("1.5"));
    assert_eq!(eval("(-2.0).toString()"), string("-2.0"));
    assert_eq!(eval("1.toString()"), string("1"));
    assert_eq!(eval("1 'wk'.toString()"), string("1 'wk'")); // testQuantityLiteralWkToString
    assert_eq!(eval("4 days.toString()"), string("4 'day'"));
    assert_eq!(eval("@2014-12-14.toString()"), string("2014-12-14"));
    assert_eq!(eval("{}.toString()"), vec![]);
    assert!(evaluate("(1 | 2).toString()", &[]).is_err());
    assert!(evaluate("(1 | 2).convertsToString()", &[]).is_err());
}
