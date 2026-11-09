## Why

Les champs `_nom` étaient ignorés à toute profondeur : un `_bogus`, un `_name` (type complexe) ou un contenu invalide dans `_birthDate` passaient sans diagnostic.

## What Changes

- Un `_nom` est accepté seulement si `nom` résout un élément (ou une variante de choix) de type primitif ; sinon `STRUCTURE_001` au chemin de `_nom`.
- Un `_nom` accepté est parcouru comme le type complexe `Element` (`id`, `extension`) ; tableau parcouru élément par élément, `null` ignorés.
- Une valeur non objet (ou un élément de tableau ni objet ni `null`) produit `TYPE_001`.
- Règles de présence/cardinalité de `_nom` inchangées.

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-validator`: STRUCTURE_001 et parcours imbriqué pour les champs `_nom`.

## Impact

- `crates/fhir-validator/src/nested.rs` (`walk`), tests dans `crates/fhir-validator/tests/validator_tests.rs`.
