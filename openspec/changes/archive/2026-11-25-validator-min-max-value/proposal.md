## Why
Profiles may bound values with minValue[x]/maxValue[x]; the validator ignored them.

## What Changes
- New error PROFILE_011 for values below the minimum or above the maximum.

## Capabilities
### Modified Capabilities
- fhir-validator

## Impact
crates/fhir-validator/src/min_max_value.rs, fixed_pattern.rs
