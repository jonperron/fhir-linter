//! Membership under FHIRPath equality, with a hash index for the values whose
//! equality is plain equality of a key, so set operations on large
//! collections of strings, integers or booleans stay linear.
use std::collections::HashSet;
use std::sync::Arc;

use super::helpers::same_value;
use crate::value::{Collection, Value};

/// The part of a value that decides its FHIRPath equality, for the kinds
/// where that is exact: `values_equal` on two of them is `Some(key == key)`,
/// Integer and Long included (`1 = 1L`). A keyed value never equals a
/// complex value, and only equals a Decimal by value (`1 = 1.0`), which the
/// unkeyed scan handles.
#[derive(Hash, PartialEq, Eq)]
enum EqualityKey {
    Bool(bool),
    Integer(i64),
    String(Arc<str>),
}

/// shortcut: Decimal, temporal, Quantity and complex values have no key, so
/// they keep the linear scan; add a key when one is proven exact.
fn equality_key(value: &Value) -> Option<EqualityKey> {
    match value.primitive_value() {
        Value::Bool(b) => Some(EqualityKey::Bool(*b)),
        Value::Integer(i) | Value::Long(i) => Some(EqualityKey::Integer(*i)),
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
    /// Indexes in `values` of the members without a key.
    unkeyed: Vec<usize>,
}

impl ValueSet {
    pub(super) fn contains(&self, value: &Value) -> bool {
        match equality_key(value) {
            // Keyed members match by key; unkeyed ones (a Decimal) may still match.
            Some(key) => {
                self.keys.contains(&key)
                    || self
                        .unkeyed
                        .iter()
                        .any(|&index| same_value(&self.values[index], value))
            }
            None => self.values.iter().any(|member| same_value(member, value)),
        }
    }

    /// Adds `value`, even when an equal one is already a member.
    pub(super) fn push(&mut self, value: Value) {
        match equality_key(&value) {
            Some(key) => {
                self.keys.insert(key);
            }
            None => self.unkeyed.push(self.values.len()),
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
