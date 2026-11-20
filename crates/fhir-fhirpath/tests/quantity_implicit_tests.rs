//! N1 §5.5: Integer and Decimal convert implicitly to a Quantity with the
//! default unit `'1'` (§5.5.7), so `=`, `~`, `!=`, the comparison operators
//! and the set operations treat `1` and `1 '1'` as the same value.
use fhir_fhirpath::{Value, evaluate};

/// The result of `expr` as `Some(bool)`, or `None` when empty.
fn boolean(expr: &str) -> Option<bool> {
    match evaluate(expr, &[]).unwrap().as_slice() {
        [] => None,
        [Value::Bool(b)] => Some(*b),
        other => panic!("{expr}: not a Boolean: {other:?}"),
    }
}

fn count(expr: &str) -> usize {
    match evaluate(expr, &[]).unwrap().as_slice() {
        [Value::Integer(n)] => usize::try_from(*n).unwrap(),
        other => panic!("{expr}: not an Integer: {other:?}"),
    }
}

#[test]
fn equals_converts_a_number_to_a_quantity_with_unit_one() {
    assert_eq!(boolean("1 = 1 '1'"), Some(true));
    assert_eq!(boolean("1 '1' = 1"), Some(true));
    assert_eq!(boolean("1.5 = 1.5 '1'"), Some(true));
    assert_eq!(boolean("1 = 2 '1'"), Some(false));
    // UCUM: '%' is the dimensionless 1/100.
    assert_eq!(boolean("1 = 100 '%'"), Some(true));
    // A number converts to unit '1' only: 'mg' cannot be compared.
    assert_eq!(boolean("1 = 1 'mg'"), None);
}

/// FHIRPath v3 §5.5 table: Long -> Decimal is Implicit, Long -> Quantity is
/// "-" (no conversion defined), so a Long never converts to a Quantity.
#[test]
fn long_does_not_convert_to_a_quantity() {
    assert_eq!(boolean("1L = 1 '1'"), None);
    assert_eq!(boolean("1L ~ 1 '1'"), Some(false));
    assert_eq!(count("(1L | 1 '1').count()"), 2);
    assert_eq!(count("(1 '1' | 1L).count()"), 2);
}

#[test]
fn not_equals_is_the_converse() {
    assert_eq!(boolean("1 != 1 '1'"), Some(false));
    assert_eq!(boolean("1 != 2 '1'"), Some(true));
}

#[test]
fn equivalent_converts_a_number_to_a_quantity_with_unit_one() {
    assert_eq!(boolean("1 ~ 1 '1'"), Some(true));
    assert_eq!(boolean("1.0 '1' ~ 1"), Some(true));
    // Rounded to the precision of the least precise operand.
    assert_eq!(boolean("1.5 ~ 1.52 '1'"), Some(true));
    assert_eq!(boolean("1 ~ 2 '1'"), Some(false));
    // Units that cannot be compared: false, not empty.
    assert_eq!(boolean("1 ~ 1 'mg'"), Some(false));
    assert_eq!(boolean("1 !~ 1 '1'"), Some(false));
    assert_eq!(boolean("1 !~ 1 'mg'"), Some(true));
}

#[test]
fn comparable_stays_defined_on_quantities_only() {
    assert_eq!(boolean("1 '1'.comparable(2 '1')"), Some(true));
    assert_eq!(boolean("1 '1'.comparable(2)"), None);
    assert_eq!(boolean("1.comparable(2 '1')"), None);
}

#[test]
fn comparison_converts_a_number_to_a_quantity_with_unit_one() {
    assert_eq!(boolean("2 > 1 '1'"), Some(true));
    assert_eq!(boolean("1 '1' < 2"), Some(true));
    assert_eq!(boolean("1.5 <= 1.5 '1'"), Some(true));
    assert_eq!(boolean("1 >= 2 '1'"), Some(false));
    assert_eq!(boolean("2 > 1 'mg'"), None);
}

#[test]
fn sort_and_max_order_numbers_with_quantities_of_unit_one() {
    assert_eq!(boolean("(2 | 1 '1').max() = 2"), Some(true));
    assert_eq!(boolean("(2 | 1 '1').min() = 1"), Some(true));
    assert_eq!(boolean("(2 | 1 '1').sort().first() = 1 '1'"), Some(true));
}

#[test]
fn set_operations_agree_with_equals() {
    assert_eq!(count("(1 | 1 '1').count()"), 1);
    assert_eq!(count("(1 '1' | 1).count()"), 1);
    assert_eq!(count("(1 '1').combine(1.0).distinct().count()"), 1);
    assert_eq!(boolean("(1 '1').combine(1).isDistinct()"), Some(false));
    assert_eq!(boolean("1 in (1 '1')"), Some(true));
    assert_eq!(boolean("(1 '1') contains 1"), Some(true));
    assert_eq!(boolean("(1 | 2).intersect(1 '1') = 1"), Some(true));
    assert_eq!(boolean("(1 | 2).exclude(100 '%') = 2"), Some(true));
    assert_eq!(boolean("(1).subsetOf(1 '1' | 2 'mg')"), Some(true));
    assert_eq!(count("(1 | 2 'mg' | 1 '1').count()"), 2);
}
