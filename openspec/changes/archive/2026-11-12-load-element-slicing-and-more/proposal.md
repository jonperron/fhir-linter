## Why
Validator checks (slicing, profiles, fixed values, additional bindings) need data the loader dropped.

## What Changes
- ElementDefinition loads contentReference, type profiles, fixed/pattern/min/max values, slicing, maxLength.
- Binding loads additional bindings.
- Load only; the validator does not use them yet.

## Capabilities
### Modified
- fhir-definitions-registry
- fhir-model

## Impact
`fhir-definitions` public API gains five exported types.
