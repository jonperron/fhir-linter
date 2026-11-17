## Why
Bundle references by fullUrl (urn:uuid) were never type-checked, and entry fullUrl/id disagreement went unreported.

## What Changes
- REFERENCE_003/005 resolve absolute references against the innermost Bundle's entry fullUrl.
- New REFERENCE_007 for RESTful fullUrl whose id differs from the resource id.

## Capabilities
### Modified
- fhir-validator

## Impact
crates/fhir-validator (references.rs, nested.rs, embedded.rs, lib.rs).
