## Why
Pass/fail on has-errors hides wrong error counts.

## What Changes
Manifest harness compares error counts, prints a mismatch table, and enforces a floor MIN_ERROR_COUNT_MATCHES = (78, 78).

## Capabilities
### Modified
- fhir-validator

## Impact
crates/fhir-validator/tests/manifest_tests.rs only.
