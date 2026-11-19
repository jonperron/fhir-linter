## Context

Exact decimals align both operands to a common scale; that overflowed i128 for distant scales.

## Goals / Non-Goals

Goal: defined results wherever the true result is representable. Non-goal: exponent String output.

## Decisions

Compute the quotient from the scale difference instead of aligning; a divisor beyond i128 exceeds any mantissa. toString returns empty when Display would use an exponent, as the String grammar has no form for it.

## Risks

Other overflows still return a type error.
