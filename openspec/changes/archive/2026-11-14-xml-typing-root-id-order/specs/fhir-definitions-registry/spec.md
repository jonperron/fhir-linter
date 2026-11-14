## MODIFIED Requirements

### Requirement: Données snapshot de StructureDefinition

Le système SHALL charger les éléments du snapshot d'une `StructureDefinition` avec leurs cardinalités, types et `slice_name: Option<Arc<str>>` (valeur de `sliceName`, `None` sinon). Une `StructureDefinition` SHALL être construite par `StructureDefinition::new(url, name, kind, is_abstract, snapshot)`, qui indexe une seule fois les éléments par `path` ; le snapshot ne SHALL plus être modifié ensuite. `element(path)` SHALL retourner le premier élément du snapshot dont le `path` est exactement `path` (premier élément pour les chemins répétés des slices), `None` sinon. `position(path)` SHALL retourner l'indice dans le snapshot de `element(path)` (sa place dans l'ordre des éléments), `None` si `path` est inconnu. `descendants(path)` SHALL retourner la suite contiguë d'éléments qui suit cet élément et dont le `path` commence par `path.` (ou répète `path` pour les slices), et une suite vide si `path` est inconnu. Un élément du snapshot sans `path` ou sans `max` SHALL faire échouer `load_bundle` avec `Error::MalformedBundle` dont le message nomme le fichier, l'URL de la `StructureDefinition` et l'élément (son index s'il n'a pas de `path`, son `path` sinon) ; aucun élément ne SHALL être écarté en silence. Une `StructureDefinition` sans `url`, `name` ou `kind` SHALL rester ignorée sans erreur.

#### Scenario: Snapshot contient les éléments avec path et types

- **WHEN** une `StructureDefinition` est chargée depuis un bundle FHIR R5
- **THEN** son champ `snapshot` SHALL contenir des `ElementDefinition` avec `path`, `min`, `max`, et `types` renseignés selon les données JSON source

#### Scenario: Élément de snapshot sans path

- **WHEN** un bundle contient une `StructureDefinition` dont un élément du snapshot n'a pas de `path`
- **THEN** `load_bundle` SHALL retourner `Error::MalformedBundle` dont le message contient l'URL de la `StructureDefinition` et l'index de l'élément

#### Scenario: Élément de snapshot sans max

- **WHEN** un bundle contient une `StructureDefinition` dont un élément du snapshot n'a pas de `max`
- **THEN** `load_bundle` SHALL retourner `Error::MalformedBundle` dont le message contient l'URL de la `StructureDefinition` et le `path` de l'élément

#### Scenario: StructureDefinition sans url ignorée

- **WHEN** un bundle contient une `StructureDefinition` sans `url`, `name` ou `kind`
- **THEN** elle SHALL être ignorée et `load_bundle` SHALL réussir

#### Scenario: Recherche d'un élément par chemin

- **WHEN** `element("Patient.name")` est appelé sur la StructureDefinition `Patient`
- **THEN** l'élément du snapshot de path `Patient.name` SHALL être retourné, et `element("Patient.bogus")` SHALL retourner `None`

#### Scenario: Descendants d'un élément

- **WHEN** `descendants("Observation.component")` est appelé sur la StructureDefinition `Observation`
- **THEN** les éléments `Observation.component.*` qui suivent `Observation.component` dans le snapshot SHALL être retournés, sans élément hors de ce sous-arbre

#### Scenario: Descendants d'un chemin inconnu

- **WHEN** `descendants("Patient.bogus")` est appelé
- **THEN** une suite vide SHALL être retournée
