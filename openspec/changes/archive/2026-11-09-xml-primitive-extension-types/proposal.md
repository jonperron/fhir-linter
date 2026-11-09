## Why

Le remodelage XML laissait les primitifs des extensions de `_nom` en chaînes (faux `TYPE_001`) et supprimait les primitifs vides comme `<active/>`, masquant une ressource invalide.

## What Changes

- La partie `_nom` d'un primitif XML est remodelée comme un `Element` : les primitifs de ses extensions sont typés.
- Un primitif XML sans valeur ni `id`/`extension` est conservé tel quel, et `validate` émet `TYPE_001`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `fhir-validator`: remodelage XML des extensions de primitif et des primitifs vides.

## Impact

- `crates/fhir-validator/src/xml_shape.rs`, tests du validateur.
