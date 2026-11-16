//! FHIRPath N2 functions found missing by the official suite: `today()`,
//! `now()`, `union()`, `toChars()`, and Date against Time equality.
//!
//! Expressions come from `tests-fhir-r5.xml` where a case exists.
use fhir_fhirpath::{Value, evaluate};

fn eval(expr: &str) -> Vec<Value> {
    evaluate(expr, &[]).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

fn is_true(expr: &str) {
    assert_eq!(eval(expr), vec![Value::Bool(true)], "{expr}");
}

#[test]
fn today_is_a_date_and_now_a_date_time_with_a_zone() {
    is_true("today().is(Date)"); // a Date has no time part
    is_true("today().toString().length() = 10"); // testToday2
    is_true("now().is(DateTime)");
    is_true("now().toString().length() > 10"); // testNow2
    is_true("@1974-12-25 < today()"); // testToday1
    is_true("today() > @1974-12-25"); // testDateGreaterThanDate
    // N2 §5.10: now() carries a zone, so it compares with zoned date-times.
    // Against a Date it stays empty, as testDateTimeGreaterThanDate2 expects;
    // testNow1 (`@1974-12-25 < now()`, expected true) waits on the pending
    // decision on the zone of a Date (see `temporal::compare`).
    is_true("now() > @2000-01-01T00:00:00Z");
    assert_eq!(eval("now() > today()"), vec![]);
}

#[test]
fn now_and_today_keep_one_value_per_evaluation() {
    is_true("now() = now()");
    is_true("today() = today()");
    is_true("(1 | 2 | 3).select(now()).distinct().count() = 1");
}

#[test]
fn today_and_now_take_no_argument() {
    assert!(evaluate("today(1)", &[]).is_err());
    assert!(evaluate("now(1)", &[]).is_err());
}

#[test]
fn union_function_matches_the_union_operator() {
    is_true("1.union(2).union(3).count() = 3"); // testUnion4
    is_true("1.union(2.union(3)).count() = 3"); // testUnion5
    is_true("1.combine(1).union(2).count() = 2"); // testUnion8
    is_true("(1 | 2).union(2 | 3) = (1 | 2 | 3)");
    is_true("{}.union({}).empty()");
    assert!(evaluate("1.union()", &[]).is_err());
}

#[test]
fn union_function_deduplicates_and_handles_empty_like_the_operator() {
    is_true("(1 | 1 | 2).union(2 | 2).count() = 2");
    is_true("1.union(1).count() = 1");
    is_true("1.union({}) = 1");
    is_true("{}.union(1) = 1");
}

#[test]
fn to_chars_keeps_multibyte_characters_whole() {
    is_true("'日本'.toChars().count() = 2");
    is_true("'a b'.toChars().count() = 3");
}

#[test]
fn has_value_on_primitives_without_a_resource() {
    is_true("'a'.hasValue()");
    is_true("{}.hasValue().not()");
    is_true("today().hasValue()");
    is_true("(1 | 2).hasValue().not()");
}

#[test]
fn to_chars_splits_a_string_into_characters() {
    is_true("'t2'.toChars() = 't' | '2'"); // testToChars1
    is_true("'aa'.toChars().count() = 2"); // duplicates are kept
    is_true("'ée'.toChars() = 'é' | 'e'"); // characters, not bytes
    is_true("''.toChars().empty()");
    is_true("{}.toChars().empty()");
    assert!(evaluate("('a' | 'b').toChars()", &[]).is_err());
    assert!(evaluate("1.toChars()", &[]).is_err());
}

// N2 §6.1.1: single items of types that cannot be converted to one type are
// not equal ("Otherwise, equals returns false"); a Date never converts to a Time.
#[test]
fn date_and_time_are_never_equal() {
    is_true("@1974-12-25 != @T12:14:15"); // testDateNotEqualTimeSecond
    is_true("@1974-12-25 != @T12:14"); // testDateNotEqualTimeMinute
    is_true("(@1974-12-25T12:14 = @T12:14) = false");
    is_true("(@T12:14 = @1974-12-25) = false");
    // Date against DateTime still compares, and differing precision stays empty.
    assert_eq!(eval("@1974-12-25 = @1974-12-25T12:14"), vec![]);
}
