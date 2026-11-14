## Why

Les valeurs construites depuis du XML gardaient les booléens et nombres en `String`, et un `<id>` racine hors ordre n'était pas détecté car le parseur le retire de `fields`.

## What Changes

- FHIRPath : le texte XML `boolean`/`integer`/`unsignedInt`/`positiveInt`/`decimal` est typé par le type déclaré ; un élément `Resource` enveloppant une ressource est converti ; helper `typed_xml_text`. Plancher de la suite officielle : 992.
- Parseur : `Resource.id` devient `Option<ResourceId>` (`value`, `span`).
- Validateur : l'`<id>` racine participe au contrôle STRUCTURE_004 ; positions via `StructureDefinition::position`.

## Capabilities

### New Capabilities

Aucune.

### Modified Capabilities

- `fhir-fhirpath`, `fhir-validator`, `fhir-xml-parser`, `fhir-json-parser`, `fhir-definitions-registry`.

## Impact

`crates/fhir-fhirpath` (value.rs, xml_text.rs), `crates/fhir-parser` (types.rs, json.rs, xml.rs), `crates/fhir-validator` (xml_shape.rs, embedded.rs, nested.rs), `crates/fhir-definitions/src/model.rs`.
