//! `sort()` (FHIRPath v3 CI build, fn-sort): "Attempting to sort items with
//! incompatible types will result in an error."
use fhir_fhirpath::evaluate;

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

#[test]
fn sort_mixed_type_keys_is_an_error_without_panic() {
    for len in [3, 20, 33, 64, 120] {
        let expr = format!("{}.sort($this)", mixed_expression(len));
        let outcome = std::panic::catch_unwind(|| evaluate(&expr, &[]));
        let result = outcome.unwrap_or_else(|_| panic!("sort($this) panicked at length {len}"));
        assert!(result.is_err(), "length {len}: {result:?}");
    }
}
