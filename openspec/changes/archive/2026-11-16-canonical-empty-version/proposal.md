## Why
A canonical ending in `|` has an empty version, which R5 forbids (version is a string, never empty).

## What Changes
- `canonical` ending in `|` yields TYPE_003 after the regex/maxLength check; uri/url unaffected.

## Capabilities
### Modified Capabilities
- fhir-validator

## Impact
crates/fhir-validator/src/type_check.rs
