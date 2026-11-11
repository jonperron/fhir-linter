## Why

L'opérateur `is` (`X is Type`) retournait `[false]` sur une entrée vide et `true` si un élément quelconque correspondait sur plusieurs éléments, en contradiction avec N1 §6.3.1 qui lui donne la même sémantique que la fonction `is()`.

## What Changes

- L'opérateur `is` partage la sémantique de `is()` : vide -> vide, un élément -> `[true]`/`[false]`, plusieurs éléments -> `EvalError::Type`.
- L'opérateur `as` est inchangé.

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-fhirpath`: sémantique de l'opérateur `is`.

## Impact

`crates/fhir-fhirpath/src/eval/` (mod.rs, collection.rs, helpers.rs) et `tests/fhirpath_tests.rs`.
