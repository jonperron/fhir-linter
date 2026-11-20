## Context
Comparisons of a number with a Quantity previously yielded empty or a type error.

## Goals / Non-Goals
- Goal: follow N1 §5.5 for `=`, `!=`, `~`, `!~`, ordering, sort/min/max, set operations.
- Non-goal: `comparable()`, `+`, `-`, `div`, `mod` on Quantity.

## Decisions
- The number becomes `value '1'` and the existing UCUM comparison applies, so `1 = 100 '%'` is true and `1 = 1 'mg'` is empty.
- Set operations reuse equality so they agree with `=`.

## Risks
- Equality is no longer purely type-based; mixed number/Quantity collections can now deduplicate.
