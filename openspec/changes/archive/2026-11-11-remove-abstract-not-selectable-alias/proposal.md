## Why

L'audit du registre (`2026-11-11-definitions-registry-audit`) acceptait la propriété `abstract` comme alias de `notSelectable`. FHIR R5 ne définit pas d'alias `abstract` : la propriété standard est identifiée par l'URI `http://hl7.org/fhir/concept-properties#notSelectable`, et un CodeSystem peut lui donner n'importe quel code local dans `CodeSystem.property`. Le comportement précédent marquait à tort des concepts et ignorait ceux qui utilisent un code local.

## What Changes

- Suppression de l'alias `abstract` pour `notSelectable`.
- Un concept est non sélectionnable lorsque sa `property` dont le code est le code local déclaré dans `CodeSystem.property` avec `uri` = `http://hl7.org/fhir/concept-properties#notSelectable` porte `valueBoolean: true`.
- Repli sur le code `notSelectable` uniquement lorsqu'aucune propriété déclarée ne porte cette URI.

## Capabilities

### New Capabilities

_Aucune._

### Modified Capabilities

- `fhir-definitions-registry` : calcul de `CodeSystem.not_selectable` et scénario associé dans `value_set_contains`.

## Impact

- `crates/fhir-definitions/src/loader.rs`, `crates/fhir-definitions/src/model.rs`.
- Un concept porté par `abstract` ne rend plus `value_set_contains` indécidable ; un concept porté par un code local mappé sur l'URI le rend indécidable.
