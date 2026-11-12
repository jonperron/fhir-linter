## Context
Load-only extension of the snapshot parser.

## Goals / Non-Goals
Goal: expose the data. Non-goal: validation using it.

## Decisions
- Choice values keep the JSON key and raw value; the prefix must be followed by an uppercase letter.
- Large optional structures are boxed.
- Malformed slicing fails the bundle load like a missing path/max; incomplete additional bindings are skipped like a binding without strength.

## Risks
Public API growth; mitigated by load-only scope.
