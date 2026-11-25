## MODIFIED Requirements

### Requirement: Type Binding sur ElementDefinition

Le système SHALL fournir un type `Binding` public avec les champs : `strength: Arc<str>`, `value_set: Option<Arc<str>>`, `additional: Vec<AdditionalBinding>`. Chaque `ElementDefinition` SHALL exposer un champ `binding: Option<Binding>`. `AdditionalBinding` SHALL avoir les champs `purpose`, `value_set` (suffixe `|version` conservé tel qu'écrit) et `any: bool` (défaut `false`).

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
- **THEN** `Binding.additional` SHALL contenir une `AdditionalBinding` dont `value_set` conserve son suffixe `|version` et dont `any` vaut `false` si absent

#### Scenario: Binding additionnel incomplet

- **WHEN** un binding additionnel n'a pas de `purpose` ou pas de `valueSet`
- **THEN** il SHALL être ignoré, comme un binding sans `strength`

### Requirement: ValueSet includes et CodeSystem codes

Le type `ValueSet` SHALL exposer `version: Option<Arc<str>>` (`ValueSet.version`), `includes: Vec<Include>` (chaque `Include` : `system`, `version: Option<Arc<str>>`, `concepts: Vec<Concept>`, `has_filter`, `value_sets`) et `has_exclude: bool`. Le chargeur SHALL conserver le suffixe `|version` de `binding.valueSet`, de `binding.additional[].valueSet` et de `compose.include.valueSet` (`Include.value_sets`) tel qu'écrit. Le type `CodeSystem` SHALL exposer `version: Option<Arc<str>>`, `codes: HashSet<Arc<str>>` (concepts aplatis), `not_selectable: HashSet<Arc<str>>` (codes dont la `property` notSelectable porte `valueBoolean: true`, à tout niveau) et `concepts: Vec<Concept>` (concepts aplatis dans l'ordre du document, `Concept` : `code`, `display`), et `complete: bool` (faux pour un fragment ou contenu non présent). La propriété notSelectable SHALL être identifiée par le code local que le CodeSystem déclare dans `CodeSystem.property` avec `uri` = `http://hl7.org/fhir/concept-properties#notSelectable` ; uniquement lorsqu'aucune propriété déclarée ne porte cette URI, le code `notSelectable` SHALL être utilisé en repli. Le code `abstract` SHALL NOT être traité comme un alias de notSelectable, R5 ne le définissant pas. `Include`, `Concept`, `ConceptMap`, `ConceptMapGroup`, `ConceptMapElement` et `ConceptMapTarget` SHALL être exportés au premier niveau de `fhir-definitions`.

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

#### Scenario: Version de ValueSet et références versionnées conservées

- **WHEN** un ValueSet déclare `version`, ou un binding / un `compose.include.valueSet` référence `url|version`
- **THEN** `ValueSet.version` SHALL porter la version et les références SHALL conserver leur suffixe `|version`

### Requirement: Appartenance d'un code à un ValueSet

Le système SHALL fournir `Registry::value_set_contains(url, system: Option<&str>, code) -> Option<bool>`. Le résultat SHALL être `Some(true)` / `Some(false)` lorsque l'appartenance est décidable depuis les définitions chargées, et `None` sinon : ValueSet ou CodeSystem inconnu, ValueSet sans `compose.include` (`compose` est 0..1 en R5) ou important un tel ValueSet, `filter`, `exclude`, CodeSystem incomplet, imports de ValueSets cycliques (profondeur limitée), ou code correspondant à un concept de `CodeSystem.not_selectable`. Avec `system == None`, le système du code est inconnu : `Some(false)` reste valable si aucun système ne contient le code, mais `Some(true)` SHALL être donné seulement si les codes du ValueSet (imports compris) viennent d'un seul système, sinon `None`. Un include dont la `version` est renseignée, différente de `"*"` et de `CodeSystem.version` SHALL NOT utiliser le CodeSystem chargé, qui est alors traité comme non chargé : un include sur le système entier SHALL donner `None`, un include énumérant ses codes SHALL décider d'après sa liste. Une référence `url|version` (argument `url`, ou `include.valueSet` importé) SHALL être lue par `Registry::value_set` : le ValueSet chargé n'est retourné que si sa `version` est égale à celle référencée, exactement (une version partielle comme `5.0` face à `5.0.0` ne correspond pas) ; une version différente ou absente donne `None`, donc un résultat indécidé, y compris par imports imbriqués. Une référence sans version se comporte comme avant.

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

#### Scenario: Version de ValueSet différente

- **WHEN** `url|version` référence une version différente de `ValueSet.version` chargé (ou le ValueSet chargé n'a pas de version), y compris via un `include.valueSet` imbriqué
- **THEN** `value_set_contains` SHALL retourner `None`

#### Scenario: Version de ValueSet identique

- **WHEN** `url|version` référence exactement `ValueSet.version` chargé
- **THEN** l'appartenance SHALL être décidée d'après ce ValueSet

### Requirement: Énumération exacte d'un ValueSet

Le système SHALL fournir `Registry::expand(vs_url) -> Option<Vec<(system, code, display)>>`, limitée à l'énumération exacte : CodeSystem complet entier ou liste de concepts, imports de ValueSets intersectés avec la profondeur limitée à 8, dédoublonnage par `(system, code)`, ordre du `compose`. Le résultat SHALL être `None` en présence de `filter`, d'`exclude`, d'un CodeSystem inconnu, incomplet ou épinglé par une `version` différente pour un include sur le système entier, d'un code `notSelectable`, d'un include dont une entrée `concept` n'a pas de `code` (indécidable), d'un cycle d'imports, ou d'une référence `url|version` (ValueSet demandé ou importé) dont la version diffère de `ValueSet.version` chargé (voir `value_set_contains`). Un CodeSystem complet énuméré SHALL être listé quelle que soit sa règle de casse (`caseSensitive`) ; seules les preuves d'absence restent limitées aux systèmes sensibles à la casse. Lorsque `compose.inactive` vaut `false`, le résultat SHALL être `None` (les concepts inactifs exclus sont indécidables car le Registry ne conserve pas le statut des concepts) ; absent ou `true`, tous les concepts sont inclus. La recherche SHALL se replier sur la base de l'overlay.

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

#### Scenario: ValueSet sans compose.include

- **WHEN** un ValueSet n'a aucun `compose.include` (`compose` est 0..1 en R5, ex. ValueSet à `expansion` seule), ou importe un tel ValueSet
- **THEN** `expand` et `value_set_contains` SHALL retourner `None` (indécidé), et non `Some([])` / `Some(false)`

#### Scenario: Version de ValueSet différente

- **WHEN** `expand` reçoit `url|version` dont la version diffère de celle du ValueSet chargé, ou un `include.valueSet` versionné de la sorte
- **THEN** `expand` SHALL retourner `None`
