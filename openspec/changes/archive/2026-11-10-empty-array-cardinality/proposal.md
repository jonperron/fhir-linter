## Why

Audit 11.2-A1 (décision utilisateur 2026-11-10) : un tableau JSON vide (`[]`) ou ne contenant que des `null` satisfaisait la cardinalité d'un élément requis. FHIR R5 (json.html) stipule que les objets ne sont jamais vides et qu'un tableau de seuls `null` SHOULD être omis.

## What Changes

- Un tableau vide ou de seuls `null` compte comme absent pour la cardinalité, à toute profondeur (parcours unique, réutilisé par la couche profil).
- Élément requis (`min >= 1`) : `CARDINALITY_001` (Error).
- Élément optionnel (`min = 0`) : nouveau code `CARDINALITY_003` (Warning).
- Un `_nom` contenant un objet garde l'élément primitif présent.

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-validator` : CARDINALITY_001 étendu, nouvelle exigence CARDINALITY_003.

## Impact

`crates/fhir-validator/src/nested.rs` ; nouveaux tests dans `crates/fhir-validator/tests/validator_tests.rs`. Des ressources auparavant valides reçoivent désormais un warning ou une erreur.
