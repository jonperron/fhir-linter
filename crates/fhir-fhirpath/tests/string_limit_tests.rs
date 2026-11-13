/// The evaluator's string-length cap: a string result longer than 1,048,576
/// characters (R5's string maximum) stops with `StringTooLong`, so the
/// validator reports `INVARIANT_SKIPPED` instead of exhausting memory.
use fhir_fhirpath::{EvalError, Value, evaluate};

const LIMIT: usize = 1_048_576;

/// `seed` followed by `steps` doublings with `op`: 2^steps copies of `seed`.
fn doubling_chain(seed: &str, op: &str, steps: usize) -> String {
    format!(
        "{seed}{}",
        format!(".select($this {op} $this)").repeat(steps)
    )
}

fn is_too_long(expr: &str) -> bool {
    matches!(evaluate(expr, &[]), Err(EvalError::StringTooLong(LIMIT)))
}

fn string_len(expr: &str) -> Result<usize, EvalError> {
    match evaluate(expr, &[])?.as_slice() {
        [Value::String(s)] => Ok(s.chars().count()),
        other => Err(EvalError::Type(format!(
            "expected one String, got {other:?}"
        ))),
    }
}

#[test]
fn doubling_select_chain_stops_at_the_limit() -> Result<(), EvalError> {
    for op in ["+", "&"] {
        assert!(is_too_long(&doubling_chain("'a'", op, 21)), "{op}");
        assert_eq!(string_len(&doubling_chain("'a'", op, 20))?, LIMIT, "{op}");
    }
    Ok(())
}

#[test]
fn doubling_aggregate_stops_at_the_limit() -> Result<(), EvalError> {
    let items = |n: usize| (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join("|");
    let over = format!("({}).aggregate($total + $total, 'a')", items(21));
    assert!(is_too_long(&over), "{over}");
    let at = format!("({}).aggregate($total + $total, 'a')", items(20));
    assert_eq!(string_len(&at)?, LIMIT);
    Ok(())
}

#[test]
fn string_functions_stop_at_the_limit() {
    let half = doubling_chain("'a'", "+", 19);
    let full = doubling_chain("'a'", "+", 20);
    let half_angle_brackets = doubling_chain("'<'", "+", 19);
    for (index, expr) in [
        format!("{half}.select($this.combine($this).combine('b')).join('')"),
        format!("{half}.select($this.combine($this)).join('-')"),
        format!("{full}.replace('a', 'aa')"),
        format!("'abc'.replace('', {half})"),
        format!("{full}.replaceMatches('a', 'aa')"),
        format!("{half}.replaceMatches('(a+)', '$1$1$1')"),
        format!("'abc'.replaceMatches('', {half})"),
        format!("{half_angle_brackets}.escape('html')"),
        format!("{full}.encode('base64')"),
        format!("{}.upper()", doubling_chain("'ß'", "+", 20)),
    ]
    .iter()
    .enumerate()
    {
        assert!(is_too_long(expr), "case {index}");
    }
}

#[test]
fn string_functions_under_the_limit_still_evaluate() -> Result<(), EvalError> {
    let half = doubling_chain("'a'", "+", 19);
    assert_eq!(
        string_len(&format!("{half}.select($this.combine($this)).join('')"))?,
        LIMIT
    );
    assert_eq!(string_len(&format!("{half}.replace('a', 'b')"))?, LIMIT / 2);
    assert_eq!(
        string_len(&format!("{half}.replaceMatches('a', 'b')"))?,
        LIMIT / 2
    );
    assert_eq!(
        string_len(&format!("{half}.replaceMatches('(a+)', '$1$1')"))?,
        LIMIT
    );
    assert_eq!(string_len("'abc'.replace('', '-')")?, 7);
    // The cap counts characters, not bytes: 2^20 'é' is 2 MiB of UTF-8.
    assert_eq!(string_len(&doubling_chain("'é'", "+", 20))?, LIMIT);
    for expr in [
        format!("{half}.replace('a', 'aa')"),
        format!("{}.escape('html')", doubling_chain("'<'", "+", 18)),
        format!("{}.upper()", doubling_chain("'ß'", "+", 19)),
    ] {
        assert_eq!(string_len(&expr)?, LIMIT);
    }
    assert_eq!(
        evaluate("'abc'.replaceMatches('(b)', '[$1$$]')", &[])?,
        vec![Value::String("a[b$]c".into())]
    );
    Ok(())
}
