## Context

Sources: datatypes.html#Coding, valueset-definitions.html#ValueSet.compose.include.system.

## Goals / Non-Goals

- Goal: flag systems equal to a Registry ValueSet URL (Coding at any depth, bound or not; compose include/exclude).
- Non-goal: flagging URIs unknown to the Registry.

## Decisions

- Lookup is against the Registry only, so unknown URIs never produce a diagnostic.

## Risks

- ValueSets absent from the Registry go unreported.
