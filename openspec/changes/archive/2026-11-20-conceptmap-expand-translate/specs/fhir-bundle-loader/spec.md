## MODIFIED Requirements

### Requirement: Gestion des types de ressources inconnus

Le système SHALL ignorer silencieusement toute entrée de bundle dont le `resourceType` n'est pas `StructureDefinition`, `ValueSet`, `CodeSystem` ou `ConceptMap`.

#### Scenario: Type inconnu ignoré

- **WHEN** un bundle contient une entrée `Observation` ou tout autre type non géré
- **THEN** cette entrée SHALL être ignorée sans erreur, et les autres entrées SHALL être traitées normalement

### Requirement: Fichiers de définitions FHIR R5 supportés

Le système SHALL supporter le chargement des bundles FHIR R5 officiels suivants : `profiles-resources.json`, `profiles-types.json`, `profiles-others.json`, `valuesets.json`, `conceptmaps.json`.

#### Scenario: Chargement des profils officiels

- **WHEN** les cinq fichiers FHIR R5 officiels sont chargés
- **THEN** le résultat SHALL contenir des StructureDefinitions, ValueSets, CodeSystems et ConceptMaps issus des données officielles HL7
