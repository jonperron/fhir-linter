## Why

A `Coding.system` or `ValueSet.compose.include.system` that is the canonical URL of a ValueSet is invalid. The Java validator reports it (official cases code-wrong-system-vs.xml, vs-wrong-vs.xml).

## What Changes

- Emit `TERMINOLOGY_003` (error) at the system path when such a system is a ValueSet loaded in the Registry.

## Capabilities

### Modified Capabilities
- `fhir-validator`: new requirement TERMINOLOGY_003.

## Impact

`crates/fhir-validator/src/terminology.rs`; tests in `coding_system_value_set_tests.rs`.
