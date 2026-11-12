## Why

FHIRPath saw FHIR primitives as plain strings and could not see primitive extensions: dates compared as text, `_x` extensions were hidden or exposed as `_x` keys, and `hasValue()` could not tell a primitive without a value. Invariants such as `app-5`, `per-1` or `bdl-10` gave wrong results, and the official suite lost cases (F10).

## What Changes

- `resource_to_typed_value` types `date`/`dateTime`/`instant`/`time` from the declared element type, via the new `Registry::child_element`.
- New `Value::Primitive(value, children)`: JSON `_x` merged into `x` (by index for arrays), XML primitives with children, primitives without value.
- `hasValue()` true / empty / false semantics (deviation from R5 text, documented).
- `children()`/`descendants()` exclude `resourceType`; JSON nulls are not items.
- `=` between a complex value and a primitive is `false`.
- The validator evaluates invariants on typed values; the official suite floor rises from 865 to 877.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `fhir-fhirpath`: data model, typed conversion, navigation, `hasValue()`, equality, suite floor.
- `fhir-definitions-registry`: `child_element`.
- `fhir-validator`: invariants evaluated on typed values.

## Impact

`crates/fhir-fhirpath` (value.rs, eval/*), `crates/fhir-definitions/src/registry.rs`, `crates/fhir-validator/src/invariants.rs` (new `child_element` parameter on `validate`).
