//! Quantities with exact Decimal amounts (N1 §4.1.8: "the `value` component
//! is defined as a `Decimal`"): `*` and `/` with UCUM unit algebra (N1
//! §6.6), and the exact unit conversion behind `=`, `~` and the order
//! operators (N1 §6.1, §6.2).

use std::cmp::Ordering;
use std::sync::Arc;

use super::ArithOp;
use super::helpers::as_quantity;
use crate::Decimal;
use crate::error::EvalError;
use crate::ucum::{Dimension, Ratio};
use crate::value::{Collection, Value, quantity_unit_factor, ucum_unit};

/// N1 §6.6 `*` and `/` with a Quantity operand: the amounts combine as
/// numbers and the units through UCUM (`12 'cm2' / 3 'cm'` is `4 'cm'`); an
/// Integer or Decimal has unit `'1'`, which leaves the other unit as it is.
/// A Long is not a number here: v3 §5.5 defines no Long to Quantity
/// conversion ("-"), as for the comparison operators. Empty on
/// division by zero (N1 §6.6), for a Quantity without a unit, and when the
/// units have no UCUM product (unknown, or a year or month keyword); an
/// amount past 38 digits is an overflow error, as for Decimal arithmetic.
/// `None` when neither operand is a Quantity or one is not a number.
pub(super) fn quantity_product(
    lv: &Value,
    rv: &Value,
    op: &ArithOp,
) -> Option<Result<Collection, EvalError>> {
    let (left, right) = (as_quantity(lv), as_quantity(rv));
    if left.is_none() && right.is_none() {
        return None;
    }
    let ((x, x_unit), (y, y_unit)) = (left.or_else(|| number(lv))?, right.or_else(|| number(rv))?);
    let power = match op {
        ArithOp::Div => -1,
        ArithOp::Mul => 1,
        _ => return None,
    };
    let (Some(x_unit), Some(y_unit)) = (x_unit, y_unit) else {
        return Some(Ok(vec![]));
    };
    let unit = match (x_unit, y_unit) {
        (unit, "1") => Some(unit.to_owned()),
        ("1", unit) if power == 1 => Some(unit.to_owned()),
        _ => ucum_unit(x_unit, false)
            .zip(ucum_unit(y_unit, false))
            .and_then(|(x_unit, y_unit)| crate::ucum::combine(x_unit, y_unit, power)),
    };
    let Some(unit) = unit.filter(|_| !(power == -1 && y.is_zero())) else {
        return Some(Ok(vec![]));
    };
    // Exact, like Decimal `*` and `/`.
    let amount = if power == 1 {
        x.checked_mul(y)
    } else {
        x.checked_div(y)
    };
    Some(
        amount
            .map(|amount| vec![Value::Quantity(amount, Arc::from(unit))])
            .ok_or_else(|| EvalError::Type("decimal overflow".into())),
    )
}

/// An Integer or Decimal as a Quantity amount with unit `'1'` (N1 §5.5:
/// both convert implicitly; §5.5.7 gives the default unit). v3 §5.5 has no
/// Long to Quantity conversion ("-"), so a Long is `None`.
fn number(v: &Value) -> Option<(Decimal, Option<&str>)> {
    match v {
        Value::Integer(_) | Value::Decimal(_) => Some((v.as_decimal()?, Some("1"))),
        _ => None,
    }
}

/// N1 §6.1, §6.2 on two Quantities: `Some(Some((x, y, factor)))` when `b`'s
/// unit converts into `a`'s (`y * factor` is `b` in `a`'s unit), `Some(None)`
/// when they cannot be compared (no conversion, or a unit on one side only),
/// `None` when neither is a Quantity, or the other is not a number.
pub(super) fn quantity_operands(
    a: &Value,
    b: &Value,
    equivalent: bool,
) -> Option<Option<(Decimal, Decimal, Ratio)>> {
    let ((x, x_unit), (y, y_unit)) = match (as_quantity(a), as_quantity(b)) {
        (Some(x), Some(y)) => (x, y),
        (Some(x), None) => (x, number(b)?),
        (None, Some(y)) => (number(a)?, y),
        (None, None) => return None,
    };
    let factor = match (x_unit, y_unit) {
        (None, None) => Some(Ratio::ONE),
        (Some(x_unit), Some(y_unit)) => quantity_unit_factor(y_unit, x_unit, equivalent),
        _ => None,
    };
    Some(factor.map(|factor| (x, y, factor)))
}

/// `a` against `b` converted into `a`'s unit, exactly: `x` against
/// `y * n / d` is `x * d` against `y * n`, as the factor's `d` is positive.
/// `Some(None)` when they cannot be compared, `None` as for
/// [`quantity_operands`]. When the factor or a cross product overflows,
/// equal amounts in base units are still equal, so `=` and the order
/// operators always agree with the set key built from [`quantity_base`].
/// shortcut: past 38 significant digits, unequal amounts have no order
/// (empty) in the direction that overflows, which can differ from the
/// other operand order; an overflowing f64 had none either. Upgrade with a
/// wider Decimal.
pub(super) fn quantity_order(a: &Value, b: &Value) -> Option<Option<Ordering>> {
    let operands = quantity_operands(a, b, false)?;
    let exact = operands.and_then(|(x, y, factor)| {
        if factor == Ratio::ONE {
            return Some(x.cmp(&y));
        }
        let x = x.trim().checked_mul(whole(factor.denom()))?;
        let y = y.trim().checked_mul(whole(factor.numer()))?;
        Some(x.cmp(&y))
    });
    Some(exact.or_else(|| {
        let (x, y) = (quantity_base(a)?, quantity_base(b)?);
        (x == y).then_some(Ordering::Equal)
    }))
}

/// N1 §6.1.1 `=` with a Quantity operand: [`quantity_order`], except that
/// amounts of one dimension whose cross products overflow are still known
/// to differ when their amounts in base units differ.
pub(super) fn quantities_equal(a: &Value, b: &Value) -> Option<Option<bool>> {
    let order = quantity_order(a, b)?;
    Some(order.map(Ordering::is_eq).or_else(|| {
        let ((x, x_dimension), (y, y_dimension)) = (quantity_base(a)?, quantity_base(b)?);
        (x_dimension == y_dimension).then_some(x == y)
    }))
}

/// The amount of a Quantity (or of an Integer or Decimal, unit `'1'`) in
/// UCUM base units, with its dimension: two such values are `=` exactly
/// when both parts match. `None` for a unit with no base form: unknown,
/// special (`Cel`), a year or month (equal only to the same keyword), no
/// unit at all.
/// shortcut: also `None` when the exact i128 ratio overflows, which leaves
/// the value to the linear scan; upgrade with a wider ratio.
pub(super) fn quantity_base(v: &Value) -> Option<(Ratio, Dimension)> {
    let (amount, unit) = as_quantity(v).or_else(|| number(v))?;
    let (factor, dimension) = crate::ucum::base(ucum_unit(unit?, false)?)?;
    let amount = amount.trim();
    let places = Ratio::new(10, 1)?.pow(amount.scale().checked_neg()?)?;
    let amount = Ratio::new(amount.mantissa(), 1)?.mul(places)?;
    Some((amount.mul(factor)?, dimension))
}

/// N1 §6.1.2 `~` on two comparable Quantities: equal once both are rounded
/// to the precision of the least precise one (trailing zeros ignored, as
/// for Decimals), measured in `x`'s unit so the result does not depend on
/// the operand order (`4 'g' ~ 4040 'mg'`). Exact: the step is `10^-sx`,
/// or `10^-sy * n / d` when `y`'s last digit is coarser in `x`'s unit, and
/// both amounts are rounded to whole steps half away from zero.
/// shortcut: `false` past 38 significant digits; upgrade with a wider
/// Decimal.
pub(super) fn quantities_equivalent(x: Decimal, y: Decimal, factor: Ratio) -> bool {
    let (x, y) = (x.trim(), y.trim());
    let (numer, denom) = (whole(factor.numer()), whole(factor.denom()));
    // The step is `step / step_denom`: 10^-sx, or 10^-sy * n / d, whichever
    // is larger (compared times d).
    let (step, step_denom) =
        if Decimal::new(factor.denom(), x.scale()) >= Decimal::new(factor.numer(), y.scale()) {
            (Decimal::new(1, x.scale()), Decimal::from(1))
        } else {
            (Decimal::new(factor.numer(), y.scale()), denom)
        };
    let steps_x = x
        .checked_mul(step_denom)
        .and_then(|p| rounded_quotient(p, step));
    let steps_y = y
        .checked_mul(numer)
        .and_then(|p| p.checked_mul(step_denom))
        .zip(denom.checked_mul(step))
        .and_then(|(p, q)| rounded_quotient(p, q));
    steps_x.zip(steps_y).is_some_and(|(x, y)| x == y)
}

/// `n` with its trailing zeros moved into the scale, so a product with a
/// prefix factor such as 10^12 shifts the scale instead of overflowing.
pub(super) fn whole(n: i128) -> Decimal {
    let (mut mantissa, mut scale) = (n, 0);
    while mantissa != 0 && mantissa % 10 == 0 {
        mantissa /= 10;
        scale -= 1;
    }
    Decimal::new(mantissa, scale)
}

/// `p / q` rounded to a whole number, half away from zero; `None` on
/// overflow. `q` is positive.
fn rounded_quotient(p: Decimal, q: Decimal) -> Option<Decimal> {
    let whole = p.checked_div_integer(q)?;
    let rest = p.checked_sub(whole.checked_mul(q)?)?.checked_abs()?;
    if rest.checked_add(rest)? < q {
        return Some(whole);
    }
    let away = if p.mantissa() < 0 { -1 } else { 1 };
    whole.checked_add(Decimal::from(away))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_quotient_rounds_half_away_from_zero() {
        let d = |text: &str| text.parse::<Decimal>().unwrap();
        assert_eq!(rounded_quotient(d("2.5"), d("1")), Some(d("3")));
        assert_eq!(rounded_quotient(d("-2.5"), d("1")), Some(d("-3")));
        assert_eq!(rounded_quotient(d("2.49"), d("1")), Some(d("2")));
        assert_eq!(rounded_quotient(d("-7"), d("2")), Some(d("-4")));
        assert_eq!(rounded_quotient(d("1"), d("3")), Some(d("0")));
    }
}
