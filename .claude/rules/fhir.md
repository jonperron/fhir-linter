---
description: "Use when changing FHIR parsing, validation, FHIRPath or definitions loading. Lessons from the audit rounds on the validator, parser and FHIRPath crates."
paths: "crates/**/*.rs"
---
# FHIR rules

## Golden rule

The FHIR R5 specification is always right. The official artifacts in
`definitions/r5/` (StructureDefinitions, regexes, constraints) and the
spec text decide every behaviour. Any decision the FHIR specification does
not describe is wrong: no invented defaults, leniencies, shortcuts or
hard-coded lists. When the code and the spec disagree, the code is the bug.
When the spec is silent, flag the case to the user instead of choosing.

## Derive everything from the Registry

- Never hard-code element names, choice suffixes or type lists. A choice
  suffix is valid only if it names one of the element's declared types.
- Look elements up with `StructureDefinition::element(path)` and
  `descendants(path)` (indexed once at load), not linear `snapshot.iter()`
  scans.
- Type codes `http://hl7.org/fhirpath/System.*` resolve to the FHIR type in
  their `structuredefinition-fhir-type` extension; do not skip them.
- Elements with a `contentReference` and no type are not unknown: do not
  report `STRUCTURE_001` for them.
- `extension` and `modifierExtension` are typed `Extension` by the snapshot;
  validate them like any element, never skip keys by name.
- The registry loads every required bundle, `profiles-others.json`
  included; a missing bundle is `Error::Io`, never a silent gap.

## One walk, every depth

- Structure, cardinality and type checks run in the single recursive walk
  in `crates/fhir-validator/src/nested.rs`. Do not add a separate
  top-level pass: copies drift and disagree.
- A rule fixed at the top level must hold inside backbone elements and
  extensions, and the reverse. Test both depths.
- Profiles must be enforced at every depth, against the profile's own
  StructureDefinition, not the base one. Today `profile.rs` only checks
  the root's children with `check_children_cardinality`; nested profile
  constraints are a known gap. Close it by walking the profile snapshot
  with the shared helpers, never by copying them.

## Choice elements (`[x]`)

- Count a choice across all its variants plus primitive `_variant`
  extensions. None with `min > 0` gives `CARDINALITY_001` at the choice
  path; two variants give `CARDINALITY_002`.
- Type-check each variant against the type its suffix selects
  (`nested::resolve`).
- Keys containing `.`, `[` or `]` (for example `component.code` or a
  literal `value[x]`) are never resolved: `STRUCTURE_001`, with or without a
  `_` prefix, at every depth.

## Primitive extensions (`_name`)

- `_name` counts as presence only for primitive-typed elements, and only
  when it is an object, or an array of objects and nulls holding at least
  one object. `"_status": 5`, `[]` or `[null]` hide nothing.
- An over-long `_name` array gives `CARDINALITY_002`.

## Primitive formats

- Follow the R5 regexes exactly. date, dateTime, instant and time: year
  0001-9999, fraction of 1-9 digits, seconds up to 60, offset up to 14:00,
  day valid for its month (leap years included), no trailing garbage.
- instant always needs a zone; a dateTime with a time needs a zone; time
  takes no zone.
- Ranges (`TYPE_002` outside): integer is -2147483648..2147483647,
  unsignedInt is 0..2147483647, positiveInt is 1..2147483647. integer64
  follows `[0]|[-+]?[1-9][0-9]*` and must fit in i64.
- Resource `id` follows the FHIR id format (`TYPE_005` at `<Type>.id`).
  Nested element `id` children are checked as `string` (Element.id).
- A scalar where a complex or backbone element is expected
  (`"meta": null`, `"extension": "oops"`) is `TYPE_001`, not ignored.
- Never byte-slice strings. Formats whose R5 regex is ASCII-only (date,
  dateTime, instant, time, id) reject non-ASCII input with a diagnostic,
  never a panic. `string` and `markdown` accept any Unicode.

## Parser and JSON numbers

- JSON integers beyond i64 become decimals, so the validator reports
  `TYPE_001` instead of the whole parse failing. Numbers beyond f64 are
  "number out of range", never infinity.
- Duplicate JSON keys are rejected. JSON and XML nesting is capped
  (`MAX_DEPTH`) rather than overflowing the stack.

## FHIRPath

- Integer and Long arithmetic uses checked i64; overflow is
  `EvalError::Type`, never saturation or a silent switch to f64.
- Equality in union, distinct, in, contains, intersect, exclude and
  subsetOf is FHIRPath equality, through one shared helper.
- `matchesFull` anchors with `\A..\z`; invalid base64 in `decode()` is an
  error; descendants stay depth-first pre-order.
- Keep the parser limits (`MAX_INPUT_LEN`, `MAX_NESTING`) sized from the
  real constraints in `definitions/r5/`, and test worst-case inputs on a
  2 MB thread.

## Diagnostics never hide work

- A constraint that fails to parse or evaluate emits one Information
  `INVARIANT_SKIPPED` naming the key and reason. It is never a violation,
  so tests on valid resources filter by severity, not by count.
- Never drop a check silently: a skipped element, invariant or profile
  must be visible in the output or in a test.

## Tests that prove it

- Every audit finding gets a test that fails without the fix, in the same
  change, against the real `Registry` where the rule depends on it.
- A manifest case that fails to parse counts as an error in the pass rate;
  a regression must lower the rate, not raise it.
- Behaviour-neutral refactors and perf changes must keep fixture
  diagnostics byte-identical (`behaviour-differ`).
