//! Exact decimal that keeps the scale it was written with: FHIR R5 `decimal`
//! ("the precision of the decimal value has significance") and FHIRPath N1
//! `Decimal` (exact, not binary floating point). `12.500` keeps three places;
//! `9.5e2` is precise to the tens.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Beyond this many places either way, `Display` writes an exponent instead of
/// expanding zeros, so a hostile `1e-999999999` stays a short string.
const MAX_EXPANDED_SCALE: u32 = 1000;

/// The least number of places a quotient carries (N1: at least 8).
const DIVISION_SCALE: i32 = 8;

/// `mantissa × 10^-scale`: `12.500` is (12500, 3) and `9.5e2` is (95, -1).
///
/// The i128 mantissa holds 38 digits, more than the 35 the R5 decimal regex
/// allows; every operation is checked and returns `None` on overflow.
/// Equality and order compare values, so `1.0 == 1.00`.
#[derive(Debug, Clone, Copy)]
pub struct Decimal {
    mantissa: i128,
    scale: i32,
}

/// A decimal text that is malformed or does not fit a 38-digit mantissa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid or out-of-range decimal")]
pub struct ParseDecimalError;

/// How `Decimal::rescale` drops digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    HalfAwayFromZero,
    TowardZero,
    Floor,
    Ceiling,
}

/// The signed quotient of two mantissas from its magnitude; `None` past i128.
fn signed_quotient(a: i128, b: i128, magnitude: u128) -> Option<i128> {
    let negative = (a < 0) != (b < 0);
    if negative && magnitude == i128::MIN.unsigned_abs() {
        return Some(i128::MIN);
    }
    let magnitude = i128::try_from(magnitude).ok()?;
    Some(if negative { -magnitude } else { magnitude })
}

fn pow10(exponent: i64) -> Option<i128> {
    u32::try_from(exponent)
        .ok()
        .and_then(|e| 10i128.checked_pow(e))
}

/// `(a + b) mod m` for `a, b < m`, without overflowing u128.
fn add_mod(a: u128, b: u128, m: u128) -> u128 {
    if a >= m - b { a - (m - b) } else { a + b }
}

/// `(a * b) mod m` for `a, b < m`, by doubling: no 256-bit intermediate.
fn mul_mod(a: u128, mut b: u128, m: u128) -> u128 {
    let (mut result, mut base) = (0, a);
    while b > 0 {
        if b & 1 == 1 {
            result = add_mod(result, base, m);
        }
        base = add_mod(base, base, m);
        b >>= 1;
    }
    result
}

/// `10^k mod m` by squaring.
fn pow10_mod(mut k: u64, m: u128) -> u128 {
    let (mut result, mut base) = (1 % m, 10 % m);
    while k > 0 {
        if k & 1 == 1 {
            result = mul_mod(result, base, m);
        }
        base = mul_mod(base, base, m);
        k >>= 1;
    }
    result
}

/// `n × 10^k / d` truncated, digit by digit so the product never has to fit;
/// `None` only when the quotient itself exceeds u128. `d` is not zero.
fn scaled_quotient(n: u128, k: u64, d: u128) -> Option<u128> {
    scaled_quotient_rem(n, k, d).map(|(quotient, _)| quotient)
}

/// [`scaled_quotient`] and the remainder `n × 10^k mod d`.
fn scaled_quotient_rem(n: u128, k: u64, d: u128) -> Option<(u128, u128)> {
    let (mut quotient, mut rest) = (n / d, n % d);
    let mut left = k;
    // Each step brings down one digit; a non-zero quotient overflows within
    // about 40 steps, so the loop is short even for a huge `k`.
    while left > 0 && rest != 0 {
        // 10 × rest = digit × d + next, by ten additions mod d.
        let (mut digit, mut next) = (0, 0);
        for _ in 0..10 {
            if next >= d - rest {
                next -= d - rest;
                digit += 1;
            } else {
                next += rest;
            }
        }
        quotient = quotient.checked_mul(10)?.checked_add(digit)?;
        rest = next;
        left -= 1;
    }
    // The loop stops at `left == 0` or at a zero remainder.
    if quotient == 0 || left == 0 {
        return Some((quotient, rest));
    }
    let quotient = quotient.checked_mul(10u128.checked_pow(u32::try_from(left).ok()?)?)?;
    Some((quotient, 0))
}

impl Decimal {
    pub const fn new(mantissa: i128, scale: i32) -> Self {
        Self { mantissa, scale }
    }

    pub fn mantissa(self) -> i128 {
        self.mantissa
    }

    /// Places after the point as written; negative when an exponent puts the
    /// last significant digit left of the units (`9.5e2` is -1).
    pub fn scale(self) -> i32 {
        self.scale
    }

    pub fn is_zero(self) -> bool {
        self.mantissa == 0
    }

    /// The shortest decimal text that round-trips `value`; `None` when not finite.
    pub fn from_f64(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        // Plain form keeps 950.0 at scale 0; beyond 38 digits use the exponent.
        format!("{value}")
            .parse()
            .or_else(|_| format!("{value:e}").parse())
            .ok()
    }

    /// The nearest f64; infinite or zero when the value is beyond its range.
    pub fn to_f64(self) -> f64 {
        format!("{}e{}", self.mantissa, -i64::from(self.scale))
            .parse()
            .unwrap_or(f64::NAN)
    }

    pub fn checked_neg(self) -> Option<Self> {
        Some(Self::new(self.mantissa.checked_neg()?, self.scale))
    }

    pub fn checked_abs(self) -> Option<Self> {
        Some(Self::new(self.mantissa.checked_abs()?, self.scale))
    }

    /// Both mantissas at the larger scale.
    fn aligned(self, other: Self) -> Option<(i128, i128, i32)> {
        let scale = self.scale.max(other.scale);
        let up = |d: Self| match d.mantissa {
            0 => Some(0),
            m => m.checked_mul(pow10(i64::from(scale) - i64::from(d.scale))?),
        };
        Some((up(self)?, up(other)?, scale))
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        let (a, b, scale) = self.aligned(other)?;
        Some(Self::new(a.checked_add(b)?, scale))
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        let (a, b, scale) = self.aligned(other)?;
        Some(Self::new(a.checked_sub(b)?, scale))
    }

    pub fn checked_mul(self, other: Self) -> Option<Self> {
        Some(Self::new(
            self.mantissa.checked_mul(other.mantissa)?,
            self.scale.checked_add(other.scale)?,
        ))
    }

    /// The quotient truncated to a whole number (FHIRPath `div`); `None` for a
    /// zero divisor.
    pub fn checked_div_integer(self, other: Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(Self::new(0, 0));
        }
        // The quotient is (a / b) × 10^e with e = other.scale - self.scale.
        let e = i64::from(other.scale) - i64::from(self.scale);
        let quotient = if e >= 0 {
            let magnitude = scaled_quotient(
                self.mantissa.unsigned_abs(),
                u64::try_from(e).ok()?,
                other.mantissa.unsigned_abs(),
            )?;
            signed_quotient(self.mantissa, other.mantissa, magnitude)?
        } else {
            // A divisor beyond i128 exceeds any mantissa: the quotient is 0.
            // (5 divides it, so it cannot be exactly 2^127 either.)
            match other.mantissa.checked_mul(pow10(-e).unwrap_or(0)) {
                Some(d) if d != 0 => self.mantissa.checked_div(d)?,
                _ => 0,
            }
        };
        Some(Self::new(quotient, 0))
    }

    /// Truncated remainder with the sign of `self`; `None` for a zero divisor.
    pub fn checked_rem(self, other: Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        match self.aligned(other) {
            // `wrapping_rem`: `i128::MIN % -1` is 0, not an overflow.
            Some((a, b, scale)) => Some(Self::new(a.wrapping_rem(b), scale)),
            // The divisor outgrew i128 at the finer scale: it exceeds `self`.
            None if self.scale >= other.scale => Some(self),
            // `self` outgrew i128 at the divisor's finer scale: its residue is
            // (a mod b) × (10^k mod b) mod b, and |result| < |b| always fits.
            None => {
                let m = other.mantissa.unsigned_abs();
                let k = u64::try_from(i64::from(other.scale) - i64::from(self.scale)).ok()?;
                let residue = mul_mod(self.mantissa.unsigned_abs() % m, pow10_mod(k, m), m);
                let residue = i128::try_from(residue).ok()?;
                let sign = if self.mantissa < 0 { -1 } else { 1 };
                Some(Self::new(residue * sign, other.scale))
            }
        }
    }

    /// The quotient rounded half away from zero to at least 8 places (or the
    /// operands' own scale), trailing zeros dropped; `None` for a zero divisor.
    /// When those places do not fit 38 digits beside the integer part, it keeps
    /// as many as fit (N1: `/` always gives a Decimal).
    pub fn checked_div(self, other: Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(Self::new(0, 0));
        }
        let target = DIVISION_SCALE.max(self.scale).max(other.scale);
        self.quotient_at(other, target).or_else(|| {
            let whole = self.checked_div_integer(other)?.mantissa.unsigned_abs();
            let digits = whole.checked_ilog10().map_or(0, |log| log as i32 + 1);
            // Integer digits and the places stay within 38 digits.
            let fitting = (38 - digits).max(0);
            (fitting < target)
                .then(|| self.quotient_at(other, fitting))
                .flatten()
        })
    }

    /// `self / other` rounded half away from zero to `target` places, trailing
    /// zeros dropped; `None` when it overflows. `other` is not zero.
    fn quotient_at(self, other: Self, target: i32) -> Option<Self> {
        // Rounded from the remainder, not a guard digit: a guard digit would
        // overflow for a quotient that itself fits (`1e38 / 1`).
        let shift = i64::from(target) + i64::from(other.scale) - i64::from(self.scale);
        let (numerator, divisor) = (self.mantissa.unsigned_abs(), other.mantissa.unsigned_abs());
        let (magnitude, remainder, divisor) = if shift >= 0 {
            let (magnitude, remainder) =
                scaled_quotient_rem(numerator, u64::try_from(shift).ok()?, divisor)?;
            (magnitude, remainder, divisor)
        } else {
            // A divisor beyond u128 exceeds twice any mantissa: 0, rounded down.
            let wide = u32::try_from(-shift)
                .ok()
                .and_then(|e| 10u128.checked_pow(e))
                .and_then(|p| divisor.checked_mul(p));
            match wide {
                Some(wide) => (numerator / wide, numerator % wide, wide),
                None => (0, 0, 1),
            }
        };
        // Half away from zero: round up when 2 x remainder >= divisor.
        let magnitude = if remainder >= divisor - remainder {
            magnitude.checked_add(1)?
        } else {
            magnitude
        };
        let quotient = signed_quotient(self.mantissa, other.mantissa, magnitude)?;
        Some(Self::new(quotient, target).trim())
    }

    /// The same value without trailing zeros after the point.
    pub fn trim(self) -> Self {
        // A zero would loop once per place: `0e-2000000000` is valid FHIR.
        if self.mantissa == 0 {
            return Self::new(0, self.scale.min(0));
        }
        // A non-zero i128 has at most 38 trailing zeros.
        let mut trimmed = self;
        while trimmed.scale > 0 && trimmed.mantissa % 10 == 0 {
            trimmed = Self::new(trimmed.mantissa / 10, trimmed.scale - 1);
        }
        trimmed
    }

    /// This value at `scale` places: exact when adding places, `rounding` when
    /// dropping them; `None` on overflow.
    pub fn rescale(self, scale: i32, rounding: Rounding) -> Option<Self> {
        let m = self.mantissa;
        let diff = i64::from(self.scale) - i64::from(scale);
        if diff <= 0 {
            let up = if m == 0 {
                0
            } else {
                m.checked_mul(pow10(-diff)?)?
            };
            return Some(Self::new(up, scale));
        }
        // A divisor beyond i128 exceeds any mantissa: the quotient is zero.
        let divisor = pow10(diff);
        let (quotient, remainder) = divisor.map_or((0, m), |p| (m / p, m % p));
        let away = match rounding {
            Rounding::TowardZero => false,
            Rounding::Floor => remainder < 0,
            Rounding::Ceiling => remainder > 0,
            Rounding::HalfAwayFromZero => {
                divisor.is_some_and(|p| remainder.unsigned_abs() * 2 >= p.unsigned_abs())
            }
        };
        let quotient = if away {
            quotient.checked_add(m.signum())?
        } else {
            quotient
        };
        Some(Self::new(quotient, scale))
    }

    /// The whole number `rounding` gives, if it fits an i64.
    pub fn to_i64(self, rounding: Rounding) -> Option<i64> {
        i64::try_from(self.rescale(0, rounding)?.mantissa).ok()
    }
}

impl From<i64> for Decimal {
    fn from(value: i64) -> Self {
        Self::new(i128::from(value), 0)
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Decimal {}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Some((a, b, _)) = self.aligned(*other) {
            return a.cmp(&b);
        }
        // Scales too far apart to align: compare sign, then the position of
        // the leading digit, then the significant digits.
        let sign = self.mantissa.signum().cmp(&other.mantissa.signum());
        if sign != Ordering::Equal {
            return sign;
        }
        let magnitude = |d: &Self| {
            let digits = d.mantissa.unsigned_abs().to_string();
            let leading = i64::try_from(digits.len()).unwrap_or(i64::MAX) - i64::from(d.scale);
            (leading, digits.trim_end_matches('0').to_owned())
        };
        let order = magnitude(self).cmp(&magnitude(other));
        if self.mantissa < 0 {
            order.reverse()
        } else {
            order
        }
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.mantissa < 0 { "-" } else { "" };
        let digits = self.mantissa.unsigned_abs().to_string();
        if self.scale.unsigned_abs() > MAX_EXPANDED_SCALE {
            return write!(f, "{sign}{digits}e{}", -i64::from(self.scale));
        }
        match usize::try_from(self.scale) {
            Ok(0) | Err(_) if self.mantissa == 0 => f.write_str("0"),
            Ok(0) | Err(_) => {
                let zeros = self.scale.unsigned_abs() as usize;
                write!(f, "{sign}{digits}{}", "0".repeat(zeros))
            }
            Ok(places) => {
                let padded = format!("{digits:0>width$}", width = places + 1);
                let (whole, fraction) = padded.split_at(padded.len() - places);
                write!(f, "{sign}{whole}.{fraction}")
            }
        }
    }
}

impl FromStr for Decimal {
    type Err = ParseDecimalError;

    /// `[+-]?digits(.digits)?([eE][+-]?digits)?`, the FHIR R5 and FHIRPath
    /// decimal forms; callers check their own stricter grammar first.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (negative, body) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        let (number, exponent) = match body.split_once(['e', 'E']) {
            Some((number, exponent)) => (
                number,
                exponent.parse::<i64>().map_err(|_| ParseDecimalError)?,
            ),
            None => (body, 0),
        };
        let (whole, fraction) = match number.split_once('.') {
            Some((_, "")) => return Err(ParseDecimalError),
            Some(parts) => parts,
            None => (number, ""),
        };
        let is_digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
        if whole.is_empty() || !is_digits(whole) || !is_digits(fraction) {
            return Err(ParseDecimalError);
        }
        let magnitude = whole
            .bytes()
            .chain(fraction.bytes())
            .try_fold(0i128, |acc, b| {
                acc.checked_mul(10)?.checked_add(i128::from(b - b'0'))
            })
            .ok_or(ParseDecimalError)?;
        let scale = i64::try_from(fraction.len())
            .ok()
            .and_then(|places| places.checked_sub(exponent))
            .and_then(|scale| i32::try_from(scale).ok())
            .ok_or(ParseDecimalError)?;
        Ok(Self::new(
            if negative { -magnitude } else { magnitude },
            scale,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(text: &str) -> Decimal {
        text.parse().unwrap_or_else(|_| panic!("{text}"))
    }

    #[test]
    fn parse_keeps_the_written_scale() {
        assert_eq!(dec("12.500").to_string(), "12.500");
        assert_eq!(dec("12.500").scale(), 3);
        assert_eq!(dec("9.5e2").scale(), -1);
        assert_eq!(dec("9.5e2").to_string(), "950");
        assert_eq!(dec("1.5E-3").to_string(), "0.0015");
        assert_eq!(dec("-0.0").to_string(), "0.0");
        assert_eq!(dec("+1.5").to_string(), "1.5");
        // R5's widest decimal: 18 whole and 17 fraction digits.
        let widest = "-999999999999999999.99999999999999999";
        assert_eq!(dec(widest).to_string(), widest);
    }

    #[test]
    fn parse_rejects_malformed_or_oversized_text() {
        for text in [
            "",
            "-",
            ".5",
            "1.",
            "1e",
            "1e+",
            "a",
            "1.2.3",
            "1e99999999999",
        ] {
            assert!(text.parse::<Decimal>().is_err(), "{text}");
        }
        assert!("1".repeat(40).parse::<Decimal>().is_err());
    }

    #[test]
    fn huge_exponents_display_without_expanding() {
        assert_eq!(dec("1e-999999999").to_string(), "1e-999999999");
        assert!(dec("1e-999999999") < dec("1e-999999998"));
        assert!(dec("-1e999999999") < dec("1"));
        assert!(dec("1e-999999999") > Decimal::from(0));
    }

    #[test]
    fn zero_with_a_huge_scale_trims_and_divides_at_once() {
        // Valid FHIR (exponent of up to 9 digits); must not loop per place.
        let zero = dec("0e-2000000000");
        let started = std::time::Instant::now();
        assert_eq!(zero.trim().scale(), 0);
        assert!(zero.checked_div(dec("1")).unwrap().is_zero());
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn integer_division_truncates_without_rounding_overflow() {
        let big = dec("123456789012345678901234567890");
        let quotient = big.checked_div_integer(dec("1e25")).unwrap();
        assert_eq!(quotient.to_string(), "12345");
        assert_eq!(
            dec("-7.5")
                .checked_div_integer(dec("2"))
                .unwrap()
                .to_string(),
            "-3"
        );
        assert!(dec("1").checked_div_integer(dec("0")).is_none());
    }

    #[test]
    fn values_compare_whatever_the_scale() {
        assert_eq!(dec("1.0"), dec("1.00"));
        assert_eq!(dec("9.5e2"), dec("950"));
        assert!(dec("-1.5") < dec("-1.25"));
    }

    #[test]
    fn arithmetic_is_exact_and_checked() {
        assert_eq!(
            dec("1.8").checked_sub(dec("1.2")).unwrap().to_string(),
            "0.6"
        );
        assert_eq!(
            dec("1.2").checked_rem(dec("0.5")).unwrap().to_string(),
            "0.2"
        );
        assert_eq!(
            dec("1.5").checked_mul(dec("2.0")).unwrap().to_string(),
            "3.00"
        );
        assert_eq!(
            dec("1").checked_div(dec("3")).unwrap().to_string(),
            "0.33333333"
        );
        assert_eq!(
            dec("2").checked_div(dec("3")).unwrap().to_string(),
            "0.66666667"
        );
        assert_eq!(dec("6").checked_div(dec("2")).unwrap().to_string(), "3");
        assert!(dec("1").checked_div(dec("0")).is_none());
        assert!(dec("1").checked_rem(dec("0")).is_none());
        let big = Decimal::new(i128::MAX, 0);
        assert!(big.checked_add(dec("1")).is_none());
        assert!(big.checked_mul(dec("10")).is_none());
    }

    #[test]
    fn rescale_rounds_as_asked() {
        let cases = [
            ("2.5", Rounding::HalfAwayFromZero, "3"),
            ("-2.5", Rounding::HalfAwayFromZero, "-3"),
            ("-2.5", Rounding::TowardZero, "-2"),
            ("-2.1", Rounding::Floor, "-3"),
            ("2.1", Rounding::Ceiling, "3"),
            ("1e-999999999", Rounding::Ceiling, "1"),
        ];
        for (text, rounding, expected) in cases {
            let rounded = dec(text).rescale(0, rounding).unwrap();
            assert_eq!(rounded.to_string(), expected, "{text} {rounding:?}");
        }
        assert_eq!(
            dec("1.5").rescale(3, Rounding::Floor).unwrap().to_string(),
            "1.500"
        );
        assert_eq!(dec("1e30").to_i64(Rounding::Floor), None);
    }

    #[test]
    fn f64_conversion_uses_the_shortest_text() {
        assert_eq!(
            Decimal::from_f64(0.1 + 0.2).unwrap().to_string(),
            "0.30000000000000004"
        );
        assert_eq!(Decimal::from_f64(950.0).unwrap().scale(), 0);
        assert_eq!(Decimal::from_f64(1e300).unwrap().scale(), -300);
        assert!(Decimal::from_f64(f64::INFINITY).is_none());
        assert_eq!(dec("12.5").to_f64(), 12.5);
    }

    #[test]
    fn far_apart_scales_still_divide() {
        // N1 §6.6 `div`/`/`: exact quotients, however far apart the scales.
        let (tiny, huge) = (dec("1e-30"), dec("1e30"));
        assert_eq!(tiny.checked_div_integer(huge).unwrap(), dec("0"));
        assert_eq!(tiny.checked_div(huge).unwrap().to_string(), "0");
        assert_eq!(dec("0").checked_div(dec("1e-30")).unwrap().to_string(), "0");
        assert_eq!(tiny.checked_rem(huge).unwrap(), tiny);
        let min = Decimal::new(i128::MIN, 0);
        assert_eq!(min.checked_rem(Decimal::from(-1)).unwrap(), dec("0"));
    }

    #[test]
    fn rescale_of_the_minimum_mantissa_does_not_panic() {
        let min = Decimal::new(i128::MIN, 0);
        let half = Rounding::HalfAwayFromZero;
        assert!(min.rescale(-1, half).is_some());
        assert_eq!(min.rescale(-39, half).unwrap().mantissa(), 0);
    }

    #[test]
    fn mod_is_always_representable() {
        // |a mod b| < |b|, so no operand scales may make it overflow.
        assert_eq!(dec("1e30").checked_rem(dec("0.5")).unwrap(), dec("0"));
        assert_eq!(dec("7e30").checked_rem(dec("3")).unwrap(), dec("1"));
        assert_eq!(dec("-7e30").checked_rem(dec("3")).unwrap(), dec("-1"));
        assert_eq!(dec("1e1000000").checked_rem(dec("7")).unwrap(), dec("4"));
    }

    #[test]
    fn div_overflows_only_when_the_quotient_does() {
        // 1e20 over 1e18 is 100, though 10^39 never fits an i128 on the way.
        let divisor = Decimal::new(10i128.pow(37), 19);
        assert_eq!(
            dec("1e20").checked_div_integer(divisor).unwrap(),
            dec("100")
        );
        assert_eq!(dec("1e20").checked_div(divisor).unwrap().to_string(), "100");
        // 1e30 over 1e-30 is 10^60: beyond 38 digits, so the overflow error is
        // the right answer, not a lost result.
        assert!(dec("1e30").checked_div_integer(dec("1e-30")).is_none());
        assert!(dec("1e30").checked_div(dec("1e-30")).is_none());
    }

    #[test]
    fn a_wide_quotient_keeps_the_places_that_fit() {
        // N1 §6.6 `/`: the result is always a Decimal. 1e31 / 3 has 31 integer
        // digits, so 7 places fit beside them in 38 digits, not 8.
        assert_eq!(
            dec("1e31").checked_div(dec("3")).unwrap().to_string(),
            "3333333333333333333333333333333.3333333"
        );
        assert_eq!(
            dec("-2e31").checked_div(dec("3")).unwrap().to_string(),
            "-6666666666666666666666666666666.6666667"
        );
        // 1e30 / 3 has 30 integer digits: all 8 places fit.
        assert_eq!(
            dec("1e30").checked_div(dec("3")).unwrap().to_string(),
            "333333333333333333333333333333.33333333"
        );
        // A divisor written with 38 places asks for 38 places: 37 fit beside `3`.
        let fine = Decimal::new(3 * 10i128.pow(37), 38);
        assert_eq!(
            dec("1").checked_div(fine).unwrap().to_string(),
            format!("3.{}", "3".repeat(37))
        );
        // A hostile scale on the divisor stays a short computation.
        assert!(dec("1").checked_div(dec("1e-2000000000")).is_none());
    }

    #[test]
    fn a_quotient_of_39_digits_divides_without_a_guard_digit() {
        // 10^38 fits an i128, but 10^39 (one guard digit more) does not.
        assert_eq!(dec("1e38").checked_div(dec("1")).unwrap(), dec("1e38"));
        assert_eq!(dec("-1e38").checked_div(dec("1")).unwrap(), dec("-1e38"));
        assert_eq!(
            dec("1.5e38").checked_div(dec("1.0")).unwrap(),
            dec("1.5e38")
        );
        // Rounding comes from the remainder: 1.7e38 / 3 is 5.666..e37.
        assert_eq!(
            dec("1.7e38").checked_div(dec("3")).unwrap().to_string(),
            "56666666666666666666666666666666666667"
        );
        let max = Decimal::new(i128::MAX, 0);
        assert_eq!(max.checked_div(dec("1")).unwrap(), max);
        // Half a unit rounds away from zero: i128::MAX / 2 ends in .5.
        assert_eq!(
            max.checked_div(dec("2")).unwrap(),
            Decimal::new(i128::MAX / 2 + 1, 0)
        );
        assert_eq!(
            max.checked_div(dec("-2")).unwrap(),
            Decimal::new(-(i128::MAX / 2 + 1), 0)
        );
        // The quotient itself beyond i128 still overflows.
        assert!(dec("1e38").checked_div(dec("0.1")).is_none());
    }

    #[test]
    fn a_divisor_with_a_negative_scale_rounds_from_the_remainder() {
        // 5e-9 lies half way between 0 and 1e-8 at 8 places.
        let cases = [
            ("5", "1e9", "0.00000001"),
            ("-5", "1e9", "-0.00000001"),
            ("5", "-1e9", "-0.00000001"),
            ("4", "1e9", "0"),
            ("-1", "8", "-0.125"),
            ("1", "1e2000000000", "0"),
        ];
        for (a, b, quotient) in cases {
            let result = dec(a).checked_div(dec(b)).unwrap().to_string();
            assert_eq!(result, quotient, "{a} / {b}");
        }
    }

    #[test]
    fn the_minimum_mantissa_quotient_still_fits() {
        let min = Decimal::new(i128::MIN, 0);
        assert_eq!(min.checked_div_integer(Decimal::from(1)).unwrap(), min);
        assert!(min.checked_div_integer(Decimal::from(-1)).is_none());
        assert_eq!(dec("5").checked_rem(min).unwrap(), dec("5"));
    }

    #[test]
    fn mod_and_div_keep_signs_and_digits() {
        // MAX × 10^5 overflows; the residue is (MAX mod 7) × (10^5 mod 7) mod 7.
        let max = Decimal::new(i128::MAX, 0);
        assert_eq!(
            max.checked_rem(Decimal::new(7, 5)).unwrap(),
            Decimal::new(5, 5)
        );
        assert_eq!(dec("7e1000000").checked_rem(dec("-3")).unwrap(), dec("1"));
        assert_eq!(dec("-7e1000000").checked_rem(dec("-3")).unwrap(), dec("-1"));
        for (a, b, q) in [("-6", "3", "-2"), ("6", "-3", "-2"), ("-6", "-3", "2")] {
            assert_eq!(dec(a).checked_div_integer(dec(b)).unwrap(), dec(q));
        }
        // A scaling of 10^29 on the numerator, with a repeating quotient.
        assert_eq!(
            dec("1e20").checked_div(dec("3")).unwrap().to_string(),
            "33333333333333333333.33333333"
        );
        assert_eq!(
            dec("0").checked_div_integer(dec("3e-1000000")).unwrap(),
            dec("0")
        );
    }
}
