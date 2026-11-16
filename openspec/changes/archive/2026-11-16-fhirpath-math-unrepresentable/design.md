## Context

sqrt/power produced non-finite f64 wrapped in Decimal.

## Goals / Non-Goals

Goal: empty on non-finite sqrt/power. Non-goal: ln, log, exp.

## Decisions

Check `is_finite()` on the f64 result and return empty otherwise.

## Risks

Callers relying on NaN/inf Decimals now get empty.
