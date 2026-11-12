## Why

Invariants, terminology and reference checks only ran on top-level elements (PLAN 11.2-A3).

## What Changes

- Invariants run on every instance node, once per key.
- TERMINOLOGY_001/002 and REFERENCE_001..004 run at every depth, including choice variants.
- fhir-fhirpath exposes node_to_typed_value.

## Capabilities

### Modified Capabilities
- `fhir-validator`: depth of invariant, terminology and reference layers.
- `fhir-fhirpath`: new public node_to_typed_value.

## Impact

crates/fhir-validator/src/nested.rs, crates/fhir-fhirpath.
