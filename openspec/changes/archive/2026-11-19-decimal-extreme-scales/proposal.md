## Why

Decimal `div`, `/` and `mod` failed with a type error when operand scales were far apart, and `toString()` of a decimal needing an exponent produced a malformed String.

## What Changes

- `div`, `/`, `mod` stay defined for far-apart scales (`1e-30 div 1e30` is 0; `mod` returns the dividend when the divisor outgrows i128; `i128::MIN mod -1` is 0).
- `toString()` of a decimal needing an exponent (scale beyond 1000) is empty.

## Capabilities

### Modified Capabilities
- `fhir-fhirpath`: math functions (decimal arithmetic) and conversion functions (toString).

## Impact

`crates/fhir-parser/src/decimal.rs`, `crates/fhir-fhirpath/src/eval/type_conv.rs`.
