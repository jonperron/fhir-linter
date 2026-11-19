//! Typing of XML primitive text, where every value is a string (xml.html), by the
//! element's declared FHIR type, so XML values match their JSON counterparts.

use std::sync::Arc;

use fhir_parser::Value;

/// The JSON value for XML text of primitive type `ty`; the text itself when it does not
/// fit, so the type check still reports it. Public so the validator's XML reshape types
/// text exactly as FHIRPath does.
pub fn typed_xml_text(text: &Arc<str>, ty: &str) -> Value {
    let keep = || Value::Str(Arc::clone(text));
    match ty {
        "boolean" => match text.as_ref() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => keep(),
        },
        // Official integer regex `[0]|[-+]?[1-9][0-9]*`.
        "integer" | "unsignedInt" | "positiveInt" => {
            let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
            let canonical = text.as_ref() == "0"
                || (digits.bytes().all(|b| b.is_ascii_digit()) && !digits.starts_with('0'));
            match text.parse::<i64>() {
                Ok(n) if canonical => Value::Integer(n),
                _ => keep(),
            }
        }
        // Exact with its written scale, or the f64 value beyond 38 digits, as
        // in JSON; beyond f64 it stays text.
        "decimal" if is_decimal(text) => match text.parse::<f64>() {
            Ok(float) if float.is_finite() => text
                .parse()
                .ok()
                .or_else(|| fhir_parser::Decimal::from_f64(float))
                .map_or_else(keep, Value::Decimal),
            _ => keep(),
        },
        _ => keep(),
    }
}

/// Official decimal regex `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`.
fn is_decimal(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (mantissa, exponent) = s
        .split_once(['e', 'E'])
        .map_or((s, None), |(m, e)| (m, Some(e)));
    let (int, frac) = mantissa
        .split_once('.')
        .map_or((mantissa, None), |(i, f)| (i, Some(f)));
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    (int == "0" || (digits(int) && !int.starts_with('0')))
        && frac.is_none_or(digits)
        && exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_converts_only_well_formed_text() {
        let v = |t: &str, ty| typed_xml_text(&Arc::from(t), ty);
        assert!(matches!(v("2", "integer"), Value::Integer(2)));
        assert!(matches!(v("007", "integer"), Value::Str(_)));
        assert!(matches!(v("1.50", "decimal"), Value::Decimal(_)));
        assert!(matches!(v("1e999", "decimal"), Value::Str(_)));
        assert!(matches!(v("NaN", "decimal"), Value::Str(_)));
        assert!(matches!(v("false", "boolean"), Value::Bool(false)));
        assert!(matches!(v("yes", "boolean"), Value::Str(_)));
    }
}
