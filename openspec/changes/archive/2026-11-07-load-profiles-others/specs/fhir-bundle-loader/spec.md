## MODIFIED Requirements

### Requirement: Fichiers de définitions FHIR R5 supportés

Le système SHALL supporter le chargement des bundles FHIR R5 officiels suivants : `profiles-resources.json`, `profiles-types.json`, `profiles-others.json`, `valuesets.json`.

#### Scenario: Chargement des profils officiels

- **WHEN** les quatre fichiers FHIR R5 officiels sont chargés
- **THEN** le résultat SHALL contenir des StructureDefinitions, ValueSets et CodeSystems issus des données officielles HL7
