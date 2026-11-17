## Context
bundle.html 2.4.5.1 and Bundle.entry.fullUrl define resolution and the id constraint.

## Goals / Non-Goals
Goals: type checks for fullUrl references; REFERENCE_007. Non-goals: relative resolution against fullUrl bases; comparing the type part.

## Decisions
Innermost Bundle is the scope; ambiguous fullUrls (different types) are skipped to avoid false positives.

## Risks
Unresolved references are silent by design.
