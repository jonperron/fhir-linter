//! The evaluator's per-evaluation step budget: an expression that does more
//! than 10,000,000 steps of work (expression nodes evaluated plus values
//! produced) stops with `StepBudgetExceeded`, even when every collection
//! stays under `MAX_COLLECTION_LEN` and no string is built.
use fhir_fhirpath::{EvalError, Value, evaluate};

const BUDGET: usize = 10_000_000;

/// `2^log2` copies of `1`, built by doubling.
fn copies(log2: usize) -> String {
    format!("1{}", ".select($this.combine($this))".repeat(log2))
}

fn is_over_budget(expr: &str) -> bool {
    matches!(
        evaluate(expr, &[]),
        Err(EvalError::StepBudgetExceeded(BUDGET))
    )
}

#[test]
fn nested_iteration_stops_at_the_budget() {
    // 8,192 items, each re-building 8,192 items: about 6.7e7 values produced,
    // the shape of `descendants().select(%resource.descendants().count())`.
    let inner = copies(13);
    let expr = format!("{inner}.select({inner}.count())");
    assert!(is_over_budget(&expr), "{expr}");
}

#[test]
fn quadratic_aggregate_stops_at_the_budget() {
    // `$total` grows by one per item: about 8,192^2 / 2 = 3.4e7 values cloned.
    let expr = format!("{}.aggregate($total.combine($this))", copies(13));
    assert!(is_over_budget(&expr), "{expr}");
}

#[test]
fn linear_work_on_a_large_collection_stays_under_the_budget() -> Result<(), EvalError> {
    // 65,536 items through where, select, all and aggregate: linear work.
    let items = copies(16);
    let expr = format!(
        "{items}.where($this = 1).select($this + 1).all($this = 2) \
         and {items}.aggregate($total + $this, 0) = 65536",
    );
    assert_eq!(evaluate(&expr, &[])?, vec![Value::Bool(true)]);
    Ok(())
}
