//! Helpers on FHIR type codes shared by the validator and the FHIRPath evaluator.

/// Defined in `fhir-fhirpath` (this crate depends on it); re-exported as the shared entry point.
pub use fhir_fhirpath::is_primitive_type;

/// `code` with its first character upper-cased: the choice suffix it selects
/// (`dateTime` gives `valueDateTime`).
pub fn capitalise(code: &str) -> String {
    let mut chars = code.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalise_upper_cases_only_the_first_character() {
        assert_eq!(capitalise("dateTime"), "DateTime");
        assert_eq!(capitalise("Quantity"), "Quantity");
        assert_eq!(capitalise(""), "");
        assert_eq!(capitalise("ßx"), "SSx");
    }

    #[test]
    fn is_primitive_type_needs_an_ascii_lowercase_first_letter() {
        assert!(is_primitive_type("dateTime"));
        assert!(!is_primitive_type("Quantity"));
        assert!(!is_primitive_type(""));
        assert!(!is_primitive_type("éa"));
    }
}
