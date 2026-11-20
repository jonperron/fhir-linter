## Why
FHIRPath v3 §5.5 defines no Long -> Quantity implicit conversion. The spec wrongly lists Long; the code converts only Integer and Decimal.

## What Changes
- Remove Long from the implicit number to Quantity conversion requirement; add a Long scenario.

## Capabilities
### Modified Capabilities
- `fhir-fhirpath`: implicit number to Quantity conversion

## Impact
Spec only; code already matches (`crates/fhir-fhirpath/src/eval/helpers.rs`).
