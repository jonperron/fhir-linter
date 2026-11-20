## Why

Les utilisateurs doivent charger des définitions supplémentaires (profils, ValueSets, CodeSystems) sans modifier ni dupliquer le registry R5 partagé.

## What Changes

- `Registry::with_extra(base, paths)` construit un registry overlay immuable au-dessus d'un `Arc<Registry>`.
- Les `StructureDefinition` sans snapshot sont écartées de la map et signalées par `Registry::needs_snapshot`.
- `read_capped` et `MAX_INPUT_BYTES` (64 MiB) deviennent publics, partagés avec la CLI.

## Capabilities

### Modified Capabilities

- `fhir-definitions-registry`: overlay, snapshots absents, lecture plafonnée.

## Impact

`crates/fhir-definitions` (registry/overlay.rs, input.rs, loader.rs, lib.rs). Comportement de la CLI inchangé.
