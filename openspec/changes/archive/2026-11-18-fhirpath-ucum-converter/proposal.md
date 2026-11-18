## Why
Quantity comparison and arithmetic need UCUM unit conversion. This adds the converter first, unused by evaluation.

## What Changes
- Crate-private UCUM converter (`convert`, `comparable`) backed by the UCUM essence file.
- No change to FHIRPath evaluation behaviour.

## Capabilities
### New
None
### Modified
- fhir-fhirpath

## Impact
`crates/fhir-fhirpath` (ucum module, data/ucum-essence.xml, quick_xml dependency).
