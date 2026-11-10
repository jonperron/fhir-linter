## Why

Les règles LINT_001/002/003 manquaient des cas que la spécification FHIR R5 considère comme absents (narratif vide, tableaux vides ou nuls) et signalaient des Bundles transitoires. Les diagnostics n'avaient pas de position.

## What Changes

- `narrative-absent` : déclenche aussi sur `text.status = "empty"` et sur un div sans contenu (narrative.html, txt-2) ; surensemble documenté de dom-6.
- `missing-identifier` : tableau vide ou nul = absent ; Bundle signalé seulement pour `document`, `message`, `collection`.
- `missing-meta-profile` : tableau vide ou nul = absent.
- Positions : LINT_002/003 sur `meta`, LINT_001 sur `text`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-linter` : précision des déclencheurs et des positions des règles intégrées.

## Impact

`crates/fhir-linter/src/rules.rs`, `crates/fhir-linter/tests/rules_tests.rs`.
