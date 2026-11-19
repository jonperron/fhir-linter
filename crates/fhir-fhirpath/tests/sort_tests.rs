//! `sort()` (FHIRPath v3 CI build, fn-sort): "Attempting to sort items with
//! incompatible types will result in an error."
use fhir_fhirpath::{Value, evaluate};

/// Integers and strings interleaved: no total order exists, so the sort must
/// end in an error, never a panic of the standard sort.
fn mixed_expression(len: usize) -> String {
    let items: Vec<String> = (0..len)
        .map(|i| match i % 3 {
            0 => format!("{}", (i * 7919) % 101),
            1 => format!("'s{}'", (i * 104_729) % 97),
            _ => format!("{}.5", (i * 31) % 89),
        })
        .collect();
    format!("({})", items.join(" | "))
}

#[test]
fn sort_mixed_types_is_an_error_without_panic() {
    for len in [3, 20, 33, 64, 120] {
        let expr = format!("{}.sort()", mixed_expression(len));
        let outcome = std::panic::catch_unwind(|| evaluate(&expr, &[]));
        let result = outcome.unwrap_or_else(|_| panic!("sort() panicked at length {len}"));
        assert!(result.is_err(), "length {len}: {result:?}");
    }
}

/// fn-sort: "the same comparison semantics as defined for the equals (`=`)
/// and comparison operators": zones are normalised (N1 §6.2).
#[test]
fn sort_date_times_normalises_zones() {
    let result = evaluate(
        "(@2020-01-01T09:00:00Z | @2020-01-01T10:00:00+02:00).sort() \
         = (@2020-01-01T10:00:00+02:00 | @2020-01-01T09:00:00Z)",
        &[],
    )
    .unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn sort_date_time_key_normalises_zones() {
    let result = evaluate(
        "(@2020-01-01T09:00:00Z | @2020-01-01T10:00:00+02:00).sort($this).first() \
         = @2020-01-01T10:00:00+02:00",
        &[],
    )
    .unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

/// `@2020 < @2020-06` is empty (N1 §6.2): the order is not known, so the
/// values cannot be sorted.
#[test]
fn sort_dates_of_undecided_order_is_an_error() {
    assert!(evaluate("(@2020-06 | @2020).sort()", &[]).is_err());
    assert!(evaluate("(@T10:00 | @T10).sort($this)", &[]).is_err());
}

#[test]
fn sort_dates_of_different_precision_with_known_order() {
    let result = evaluate("(@2021-03 | @2020).sort() = (@2020 | @2021-03)", &[]).unwrap();
    assert_eq!(result, vec![Value::Bool(true)]);
}

#[test]
fn sort_mixed_type_keys_is_an_error_without_panic() {
    for len in [3, 20, 33, 64, 120] {
        let expr = format!("{}.sort($this)", mixed_expression(len));
        let outcome = std::panic::catch_unwind(|| evaluate(&expr, &[]));
        let result = outcome.unwrap_or_else(|_| panic!("sort($this) panicked at length {len}"));
        assert!(result.is_err(), "length {len}: {result:?}");
    }
}

fn is_true(expr: &str) {
    assert_eq!(
        evaluate(expr, &[]).unwrap(),
        vec![Value::Bool(true)],
        "{expr}"
    );
}

/// Official testSort8: a `-` prefix sorts any orderable key descending.
#[test]
fn sort_minus_prefix_sorts_strings_descending() {
    is_true("('a' | 'c' | 'b').sort(-$this) = ('c' | 'b' | 'a')");
}

/// fn-sort examples: `asc` and `desc` qualifiers.
#[test]
fn sort_asc_and_desc_qualifiers() {
    is_true("(3 | 1 | 2).sort($this desc) = (3 | 2 | 1)");
    is_true("(3 | 1 | 2).sort($this asc) = (1 | 2 | 3)");
    is_true("('c' | 'a' | 'b').sort($this desc) = ('c' | 'b' | 'a')");
}

/// fn-sort: "if the values for the first key selector are equal, the
/// comparison proceeds to the next key selector".
#[test]
fn sort_several_keys() {
    is_true("(12 | 21 | 11 | 22).sort($this div 10, -($this mod 10)) = (12 | 11 | 22 | 21)");
    is_true("(12 | 21 | 11 | 22).sort($this div 10 desc, $this mod 10) = (21 | 22 | 11 | 12)");
    is_true("('ab' | 'b' | 'aa').sort(length(), $this desc) = ('b' | 'ab' | 'aa')");
}

/// fn-sort: "An empty value is considered lower than all other values".
#[test]
fn sort_empty_key_is_lowest() {
    is_true("(1 | 2 | 3).sort(iif($this = 2, {}, $this)) = (2 | 1 | 3)");
    is_true("(1 | 2 | 3).sort(iif($this = 2, {}, $this) desc) = (3 | 1 | 2)");
    // A negated empty key is still empty, so still first (official testSort10).
    is_true("(1 | 2 | 3).sort(-iif($this = 2, {}, $this)) = (2 | 3 | 1)");
    is_true("('a' | 'b' | 'c').sort(-iif($this = 'b', {}, $this)) = ('b' | 'c' | 'a')");
}

/// fn-sort: a key with several items is an error.
#[test]
fn sort_key_with_several_items_is_an_error() {
    assert!(evaluate("(1 | 2 | 2.5).sort($this | 5)", &[]).is_err());
}

/// fn-sort: "subsequent selectors are only evaluated for items where the
/// previous `keySelector` comparison resulted in equality".
#[test]
fn sort_later_keys_run_only_on_ties() {
    is_true("(2 | 1).sort($this, $this | 5) = (1 | 2)");
}

#[test]
fn sort_direction_is_only_allowed_in_sort() {
    assert!(evaluate("(1 | 2).where($this desc)", &[]).is_err());
}

#[test]
fn sort_key_named_desc_is_still_a_path() {
    is_true("(2 | 1).sort(desc) = (2 | 1)");
}

/// v3 min()/max(): "Comparison semantics are defined by the Comparison
/// Operators": zones are normalised.
#[test]
fn min_and_max_normalise_zones() {
    is_true(
        "(@2020-01-01T09:00:00Z | @2020-01-01T10:00:00+02:00).min() = @2020-01-01T10:00:00+02:00",
    );
    is_true("(@2020-01-01T10:00:00+02:00 | @2020-01-01T09:00:00Z).max() = @2020-01-01T09:00:00Z");
}

/// The operators give empty for @2020 vs @2020-06, and the spec names no
/// error for it, so min() and max() give empty too.
#[test]
fn min_and_max_of_undecided_dates_are_empty() {
    assert_eq!(evaluate("(@2020-06 | @2020).min()", &[]).unwrap(), vec![]);
    assert_eq!(evaluate("(@2020 | @2020-06).max()", &[]).unwrap(), vec![]);
}

/// §min1: "All items in the input collection SHALL be the same type,
/// otherwise an exception is thrown."
#[test]
fn min_of_a_date_and_a_date_time_is_an_error() {
    assert!(evaluate("(@2019-01-01 | @2020-06-01T10:00:00Z).min()", &[]).is_err());
    assert!(evaluate("(@2019-01-01 | @2020-06-01T10:00:00Z).max()", &[]).is_err());
}

/// Several keys tied on every level stay in input order (stable sort).
#[test]
fn sort_all_keys_tied_keeps_input_order() {
    is_true("(3 | 1 | 2).sort(1, 'a', {}) = (3 | 1 | 2)");
}
