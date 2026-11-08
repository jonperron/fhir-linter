## Context

R5 snapshots type `*.id` elements as `http://hl7.org/fhirpath/System.String` with the real FHIR type in the `structuredefinition-fhir-type` extension.

## Goals / Non-Goals

**Goals:** type-check resource ids and system-typed elements. **Non-Goals:** source spans for the resource id.

## Decisions

- Resolve the extension in the loader (`type_code`), falling back to the raw code when absent.
- Resource `id` is checked in `type_check.rs` from `resource.id`; the parser keeps no span for it, so the diagnostic has no location.
- Nested `id` children are checked as `string`: R5 labels many of them `id`, but real element ids contain `:` or exceed 64 characters.

## Risks / Trade-offs

- TYPE_005 for the resource id carries no source location.
