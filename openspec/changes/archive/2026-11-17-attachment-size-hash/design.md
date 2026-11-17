## Context

Attachment objects are found while descending nested elements (`descend` in `nested.rs`).

## Goals / Non-Goals

- Goal: compare decoded `data` with `size` and `hash`.
- Non-goal: format checks of `data`, `hash`, `size`; reported elsewhere.

## Decisions

- Skip silently when `data` is absent or undecodable, `hash` undecodable, or `size` not an integer, to avoid duplicate diagnostics.
- ASCII whitespace is ignored when decoding base64Binary.

## Risks

- Large `data` is decoded in memory.
