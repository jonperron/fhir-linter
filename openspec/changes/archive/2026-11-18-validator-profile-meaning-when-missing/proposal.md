## Why

ElementDefinition.meaningWhenMissing: "Implicit meanings for missing values can only be specified on a resource, data type, or extension definition, and never in a profile that applies to one of these." The validator did not flag it.

## What Changes

- New Error `DEFINITION_001` for a StructureDefinition with derivation = constraint whose `differential.element[i]` sets `meaningWhenMissing`.
- Exempt: specialization or absent derivation, extension definitions, constraints without `type`. Snapshot not checked.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `fhir-validator`: adds the DEFINITION_001 requirement.

## Impact

`crates/fhir-validator/src/structure_definition.rs`, called from `validate_one` in `lib.rs`.
