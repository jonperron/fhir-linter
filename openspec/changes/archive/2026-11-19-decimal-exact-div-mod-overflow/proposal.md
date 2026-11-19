## Why
Decimal `mod`, `div` and `/` failed with an overflow error when the scaling intermediate exceeded i128, even though the result fit.

## What Changes
- `mod` never overflows (modular arithmetic, |a mod b| < |b|).
- `div` and `/` overflow only when the true quotient exceeds the 38 figure mantissa.

## Capabilities
### Modified Capabilities
- `fhir-fhirpath`: scenario "Arithmétique décimale exacte".

## Impact
`crates/fhir-parser/src/decimal.rs`.
