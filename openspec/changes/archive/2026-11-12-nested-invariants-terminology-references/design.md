## Context
The nested walk handled structure; semantic layers were root-only.

## Goals / Non-Goals
Goals: one walk evaluates all layers at every depth. Non-goals: contained/Bundle entries, contentReference, profile depth.

## Decisions
Expressions parsed once and cached; each key evaluated once per node.

## Risks
One INVARIANT_SKIPPED per node on unsupported expressions.
