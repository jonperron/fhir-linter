## MODIFIED Requirements

### Requirement: Données snapshot de StructureDefinition

Le système SHALL charger les éléments du snapshot d'une `StructureDefinition` avec leurs cardinalités et types. Une `StructureDefinition` SHALL être construite par `StructureDefinition::new(url, name, kind, is_abstract, base_definition, snapshot)`, qui indexe une seule fois les éléments par `path` ; le snapshot ne SHALL plus être modifié ensuite. `element(path)` SHALL retourner le premier élément du snapshot dont le `path` est exactement `path` (premier élément pour les chemins répétés des slices), `None` sinon. `descendants(path)` SHALL retourner la suite contiguë d'éléments qui suit cet élément et dont le `path` commence par `path.` (ou répète `path` pour les slices), et une suite vide si `path` est inconnu.

#### Scenario: Snapshot contient les éléments avec path et types

- **WHEN** une `StructureDefinition` est chargée depuis un bundle FHIR R5
- **THEN** son champ `snapshot` SHALL contenir des `ElementDefinition` avec `path`, `min`, `max`, et `types` renseignés selon les données JSON source

#### Scenario: base_definition renseignée pour les ressources dérivées

- **WHEN** une `StructureDefinition` a un champ `baseDefinition` dans le JSON source
- **THEN** `base_definition` SHALL contenir l'URL de la définition parente

#### Scenario: Recherche d'un élément par chemin

- **WHEN** `element("Patient.name")` est appelé sur la StructureDefinition `Patient`
- **THEN** l'élément du snapshot de path `Patient.name` SHALL être retourné, et `element("Patient.bogus")` SHALL retourner `None`

#### Scenario: Descendants d'un élément

- **WHEN** `descendants("Observation.component")` est appelé sur la StructureDefinition `Observation`
- **THEN** les éléments `Observation.component.*` qui suivent `Observation.component` dans le snapshot SHALL être retournés, sans élément hors de ce sous-arbre

#### Scenario: Descendants d'un chemin inconnu

- **WHEN** `descendants("Patient.bogus")` est appelé
- **THEN** une suite vide SHALL être retournée
