## Why

Une ressource issue de `parse_xml` ne contient que des chaînes ; le validateur, conçu pour la forme JSON, ne pouvait ni typer les primitifs ni lire `id`/`extension` d'un primitif.

## What Changes

- `Resource` gagne `from_xml: bool` (vrai pour `parse_xml`).
- `validate` remodèle une fois les ressources XML en forme JSON (`xml_shape.rs`).

## Capabilities

### Modified Capabilities

- `fhir-validator`: remodelage XML avant validation.

## Impact

`crates/fhir-parser/src/{types,xml,json}.rs`, `crates/fhir-validator/src/{lib,xml_shape}.rs`.
