## MODIFIED Requirements

### Requirement: ValueSet includes et CodeSystem codes

Le type `ValueSet` SHALL exposer `includes: Vec<Include>` (chaque `Include` : `system`, `version: Option<Arc<str>>`, `codes`, `has_filter`, `value_sets`) et `has_exclude: bool`. Le type `CodeSystem` SHALL exposer `version: Option<Arc<str>>`, `codes: HashSet<Arc<str>>` (concepts aplatis), `not_selectable: HashSet<Arc<str>>` (codes dont la `property` notSelectable porte `valueBoolean: true`, à tout niveau) et `complete: bool` (faux pour un fragment ou contenu non présent). La propriété notSelectable SHALL être identifiée par le code local que le CodeSystem déclare dans `CodeSystem.property` avec `uri` = `http://hl7.org/fhir/concept-properties#notSelectable` ; uniquement lorsqu'aucune propriété déclarée ne porte cette URI, le code `notSelectable` SHALL être utilisé en repli. Le code `abstract` SHALL NOT être traité comme un alias de notSelectable, R5 ne le définissant pas. `Include` SHALL être exporté au premier niveau de `fhir-definitions`.

#### Scenario: Include chargé depuis compose

- **WHEN** un ValueSet déclare `compose.include` avec `system`, `version` et `concept`
- **THEN** `includes` SHALL contenir un `Include` avec ce `system`, cette `version` et ces `codes`

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
