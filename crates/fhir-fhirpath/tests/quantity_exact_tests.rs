//! Exact Quantity amounts. N1 §4.1.8: "The `Quantity` type represents
//! quantities with a specified unit, where the `value` component is defined
//! as a `Decimal`", so amounts compare, convert and print as exact decimals,
//! like Integer and Decimal values.
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

fn text(expr: &str) -> String {
    match evaluate(expr, &[]).unwrap().as_slice() {
        [Value::String(s)] => s.to_string(),
        other => panic!("{expr}: not a String: {other:?}"),
    }
}

#[test]
fn numbers_beyond_f64_compare_exactly_with_quantities() {
    assert_eq!(
        boolean("100000000000000001 = 100000000000000000 '1'"),
        Some(false)
    );
    assert_eq!(
        boolean("100000000000000001 > 100000000000000000 '1'"),
        Some(true)
    );
    assert_eq!(boolean("0.10000000000000001 = 0.1 '1'"), Some(false));
    assert_eq!(boolean("0.10000000000000001 '1' > 0.1"), Some(true));
    assert_eq!(
        boolean("100000000000000001 'g' = 100000000000000000 'g'"),
        Some(false)
    );
}

#[test]
fn unit_conversion_is_exact() {
    // A relative tolerance made these equal.
    assert_eq!(boolean("1 = 100.00000000005 '%'"), Some(false));
    assert_eq!(boolean("1 'm' = 100.0000000000001 'cm'"), Some(false));
    assert_eq!(boolean("1 'm' < 100.0000000000001 'cm'"), Some(true));
    // Exact factors: 1 [in_i] is 2.54 cm, so 50 [in_i] is 127 cm.
    assert_eq!(boolean("0.3 'm' = 30 'cm'"), Some(true));
    assert_eq!(boolean("1 '[in_i]' = 2.54 'cm'"), Some(true));
    assert_eq!(boolean("127 'cm' = 50 '[in_i]'"), Some(true));
    assert_eq!(boolean("1 'cm' < 1 '[in_i]'"), Some(true));
    assert_eq!(boolean("4 'g' ~ 4040 'mg'"), Some(true));
    assert_eq!(boolean("4 'g' ~ 4600 'mg'"), Some(false));
    // As for Decimals, trailing zeros do not count as precision.
    assert_eq!(boolean("1.0 ~ 1.49"), Some(true));
    assert_eq!(boolean("1.0 'g' ~ 1.49 'g'"), Some(true));
    // Rounded half away from zero, on either sign.
    assert_eq!(boolean("4 'g' ~ 4500 'mg'"), Some(false));
    assert_eq!(boolean("4 'g' ~ 4499 'mg'"), Some(true));
    assert_eq!(boolean("-4 'g' ~ -4499 'mg'"), Some(true));
    assert_eq!(boolean("-4 'g' ~ -4500 'mg'"), Some(false));
}

#[test]
fn set_operations_agree_with_equals() {
    assert_eq!(count("(1 'm' | 100 'cm' | 1000.0 'mm').count()"), 1);
    assert_eq!(count("(1 | 1 '1' | 100 '%' | 1.0).count()"), 1);
    assert_eq!(count("(0.1 '1' | 10 '%').count()"), 1);
    assert_eq!(count("(1 '[in_i]' | 2.54 'cm').count()"), 1);
    assert_eq!(count("(1 | 100.00000000005 '%').count()"), 2);
    assert_eq!(count("(1 | 2).intersect(1 '1' | 200 '%').count()"), 2);
    assert_eq!(count("(1 'm' | 2 'm').exclude(100 'cm').count()"), 1);
    assert_eq!(boolean("(100 'cm').subsetOf(1 'm' | 3 'm')"), Some(true));
    assert_eq!(count("(1 '%' | 0.01).count()"), 1);
    assert_eq!(count("(1 'm' | 1 's').count()"), 2);
    // v3 §5.5: Long -> Quantity is "-", while Long and Integer are equal.
    assert_eq!(count("(1L | 1 '1').count()"), 2);
    assert_eq!(count("(1 '1' | 1L).count()"), 2);
    assert_eq!(count("(1 | 1L | 1 '1').count()"), 1);
    // Calendar years and months have no canonical form: they equal only
    // the same keyword.
    assert_eq!(count("(1 year | 12 months | 1 'a' | 1 year).count()"), 3);
    assert_eq!(count("(1 day | 24 'h' | 1 'd').count()"), 1);
    assert_eq!(count("(1 'foo' | 1.0 'foo' | 1 'bar').count()"), 2);
    assert_eq!(
        boolean("(1 'm' | 2 'm').distinct().count() = 2"),
        Some(true)
    );
    assert_eq!(boolean("(1 'm' | 100 'cm').isDistinct()"), Some(true));
    assert_eq!(
        boolean("(1 'm').combine(100 'cm').isDistinct()"),
        Some(false)
    );
    assert_eq!(boolean("100 'cm' in (1 'm' | 2 'm')"), Some(true));
    assert_eq!(
        boolean("(1 'm' | 2 'm').intersect(200 'cm').count() = 1"),
        Some(true)
    );
}

/// v3 §5.5 table: Long -> Quantity is "-" (no conversion defined), as for
/// the comparison operators.
#[test]
fn long_times_a_quantity_is_not_a_quantity_product() {
    assert!(evaluate("2L * 3 'kg'", &[]).is_err());
    assert!(evaluate("3 'kg' / 2L", &[]).is_err());
    assert_eq!(boolean("2 * 3 'kg' = 6 'kg'"), Some(true));
}

#[test]
fn amounts_keep_their_written_scale() {
    assert_eq!(text("1.50 'g'.toString()"), "1.50 'g'");
    assert_eq!(text("1.0.toQuantity().toString()"), "1.0 '1'");
    assert_eq!(text("true.toQuantity().toString()"), "1.0 '1'");
    assert_eq!(text("(1.0 'g' * 2.0).toString()"), "2.00 'g'");
    assert_eq!(text("(-1.50 'g').toString()"), "-1.50 'g'");
    // The boundary is half a unit of the last written digit.
    assert_eq!(boolean("1.0 'g'.lowBoundary() = 0.95 'g'"), Some(true));
    assert_eq!(boolean("1 'g'.highBoundary() = 1.5 'g'"), Some(true));
    // The product of exact amounts is exact.
    assert_eq!(boolean("0.1 'g' * 3 = 0.3 'g'"), Some(true));
}

#[test]
fn to_quantity_converts_exactly_or_like_division() {
    // 1.0 g is precise to 100 mg: the factor moves the scale, not the digits.
    assert_eq!(text("1.0 'g'.toQuantity('mg').toString()"), "1000 'mg'");
    assert_eq!(text("1500 'mg'.toQuantity('g').toString()"), "1.5 'g'");
    // 50/127 has no exact decimal: rounded as `/` rounds, to 8 places.
    assert_eq!(
        text("1 'cm'.toQuantity('[in_i]').toString()"),
        "0.39370079 '[in_i]'"
    );
}

/// `y * 127` overflows 38 digits, so the order falls back to the amounts
/// in base units: 2e37 [in_i] is 5.08e35 m, still equal, as the set key says.
#[test]
fn an_overflowing_cross_product_still_finds_equal_amounts() {
    let metres = format!("508{}", "0".repeat(33));
    let inches = format!("2{}", "0".repeat(37));
    let same = format!("{metres} 'm' = {inches} '[in_i]'");
    assert_eq!(boolean(&same), Some(true));
    assert_eq!(
        boolean(&format!("{metres} 'm' <= {inches} '[in_i]'")),
        Some(true)
    );
    assert_eq!(
        count(&format!("({metres} 'm' | {inches} '[in_i]').count()")),
        1
    );
}

/// Unequal amounts whose cross product overflows: `=` still knows they
/// differ (their amounts in base units do), `comparable()` looks at the
/// units only, and the order is the documented empty shortcut.
#[test]
fn an_overflowing_cross_product_keeps_unequal_amounts_apart() {
    let metres = format!("508{}", "0".repeat(33));
    let inches = format!("3{}", "0".repeat(37));
    assert_eq!(
        boolean(&format!("{metres} 'm' = {inches} '[in_i]'")),
        Some(false)
    );
    assert_eq!(
        boolean(&format!("{metres} 'm' != {inches} '[in_i]'")),
        Some(true)
    );
    assert_eq!(
        boolean(&format!("{metres} 'm'.comparable({inches} '[in_i]')")),
        Some(true)
    );
    assert_eq!(boolean(&format!("{metres} 'm' < {inches} '[in_i]'")), None);
}

/// N1 §6.6 gives empty only for a division by zero; an amount past 38
/// digits is an overflow error, as `*` on two Decimals is.
#[test]
fn an_overflowing_product_is_an_error() {
    let big = "9".repeat(38);
    assert!(evaluate(&format!("{big} 'g' * 10"), &[]).is_err());
    assert!(evaluate(&format!("{big} 'g' * 10 'm'"), &[]).is_err());
    assert_eq!(evaluate("1 'g' / 0", &[]).unwrap(), vec![]);
}
