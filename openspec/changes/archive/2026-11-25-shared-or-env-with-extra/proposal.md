## Why

Les bindings Python et Node dupliquaient la résolution base + surcouche. Elle est centralisée dans `fhir-definitions`.

## What Changes

- Ajout de `Registry::shared_or_env_with_extra` et de `SharedRegistryError`.
- Les bindings l'appellent ; leur API et leurs messages sont inchangés.

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-definitions-registry`: nouvelle exigence sur la résolution avec surcouche.

## Impact

`crates/fhir-definitions/src/shared.rs`, bindings Python et Node. Comportement neutre.
