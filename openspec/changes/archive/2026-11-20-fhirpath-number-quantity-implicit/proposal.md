## Why
N1 §5.5 converts Integer/Decimal (and Long) implicitly to a Quantity with unit '1' when compared with a Quantity. The evaluator now does so; the spec must say it.

## What Changes
- Equality, equivalence, ordering, sort/min/max and set operations treat a number opposite a Quantity as a Quantity with unit '1'.
- `comparable()` is unchanged (still two Quantities).

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `fhir-fhirpath`: new requirement for implicit number-to-Quantity conversion.

## Impact
`crates/fhir-fhirpath` only; no API change.
