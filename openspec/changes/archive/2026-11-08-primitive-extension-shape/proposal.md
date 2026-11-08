## Why

Une extension de primitif `_nom` mal formée (scalaire, `[]`, `[null]`) satisfaisait un élément requis ou un choix requis, masquant l'absence réelle de valeur.

## What Changes

- `_nom` (et `_variante` d'un choix primitif) ne compte comme présent que si sa valeur est un objet, ou un tableau d'objets et de `null` contenant au moins un objet.
- Sinon `CARDINALITY_001` est émis (ex. `Observation.status`, `Immunization.occurrence[x]`), à toute profondeur.

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-validator`: règle de présence `_nom` pour CARDINALITY_001 et la cardinalité des choix.

## Impact

`crates/fhir-validator/src/nested.rs` (`is_extension_shape`, `present`, `check_choice`) et tests dans `crates/fhir-validator/tests/validator_tests.rs`.
