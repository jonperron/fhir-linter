## Why

`fhir_validator::validate` évaluait le FHIRPath en mode non strict : un nom d'élément mal orthographié donnait une collection vide, donc un faux résultat silencieux.

## What Changes

- `validate` est toujours strict (un `Strict` par appel, issu de `Registry::has_element`) ; `validate_strict` est supprimé.
- Une expression nommant un élément inconnu donne `INVARIANT_SKIPPED` (message `constraint <key> not checked: <reason> (in '<expression>')`) ou `SLICING_SKIPPED` pour un discriminateur.
- Le test ignoré de comparaison de corpus est remplacé par `core_invariants_resolve_every_name_on_a_corpus_sample`.

## Capabilities

### Modified Capabilities

- `fhir-validator`: navigation stricte, messages INVARIANT_SKIPPED et SLICING_SKIPPED.
- `fhir-fhirpath`: description du crochet strict côté validateur.

## Impact

- `crates/fhir-validator` (invariants, profils, slicing) ; aucun changement de plancher de taux de réussite.
