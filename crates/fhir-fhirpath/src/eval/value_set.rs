//! Membership under FHIRPath equality, with a hash index for the values whose
//! equality is plain equality of a key, so set operations on large
//! collections of strings, numbers, booleans or Quantities stay linear.
use std::collections::HashSet;
use std::sync::Arc;

use super::helpers::{as_quantity, same_value};
use super::quantity::quantity_base;
use crate::Decimal;
use crate::ucum::{Dimension, Ratio};
use crate::value::{Collection, Value};

/// Which values a number key stands for. FHIRPath equality on them is not
/// transitive: `1L = 1` and `1 = 1 '1'`, yet `1L = 1 '1'` is empty, as v3
/// §5.5 defines no Long to Quantity conversion. A lookup therefore probes
/// each kind its value equals ([`NumberKind::equals`]).
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
enum NumberKind {
    /// Integer or Decimal.
    Number,
    Long,
    /// A Quantity equal to a number: dimensionless, with a finite decimal
    /// amount in base units (`1 '1'`, `100 '%'`).
    Dimensionless,
}

impl NumberKind {
    /// The kinds whose values a value of this kind can equal.
    fn equals(self) -> &'static [NumberKind] {
        match self {
            NumberKind::Number => &[
                NumberKind::Number,
                NumberKind::Long,
                NumberKind::Dimensionless,
            ],
            NumberKind::Long => &[NumberKind::Number, NumberKind::Long],
            NumberKind::Dimensionless => &[NumberKind::Number, NumberKind::Dimensionless],
        }
    }
}

/// The part of a value that decides its FHIRPath equality, for the kinds
/// where that is exact: two values of compatible kinds are equal exactly
/// when their keys are. A keyed value never equals an unkeyed one, except
/// a number or a Quantity against an unkeyed Quantity (see [`ValueSet`]).
#[derive(Hash, PartialEq, Eq)]
enum EqualityKey {
    Bool(bool),
    /// N1 §6.1.1 "Decimal: values must be equal, trailing zeroes after the
    /// decimal are ignored", and Integer converts implicitly to Decimal: the
    /// mantissa and scale with every trailing zero stripped, so `1`, `1L`,
    /// `1.0` and `1.00` are (1, 0), and `950` and `9.5e2` are (95, -1).
    Number(NumberKind, i128, i64),
    String(Arc<str>),
    /// N1 §6.1.1 "the dimensions of each quantity must be the same, but not
    /// necessarily the unit": the exact amount in UCUM base units, reduced,
    /// and the dimension (`1 'm'`, `100 'cm'` and `1000.0 'mm'` share it).
    /// A dimensionless amount is a [`NumberKind::Dimensionless`] number
    /// instead, so it meets numbers.
    Quantity(i128, i128, Dimension),
}

/// The one (mantissa, scale) pair of `value` whose mantissa does not end in 0.
fn number_key(kind: NumberKind, value: Decimal) -> EqualityKey {
    let (mut mantissa, mut scale) = (value.mantissa(), i64::from(value.scale()));
    if mantissa == 0 {
        return EqualityKey::Number(kind, 0, 0);
    }
    // A non-zero i128 has at most 38 trailing zeros.
    while mantissa % 10 == 0 {
        mantissa /= 10;
        scale -= 1;
    }
    EqualityKey::Number(kind, mantissa, scale)
}

/// A reduced ratio as a decimal, if its denominator divides a power of ten
/// that fits: `1/8` is `0.125`, `1/3` has none.
fn finite_decimal(ratio: Ratio) -> Option<Decimal> {
    let mut rest = ratio.denom();
    let (mut twos, mut fives) = (0u32, 0u32);
    while rest % 2 == 0 {
        rest /= 2;
        twos += 1;
    }
    while rest % 5 == 0 {
        rest /= 5;
        fives += 1;
    }
    if rest != 1 {
        return None;
    }
    let places = twos.max(fives);
    let multiplier = 10i128.checked_pow(places)? / ratio.denom();
    let mantissa = ratio.numer().checked_mul(multiplier)?;
    Some(Decimal::new(mantissa, i32::try_from(places).ok()?))
}

fn quantity_key(value: &Value) -> Option<EqualityKey> {
    let (amount, dimension) = quantity_base(value)?;
    if dimension != [0; 7] {
        return Some(EqualityKey::Quantity(
            amount.numer(),
            amount.denom(),
            dimension,
        ));
    }
    // A dimensionless amount equals a number exactly when it is a finite
    // decimal; one that fits no Decimal stays unkeyed, so lookups scan it.
    finite_decimal(amount).map(|decimal| number_key(NumberKind::Dimensionless, decimal))
}

/// shortcut: temporal and complex values, and Quantities without a base
/// form (unknown or special units, years and months, overflow), have no
/// key, so they keep the linear scan; add a key when one is proven exact.
fn equality_key(value: &Value) -> Option<EqualityKey> {
    match value.primitive_value() {
        Value::Bool(b) => Some(EqualityKey::Bool(*b)),
        Value::Integer(i) => Some(number_key(NumberKind::Number, Decimal::from(*i))),
        Value::Long(i) => Some(number_key(NumberKind::Long, Decimal::from(*i))),
        Value::Decimal(d) => Some(number_key(NumberKind::Number, *d)),
        Value::String(s) => Some(EqualityKey::String(Arc::clone(s))),
        other => quantity_key(other),
    }
}

/// Whether [`ValueSet::contains`] answers `value` from the hash index (plus,
/// for a number or Quantity, a scan of the unkeyed Quantity members).
pub(super) fn is_keyed(value: &Value) -> bool {
    equality_key(value).is_some()
}

/// Values in insertion order; [`ValueSet::contains`] gives the same answer as
/// a scan with [`same_value`]`(member, value)`.
#[derive(Default)]
pub(super) struct ValueSet {
    values: Collection,
    keys: HashSet<EqualityKey>,
    /// Indices of the Quantity members without a key: a number or a keyed
    /// Quantity can still equal one when its base form overflowed.
    unkeyed_quantities: Vec<usize>,
}

impl ValueSet {
    pub(super) fn contains(&self, value: &Value) -> bool {
        let scan_unkeyed = || {
            self.unkeyed_quantities
                .iter()
                .any(|&i| same_value(&self.values[i], value))
        };
        match equality_key(value) {
            Some(EqualityKey::Number(kind, mantissa, scale)) => {
                kind.equals().iter().any(|&other| {
                    self.keys
                        .contains(&EqualityKey::Number(other, mantissa, scale))
                }) || scan_unkeyed()
            }
            Some(key @ EqualityKey::Quantity(..)) => self.keys.contains(&key) || scan_unkeyed(),
            Some(key) => self.keys.contains(&key),
            None => self.values.iter().any(|member| same_value(member, value)),
        }
    }

    /// Adds `value`, even when an equal one is already a member.
    pub(super) fn push(&mut self, value: Value) {
        match equality_key(&value) {
            Some(key) => {
                self.keys.insert(key);
            }
            None if as_quantity(value.primitive_value()).is_some() => {
                self.unkeyed_quantities.push(self.values.len());
            }
            None => {}
        }
        self.values.push(value);
    }

    /// Adds `value` unless an equal one is a member; `true` when added.
    pub(super) fn insert(&mut self, value: Value) -> bool {
        let added = !self.contains(&value);
        if added {
            self.push(value);
        }
        added
    }

    pub(super) fn len(&self) -> usize {
        self.values.len()
    }

    pub(super) fn into_values(self) -> Collection {
        self.values
    }
}

impl FromIterator<Value> for ValueSet {
    fn from_iter<I: IntoIterator<Item = Value>>(values: I) -> Self {
        let mut set = Self::default();
        for value in values {
            set.push(value);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_share_a_key_whatever_the_scale() {
        // 950 written as 9.5e2 has a negative scale; it still equals 950.
        let set: ValueSet = [Value::Decimal(Decimal::new(95, -1))].into_iter().collect();
        assert!(set.contains(&Value::Integer(950)));
        assert!(set.contains(&Value::Decimal(Decimal::new(95_000, 2))));
        assert!(!set.contains(&Value::Integer(95)));
        let zero: ValueSet = [Value::Decimal(Decimal::new(0, 7))].into_iter().collect();
        assert!(zero.contains(&Value::Long(0)));
        assert!(zero.contains(&Value::Decimal(Decimal::new(0, -3))));
        // Beyond f64's 2^53, keys stay exact.
        let long: ValueSet = [Value::Long(i64::MAX)].into_iter().collect();
        let max = i128::from(i64::MAX);
        assert!(long.contains(&Value::Decimal(Decimal::new(max * 100, 2))));
        assert!(!long.contains(&Value::Decimal(Decimal::new(max * 100 - 1, 2))));
    }

    #[test]
    fn finite_decimal_needs_a_power_of_ten_denominator() {
        let ratio = |n, d| Ratio::new(n, d).unwrap();
        assert_eq!(finite_decimal(ratio(1, 8)), Some(Decimal::new(125, 3)));
        assert_eq!(finite_decimal(ratio(-3, 40)), Some(Decimal::new(-75, 3)));
        assert_eq!(finite_decimal(ratio(7, 1)), Some(Decimal::new(7, 0)));
        assert_eq!(finite_decimal(ratio(1, 3)), None);
        // 2^100 has no power of ten within i128.
        assert_eq!(finite_decimal(ratio(1, 1 << 100)), None);
    }

    fn literal(expr: &str) -> Value {
        crate::evaluate(expr, &[]).unwrap().remove(0)
    }

    #[test]
    fn quantities_with_a_canonical_form_are_keyed() {
        for keyed in ["1 'm'", "1 '%'", "1 '[in_i]'", "4 days", "1 '1'"] {
            assert!(is_keyed(&literal(keyed)), "{keyed}");
        }
        // Equality differs for years and months; unknown units have no base.
        for scanned in ["1 year", "2 months", "1 'foo'", "1 'Cel'"] {
            assert!(!is_keyed(&literal(scanned)), "{scanned}");
        }
    }
}
