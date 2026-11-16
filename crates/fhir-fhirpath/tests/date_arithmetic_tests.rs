//! Date, DateTime and Time plus or minus a calendar duration (FHIRPath N2
//! §6.6.7 `+` and §6.6.8 `-`).
use std::sync::Arc;

use fhir_fhirpath::{Value, evaluate};

fn eval(expr: &str) -> Vec<Value> {
    evaluate(expr, &[]).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

fn date(expr: &str, expected: &str) {
    assert_eq!(eval(expr), vec![Value::Date(Arc::from(expected))], "{expr}");
}

fn date_time(expr: &str, expected: &str) {
    assert_eq!(
        eval(expr),
        vec![Value::DateTime(Arc::from(expected))],
        "{expr}"
    );
}

fn time(expr: &str, expected: &str) {
    assert_eq!(eval(expr), vec![Value::Time(Arc::from(expected))], "{expr}");
}

fn fails(expr: &str) {
    assert!(evaluate(expr, &[]).is_err(), "{expr} should be an error");
}

#[test]
fn official_suite_cases() {
    date("@1973-12-25 + 7 days", "1974-01-01"); // testPlusDate1
    date("@1973-12-25 + 1 month", "1974-01-25"); // testPlusDate10
    date("@1973-12-25 + 1 'wk'", "1974-01-01"); // testPlusDate15
    date("@1974-12-25 - 1 'month'", "1974-11-25"); // testMinus5
    date_time(
        "@1973-12-25T00:00:00.000+10:00 + 10 'ms'", // testPlusDate20
        "1973-12-25T00:00:00.010+10:00",
    );
    time("@T23:00:00 + 50 hours", "01:00:00"); // testPlusTime3
    time("@T01:00:00 - 2 hours", "23:00:00"); // testMinus8
}

#[test]
fn month_and_year_clamp_to_the_end_of_the_month() {
    date("@2024-01-31 + 1 month", "2024-02-29");
    date("@2023-01-31 + 1 month", "2023-02-28");
    date("@2024-02-29 + 1 year", "2025-02-28");
    date("@2024-03-31 - 1 month", "2024-02-29");
    date("@1974-01-15 - 1 month", "1973-12-15");
    date("@2024-02-28 + 1 day", "2024-02-29");
    date("@2023-02-28 + 1 day", "2023-03-01");
}

#[test]
fn decimal_amounts_above_seconds_are_truncated() {
    date("@1973-12-25 + 7.7 days", "1974-01-01");
    date("@1973-12-25 + 1.9 weeks", "1974-01-01");
    date_time("@2015-02-04T10:00Z + 1.5 hours", "2015-02-04T11:00Z");
    date_time(
        "@2015-02-04T10:00:00.000Z + 1.5 's'",
        "2015-02-04T10:00:01.500Z",
    );
}

#[test]
fn a_finer_unit_converts_to_the_value_precision() {
    date("@2019-03-01 + 24 hours", "2019-03-02");
    date("@2019-03-01 + 23 hours", "2019-03-01");
    date_time(
        // 1.005 * 1000 is 1004.999... in f64.
        "@2015-02-04T10:00:00.000Z + 1.005 seconds",
        "2015-02-04T10:00:01.005Z",
    );
    time("@T10 + 150 minutes", "12");
    date("@2014 + 24 months", "2016");
    date("@2014 + 11 months", "2014");
    date("@2014-06 + 1 year", "2015-06");
    date_time("@2015-02-04T10:00:00Z + 1.5 's'", "2015-02-04T10:00:01Z");
    date_time("@2015-02-04T10Z + 90 minutes", "2015-02-04T11Z");
}

#[test]
fn time_wraps_around_midnight_and_zones_are_kept() {
    time("@T00:30 - 1 hour", "23:30");
    time("@T23:59:59.999 + 1 millisecond", "00:00:00.000");
    date_time(
        "@2015-02-04T23:30:00+10:00 + 1 hour",
        "2015-02-05T00:30:00+10:00",
    );
    date_time(
        "@2015-12-31T23:00-05:00 + 2 hours",
        "2016-01-01T01:00-05:00",
    );
}

#[test]
fn non_calendar_units_and_bad_operands_are_errors() {
    fails("@1973-12-25 + 1 'mo'"); // testPlusDate14
    fails("@1973-12-25 + 1 'a'"); // testPlusDate16
    fails("@1974-12-25 - 1 'cm'"); // testMinus6
    fails("@1974-12-25 + 7"); // testPlus6
    fails("7 days + @1974-12-25");
    fails("@9999-12-31 + 1 day");
    fails("@0001-01-01 - 1 year");
    // N2 gives no day length in months or years, so a day cannot convert.
    fails("@2014 + 400 days");
    // N2 does not say what a day or more means for a Time.
    fails("@T10:00 + 1 day");
}

#[test]
fn sub_step_amounts_truncate_and_invalid_values_are_errors() {
    time("@T10:00:00 + 0.9999996 seconds", "10:00:00");
    date_time(
        "@2015-02-04T10:00:00.000Z + 1.005 seconds",
        "2015-02-04T10:00:01.005Z",
    );
    fails("@2016-12-31T23:59:60Z + 0 seconds");
    date_time("@2016-12-31T23:59:60Z + 1 year", "2017-12-31T23:59:60Z");
    date("@2000-02-29 + 1 day", "2000-03-01");
    date("@1900-02-28 + 1 day", "1900-03-01");
    fails("@1900-02-29 + 1 day");
    fails("@2015-13-01 + 1 day");
    fails("@2015-02-30 + 1 day");
    fails("@2015-02-04T24:00:00Z + 1 hour");
    fails("@2015-02-04T10:60:00Z + 1 hour");
    fails("@0000-01-01 + 1 day");
    fails("@2014T10:00 + 1 hour");
}
