## Why

Resource ids and every element typed with a FHIRPath system type (`http://hl7.org/fhirpath/System.*`, e.g. `*.id`) were never type-checked: the parser lifts `id` out of `fields` and the system type codes matched no FHIR type.

## What Changes

- The loader resolves `http://hl7.org/fhirpath/System.*` type codes to the FHIR type in the `structuredefinition-fhir-type` extension's `valueUrl`.
- The validator checks the resource `id` against the FHIR `id` format (TYPE_005 at `<Type>.id`, no source location).
- Nested element `id` children are checked as `string` (the base `Element.id` type).

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-bundle-loader`: FHIRPath system type codes resolved to FHIR types.
- `fhir-validator`: TYPE_005 covers the resource id; element ids checked as `string`.

## Impact

`crates/fhir-definitions/src/loader.rs`, `crates/fhir-validator/src/type_check.rs`, `crates/fhir-validator/src/nested.rs`.
