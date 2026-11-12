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
