## Context

`add_duration` in `eval/temporal.rs` already implemented the behaviour (commit 2399f4c); the spec lagged.

## Goals / Non-Goals

- Goal: spec matches the code. Non-goal: new behaviour.

## Decisions

- Year precision: 365 days/year (N1 example `@2016 + 365 days` = `@2017`). Month precision: 30 days/month (N2 conversion table; N1 silent).
- Count over i64 is an error rather than saturating.

## Risks

- The 30-day month conversion rests on N2, not N1.
