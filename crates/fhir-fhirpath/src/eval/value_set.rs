//! Membership under FHIRPath equality, with a hash index for the values whose
//! equality is plain equality of a key, so set operations on large
//! collections of strings, numbers or booleans stay linear.
use std::collections::HashSet;
use std::sync::Arc;

use super::helpers::same_value;
use crate::Decimal;
use crate::value::{Collection, Value};

/// The part of a value that decides its FHIRPath equality, for the kinds
/// where that is exact: `values_equal` on two of them is `Some(key == key)`.
/// A keyed value never equals an unkeyed one (temporal, Quantity, complex).
#[derive(Hash, PartialEq, Eq)]
enum EqualityKey {
    Bool(bool),
    /// N1 §6.1.1 "Decimal: values must be equal, trailing zeroes after the
    /// decimal are ignored", and Integer converts implicitly to Decimal: the
    /// mantissa and scale with every trailing zero stripped, so `1`, `1L`,
    /// `1.0` and `1.00` are (1, 0), and `950` and `9.5e2` are (95, -1).
    Number(i128, i64),
    String(Arc<str>),
}

/// The one (mantissa, scale) pair of `value` whose mantissa does not end in 0.
fn number_key(value: Decimal) -> EqualityKey {
    let (mut mantissa, mut scale) = (value.mantissa(), i64::from(value.scale()));
    if mantissa == 0 {
        return EqualityKey::Number(0, 0);
    }
    // A non-zero i128 has at most 38 trailing zeros.
    while mantissa % 10 == 0 {
        mantissa /= 10;
        scale -= 1;
    }
    EqualityKey::Number(mantissa, scale)
}

/// shortcut: temporal, Quantity and complex values have no key, so they keep
/// the linear scan; add a key when one is proven exact.
fn equality_key(value: &Value) -> Option<EqualityKey> {
    match value.primitive_value() {
        Value::Bool(b) => Some(EqualityKey::Bool(*b)),
        Value::Integer(i) | Value::Long(i) => Some(number_key(Decimal::from(*i))),
        Value::Decimal(d) => Some(number_key(*d)),
        Value::String(s) => Some(EqualityKey::String(Arc::clone(s))),
        _ => None,
    }
}

/// Whether [`ValueSet::contains`] answers `value` from the hash index.
pub(super) fn is_keyed(value: &Value) -> bool {
    equality_key(value).is_some()
}

/// Values in insertion order; [`ValueSet::contains`] gives the same answer as
/// a scan with [`same_value`]`(member, value)`.
#[derive(Default)]
pub(super) struct ValueSet {
    values: Collection,
    keys: HashSet<EqualityKey>,
}

impl ValueSet {
    pub(super) fn contains(&self, value: &Value) -> bool {
        match equality_key(value) {
            Some(key) => self.keys.contains(&key),
            None => self.values.iter().any(|member| same_value(member, value)),
        }
    }

    /// Adds `value`, even when an equal one is already a member.
    pub(super) fn push(&mut self, value: Value) {
        if let Some(key) = equality_key(&value) {
            self.keys.insert(key);
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
}
