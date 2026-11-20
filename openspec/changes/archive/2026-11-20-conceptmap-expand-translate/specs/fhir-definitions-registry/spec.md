## ADDED Requirements

### Requirement: Chargement et accès aux ConceptMaps

Le `Registry` SHALL charger les ressources `ConceptMap` (via `load_resource`, donc aussi par les overlays) et exposer `Registry::concept_map(url) -> Option<&ConceptMap>` (overlay puis base). `ConceptMap` SHALL porter `url`, `version` et `groups: Vec<ConceptMapGroup>` (`source`, `target`, `elements`) ; `ConceptMapElement` SHALL porter `code` et `targets` ; `ConceptMapTarget` SHALL porter `code` et `relationship`. Une cible sans `code` ou sans `relationship`, ou portant `dependsOn`, SHALL marquer son élément comme indécidé ; `group.unmapped` ou un élément sans `code` SHALL marquer les codes non listés du groupe comme indécidés.

#### Scenario: ConceptMap chargé

- **WHEN** `conceptmaps.json` contient un ConceptMap avec un groupe, un élément et une cible
- **THEN** `concept_map(url)` SHALL retourner ce ConceptMap avec sa `version`, son groupe, son élément et la cible (`code`, `relationship`)

#### Scenario: Cible incomplète

- **WHEN** une cible n'a pas de `code`, pas de `relationship`, ou porte `dependsOn`
- **THEN** l'élément SHALL être marqué indécidé

#### Scenario: Groupe avec unmapped

- **WHEN** un groupe déclare `unmapped` ou contient un élément sans `code`
- **THEN** les codes non listés du groupe SHALL être marqués indécidés

### Requirement: Énumération exacte d'un ValueSet

Le système SHALL fournir `Registry::expand(vs_url) -> Option<Vec<(system, code, display)>>`, limitée à l'énumération exacte : CodeSystem complet entier ou liste de concepts, imports de ValueSets intersectés avec la profondeur limitée à 8, dédoublonnage par `(system, code)`, ordre du `compose`. Le résultat SHALL être `None` en présence de `filter`, d'`exclude`, d'un CodeSystem inconnu, incomplet ou épinglé par une `version` différente pour un include sur le système entier, d'un code `notSelectable`, ou d'un cycle d'imports. `compose.inactive` SHALL NOT être lu. La recherche SHALL se replier sur la base de l'overlay.

#### Scenario: Liste de concepts

- **WHEN** un ValueSet inclut une liste de concepts d'un système
- **THEN** `expand` SHALL retourner ces `(system, code, display)` dans l'ordre du compose, sans doublon

#### Scenario: CodeSystem complet entier

- **WHEN** un include référence un CodeSystem complet sans liste de concepts
- **THEN** `expand` SHALL retourner tous ses concepts dans l'ordre du CodeSystem

#### Scenario: Import de ValueSet

- **WHEN** un ValueSet importe un autre ValueSet
- **THEN** `expand` SHALL retourner l'intersection des énumérations

#### Scenario: Énumération impossible

- **WHEN** le ValueSet a un `filter` ou un `exclude`, référence un CodeSystem inconnu, incomplet ou à version différente (système entier), contient un code `notSelectable`, ou forme un cycle
- **THEN** `expand` SHALL retourner `None`

### Requirement: Traduction par ConceptMap

Le système SHALL fournir `Registry::translate(cm_url, system: Option<&str>, code) -> Option<Vec<(target_system, target_code, relationship)>>`. Le résultat SHALL être `None` lorsque le résultat est incertain : ConceptMap inconnu, code sans système avec plusieurs sources de groupe, système donné avec un groupe sans source, groupe sans `target` dont un élément correspondant porte des cibles, élément ou groupe indécidé. Les sorties de produit SHALL NOT être retournées. La recherche SHALL se replier sur la base de l'overlay.

#### Scenario: Traduction décidable

- **WHEN** un élément du groupe correspondant liste des cibles avec `code` et `relationship`
- **THEN** `translate` SHALL retourner les triplets `(target_system, target_code, relationship)`

#### Scenario: Traduction incertaine

- **WHEN** le ConceptMap est inconnu, ou l'élément ou le groupe est indécidé
- **THEN** `translate` SHALL retourner `None`

#### Scenario: Code sans système ambigu

- **WHEN** `system` est `None` et plusieurs groupes ont une source
- **THEN** `translate` SHALL retourner `None`

## MODIFIED Requirements

### Requirement: Construction depuis un répertoire de définitions

Le système SHALL fournir `Registry::from_definitions_dir(dir)` pour construire un registry en chargeant les bundles FHIR R5 officiels depuis un répertoire.

#### Scenario: Chargement réussi depuis le répertoire definitions r5

- **WHEN** `Registry::from_definitions_dir` est appelé avec un chemin vers un répertoire contenant les fichiers FHIR R5 (`profiles-resources.json`, `profiles-types.json`, `profiles-others.json`, `valuesets.json`, `conceptmaps.json`)
- **THEN** le registry SHALL être construit avec succès et contenir des StructureDefinitions, ValueSets et CodeSystems

#### Scenario: Profils core résolus

- **WHEN** le registry est construit depuis le répertoire definitions r5
- **THEN** `registry.structure_definition("http://hl7.org/fhir/StructureDefinition/vitalsigns")` SHALL retourner `Some`

#### Scenario: Répertoire inexistant retourne une erreur

- **WHEN** `Registry::from_definitions_dir` est appelé avec un chemin inexistant
- **THEN** il SHALL retourner une erreur `Error::Io`

#### Scenario: Fichier de définitions manquant retourne une erreur

- **WHEN** l'un des fichiers requis (dont `profiles-others.json` et `conceptmaps.json`) est absent du répertoire
- **THEN** il SHALL retourner une erreur `Error::Io`

### Requirement: ValueSet includes et CodeSystem codes

Le type `ValueSet` SHALL exposer `includes: Vec<Include>` (chaque `Include` : `system`, `version: Option<Arc<str>>`, `concepts: Vec<Concept>`, `has_filter`, `value_sets`) et `has_exclude: bool`. Le type `CodeSystem` SHALL exposer `version: Option<Arc<str>>`, `codes: HashSet<Arc<str>>` (concepts aplatis), `not_selectable: HashSet<Arc<str>>` (codes dont la `property` notSelectable porte `valueBoolean: true`, à tout niveau) et `concepts: Vec<Concept>` (concepts aplatis dans l'ordre du document, `Concept` : `code`, `display`), et `complete: bool` (faux pour un fragment ou contenu non présent). La propriété notSelectable SHALL être identifiée par le code local que le CodeSystem déclare dans `CodeSystem.property` avec `uri` = `http://hl7.org/fhir/concept-properties#notSelectable` ; uniquement lorsqu'aucune propriété déclarée ne porte cette URI, le code `notSelectable` SHALL être utilisé en repli. Le code `abstract` SHALL NOT être traité comme un alias de notSelectable, R5 ne le définissant pas. `Include`, `Concept`, `ConceptMap`, `ConceptMapGroup`, `ConceptMapElement` et `ConceptMapTarget` SHALL être exportés au premier niveau de `fhir-definitions`.

#### Scenario: Include chargé depuis compose

- **WHEN** un ValueSet déclare `compose.include` avec `system`, `version` et `concept`
- **THEN** `includes` SHALL contenir un `Include` avec ce `system`, cette `version` et ces `concepts`

#### Scenario: Concepts imbriqués d'un CodeSystem

- **WHEN** un CodeSystem déclare des concepts imbriqués
- **THEN** `codes` SHALL contenir tous les codes, y compris ceux des niveaux enfants

#### Scenario: Concepts non sélectionnables

- **WHEN** un CodeSystem ne déclare aucune propriété portant l'URI `http://hl7.org/fhir/concept-properties#notSelectable` et qu'un de ses concepts porte une `property` `notSelectable` avec `valueBoolean: true`
- **THEN** son code SHALL figurer dans `not_selectable` en plus de `codes`

#### Scenario: Code local mappé par URI

- **WHEN** un CodeSystem déclare dans `CodeSystem.property` un code local (par exemple `nonSel`) avec `uri` = `http://hl7.org/fhir/concept-properties#notSelectable`, et qu'un de ses concepts porte la `property` `nonSel` avec `valueBoolean: true`
- **THEN** son code SHALL figurer dans `not_selectable`

#### Scenario: Propriété abstract non reconnue

- **WHEN** un concept porte une `property` `abstract` avec `valueBoolean: true` et que `abstract` n'est pas déclaré dans `CodeSystem.property` avec l'URI notSelectable
- **THEN** son code SHALL NOT figurer dans `not_selectable`

### Requirement: Registry overlay avec définitions supplémentaires

Le système SHALL fournir `Registry::with_extra(base: Arc<Registry>, paths)` qui construit un registry overlay immuable. Chaque chemin SHALL être soit un fichier `.json` (une ressource ou un Bundle), soit un répertoire lu sans récursion pour ses fichiers réguliers `*.json` en ordre trié (liens symboliques et autres fichiers ignorés). Seules les `StructureDefinition`, `ValueSet`, `CodeSystem` et `ConceptMap` SHALL être chargées ; les autres ressources et les définitions sans `url`, `name` ou `kind` SHALL être ignorées. Chaque recherche (`structure_definition`, `core`, `value_set`, `value_set_contains`, `expand`, `concept_map`, `translate`, CodeSystems) SHALL lire l'overlay puis la base ; `is_choice_variant` SHALL faire l'union des deux couches ; la borne de pas de `type_ancestors` SHALL compter les deux couches. La base et `Registry::shared` SHALL rester inchangés et les overlays SHALL NOT être mis en cache.

#### Scenario: Définition supplémentaire résolue

- **WHEN** `with_extra` charge un fichier contenant une StructureDefinition d'URL inconnue de la base
- **THEN** `structure_definition(url)` SHALL la retourner et la base SHALL rester sans cette URL

#### Scenario: Repli sur la base

- **WHEN** une URL n'est pas dans l'overlay mais est dans la base
- **THEN** la recherche SHALL retourner la définition de la base

#### Scenario: Répertoire non récursif

- **WHEN** un chemin est un répertoire contenant des sous-répertoires, des liens symboliques et des fichiers non `.json`
- **THEN** seuls les fichiers réguliers `*.json` du premier niveau SHALL être chargés, en ordre trié

#### Scenario: ConceptMap supplémentaire

- **WHEN** `with_extra` charge un fichier contenant un ConceptMap d'URL inconnue de la base
- **THEN** `concept_map(url)` SHALL le retourner ; si l'URL existe déjà dans la base, `with_extra` SHALL retourner `Error::Conflict`
