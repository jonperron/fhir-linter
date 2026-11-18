//! Quantity `*` and `/` with unit algebra (N1 §6.6), unary minus and
//! `abs()` on Quantities, and `toQuantity(unit)` converting through UCUM
//! (N1 §5.5.7).
use std::path::PathBuf;

use fhir_definitions::Registry;
use fhir_fhirpath::{Value, evaluate, resource_to_typed_value};
use fhir_parser::parse_json;

fn eval(expr: &str) -> Vec<Value> {
    evaluate(expr, &[]).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

/// The single Quantity `expr` gives, as `(value, unit)`.
fn quantity(expr: &str) -> (f64, String) {
    match eval(expr).as_slice() {
        [Value::Quantity(value, unit)] => (*value, unit.to_string()),
        other => panic!("{expr}: not a Quantity: {other:?}"),
    }
}

fn is_true(expr: &str) -> bool {
    matches!(eval(expr).as_slice(), [Value::Bool(true)])
}

#[test]
fn multiplication_combines_units() {
    // N1 §6.6 examples.
    assert_eq!(quantity("12 'cm' * 3 'cm'"), (36.0, "cm2".into()));
    assert_eq!(quantity("3 'cm' * 12 'cm2'"), (36.0, "cm3".into()));
    assert_eq!(quantity("2.0 'cm' * 2.0 'm'"), (4.0, "cm.m".into()));
    assert!(is_true("2.0 'cm' * 2.0 'm' = 0.040 'm2'"));
    assert_eq!(quantity("2 'kg' * 3"), (6.0, "kg".into()));
    assert_eq!(quantity("3 * 2 'kg'"), (6.0, "kg".into()));
    assert_eq!(quantity("2 'kg' * 3 '1'"), (6.0, "kg".into()));
}

#[test]
fn division_combines_and_cancels_units() {
    assert_eq!(quantity("12 'cm2' / 3 'cm'"), (4.0, "cm".into()));
    assert_eq!(quantity("4.0 'g' / 2.0 'm'"), (2.0, "g/m".into()));
    assert!(is_true("4.0 'g' / 2.0 'm' = 2 'g/m'"));
    assert_eq!(quantity("1.0 'm' / 1.0 'm'"), (1.0, "1".into()));
    assert!(is_true("1.0 'm' / 1.0 'm' = 1 '1'"));
    assert_eq!(quantity("6 'kg' / 3"), (2.0, "kg".into()));
    assert_eq!(quantity("6 / 3 's'"), (2.0, "1/s".into()));
    assert_eq!(quantity("1 'm' / 2 's2' * 4 's'"), (2.0, "m/s".into()));
    assert_eq!(quantity("2 '10*3' * 3 '10*3'"), (6.0, "10*6".into()));
    assert_eq!(quantity("2 '/m' * 3 'g'"), (6.0, "g/m".into()));
    assert_eq!(quantity("2 'm.m' * 3 's'"), (6.0, "m2.s".into()));
    assert_eq!(quantity("2 'g' * 3 '/m/s'"), (6.0, "g.s/m".into()));
    // Calendar keywords from weeks down stand for their UCUM unit.
    assert_eq!(quantity("10 'm' / 2 seconds"), (5.0, "m/s".into()));
    // Units the simple form cannot hold are kept whole in parentheses.
    assert_eq!(
        quantity("4 'kg/(m.s)' / 2 's'"),
        (2.0, "(kg/(m.s))/(s)".into())
    );
}

#[test]
fn algebra_without_a_result_is_empty() {
    assert!(eval("4 'g' / 0 'm'").is_empty());
    assert!(eval("4 'g' / 0").is_empty());
    // Unknown units and indefinite calendar durations have no product.
    assert!(eval("2 'foo' * 2 'm'").is_empty());
    assert!(eval("2 years * 2 'm'").is_empty());
}

#[test]
fn algebra_keeps_other_atoms_and_rejects_other_operators() {
    assert_eq!(quantity("2 'm.s' / 1 'm'"), (2.0, "s".into()));
    assert!(evaluate("2 'm' * 'a'", &[]).is_err());
    assert!(evaluate("2 'm' mod 1 'm'", &[]).is_err());
}

#[test]
fn fhir_quantity_multiplies_by_its_ucum_code() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
    let registry = Registry::from_definitions_dir(dir).expect("R5 definitions");
    let resource = parse_json(
        r#"{"resourceType":"Observation","status":"final","code":{"text":"w"},
        "valueQuantity":{"value":3,"system":"http://unitsofmeasure.org","code":"kg"}}"#,
    )
    .unwrap();
    let context = [resource_to_typed_value(&resource, &|parent, name| {
        registry.child_element(parent, name)
    })];
    let result = evaluate("Observation.valueQuantity * 2 'm'", &context).unwrap();
    assert!(
        matches!(result.as_slice(), [Value::Quantity(v, u)] if *v == 6.0 && &**u == "kg.m"),
        "{result:?}"
    );
}

#[test]
fn negation_and_abs_keep_the_unit() {
    assert_eq!(quantity("-(5.5 'mg')"), (-5.5, "mg".into()));
    assert!(is_true("(-5.5 'mg').abs() = 5.5 'mg'"));
}

#[test]
fn to_quantity_converts_to_the_requested_unit() {
    assert_eq!(quantity("1 'g'.toQuantity('mg')"), (1000.0, "mg".into()));
    assert_eq!(quantity("'7 days'.toQuantity('wk')"), (1.0, "wk".into()));
    assert_eq!(quantity("1 'wk'.toQuantity('days')"), (7.0, "day".into()));
    assert_eq!(quantity("1 'g'.toQuantity('g')"), (1.0, "g".into()));
    assert!(eval("1 'g'.toQuantity('m')").is_empty());
    assert!(is_true("1 'g'.convertsToQuantity('kg')"));
    assert!(is_true("1 'g'.convertsToQuantity('m').not()"));
}
