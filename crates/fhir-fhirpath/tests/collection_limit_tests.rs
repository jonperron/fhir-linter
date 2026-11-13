/// The evaluator's collection-size cap: an expression whose intermediate
/// collections grow past 100,000 items stops with `CollectionTooLarge`, so the
/// validator reports `INVARIANT_SKIPPED` instead of exhausting memory.
use fhir_fhirpath::{EvalError, evaluate};

const LIMIT: usize = 100_000;

/// `1` followed by `steps` doublings: 2^steps items.
fn doubling_chain(steps: usize) -> String {
    format!("1{}", ".select($this.combine($this))".repeat(steps))
}

fn is_too_large(expr: &str) -> bool {
    matches!(
        evaluate(expr, &[]),
        Err(EvalError::CollectionTooLarge(LIMIT))
    )
}

#[test]
fn doubling_select_chain_stops_at_the_limit() {
    let chain = doubling_chain(20);
    assert_eq!(chain.len(), 581);
    assert!(is_too_large(&chain), "2^20 items");
    assert!(is_too_large(&doubling_chain(17)), "2^17 items");
}

#[test]
fn doubling_aggregate_stops_at_the_limit() {
    let items: Vec<String> = (1..=20).map(|n| n.to_string()).collect();
    let expr = format!("({}).aggregate($total.combine($total), 1)", items.join("|"));
    assert!(is_too_large(&expr), "{expr}");
}

// 2^16 copies of a node stay under the cap; expanding each one must stop at
// the cap instead of building copies x children first. Only peak memory
// differs from a post-check, so this pins the outcome, not the allocation.
#[test]
fn navigation_over_many_copies_stops_at_the_limit() -> Result<(), Box<dyn std::error::Error>> {
    let patient = fhir_parser::parse_json(
        r#"{"resourceType":"Patient","name":[{"given":["a","b","c"]},{"given":["d","e"]}],
        "extension":[{"url":"u","valueString":"x"},{"url":"u","valueString":"y"}]}"#,
    )?;
    let context = [fhir_fhirpath::resource_to_value(&patient)];
    let copies = doubling_chain(16).replacen('1', "$this", 1);
    for step in [
        "descendants()",
        "children()",
        "name.given",
        "extension('u')",
    ] {
        let expr = format!("{copies}.{step}");
        assert!(
            matches!(
                evaluate(&expr, &context),
                Err(EvalError::CollectionTooLarge(LIMIT))
            ),
            "{step}"
        );
    }
    Ok(())
}

#[test]
fn navigation_under_the_limit_still_evaluates() -> Result<(), Box<dyn std::error::Error>> {
    let patient = fhir_parser::parse_json(
        r#"{"resourceType":"Patient","name":[{"given":["a","b","c"]},{"given":["d","e"]}],
        "extension":[{"url":"u","valueString":"x"},{"url":"u","valueString":"y"}]}"#,
    )?;
    let context = [fhir_fhirpath::resource_to_value(&patient)];
    let copies = doubling_chain(10).replacen('1', "$this", 1);
    for (step, expected) in [("name.given", 5 * 1024), ("extension('u')", 2 * 1024)] {
        let found = evaluate(&format!("{copies}.{step}"), &context)?;
        assert_eq!(found.len(), expected, "{step}");
    }
    for step in ["children()", "descendants()"] {
        assert!(
            evaluate(&format!("{copies}.{step}"), &context)?.len() < LIMIT,
            "{step}"
        );
    }
    Ok(())
}

#[test]
fn collections_under_the_limit_still_evaluate() -> Result<(), EvalError> {
    assert_eq!(evaluate(&doubling_chain(16), &[])?.len(), 65_536);
    let items: Vec<String> = (1..=16).map(|n| n.to_string()).collect();
    let expr = format!("({}).aggregate($total.combine($total), 1)", items.join("|"));
    assert_eq!(evaluate(&expr, &[])?.len(), 65_536);
    Ok(())
}
