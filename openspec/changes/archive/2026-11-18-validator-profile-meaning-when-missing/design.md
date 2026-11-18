## Context

Runs in `validate_one`, so it covers the root and every embedded resource.

## Goals / Non-Goals

- Goal: flag meaningWhenMissing in profile differentials.
- Non-goal: checking the snapshot (a profile snapshot copies the base's meanings).

## Decisions

- Extension definition = `type` Extension and `baseDefinition` (without `|version`) equal to the core Extension StructureDefinition url from the Registry; a profile on another extension is still reported.
- A constraint without `type` is left to the required-field check.

## Risks

Version-suffix stripping only handles `|`; other canonical forms are not normalised.
