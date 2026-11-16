## Why

FHIRPath N2 §6.6.7/§6.6.8 define Date/DateTime/Time +/- calendar duration; the official suite needs it.

## What Changes

- Date, DateTime and Time `+`/`-` a calendar duration Quantity.
- Official suite floor raised from 1078 to 1106.

## Capabilities

### New Capabilities
- None

### Modified Capabilities
- `fhir-fhirpath`: new date arithmetic requirement; suite floor 1106.

## Impact

`crates/fhir-fhirpath/src/eval/temporal.rs`, `helpers.rs`, tests.
