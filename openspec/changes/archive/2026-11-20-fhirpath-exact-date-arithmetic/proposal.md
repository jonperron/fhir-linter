## Why

Date/time arithmetic used binary floats and cited the wrong spec sections. N1 §6.7.1/§6.7.2 requires exact decimal amounts, ignores decimals above seconds, and restricts Date units.

## What Changes

- Amounts are exact decimals, truncated toward zero; an amount whose count exceeds i64 is an error.
- Decimals ignored for all units above seconds (calendar keywords and UCUM alike).
- Hour/minute/second/millisecond units on a Date are errors.
- Day-or-finer units on year (365 d) or month (30 d) precision values convert instead of failing.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `fhir-fhirpath`: requirement "Arithmétique de dates avec durées calendaires".

## Impact

`crates/fhir-fhirpath/src/eval/temporal.rs`, `crates/fhir-fhirpath/tests/date_arithmetic_tests.rs`.
