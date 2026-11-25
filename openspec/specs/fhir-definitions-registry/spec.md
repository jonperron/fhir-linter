# Spec: fhir-definitions-registry

## Purpose

Registry FHIR R5 indexé par URL canonique, thread-safe, pour la résolution de définitions.
## Requirements
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

Le système SHALL fournir un type `Binding` public avec les champs : `strength: Arc<str>`, `value_set: Option<Arc<str>>`, `additional: Vec<AdditionalBinding>`. Chaque `ElementDefinition` SHALL exposer un champ `binding: Option<Binding>`. `AdditionalBinding` SHALL avoir les champs `purpose`, `value_set` (version canonique retirée) et `any: bool` (défaut `false`).

#### Scenario: Binding chargé depuis le snapshot

- **WHEN** un élément du snapshot déclare un binding (`binding` dans le JSON)
- **THEN** `element.binding` SHALL être `Some(Binding)` avec `strength` et `value_set` renseignés

#### Scenario: Binding sans valueSet

- **WHEN** un binding JSON ne possède pas de champ `valueSet`
- **THEN** `Binding.value_set` SHALL être `None`

#### Scenario: Élément sans binding

- **WHEN** un élément du snapshot ne déclare pas de binding
- **THEN** `element.binding` SHALL être `None`

#### Scenario: Binding additionnel chargé

- **WHEN** un binding déclare une entrée `additional` avec `purpose` et `valueSet` versionné
- **THEN** `Binding.additional` SHALL contenir une `AdditionalBinding` dont `value_set` n'a pas de version et dont `any` vaut `false` si absent

#### Scenario: Binding additionnel incomplet

- **WHEN** un binding additionnel n'a pas de `purpose` ou pas de `valueSet`
- **THEN** il SHALL être ignoré, comme un binding sans `strength`

---

### Requirement: Export public de Constraint et Binding

Le crate `fhir-definitions` SHALL exporter `Constraint`, `Binding`, `AdditionalBinding`, `ChoiceValue`, `Discriminator`, `Slicing` et `TypeProfiles` dans son API publique de premier niveau.

#### Scenario: Importation directe depuis fhir-definitions

- **WHEN** un crate dépendant importe `use fhir_definitions::{Binding, Constraint, AdditionalBinding, ChoiceValue, Discriminator, Slicing, TypeProfiles}`
- **THEN** la compilation SHALL réussir sans erreur

---

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

### Requirement: Conformité d'un type à une structure

Le système SHALL fournir `Registry::conforms_to(fhir_type, url) -> Option<Option<bool>>`. `None` SHALL signaler une URL inconnue ; `Some(Some(bool))` une définition de base, `true` si et seulement si elle définit exactement `fhir_type` ; `Some(None)` un profil de contrainte, non vérifié.

#### Scenario: Définition de base

- **WHEN** `conforms_to("Patient", url_Patient)` est appelé
- **THEN** le résultat SHALL être `Some(Some(true))`, et `Some(Some(false))` pour `Person` ou `DomainResource`

#### Scenario: Profil ou URL inconnue

- **WHEN** l'URL désigne un profil de contrainte ou n'est pas connue
- **THEN** le résultat SHALL être respectivement `Some(None)` et `None`

### Requirement: Appartenance d'un code à un ValueSet

Le système SHALL fournir `Registry::value_set_contains(url, system: Option<&str>, code) -> Option<bool>`. Le résultat SHALL être `Some(true)` / `Some(false)` lorsque l'appartenance est décidable depuis les définitions chargées, et `None` sinon : ValueSet ou CodeSystem inconnu, `filter`, `exclude`, CodeSystem incomplet, imports de ValueSets cycliques (profondeur limitée), ou code correspondant à un concept de `CodeSystem.not_selectable`. Avec `system == None`, le système du code est inconnu : `Some(false)` reste valable si aucun système ne contient le code, mais `Some(true)` SHALL être donné seulement si les codes du ValueSet (imports compris) viennent d'un seul système, sinon `None`. Un include dont la `version` est renseignée, différente de `"*"` et de `CodeSystem.version` SHALL NOT utiliser le CodeSystem chargé, qui est alors traité comme non chargé : un include sur le système entier SHALL donner `None`, un include énumérant ses codes SHALL décider d'après sa liste.

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

- **WHEN** le code correspond, dans le CodeSystem chargé, à un concept dont la propriété notSelectable (code local déclaré avec l'URI `http://hl7.org/fhir/concept-properties#notSelectable` ou, à défaut, code `notSelectable`) porte `valueBoolean: true`
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

Le système SHALL aussi fournir `Registry::shared_or_env(dir: Option<String>) -> Option<Result<Arc<Registry>>>` et la constante publique `DEFINITIONS_DIR_ENV = "FHIR_DEFINITIONS_DIR"`. Cette fonction SHALL utiliser `dir` s'il est fourni, sinon la variable d'environnement (absente ou non UTF-8 comptant comme non définie), charger via `Registry::shared`, et retourner `None` lorsqu'aucun répertoire n'est désigné, afin que chaque appelant formule sa propre erreur.

#### Scenario: Chemins équivalents partagent le même registry

- **WHEN** `Registry::shared` est appelé plusieurs fois avec des chemins désignant le même répertoire de définitions
- **THEN** chaque appel SHALL retourner le même `Arc` (`Arc::ptr_eq` vrai)

#### Scenario: Chemin non canonicalisable retourne une erreur

- **WHEN** `Registry::shared` est appelé avec un chemin inexistant
- **THEN** il SHALL retourner une erreur `Error::Io` et rien ne SHALL être mis en cache

#### Scenario: Répertoire explicite prioritaire sur l'environnement

- **WHEN** `Registry::shared_or_env(Some(dir))` est appelé alors que `FHIR_DEFINITIONS_DIR` est définie
- **THEN** le registry SHALL être chargé depuis `dir`

#### Scenario: Repli sur la variable d'environnement

- **WHEN** `Registry::shared_or_env(None)` est appelé et `FHIR_DEFINITIONS_DIR` désigne un répertoire
- **THEN** le registry SHALL être chargé depuis ce répertoire

#### Scenario: Aucun répertoire désigné

- **WHEN** `Registry::shared_or_env(None)` est appelé et `FHIR_DEFINITIONS_DIR` est absente ou non UTF-8
- **THEN** le résultat SHALL être `None`

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

### Requirement: Représentation XML d'un ElementDefinition

Chaque `ElementDefinition` SHALL exposer un champ `xml_attr: bool`, vrai lorsque sa `representation` contient `xmlAttr`, faux sinon.

#### Scenario: Élément représenté en attribut

- **WHEN** l'élément `Extension.url` est chargé depuis les définitions R5
- **THEN** son `xml_attr` SHALL être vrai

#### Scenario: Élément ordinaire

- **WHEN** l'élément `Patient.name` est chargé
- **THEN** son `xml_attr` SHALL être faux

### Requirement: Déclaration d'élément

Le `Registry` SHALL exposer `has_element(type_or_path: &str, name: &str) -> Option<bool>` : `Some(true)` si le type déclare `{racine}.{name}` ou `{racine}.{name}[x]`, `Some(false)` sinon (sans recherche de suffixe de choix : `valueQuantity` vaut `Some(false)` ; un nom contenant `.`, `[` ou `]` vaut `Some(false)`), et `None` pour `BackboneElement`, `Element` et les types inconnus. `type_or_path` peut aussi être un chemin d'instance (ex. `Patient.contact`, `Questionnaire.item.item`), résolu étape par étape avec `child_element` (à travers les types complexes et les `contentReference`) ; il vaut `None` si le chemin ne désigne aucun élément.

#### Scenario: Déclaration

- **WHEN** `has_element("Patient", "birthDate")`, `has_element("Observation", "valueQuantity")` et `has_element("Bogus", "x")` sont appelés
- **THEN** les résultats SHALL être `Some(true)`, `Some(false)` et `None`

#### Scenario: Chemin d'instance

- **WHEN** `has_element("Patient.contact", "name")`, `has_element("Questionnaire.item.item", "linkId")` et `has_element("Patient.bogus", "x")` sont appelés
- **THEN** les résultats SHALL être `Some(true)`, `Some(true)` et `None`

### Requirement: Élément enfant typé

Le `Registry` SHALL exposer `child_element(parent: &str, name: &str) -> Option<(Arc<str>, Arc<str>)>`, où `parent` est un chemin de définition (nom de type comme `Patient` ou `HumanName`, ou chemin d'élément backbone comme `Patient.contact`). Il SHALL retourner le chemin de définition de l'enfant et son code de type : le chemin de l'élément lui-même pour `BackboneElement` et `Element`, sinon le code de type. Pour un élément choix, le suffixe capitalisé SHALL choisir le type parmi les types déclarés de `base[x]`. Il SHALL retourner `None` pour un parent, un élément ou un suffixe inconnu, et pour un élément sans type ; les éléments `contentReference` (local `Questionnaire.item` ou `url#chemin`) SHALL être suivis vers l'élément cible.

#### Scenario: Primitive et backbone

- **WHEN** `child_element("Patient", "birthDate")` puis `child_element("Patient", "contact")` sont appelés
- **THEN** les résultats SHALL être `("date", "date")` puis `("Patient.contact", "BackboneElement")`

#### Scenario: contentReference suivi

- **WHEN** `child_element("Questionnaire.item", "item")` est appelé
- **THEN** le résultat SHALL être `("Questionnaire.item", "BackboneElement")`

#### Scenario: Variante d'un élément choix

- **WHEN** `child_element("Observation", "valueTime")` puis `child_element("Observation", "valueBogus")` sont appelés
- **THEN** les résultats SHALL être `("time", "time")` puis `None`

### Requirement: Références de contenu et profils de types

Chaque `ElementDefinition` SHALL exposer `content_reference: Option<Arc<str>>` (un `#chemin` local perd le `#` ; une référence absolue `url#chemin` est conservée entière) et `type_profiles: Vec<TypeProfiles>` où `TypeProfiles` a `code`, `profiles`, `target_profiles`. Une entrée SHALL exister uniquement pour un code de type déclarant `profile` ou `targetProfile`, versions canoniques retirées.

#### Scenario: contentReference locale

- **WHEN** un élément déclare `contentReference` valant `#Bundle.entry`
- **THEN** `content_reference` SHALL valoir `Bundle.entry`

#### Scenario: contentReference absolue

- **WHEN** un élément déclare `contentReference` valant `http://x/SD#A.b`
- **THEN** `content_reference` SHALL valoir `http://x/SD#A.b`

#### Scenario: Type sans profil

- **WHEN** un type ne déclare ni `profile` ni `targetProfile`
- **THEN** `type_profiles` SHALL ne contenir aucune entrée pour ce code

### Requirement: Valeurs fixed, pattern, minValue et maxValue

Chaque `ElementDefinition` SHALL exposer `fixed`, `pattern`, `min_value`, `max_value` de type `Option<Box<ChoiceValue>>`, où `ChoiceValue` a `key` (clé JSON, ex. `fixedCode`) et `value: serde_json::Value`. Une clé SHALL correspondre uniquement si le préfixe est suivi d'une majuscule.

#### Scenario: fixedCode chargé

- **WHEN** un élément porte `fixedCode: "x"`
- **THEN** `fixed` SHALL être `Some` avec `key == "fixedCode"` et la valeur `"x"`

#### Scenario: Clé de primitive étendue ignorée

- **WHEN** un élément ne porte que `_fixedCode`
- **THEN** `fixed` SHALL être `None`

### Requirement: isModifier et orderMeaning

Chaque `ElementDefinition` SHALL exposer `is_modifier: bool` (absent = `false`) et `order_meaning: bool` (vrai lorsque `orderMeaning` est présent).

#### Scenario: Indicateurs chargés

- **WHEN** un élément déclare `isModifier: true` et un `orderMeaning`
- **THEN** `is_modifier` et `order_meaning` SHALL être vrais ; sans ces champs, ils SHALL être faux

### Requirement: Slicing et longueur maximale

Chaque `ElementDefinition` SHALL exposer `slicing: Option<Box<Slicing>>` avec `discriminators: Vec<Discriminator{kind, path}>`, `rules` et `ordered` (défaut `false`), ainsi que `max_length: Option<u32>`. Un slicing sans `rules`, ou un discriminateur sans `type` ou `path`, SHALL faire échouer le chargement du bundle avec `MalformedBundle` nommant l'élément.

#### Scenario: Slicing chargé

- **WHEN** un élément déclare un slicing avec un discriminateur, `rules` et sans `ordered`
- **THEN** `slicing` SHALL être `Some` avec `ordered == false`

#### Scenario: Slicing sans rules

- **WHEN** un élément déclare un slicing sans `rules`
- **THEN** le chargement SHALL échouer avec `MalformedBundle` nommant l'élément

#### Scenario: Discriminateur sans path

- **WHEN** un discriminateur n'a pas de `path` ou de `type`
- **THEN** le chargement SHALL échouer avec `MalformedBundle` nommant l'élément

### Requirement: Regex des types primitifs

`Registry::primitive_regex(code: &str) -> Option<&str>` SHALL retourner le texte brut de l'extension R5 `regex` (`http://hl7.org/fhir/StructureDefinition/regex`) portée par le type de l'élément snapshot `<type>.value` de la StructureDefinition primitive. Le champ `StructureDefinition::value_regex` SHALL être renseigné par le chargeur uniquement pour le kind `primitive-type`. Le texte SHALL être conservé tel que publié : l'appelant doit l'ancrer en `\A(?:...)\z` (R5 datatypes.html), et la regex R5 de `decimal` contient une coquille (`[0-9]{1,9}}`). Le validateur ne l'utilise pas encore.

#### Scenario: Regex d'une primitive

- **WHEN** `primitive_regex("uri")` est appelé sur le bundle R5
- **THEN** le résultat SHALL être `Some("\S*")`

#### Scenario: Types sans regex

- **WHEN** `primitive_regex` reçoit un type complexe, une ressource, un code inconnu ou `xhtml`
- **THEN** le résultat SHALL être `None`

#### Scenario: Couverture du bundle R5

- **WHEN** le bundle R5 est chargé
- **THEN** 20 des 21 primitives SHALL exposer une regex

### Requirement: Ascendance des types

Le chargeur SHALL conserver `StructureDefinition.baseDefinition`, sans suffixe de version, dans `StructureDefinition::base_definition: Option<Arc<str>>` (`None` pour `Base`, racine R5). `Registry::type_ancestors(code: &str) -> impl Iterator<Item = &str>` SHALL produire le code lui-même puis chaque ancêtre, du plus proche au plus lointain, en suivant `baseDefinition` et en nommant chaque type par son URL canonique privée de `http://hl7.org/fhir/StructureDefinition/`. La marche SHALL s'arrêter lorsque la base est hors de cet espace de noms ou n'est pas chargée, et après autant d'étapes que de définitions chargées (garde contre les cycles). Un code inconnu SHALL produire un itérateur vide.

#### Scenario: Primitive

- **WHEN** `type_ancestors("code")` est appelé sur le bundle R5
- **THEN** la séquence SHALL être `code`, `string`, `PrimitiveType`, `DataType`, `Element`, `Base`

#### Scenario: Type complexe et profil

- **WHEN** `type_ancestors("Age")` ou `type_ancestors("SimpleQuantity")` est appelé
- **THEN** la séquence SHALL être `Age`, `Quantity`, `DataType`, `Element`, `Base`, respectivement `SimpleQuantity`, `Quantity`, `DataType`, `Element`, `Base`

#### Scenario: Ressource

- **WHEN** `type_ancestors("Patient")` est appelé
- **THEN** la séquence SHALL être `Patient`, `DomainResource`, `Resource`, `Base`

#### Scenario: Racine

- **WHEN** `type_ancestors("Base")` est appelé
- **THEN** la séquence SHALL être `Base` seul

#### Scenario: Code inconnu

- **WHEN** `type_ancestors` reçoit un code absent du registre
- **THEN** l'itérateur SHALL être vide

#### Scenario: Base hors espace de noms ou cycle

- **WHEN** une base est hors de `http://hl7.org/fhir/StructureDefinition/`, non chargée, ou forme un cycle
- **THEN** la marche SHALL s'arrêter sans boucler indéfiniment

### Requirement: Recherche d'une définition du noyau par nom

Le crate SHALL exporter la constante `CORE_PREFIX` (`http://hl7.org/fhir/StructureDefinition/`). `Registry::core(name)` SHALL retourner la `StructureDefinition` dont l'URL canonique est `CORE_PREFIX` suivi de `name`, ou `None` si aucune ne correspond. La recherche SHALL porter uniquement sur le registry racine (base, chargé depuis un répertoire) et jamais sur un overlay construit par `with_extra`, car en R5 seule une spécialisation définit un type : une StructureDefinition fournie (contrainte) ne crée donc aucun type.

#### Scenario: Nom connu

- **WHEN** `core("Patient")` est appelé sur le bundle R5
- **THEN** la définition retournée SHALL avoir l'URL `http://hl7.org/fhir/StructureDefinition/Patient`

#### Scenario: Nom inconnu ou vide

- **WHEN** `core("NotAType")` ou `core("")` est appelé
- **THEN** le résultat SHALL être `None`

#### Scenario: Définition fournie absente de core

- **WHEN** un overlay `with_extra` contient une StructureDefinition de contrainte d'URL `http://hl7.org/fhir/StructureDefinition/Zzz`
- **THEN** `structure_definition(url)` SHALL la retourner mais `core("Zzz")` SHALL être `None`

### Requirement: Interfaces implémentées par une StructureDefinition

Le chargeur SHALL conserver dans `StructureDefinition.implements: Vec<Arc<str>>` (public, vide pour `StructureDefinition::new`) la `valueUri`, sans suffixe de version, de chaque extension `structuredefinition-implements`.

#### Scenario: Extension implements chargée

- **WHEN** une StructureDefinition porte l'extension `structuredefinition-implements` de `valueUri` `http://hl7.org/fhir/StructureDefinition/CanonicalResource|5.0.0`
- **THEN** `implements` SHALL contenir `http://hl7.org/fhir/StructureDefinition/CanonicalResource`

### Requirement: Registry overlay avec définitions supplémentaires

Le système SHALL fournir `Registry::with_extra(base: Arc<Registry>, paths)` qui construit un registry overlay immuable. Chaque fichier SHALL être lu une seule fois, identifié par son chemin canonique : un fichier nommé plusieurs fois (directement, sous une autre graphie ou via son répertoire) SHALL être ignoré, sans conflit d'URL. Chaque chemin SHALL être soit un fichier `.json` (une ressource ou un Bundle), soit un répertoire lu sans récursion pour ses fichiers réguliers `*.json` en ordre trié (liens symboliques et autres fichiers ignorés). Seules les `StructureDefinition`, `ValueSet`, `CodeSystem` et `ConceptMap` SHALL être chargées ; les autres ressources et les définitions sans `url`, `name` ou `kind` SHALL être ignorées. `core` SHALL lire uniquement le registry racine, et donc `type_ancestors`, qui marche via `core`, aussi. Chaque autre recherche (`structure_definition`, `value_set`, `value_set_contains`, `expand`, `concept_map`, `translate`, CodeSystems) SHALL lire l'overlay puis la base ; `is_choice_variant` SHALL faire l'union des deux couches ; la borne de pas de `type_ancestors` SHALL compter les deux couches (borne simplement plus large). La base et `Registry::shared` SHALL rester inchangés et les overlays SHALL NOT être mis en cache.

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

### Requirement: Erreurs de chargement de l'overlay

`with_extra` SHALL nommer le fichier fautif dans chaque erreur et SHALL NOT en reproduire le contenu : `Error::Conflict{url,file}` si une URL existe déjà dans la base (toute version, quel que soit le type de ressource parmi StructureDefinition, ValueSet, CodeSystem et ConceptMap, car les URL canoniques FHIR R5 sont globalement uniques ; y compris une StructureDefinition de base sans snapshot) ou apparaît deux fois parmi les extras dans deux fichiers distincts, liens physiques inclus (même type ou types différents) ; `Error::Unsupported{file,message}` pour une StructureDefinition `derivation=specialization` ou un chemin qui n'est ni un fichier `.json` ni un répertoire ; `Error::Read{file,source}` pour un fichier illisible ou de plus de 64 MiB (`io::ErrorKind::FileTooLarge`) ; `Error::Limit{file,message}` au-delà de 256 MiB au total ou de 10 000 fichiers `.json` dans un répertoire ; `Error::Json` pour un JSON invalide ; `Error::MalformedBundle` pour un Bundle dont `entry` est présent mais n'est pas un tableau. Un Bundle sans `entry` ne charge rien, sans erreur.

#### Scenario: Conflit avec la base

- **WHEN** un extra déclare une URL déjà présente dans la base
- **THEN** `with_extra` SHALL retourner `Error::Conflict` avec l'URL et le fichier

#### Scenario: Fichier nommé deux fois

- **WHEN** un même fichier est nommé deux fois (directement, via une autre graphie du chemin ou via son répertoire)
- **THEN** il SHALL être lu une seule fois et `with_extra` SHALL réussir sans `Error::Conflict` ; deux fichiers distincts (liens physiques inclus) de même URL SHALL retourner `Error::Conflict`

#### Scenario: Doublon entre extras

- **WHEN** deux extras déclarent la même URL pour le même type de ressource
- **THEN** `with_extra` SHALL retourner `Error::Conflict`

#### Scenario: Conflit entre types avec la base

- **WHEN** un ValueSet fourni déclare l'URL d'une StructureDefinition de la base
- **THEN** `with_extra` SHALL retourner `Error::Conflict` avec l'URL et le fichier

#### Scenario: Doublon entre extras de types différents

- **WHEN** deux fichiers fournis de types de ressource différents déclarent la même URL
- **THEN** `with_extra` SHALL retourner `Error::Conflict` nommant le second fichier

#### Scenario: Spécialisation refusée

- **WHEN** un extra est une StructureDefinition `derivation=specialization`
- **THEN** `with_extra` SHALL retourner `Error::Unsupported` nommant le fichier

#### Scenario: Chemin invalide

- **WHEN** un chemin n'est ni un fichier `.json` ni un répertoire
- **THEN** `with_extra` SHALL retourner `Error::Unsupported`

#### Scenario: Fichier trop gros

- **WHEN** un fichier fait plus de 64 MiB
- **THEN** `with_extra` SHALL retourner `Error::Read` de kind `FileTooLarge`

#### Scenario: Limites globales

- **WHEN** le total dépasse 256 MiB ou un répertoire contient plus de 10 000 fichiers `.json`
- **THEN** `with_extra` SHALL retourner `Error::Limit`

#### Scenario: JSON invalide

- **WHEN** un extra n'est pas du JSON valide
- **THEN** `with_extra` SHALL retourner `Error::Json`

#### Scenario: Entry de Bundle mal formé

- **WHEN** un extra est un Bundle dont `entry` est présent mais n'est pas un tableau
- **THEN** `with_extra` SHALL retourner `Error::MalformedBundle` nommant le fichier

#### Scenario: Bundle sans entry

- **WHEN** un extra est un Bundle sans `entry`
- **THEN** `with_extra` SHALL réussir sans rien charger de ce Bundle

### Requirement: StructureDefinition sans snapshot

Une `StructureDefinition` sans snapshot (absent ou liste d'éléments vide) SHALL être écartée de la map de tout registry (base comprise ; aucune SD R5 embarquée n'en est dépourvue) et SHALL être signalée par `Registry::needs_snapshot(url) -> bool`. Lorsqu'une même URL est chargée plusieurs fois dans une couche, seul son dernier état SHALL être conservé : elle est soit dans la map (avec snapshot), soit dans `needs_snapshot`, jamais les deux.

#### Scenario: SD sans snapshot signalée

- **WHEN** une StructureDefinition chargée n'a pas de snapshot
- **THEN** `structure_definition(url)` SHALL retourner `None` et `needs_snapshot(url)` SHALL retourner `true`

#### Scenario: SD avec snapshot

- **WHEN** une StructureDefinition a un snapshot non vide
- **THEN** `needs_snapshot(url)` SHALL retourner `false`

#### Scenario: Dernier état conservé

- **WHEN** une même URL de StructureDefinition est chargée plusieurs fois dans une couche, avec puis sans snapshot (ou l'inverse)
- **THEN** seul le dernier état SHALL subsister : dans la map ou dans `needs_snapshot`, jamais les deux

### Requirement: Lecture plafonnée partagée

Le crate SHALL exporter `fhir_definitions::read_capped(path, cap)` et `MAX_INPUT_BYTES` (64 MiB), partagés avec la CLI, dont le comportement SHALL rester inchangé.

#### Scenario: Fichier au-delà du plafond

- **WHEN** `read_capped` lit un fichier plus grand que `cap`
- **THEN** il SHALL échouer sans lire le fichier en mémoire

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

Le système SHALL fournir `Registry::expand(vs_url) -> Option<Vec<(system, code, display)>>`, limitée à l'énumération exacte : CodeSystem complet entier ou liste de concepts, imports de ValueSets intersectés avec la profondeur limitée à 8, dédoublonnage par `(system, code)`, ordre du `compose`. Le résultat SHALL être `None` en présence de `filter`, d'`exclude`, d'un CodeSystem inconnu, incomplet ou épinglé par une `version` différente pour un include sur le système entier, d'un code `notSelectable`, d'un include dont une entrée `concept` n'a pas de `code` (indécidable), ou d'un cycle d'imports. Un CodeSystem complet énuméré SHALL être listé quelle que soit sa règle de casse (`caseSensitive`) ; seules les preuves d'absence restent limitées aux systèmes sensibles à la casse. Lorsque `compose.inactive` vaut `false`, le résultat SHALL être `None` (les concepts inactifs exclus sont indécidables car le Registry ne conserve pas le statut des concepts) ; absent ou `true`, tous les concepts sont inclus. La recherche SHALL se replier sur la base de l'overlay.

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

- **WHEN** le ValueSet a un `filter` ou un `exclude`, référence un CodeSystem inconnu, incomplet ou à version différente (système entier), contient un code `notSelectable`, a une entrée `concept` sans `code`, ou forme un cycle
- **THEN** `expand` SHALL retourner `None`

#### Scenario: compose.inactive à false

- **WHEN** un ValueSet déclare `compose.inactive` à `false`
- **THEN** `expand` et `value_set_contains` SHALL retourner `None` (indécidé)
- **AND** si `compose.inactive` est absent ou à `true`, les concepts inactifs SHALL être inclus

### Requirement: Traduction par ConceptMap

Le système SHALL fournir `Registry::translate(cm_url, system: Option<&str>, code) -> Option<Vec<(target_system, target_code, relationship)>>`. Le résultat SHALL être `None` lorsque le résultat est incertain : ConceptMap inconnu, code sans système avec plusieurs sources de groupe, système donné avec un groupe sans source, groupe sans `target` dont un élément correspondant porte des cibles, élément ou groupe indécidé, groupe dont la `source` ou la `target` est un canonical versionné (`url|version`). Les sorties de produit SHALL NOT être retournées. La recherche SHALL se replier sur la base de l'overlay.

#### Scenario: Traduction décidable

- **WHEN** un élément du groupe correspondant liste des cibles avec `code` et `relationship`
- **THEN** `translate` SHALL retourner les triplets `(target_system, target_code, relationship)`

#### Scenario: Traduction incertaine

- **WHEN** le ConceptMap est inconnu, ou l'élément ou le groupe est indécidé
- **THEN** `translate` SHALL retourner `None`

#### Scenario: Code sans système ambigu

- **WHEN** `system` est `None` et plusieurs groupes ont une source
- **THEN** `translate` SHALL retourner `None`

#### Scenario: Groupe versionné

- **WHEN** la `source` ou la `target` du groupe est de la forme `url|version`
- **THEN** `translate` SHALL retourner `None`

#### Scenario: CodeSystem insensible à la casse

- **WHEN** un include porte sur un CodeSystem complet énuméré dont `caseSensitive` est faux
- **THEN** `expand` SHALL lister ses concepts

### Requirement: Service de terminologie FHIRPath

Le système SHALL implémenter `fhir_fhirpath::Terminologies` pour `Registry` (`registry/terminology_service.rs`), au-dessus de `Registry::expand`, `value_set_contains` et `translate`, sans autre logique de décision. `expand` SHALL retourner un ValueSet `{resourceType, url, expansion {total, contains[{system, code, display?}]}}` (R5 OperationDefinition `ValueSet-expand` ; `status` et `expansion.timestamp` sont omis, raccourci documenté) et `None` lorsque `Registry::expand` est indécidable. `validate_vs` SHALL retourner des `Parameters {parameter: [{name: result, valueBoolean}]}` : `true` si un des codings appartient au ValueSet, `false` si chacun des codings (au moins un) est décidé hors du ValueSet, sinon `None`. `translate` SHALL retourner des `Parameters` avec `result` (vrai ssi une correspondance a une relation autre que `not-related-to`) et un `match` par cible portant les parties `relationship` (`valueCode`) et `concept` (`valueCoding {system, code}`) ; `None` lorsque `Registry::translate` est indécidable.

#### Scenario: Expansion

- **WHEN** `expand` est appelé pour un ValueSet énumérable
- **THEN** le résultat SHALL être un ValueSet avec `expansion.total` et `expansion.contains`

#### Scenario: validateVS décidé

- **WHEN** `validate_vs` reçoit des codings dont un appartient au ValueSet, ou dont tous sont hors du ValueSet
- **THEN** `result` SHALL valoir respectivement `true` et `false`

#### Scenario: validateVS indécidable

- **WHEN** aucun coding n'appartient au ValueSet et au moins un est indécidable
- **THEN** `validate_vs` SHALL retourner `None`

#### Scenario: Traduction

- **WHEN** `translate` trouve des correspondances
- **THEN** `result` SHALL être vrai ssi une relation diffère de `not-related-to`, avec un `match` par cible
