## Why

Le validateur ne vérifiait que les bindings `required`, acceptait un Coding sans `system` et un `#` seul, et ne contrôlait ni la cohérence `Reference.type` / URL FHIR ni la forme des références relatives (terminologies.html 4.1.4.2, references.html 2.1.3.0.1, ref-1).

## What Changes

- `TERMINOLOGY_001` : bindings `extensible` (warning) en plus de `required` (erreur) ; CodeableConcept texte seul en erreur sous `required` ; Coding sans `system` jamais membre ; un `<coding>` XML unique lu comme un tableau.
- Nouveau `TERMINOLOGY_002` (warning) : Coding lié avec un code mais sans `system`.
- `REFERENCE_002` : `#` seul est une erreur (ressource racine).
- Nouveau `REFERENCE_003` (erreur) : `Reference.type` en désaccord avec le type d'une URL FHIR.
- Nouveau `REFERENCE_004` (warning) : référence relative mal formée vers un type connu.

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-validator` : règles terminologiques et de références.

## Impact

`crates/fhir-validator/src/terminology.rs`, `crates/fhir-validator/src/references.rs` et leurs tests.
