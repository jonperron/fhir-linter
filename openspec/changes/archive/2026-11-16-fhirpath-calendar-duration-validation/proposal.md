## Why

Calendar duration arithmetic accepted invalid operands (month 13, 24:00, 1900-02-29), shifted clocks on partial dates and rounded sub-step amounts across a step.

## What Changes

- Invalid Date/DateTime/Time operands are type errors.
- Second 60 is accepted; any day-or-finer shift of a leap second is an error (N2 is silent).
- Sub-step amounts truncate without rounding across a step.

## Capabilities

### Modified Capabilities
- `fhir-fhirpath`: calendar duration arithmetic on dates

## Impact

`crates/fhir-fhirpath/src/eval/temporal.rs`.
