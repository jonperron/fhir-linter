## Context

Follows 2026-11-16-fhirpath-calendar-duration-arithmetic.

## Goals / Non-Goals

Reject invalid operands; keep truncation exact. Non-goal: carrying leap seconds (N2 is silent).

## Decisions

- Leap seconds are refused rather than carried into the next day; month/year shifts keep them.
- Amounts are rounded to a nano-step only to absorb float noise, so 0.9999996 s stays 0.

## Risks

Refusing leap-second shifts may differ from other implementations.
