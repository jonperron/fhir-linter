## Context

The FHIRPath evaluator had no type information and saw the JSON split representation of primitives.

## Goals / Non-Goals

**Goals:** type temporal primitives by their declared type; make primitive `id`/`extension` visible the way R5 describes; fix `hasValue()`.

**Non-Goals:** typing in `resource_to_value`/`from_parser_value` (stay untyped, no Registry); following `contentReference`; static type checking.

## Decisions

- The Registry is injected as a closure (`ChildElement`) so `fhir-fhirpath` keeps no hard dependency on it at runtime.
- `Value::Primitive` wraps the value; operators and string/math/conversion functions unwrap it, a primitive without value is empty input.
- `hasValue()` returns `false` (not empty) for no value, several items or no item, because R5 constraints `exp-2`/`per-1` and the official suite expect it; empty stays for a complex value.
- `child_element` returns the element path for backbone types and the type code otherwise, so children of backbones and of data types resolve alike.

## Risks / Trade-offs

- `contentReference` elements are untyped (shortcut): their temporals compare as text.
- The `hasValue()` deviation from the R5 text is deliberate and documented.
