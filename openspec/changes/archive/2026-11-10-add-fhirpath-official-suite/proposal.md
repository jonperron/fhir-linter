## Why

Le crate `fhir-fhirpath` dispose désormais d'un test d'intégration qui exécute la suite officielle FHIRPath R5 (`tests-fhir-r5.xml`). La spécification doit décrire ce harnais et son plancher de régression, comme pour `fhir-validator`.

## What Changes

- Ajout d'une exigence « Exécution de la suite de tests officielle FHIRPath » à la spec `fhir-fhirpath`.

## Capabilities

### New Capabilities

_Aucune._

### Modified Capabilities

- `fhir-fhirpath` : nouvelle exigence sur l'exécution de la suite officielle et son plancher de 758 cas réussis.

## Impact

- Test : `crates/fhir-fhirpath/tests/official_suite.rs`.
- Fixtures : `tests/fixtures/r5/fhirpath/tests-fhir-r5.xml` (sous-module `fhir-test-cases`).
