## Context
N1 §5.5 conversion table has Integer and Decimal -> Quantity, no Long.

## Goals / Non-Goals
Align spec with code. No code change.

## Decisions
`1L = 1 '1'` is empty (no conversion, so no common type); `(1L | 1 '1').count()` is 2.

## Risks
None; narrows previously documented behaviour to the spec text.
