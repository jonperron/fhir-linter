## Why

FHIRPath N2 requires an empty result when a math result cannot be represented. `sqrt()` and `power()` returned Decimal NaN/inf.

## What Changes

- `sqrt()` and `power(exponent)` return an empty collection when the result is NaN or infinite. `ln()`, `log()`, `exp()` unchanged.

## Capabilities

### Modified Capabilities
- `fhir-fhirpath`: math functions requirement

## Impact

`crates/fhir-fhirpath/src/eval/math.rs`
