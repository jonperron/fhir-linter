## Context
R5 ElementDefinition minValue[x]/maxValue[x] are inclusive bounds.

## Goals / Non-Goals
Goal: numbers, temporal values, Quantity. Non-goal: UCUM unit conversion.

## Decisions
Called from fixed_pattern::check_value; temporal order follows FHIRPath, undecided means no diagnostic.

## Risks
Quantities in different units are never compared (known limit).
