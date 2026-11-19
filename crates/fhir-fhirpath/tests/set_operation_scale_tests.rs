/// Set operations on large collections stay linear: R5 dom-3 unions every
/// reference of a resource, so a quadratic duplicate check let one file of
/// 32k references take 46 s to validate.
use std::time::{Duration, Instant};

use fhir_fhirpath::{Decimal, Value, evaluate};

const COUNT: usize = 50_000;

/// A Patient whose first name has `COUNT` distinct given names, `g0` first.
fn many_given_names() -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let given: Vec<String> = (0..COUNT).map(|i| format!("\"g{i}\"")).collect();
    let json = format!(
        r#"{{"resourceType":"Patient","name":[{{"given":[{}]}}]}}"#,
        given.join(","),
    );
    let patient = fhir_parser::parse_json(&json)?;
    Ok(vec![fhir_fhirpath::resource_to_value(&patient)])
}

fn count(expr: &str, context: &[Value]) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(evaluate(expr, context)?.len())
}

// The quadratic scan made each of these billions of comparisons (minutes in
// a debug build); the hash path takes milliseconds. The bound is generous so
// a slow CI runner cannot flake.
#[test]
fn set_operations_on_many_strings_are_linear() -> Result<(), Box<dyn std::error::Error>> {
    let context = many_given_names()?;
    let started = Instant::now();
    let given = "Patient.name.given";
    assert_eq!(count(&format!("{given} | {given}"), &context)?, COUNT);
    assert_eq!(count(&format!("{given}.distinct()"), &context)?, COUNT);
    assert_eq!(
        count(&format!("{given}.intersect({given})"), &context)?,
        COUNT,
    );
    assert_eq!(count(&format!("{given}.exclude({given})"), &context)?, 0);
    assert_eq!(count("Patient.name.repeat(given)", &context)?, COUNT);
    for expr in [
        format!("{given}.isDistinct()"),
        format!("{given}.subsetOf({given})"),
        format!("{given}.supersetOf({given})"),
    ] {
        assert_eq!(
            evaluate(&expr, &context)?,
            vec![Value::Bool(true)],
            "{expr}"
        );
    }
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_secs(20), "took {elapsed:?}");
    Ok(())
}

// N1 §6.1.1: "Decimal: values must be equal, trailing zeroes after the
// decimal are ignored", and an Integer converts implicitly to a Decimal, so
// `i.0`, `i.00` and `i` are one value; they share a key instead of a scan.
#[test]
fn set_operations_on_many_decimals_are_linear() -> Result<(), Box<dyn std::error::Error>> {
    let context: Vec<Value> = (0..COUNT as i128)
        .map(|i| Value::Decimal(Decimal::new(i * 10, 1)))
        .collect();
    let started = Instant::now();
    assert_eq!(count("distinct()", &context)?, COUNT);
    assert_eq!(count("union(select($this * 1.0))", &context)?, COUNT);
    assert_eq!(
        count("combine(select(toInteger())).distinct()", &context)?,
        COUNT
    );
    assert_eq!(count("exclude(select($this * 1.0))", &context)?, 0);
    assert_eq!(evaluate("isDistinct()", &context)?, vec![Value::Bool(true)]);
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_secs(20), "took {elapsed:?}");
    Ok(())
}

#[test]
fn decimals_ignore_trailing_zeros_and_equal_integers() -> Result<(), Box<dyn std::error::Error>> {
    let cases: [(&str, &str); 10] = [
        ("1.0 | 1.00", "1.0"),
        ("1.00 | 1.0", "1.00"),
        ("(1.0).combine(1.00).isDistinct()", "false"),
        ("(1.1).combine(1.10).combine(1.2).distinct()", "1.10 | 1.2"),
        ("0.0 | 0 | -0.0 | 0.00", "0.0"),
        ("(1.0 | 2.00 | 3.5).intersect(2 | 1.000)", "1.0 | 2.00"),
        ("(10 | 10.5).exclude(10.0)", "10.5"),
        ("1L | 1.0", "1L"),
        ("(1.5).subsetOf(1.50 | 2)", "true"),
        ("1.1 | 1.01 | 1.10", "1.1 | 1.01"),
    ];
    for (expr, expected) in cases {
        let (got, want) = (evaluate(expr, &[])?, evaluate(expected, &[])?);
        // Value equality ignores scale, so compare the written forms too.
        let text = |values: &[Value]| values.iter().map(Value::to_string).collect::<Vec<_>>();
        assert_eq!(text(&got), text(&want), "{expr}");
    }
    Ok(())
}

// The hash path keeps FHIRPath equality and order where keyed values
// (strings, numbers, booleans) meet unkeyed ones (dates) and Integer meets
// Decimal.
#[test]
fn keyed_and_unkeyed_values_keep_fhirpath_equality() -> Result<(), Box<dyn std::error::Error>> {
    let cases: [(&str, &str); 18] = [
        ("1.0 | 2 | 1", "1.0 | 2"),
        ("(1).subsetOf(1.0 | 2)", "true"),
        ("(1.0).supersetOf(1)", "true"),
        ("('a' | 1).subsetOf(1.0)", "false"),
        ("(1 | 2).combine(1).exclude(2)", "(1).combine(1)"),
        ("(1 | 2).combine(1).distinct()", "2 | 1"),
        ("'a'.combine(@2020).combine('a').distinct()", "@2020 | 'a'"),
        ("@2020.combine(@2020-01).distinct()", "{}"),
        ("@2020.combine(@2020-01).isDistinct()", "{}"),
        ("1 | 1.0", "1"),
        ("1.0 | 1", "1.0"),
        ("1 | 1L", "1"),
        ("(1.0).combine(2).combine(1).distinct()", "2 | 1"),
        ("(1).combine(1.0).isDistinct()", "false"),
        ("(1.0).combine(1).isDistinct()", "false"),
        ("(1 | 'a' | true).intersect(1.0 | true)", "1 | true"),
        ("(1 | 'a').exclude(1.0)", "'a'"),
        ("'a'.combine(1).combine('a').distinct()", "1 | 'a'"),
    ];
    for (expr, expected) in cases {
        assert_eq!(evaluate(expr, &[])?, evaluate(expected, &[])?, "{expr}");
    }
    Ok(())
}
