# Spec: fhir-definitions-registry

## Purpose

Registry FHIR R5 indexé par URL canonique, thread-safe, pour la résolution de définitions.
## Requirements
### Requirement: Construction depuis un répertoire de définitions

Le système SHALL fournir `Registry::from_definitions_dir(dir)` pour construire un registry en chargeant les bundles FHIR R5 officiels depuis un répertoire.

#### Scenario: Chargement réussi depuis le répertoire definitions r5

- **WHEN** `Registry::from_definitions_dir` est appelé avec un chemin vers un répertoire contenant les fichiers FHIR R5 (`profiles-resources.json`, `profiles-types.json`, `valuesets.json`)
- **THEN** le registry SHALL être construit avec succès et contenir des StructureDefinitions, ValueSets et CodeSystems

#### Scenario: Répertoire inexistant retourne une erreur

- **WHEN** `Registry::from_definitions_dir` est appelé avec un chemin inexistant
- **THEN** il SHALL retourner une erreur `Error::Io`

---

### Requirement: Résolution de StructureDefinition par URL canonique

Le système SHALL permettre de résoudre une `StructureDefinition` par son URL canonique exacte.

#### Scenario: URL connue retourne Some StructureDefinition

- **WHEN** `registry.structure_definition(url)` est appelé avec l'URL canonique d'une définition chargée
- **THEN** il SHALL retourner `Some(&StructureDefinition)`

#### Scenario: URL inconnue retourne None

- **WHEN** `registry.structure_definition(url)` est appelé avec une URL non présente dans le registry
- **THEN** il SHALL retourner `None`

#### Scenario: Résolution sensible à la casse

- **WHEN** une URL est cherchée avec une casse différente de l'URL canonique
- **THEN** `structure_definition` SHALL retourner `None`

---

### Requirement: Résolution de ValueSet par URL canonique

Le système SHALL permettre de résoudre un `ValueSet` par son URL canonique exacte.

#### Scenario: URL connue retourne Some ValueSet

- **WHEN** `registry.value_set(url)` est appelé avec l'URL canonique d'un ValueSet chargé
- **THEN** il SHALL retourner `Some(&ValueSet)` avec les champs `url` et `name` correctement renseignés

#### Scenario: URL inconnue retourne None pour ValueSet

- **WHEN** `registry.value_set(url)` est appelé avec une URL non présente
- **THEN** il SHALL retourner `None`

---

### Requirement: Thread-safety via Arc

Le `Registry` SHALL être partageable entre threads via `Arc<Registry>` sans nécessiter de synchronisation externe après construction.

#### Scenario: Accès concurrent en lecture

- **WHEN** le registry est partagé via `Arc` entre plusieurs threads
- **THEN** chaque thread SHALL pouvoir appeler les méthodes de résolution de manière sûre et concurrente

---

### Requirement: Données snapshot de StructureDefinition

Le système SHALL charger les éléments du snapshot d'une `StructureDefinition` avec leurs cardinalités et types.

#### Scenario: Snapshot contient les éléments avec path et types

- **WHEN** une `StructureDefinition` est chargée depuis un bundle FHIR R5
- **THEN** son champ `snapshot` SHALL contenir des `ElementDefinition` avec `path`, `min`, `max`, et `types` renseignés selon les données JSON source

#### Scenario: base_definition renseignée pour les ressources dérivées

- **WHEN** une `StructureDefinition` a un champ `baseDefinition` dans le JSON source
- **THEN** `base_definition` SHALL contenir l'URL de la définition parente

### Requirement: Type Constraint sur ElementDefinition

Le système SHALL fournir un type `Constraint` public avec les champs : `key: Arc<str>`, `severity: Arc<str>` (valeur `"error"` ou `"warning"`), `human: Arc<str>`, `expression: Option<Arc<str>>`. Chaque `ElementDefinition` SHALL exposer un champ `constraints: Vec<Constraint>`.

#### Scenario: Contraintes chargées depuis le snapshot

- **WHEN** une `StructureDefinition` est chargée depuis un bundle FHIR R5 et contient des éléments avec des contraintes (`constraint` array dans le JSON)
- **THEN** `element.constraints` SHALL contenir les `Constraint` correspondants avec `key`, `severity`, `human` et `expression` renseignés

#### Scenario: Contrainte sans expression

- **WHEN** un élément de contrainte JSON ne possède pas de champ `expression`
- **THEN** `Constraint.expression` SHALL être `None`

#### Scenario: Élément sans contraintes produit un Vec vide

- **WHEN** un élément du snapshot ne déclare pas de contraintes
- **THEN** `element.constraints` SHALL être un `Vec` vide

---

### Requirement: Type Binding sur ElementDefinition

Le système SHALL fournir un type `Binding` public avec les champs : `strength: Arc<str>`, `value_set: Option<Arc<str>>`. Chaque `ElementDefinition` SHALL exposer un champ `binding: Option<Binding>`.

#### Scenario: Binding chargé depuis le snapshot

- **WHEN** un élément du snapshot déclare un binding (`binding` dans le JSON)
- **THEN** `element.binding` SHALL être `Some(Binding)` avec `strength` et `value_set` renseignés

#### Scenario: Binding sans valueSet

- **WHEN** un binding JSON ne possède pas de champ `valueSet`
- **THEN** `Binding.value_set` SHALL être `None`

#### Scenario: Élément sans binding

- **WHEN** un élément du snapshot ne déclare pas de binding
- **THEN** `element.binding` SHALL être `None`

---

### Requirement: Export public de Constraint et Binding

Le crate `fhir-definitions` SHALL exporter `Constraint` et `Binding` dans son API publique de premier niveau.

#### Scenario: Importation directe depuis fhir-definitions

- **WHEN** un crate dépendant importe `use fhir_definitions::{Binding, Constraint}`
- **THEN** la compilation SHALL réussir sans erreur

---

### Requirement: ValueSet includes et CodeSystem codes

Le type `ValueSet` SHALL exposer `includes: Vec<Include>` (chaque `Include` : `system`, `codes`, `has_filter`, `value_sets`) et `has_exclude: bool`. Le type `CodeSystem` SHALL exposer `codes: HashSet<Arc<str>>` (concepts aplatis) et `complete: bool` (faux pour un fragment ou contenu non présent). `Include` SHALL être exporté au premier niveau de `fhir-definitions`.

#### Scenario: Include chargé depuis compose

- **WHEN** un ValueSet déclare `compose.include` avec `system` et `concept`
- **THEN** `includes` SHALL contenir un `Include` avec ce `system` et ces `codes`

#### Scenario: Concepts imbriqués d'un CodeSystem

- **WHEN** un CodeSystem déclare des concepts imbriqués
- **THEN** `codes` SHALL contenir tous les codes, y compris ceux des niveaux enfants

---

### Requirement: Appartenance d'un code à un ValueSet

Le système SHALL fournir `Registry::value_set_contains(url, system: Option<&str>, code) -> Option<bool>`. Le résultat SHALL être `Some(true)` / `Some(false)` lorsque l'appartenance est décidable depuis les définitions chargées, et `None` sinon : ValueSet ou CodeSystem inconnu, `filter`, `exclude`, CodeSystem incomplet, ou imports de ValueSets cycliques (profondeur limitée). Avec `system == None`, n'importe quel système du ValueSet MAY correspondre.

#### Scenario: Code énuméré

- **WHEN** le ValueSet énumère explicitement le code
- **THEN** le résultat SHALL être `Some(true)`, et `Some(false)` pour un code absent

#### Scenario: Système entier

- **WHEN** l'include référence un CodeSystem complet sans liste de codes
- **THEN** l'appartenance SHALL être décidée d'après `CodeSystem.codes`

#### Scenario: Indécidable

- **WHEN** le ValueSet est inconnu ou utilise un filtre
- **THEN** le résultat SHALL être `None`
