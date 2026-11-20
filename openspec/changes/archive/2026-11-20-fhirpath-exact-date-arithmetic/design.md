## Context

PR #180 changes `add_duration` in `eval/temporal.rs`: amounts move from f64 to exact decimals, clock units on a `Date` become an error, and day-or-finer units on year- or month-precision values convert instead of failing.

## Goals / Non-Goals

- Goal: record these behaviour changes in the spec, per N1 §6.7.1/§6.7.2. Non-goal: other date/time behaviour.

## Decisions

- Year precision: 365 days/year (N1 example `@2016 + 365 days` = `@2017`). Month precision: 30 days/month (N2 conversion table; N1 silent).
- Count over i64 is an error rather than saturating.

## Risks

- The 30-day month conversion rests on N2, not N1.
