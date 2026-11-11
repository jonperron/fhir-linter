## Why

A reference such as `Practitioner/a:b` was classified as absolute because it contains `:`, so it escaped REFERENCE_001 and REFERENCE_004. RFC 3986 defines an absolute URI by its leading scheme, not by the presence of a colon.

## What Changes

- A reference is absolute only when it starts with an RFC 3986 scheme (`ALPHA *(ALPHA / DIGIT / "+" / "-" / ".")` followed by `:`).
- `Practitioner/a:b` now gets REFERENCE_004 (warning); `Foo/a:b` gets REFERENCE_001.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `fhir-validator`: REFERENCE_001 and REFERENCE_004 define relative references as having no URI scheme.

## Impact

- `crates/fhir-validator/src/references.rs`
