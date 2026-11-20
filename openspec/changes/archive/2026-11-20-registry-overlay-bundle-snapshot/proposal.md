## Why

Deux cas limites du chargement d overlay etaient non specifies : un `entry` de Bundle mal forme et le rechargement d une meme StructureDefinition.

## What Changes

- Un Bundle dont `entry` nest pas un tableau echoue avec `Error::MalformedBundle` ; sans `entry`, rien nest charge.
- Une URL de StructureDefinition chargee plusieurs fois dans une couche ne garde que son dernier etat.

## Capabilities

### Modified Capabilities

- `fhir-definitions-registry`: erreurs de overlay, StructureDefinition sans snapshot.

## Impact

`crates/fhir-definitions` (loader.rs, registry/overlay.rs). Aucun changement dAPI.
