## Context

Mirrors the existing `include.version` handling for CodeSystems.

## Goals / Non-Goals

Goal: no false positives on version mismatch. Non-goal: semver or partial-version matching.

## Decisions

- Exact string equality on version; `5.0` does not match `5.0.0` (undecided).
- Unversioned references behave as before.

## Risks

Previously reported mismatches on pinned bindings are now silent (intended).
