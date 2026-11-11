# Spec: fhir-definitions-registry

## Purpose

Registry FHIR R5 indexé par URL canonique, thread-safe, pour la résolution de définitions.
## Requirements
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

### Requirement: Registry partagé par répertoire

Le système SHALL fournir `Registry::shared(dir) -> Result<Arc<Registry>>` qui charge le registry d'un répertoire une seule fois et le partage. Le cache est propre à chaque copie liée de la crate (deux modules natifs compilés séparément dans un même processus ont chacun le leur). Le cache SHALL être indexé par le chemin canonicalisé (`defs`, `./defs` ou un lien symbolique désignent le même répertoire), SHALL être thread-safe, SHALL ne pas conserver les chargements en échec et SHALL ne jamais évincer d'entrée.

#### Scenario: Chemins équivalents partagent le même registry

- **WHEN** `Registry::shared` est appelé plusieurs fois avec des chemins désignant le même répertoire de définitions
- **THEN** chaque appel SHALL retourner le même `Arc` (`Arc::ptr_eq` vrai)

#### Scenario: Chemin non canonicalisable retourne une erreur

- **WHEN** `Registry::shared` est appelé avec un chemin inexistant
- **THEN** il SHALL retourner une erreur `Error::Io` et rien ne SHALL être mis en cache

### Requirement: Index des variantes d'éléments choix

Le `Registry` SHALL construire au chargement un index associant le nom de base de chaque élément `*.base[x]` aux codes de type déclarés, capitalisés, et SHALL exposer `is_choice_variant(base: &str, suffix: &str) -> bool`. La correspondance SHALL se faire par nom de base, indépendamment du chemin de l'élément (raccourci documenté).

#### Scenario: Variante déclarée

- **WHEN** `is_choice_variant("value", "Quantity")` est appelé sur le Registry R5
- **THEN** le résultat SHALL être `true` (`Observation.value[x]` déclare `Quantity`)

#### Scenario: Correspondance par nom, pas par chemin

- **WHEN** `is_choice_variant("dose", "Quantity")` est appelé
- **THEN** le résultat SHALL être `true` (grâce à `Dosage.doseAndRate.dose[x]`), si bien que `dose` trouve aussi `Immunization.doseQuantity`, qui n'est pas un élément choix

#### Scenario: Élément non-choix

- **WHEN** `is_choice_variant("modifier", "Extension")` est appelé
- **THEN** le résultat SHALL être `false`

