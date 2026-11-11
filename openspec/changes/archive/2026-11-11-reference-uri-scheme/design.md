## Context

Absolute/relative classification of `Reference.reference` used `contains(":")`, which misclassified relative references whose id contains a colon.

## Goals / Non-Goals

**Goals:** classify absolute references by a leading RFC 3986 scheme.

**Non-Goals:** full URI parsing or validation of absolute references beyond REFERENCE_003.

## Decisions

- Absolute iff the string matches `^[A-Za-z][A-Za-z0-9+.-]*:`. `urn:` references match too and remain ignored by REFERENCE_001.

## Risks / Trade-offs

- A relative reference whose type segment looks like a scheme (e.g. `Patient:1`) is treated as absolute; this matches RFC 3986.
