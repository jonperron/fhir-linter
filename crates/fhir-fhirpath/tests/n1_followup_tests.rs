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
