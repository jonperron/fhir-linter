## Why

R5 profiling (Slice Cardinality) requires that "The maximum cardinality on each slice cannot exceed n" and "The sum of the minimum cardinalities must be less or equal to n"; eld-2 requires Min <= Max. The validator did not check these in StructureDefinition differentials.

## What Changes

- New Error `DEFINITION_002`: slice max above the slicer max, or slice minimums summing above it.
- New Error `DEFINITION_003`: a slice child whose min exceeds the max of its unsliced counterpart.
- Applies to every StructureDefinition with a differential (profiles and specializations); DEFINITION_001 stays profile-only.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `fhir-validator`: adds the DEFINITION_002 and DEFINITION_003 requirements.

## Impact

`crates/fhir-validator/src/structure_definition.rs`. Official cases sd-card-slice and sd-card-child now pass. The Java-only minValue/maxValue rule (sd-range-*) is not implemented.
