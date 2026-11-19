//! N1 §6.6 `*` and `/` on Quantities, with UCUM unit algebra.

use std::sync::Arc;

use super::ArithOp;
use super::helpers::as_quantity;
use crate::value::{Collection, Value, ucum_unit};

/// N1 §6.6 `*` and `/` with a Quantity operand: the amounts combine as
/// numbers and the units through UCUM (`12 'cm2' / 3 'cm'` is `4 'cm'`); a
/// number has unit `'1'`, which leaves the other unit as it is. Empty on
/// division by zero, for a Quantity without a unit, and when the units have
/// no UCUM product (unknown, or a year or month keyword). `None` when
/// neither operand is a Quantity or one is not a number.
pub(super) fn quantity_product(lv: &Value, rv: &Value, op: &ArithOp) -> Option<Collection> {
    let (left, right) = (as_quantity(lv), as_quantity(rv));
    if left.is_none() && right.is_none() {
        return None;
    }
    let number = |v: &Value| {
        v.as_decimal()
            .map(|amount| (amount.to_f64(), Some("1")))
            .filter(|(x, _)| x.is_finite())
    };
    let ((x, x_unit), (y, y_unit)) = (left.or_else(|| number(lv))?, right.or_else(|| number(rv))?);
    let (Some(x_unit), Some(y_unit)) = (x_unit, y_unit) else {
        return Some(vec![]);
    };
    let (amount, power) = match op {
        ArithOp::Div if y == 0.0 => return Some(vec![]),
        ArithOp::Div => (x / y, -1),
        ArithOp::Mul => (x * y, 1),
        _ => return None,
    };
    let unit = match (x_unit, y_unit) {
        (unit, "1") => Some(unit.to_owned()),
        ("1", unit) if power == 1 => Some(unit.to_owned()),
        _ => ucum_unit(x_unit, false)
            .zip(ucum_unit(y_unit, false))
            .and_then(|(x_unit, y_unit)| crate::ucum::combine(x_unit, y_unit, power)),
    };
    Some(
        unit.filter(|_| amount.is_finite())
            .map(|unit| Value::Quantity(amount, Arc::from(unit)))
            .into_iter()
            .collect(),
    )
}
