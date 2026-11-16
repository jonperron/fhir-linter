## Context

Temporal values are stored as text; arithmetic must keep precision and zone text.

## Goals / Non-Goals

- Goals: N2 §6.6.7/§6.6.8 arithmetic, clamped month shifts, strict errors.
- Non-Goals: UCUM `a`/`mo`, day-or-larger units on Time (undefined by N2).

## Decisions

- Finer units convert to the value's precision by truncation.
- Fractions are written back as 3 digits.

## Risks

- Truncation may surprise users (`@2019-03-01 + 23 hours` is unchanged).
