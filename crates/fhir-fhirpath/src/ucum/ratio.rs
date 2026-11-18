//! Exact rational numbers for unit factors, with checked arithmetic.

/// A reduced fraction `numer / denom` with `denom > 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ratio {
    numer: i128,
    denom: i128,
}

impl Ratio {
    pub(crate) const ONE: Ratio = Ratio { numer: 1, denom: 1 };

    /// Build a reduced ratio; `None` when `denom` is zero or on overflow.
    pub(crate) fn new(numer: i128, denom: i128) -> Option<Self> {
        if denom == 0 {
            return None;
        }
        let divisor = gcd(numer, denom)?;
        let (numer, denom) = (numer / divisor, denom / divisor);
        if denom < 0 {
            return Some(Ratio {
                numer: numer.checked_neg()?,
                denom: denom.checked_neg()?,
            });
        }
        Some(Ratio { numer, denom })
    }

    pub(crate) fn numer(self) -> i128 {
        self.numer
    }

    pub(crate) fn denom(self) -> i128 {
        self.denom
    }

    pub(crate) fn mul(self, other: Ratio) -> Option<Self> {
        // Cross-reduce first so intermediate products overflow less often.
        let left = gcd(self.numer, other.denom)?;
        let right = gcd(other.numer, self.denom)?;
        Ratio::new(
            (self.numer / left).checked_mul(other.numer / right)?,
            (self.denom / right).checked_mul(other.denom / left)?,
        )
    }

    pub(crate) fn recip(self) -> Option<Self> {
        Ratio::new(self.denom, self.numer)
    }

    pub(crate) fn div(self, other: Ratio) -> Option<Self> {
        self.mul(other.recip()?)
    }

    /// Integer power by squaring, so large exponents cost O(log n).
    pub(crate) fn pow(self, exponent: i32) -> Option<Self> {
        let mut base = if exponent < 0 { self.recip()? } else { self };
        let mut remaining = exponent.unsigned_abs();
        let mut result = Ratio::ONE;
        while remaining > 0 {
            if remaining & 1 == 1 {
                result = result.mul(base)?;
            }
            remaining >>= 1;
            if remaining > 0 {
                base = base.mul(base)?;
            }
        }
        Some(result)
    }

    /// Parse a decimal literal such as `133.3220`, `254e-2` or `1e24` exactly.
    pub(crate) fn parse_decimal(text: &str) -> Option<Self> {
        let (mantissa, exponent) = match text.split_once(['e', 'E']) {
            Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
            None => (text, 0),
        };
        let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits = format!("{integer}{fraction}");
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let scale = exponent.checked_sub(i32::try_from(fraction.len()).ok()?)?;
        Ratio::new(digits.parse().ok()?, 1)?.mul(Ratio::new(10, 1)?.pow(scale)?)
    }
}

/// Greatest common divisor as a positive `i128`; `None` if it does not fit.
fn gcd(left: i128, right: i128) -> Option<i128> {
    let (mut left, mut right) = (left.unsigned_abs(), right.unsigned_abs());
    while right != 0 {
        (left, right) = (right, left % right);
    }
    i128::try_from(left.max(1)).ok()
}
