## Why

Le registry charge désormais les ConceptMaps et offre l'énumération de ValueSets et la traduction ; les specs doivent le refléter.

## What Changes

- `conceptmaps.json` devient un fichier de définitions requis.
- Chargement des ConceptMaps (overlays inclus, conflit d'URL = `Error::Conflict`).
- Nouvelles API `concept_map`, `expand`, `translate` ; `Include.codes` devient `Include.concepts`, `CodeSystem.concepts` ordonné.

## Capabilities

### Modified Capabilities
- `fhir-definitions-registry`
- `fhir-bundle-loader`

## Impact

Crate `fhir-definitions`.
