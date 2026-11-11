## MODIFIED Requirements

### Requirement: Données snapshot de StructureDefinition

Le système SHALL charger les éléments du snapshot d'une `StructureDefinition` avec leurs cardinalités, types et `slice_name: Option<Arc<str>>` (valeur de `sliceName`, `None` sinon). Une `StructureDefinition` SHALL être construite par `StructureDefinition::new(url, name, kind, is_abstract, snapshot)`, qui indexe une seule fois les éléments par `path` ; le snapshot ne SHALL plus être modifié ensuite. `element(path)` SHALL retourner le premier élément du snapshot dont le `path` est exactement `path` (premier élément pour les chemins répétés des slices), `None` sinon. `descendants(path)` SHALL retourner la suite contiguë d'éléments qui suit cet élément et dont le `path` commence par `path.` (ou répète `path` pour les slices), et une suite vide si `path` est inconnu. Un élément du snapshot sans `path` ou sans `max` SHALL faire échouer `load_bundle` avec `Error::MalformedBundle` dont le message nomme le fichier, l'URL de la `StructureDefinition` et l'élément (son index s'il n'a pas de `path`, son `path` sinon) ; aucun élément ne SHALL être écarté en silence. Une `StructureDefinition` sans `url`, `name` ou `kind` SHALL rester ignorée sans erreur.

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

### Requirement: ValueSet includes et CodeSystem codes

Le type `ValueSet` SHALL exposer `includes: Vec<Include>` (chaque `Include` : `system`, `version: Option<Arc<str>>`, `codes`, `has_filter`, `value_sets`) et `has_exclude: bool`. Le type `CodeSystem` SHALL exposer `version: Option<Arc<str>>`, `codes: HashSet<Arc<str>>` (concepts aplatis), `not_selectable: HashSet<Arc<str>>` (codes dont une `property` `notSelectable` ou `abstract` porte `valueBoolean: true`, à tout niveau) et `complete: bool` (faux pour un fragment ou contenu non présent). `Include` SHALL être exporté au premier niveau de `fhir-definitions`.

#### Scenario: Include chargé depuis compose

- **WHEN** un ValueSet déclare `compose.include` avec `system`, `version` et `concept`
- **THEN** `includes` SHALL contenir un `Include` avec ce `system`, cette `version` et ces `codes`

#### Scenario: Concepts imbriqués d'un CodeSystem

- **WHEN** un CodeSystem déclare des concepts imbriqués
- **THEN** `codes` SHALL contenir tous les codes, y compris ceux des niveaux enfants

#### Scenario: Concepts non sélectionnables

- **WHEN** un concept d'un CodeSystem porte une `property` `notSelectable` (ou `abstract`) avec `valueBoolean: true`
- **THEN** son code SHALL figurer dans `not_selectable` en plus de `codes`

### Requirement: Appartenance d'un code à un ValueSet

Le système SHALL fournir `Registry::value_set_contains(url, system: Option<&str>, code) -> Option<bool>`. Le résultat SHALL être `Some(true)` / `Some(false)` lorsque l'appartenance est décidable depuis les définitions chargées, et `None` sinon : ValueSet ou CodeSystem inconnu, `filter`, `exclude`, CodeSystem incomplet, imports de ValueSets cycliques (profondeur limitée), ou code correspondant à un concept de `CodeSystem.not_selectable`. Avec `system == None`, n'importe quel système du ValueSet MAY correspondre. Un include dont la `version` est renseignée, différente de `"*"` et de `CodeSystem.version` SHALL NOT utiliser le CodeSystem chargé, qui est alors traité comme non chargé : un include sur le système entier SHALL donner `None`, un include énumérant ses codes SHALL décider d'après sa liste.

#### Scenario: Code énuméré

- **WHEN** le ValueSet énumère explicitement le code
- **THEN** le résultat SHALL être `Some(true)`, et `Some(false)` pour un code absent

#### Scenario: Système entier

- **WHEN** l'include référence un CodeSystem complet sans liste de codes
- **THEN** l'appartenance SHALL être décidée d'après `CodeSystem.codes`

#### Scenario: Indécidable

- **WHEN** le ValueSet est inconnu ou utilise un filtre
- **THEN** le résultat SHALL être `None`

#### Scenario: Concept non sélectionnable

- **WHEN** le code correspond à un concept marqué `notSelectable` (ou `abstract`) avec `valueBoolean: true` dans le CodeSystem chargé
- **THEN** le résultat SHALL être `None`

#### Scenario: Version d'include identique ou générique

- **WHEN** l'include sur le système entier déclare une `version` égale à `CodeSystem.version` ou `"*"`
- **THEN** l'appartenance SHALL être décidée d'après le CodeSystem chargé

#### Scenario: Version d'include différente sur le système entier

- **WHEN** l'include sur le système entier déclare une `version` différente de `CodeSystem.version`
- **THEN** le résultat SHALL être `None`

#### Scenario: Version d'include différente avec codes énumérés

- **WHEN** l'include déclare une `version` différente de `CodeSystem.version` et énumère ses codes
- **THEN** l'appartenance SHALL être décidée d'après la liste énumérée
