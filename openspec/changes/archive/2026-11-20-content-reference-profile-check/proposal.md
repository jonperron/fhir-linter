## Why

Les références `contentReferenceProfile` invalides dans les profils passaient inaperçues.

## What Changes

- Nouvelle règle `DEFINITION_009` (Error).
- Planchers du tally `supporting` relevés de 31 à 38 (réussites et concordances de décompte).

## Capabilities

### New Capabilities
### Modified Capabilities
- `fhir-validator`: DEFINITION_009 ; planchers du manifeste.

## Impact

`crates/fhir-validator/src/structure_definition.rs`, test du manifeste.
