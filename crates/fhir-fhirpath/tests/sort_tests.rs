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
