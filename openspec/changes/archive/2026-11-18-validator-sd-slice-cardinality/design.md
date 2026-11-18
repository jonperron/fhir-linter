## Context

Runs in the same StructureDefinition pass as DEFINITION_001, so it covers the root and embedded resources.

## Goals / Non-Goals

- Goal: slice cardinality checks within one differential.
- Non-goal: minValue/maxValue range rules.

## Decisions

- Slicer max n is its stated max, else the max of the same-path element in the baseDefinition snapshot (Registry). Unbounded `*` or malformed n: no check.
- Slices are ids `<slicer id>:<name>`; reslices (`a/b`) are left out.
- Slice mins are summed with saturation, absent counting 0.
- Elements without `id` are skipped.

## Risks

A base max is only found when the baseDefinition is in the Registry; otherwise no check (shortcut).
