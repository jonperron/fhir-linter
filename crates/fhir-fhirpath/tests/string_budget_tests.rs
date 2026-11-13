/// The evaluator's per-evaluation string budget: building more than
/// 16,777,216 characters of String values in one evaluation stops with
/// `StringBudgetExceeded`, even when every string stays under
/// `MAX_STRING_LEN` and every collection under `MAX_COLLECTION_LEN`.
use fhir_fhirpath::{EvalError, Value, evaluate};

const BUDGET: usize = 16_777_216;
const MAX_STRING: usize = 1_048_576;

/// A String of exactly `MAX_STRING` characters, by 20 doublings of `'a'`.
fn full_string() -> String {
    format!("'a'{}", ".select($this + $this)".repeat(20))
}

/// `2^copies_log2` items sharing one full String, each then rebuilt by
/// `transform`, so every item is a new 1,048,576-character String.
fn many_full_strings(copies_log2: usize, transform: &str) -> String {
    format!(
        "{}{}.select($this.{transform})",
        full_string(),
        ".select($this.combine($this))".repeat(copies_log2),
    )
}

fn is_over_budget(expr: &str) -> bool {
    matches!(
        evaluate(expr, &[]),
        Err(EvalError::StringBudgetExceeded(BUDGET))
    )
}

#[test]
fn many_large_strings_stop_at_the_budget() {
    for transform in [
        "upper()",
        "lower()",
        "trim()",
        "escape('json')",
        "join('')",
        "replaceMatches('a', 'b')",
        "substring(0)",
        "replace('a', 'b')",
        "toString()",
        "split('-')",
    ] {
        assert!(
            is_over_budget(&many_full_strings(5, transform)),
            "{transform}"
        );
    }
    // Concatenation of two half strings, 32 times.
    let half = format!("'a'{}", ".select($this + $this)".repeat(19));
    let expr = format!(
        "{half}{}.select($this + {half})",
        ".select($this.combine($this))".repeat(5),
    );
    assert!(is_over_budget(&expr), "concatenation");
}

#[test]
fn large_strings_under_the_budget_still_evaluate() -> Result<(), EvalError> {
    let result = evaluate(&many_full_strings(3, "upper()"), &[])?;
    assert_eq!(result.len(), 8);
    assert!(
        result
            .iter()
            .all(|v| matches!(v, Value::String(s) if s.chars().count() == MAX_STRING))
    );
    Ok(())
}

#[test]
fn navigating_existing_strings_does_not_count() -> Result<(), EvalError> {
    // 32 references to one String are not 32 new Strings.
    let expr = format!(
        "{}{}.count()",
        full_string(),
        ".select($this.combine($this))".repeat(5),
    );
    assert_eq!(evaluate(&expr, &[])?, vec![Value::Integer(32)]);
    Ok(())
}
