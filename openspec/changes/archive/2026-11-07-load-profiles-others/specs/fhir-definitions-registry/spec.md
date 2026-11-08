## MODIFIED Requirements

### Requirement: Construction depuis un répertoire de définitions

Le système SHALL fournir `Registry::from_definitions_dir(dir)` pour construire un registry en chargeant les bundles FHIR R5 officiels depuis un répertoire.

#### Scenario: Chargement réussi depuis le répertoire definitions r5

- **WHEN** `Registry::from_definitions_dir` est appelé avec un chemin vers un répertoire contenant les fichiers FHIR R5 (`profiles-resources.json`, `profiles-types.json`, `profiles-others.json`, `valuesets.json`)
- **THEN** le registry SHALL être construit avec succès et contenir des StructureDefinitions, ValueSets et CodeSystems

#### Scenario: Profils core résolus

- **WHEN** le registry est construit depuis le répertoire definitions r5
- **THEN** `registry.structure_definition("http://hl7.org/fhir/StructureDefinition/vitalsigns")` SHALL retourner `Some`

#### Scenario: Répertoire inexistant retourne une erreur

- **WHEN** `Registry::from_definitions_dir` est appelé avec un chemin inexistant
- **THEN** il SHALL retourner une erreur `Error::Io`

#### Scenario: Fichier de définitions manquant retourne une erreur

- **WHEN** l'un des fichiers requis (dont `profiles-others.json`) est absent du répertoire
- **THEN** il SHALL retourner une erreur `Error::Io`
