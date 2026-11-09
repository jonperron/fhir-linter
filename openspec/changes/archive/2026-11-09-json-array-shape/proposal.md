## Why

Le validateur acceptait un champ JSON dont la forme contredit la définition de base (ex. `"name": {...}` au lieu d'un tableau, `"gender": ["male"]` au lieu d'un scalaire). Le format JSON FHIR impose qu'un élément répétable soit un tableau et qu'un élément `max="1"` n'en soit pas un.

## What Changes

- Nouveau diagnostic `STRUCTURE_002` (Error) : forme tableau/scalaire contraire à la définition de base, à toute profondeur, pour les entrées JSON uniquement.
- Le parcours imbriqué applique `STRUCTURE_002` en plus de STRUCTURE_001, CARDINALITY_* et TYPE_*.

## Capabilities

### New Capabilities

_Aucune._

### Modified Capabilities

- `fhir-validator` : ajout de la requirement STRUCTURE_002 ; mise à jour du parcours des éléments imbriqués.

## Impact

- `crates/fhir-validator/src/nested.rs` (`check_array_shape`), `crates/fhir-validator/src/lib.rs`.
- Des ressources JSON auparavant acceptées peuvent désormais produire `STRUCTURE_002`.
