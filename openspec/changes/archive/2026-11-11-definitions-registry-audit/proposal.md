## Why

L'audit du registry de définitions a relevé quatre écarts : un champ `base_definition` chargé mais jamais utilisé, des concepts `notSelectable` acceptés comme membres d'un ValueSet, un `include.version` ignoré (le CodeSystem chargé décidait même pour une autre version), et des éléments de snapshot sans `path` ou `max` écartés en silence.

## What Changes

- **BREAKING** `StructureDefinition` n'a plus de champ `base_definition` ; le constructeur devient `StructureDefinition::new(url, name, kind, is_abstract, snapshot)`.
- `CodeSystem` expose `not_selectable` et `version` ; `Include` expose `version`.
- `value_set_contains` SHALL retourner `None` pour un code correspondant à un concept `notSelectable` (ou `abstract`).
- Un include dont la `version` est renseignée, différente de `*` et de `CodeSystem.version` SHALL ignorer le CodeSystem chargé.
- Un élément de snapshot sans `path` ou `max` SHALL faire échouer `load_bundle` avec `Error::MalformedBundle`.

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-definitions-registry` : exigences « Données snapshot de StructureDefinition », « ValueSet includes et CodeSystem codes », « Appartenance d'un code à un ValueSet ».
- `fhir-model` : exigence « StructureDefinition type ».

## Impact

`crates/fhir-definitions/src/{loader,model,registry}.rs` ; appels à `StructureDefinition::new` dans `crates/fhir-validator/src/{invariants,nested}.rs`.
