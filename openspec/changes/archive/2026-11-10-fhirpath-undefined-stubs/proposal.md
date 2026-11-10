## Why

Plusieurs fonctions FHIRPath (`resolve`, `memberOf`, `conformsTo`, `today`, `now`, `timeOfDay`, `lowBoundary`, `highBoundary`, `precision`, `htmlChecks`, `comparable`) étaient implémentées comme des stubs renvoyant un résultat deviné (collection vide, `true`, ...). Un invariant les utilisant (ex. `dgr-1` sur DiagnosticReport) était alors jugé sur un faux résultat, produisant de faux positifs ou de faux négatifs.

## What Changes

- Ces onze fonctions ne sont plus des stubs : leur appel retourne `Err(EvalError::UndefinedFunction(name))`.
- Le validateur, qui traduit déjà toute erreur d'évaluation en Diagnostic `Information` `INVARIANT_SKIPPED`, signale désormais ces invariants comme non vérifiés au lieu de les juger.
- La spec FHIRPath cite FHIRPath release 2 (2.0.0), version utilisée par FHIR R5, au lieu de FHIRPath 3.0.
- Une vraie implémentation de `resolve`/`memberOf` reste un travail futur.

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-fhirpath` : fonctions diverses réduites, limitations connues étendues, grammaire rattachée à FHIRPath release 2.
- `fhir-validator` : scénario `dgr-1` ignoré (INVARIANT_SKIPPED) car `resolve()` n'est pas implémenté.

## Impact

- `crates/fhir-fhirpath/src/eval/misc.rs`, `eval/mod.rs`, `lib.rs`
- `crates/fhir-fhirpath/tests/fhirpath_tests.rs`, `crates/fhir-validator/tests/validator_tests.rs`
- Les utilisateurs voient davantage de Diagnostics `INVARIANT_SKIPPED` et moins de violations inventées.
