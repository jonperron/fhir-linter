## Context
UCUM definitions are read from an unmodified copy of the essence file (v2.0.1, UCUM License 1.1), parsed once via OnceLock.

## Goals / Non-Goals
Goals: exact i128 rational conversion. Non-goals: wiring into evaluation, special/arbitrary units.

## Decisions
- Factors are exact rationals over i128 with a 7-base-unit dimension vector; overflow yields None.
- No derived table is committed.

## Risks
235 of 310 atoms supported; units defined from [pi] or [h] are unsupported.
