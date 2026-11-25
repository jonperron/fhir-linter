## MODIFIED Requirements

### Requirement: Validation terminologique — binding required (TERMINOLOGY_001)

Le système SHALL émettre `TERMINOLOGY_001` lorsqu'un élément, à toute profondeur (y compris dans un datatype, ex. `HumanName.use`, et pour une variante de choix dont le binding s'applique au type résolu de la variante, ex. `versionAlgorithmCoding`), de type `code`, `Coding` ou `CodeableConcept`, possède un binding de force `required` (Diagnostic d'erreur) ou `extensible` (Diagnostic de sévérité warning) dont le ValueSet est connu et que le code est prouvé hors du ValueSet. Les éléments sous un `contentReference` et dans les ressources embarquées SHALL être vérifiés comme tout autre élément (parcours unique) ; les autres forces de binding (`preferred`, `example`) SHALL être ignorées. Un binding épinglé à une version (`url|version`) différente de celle du ValueSet chargé est indécidé et NE DOIT JAMAIS être signalé. Un `Coding` sans `system` NE DOIT JAMAIS être considéré comme membre du ValueSet : il compte comme hors du ValueSet, même lorsque l'appartenance serait autrement indécidable. Hors de ce cas, une appartenance indécidable (`Registry::value_set_contains` retourne `None`) NE DOIT JAMAIS être signalée. Pour un `CodeableConcept`, un seul `coding` appartenant au ValueSet suffit ; le Diagnostic n'est émis que si tous les `coding` portant un code sont hors du ValueSet. Sous un binding `required`, un `CodeableConcept` sans aucun `coding` portant un code (texte seul) SHALL être signalé avec un message commençant par `CodeableConcept with no coded Coding at` (terminologies.html 4.1.4.2.1) ; sous un binding `extensible`, le texte seul SHALL être accepté (4.1.4.2.2). Sous un binding `required`, un élément de type `Coding` sans `code` SHALL être signalé avec un message commençant par `Coding with no code at`, comme un `CodeableConcept` sans `coding` codé ; sous un binding `extensible`, il NE DOIT PAS être signalé. Un `coding` unique non encapsulé dans un tableau (un seul `<coding>` en XML) SHALL être lu comme un tableau d'un élément. Les messages `TERMINOLOGY_001` et `TERMINOLOGY_002` SHALL nommer le canonical lié tel qu'écrit, suffixe `|version` compris (ex. `...|5.0.0`).

#### Scenario: Code hors du ValueSet requis

- **WHEN** un `code` lié en `required` à un ValueSet connu contient une valeur absente de ce ValueSet
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` SHALL être émis avec le `path` de l'élément (index inclus pour les tableaux)

#### Scenario: Code hors du ValueSet extensible

- **WHEN** un `CodeableConcept` lié en `extensible` à un ValueSet connu ne contient que des `coding` prouvés hors de ce ValueSet
- **THEN** un Diagnostic `TERMINOLOGY_001` de sévérité warning SHALL être émis

#### Scenario: Coding sans code sous binding required

- **WHEN** `Appointment.recurrenceTemplate.monthlyTemplate.dayOfWeek` (`Coding` lié en `required`) ne porte qu'un `system`
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` au message commençant par `Coding with no code at` SHALL être émis

#### Scenario: Binding preferred ou example ignoré

- **WHEN** l'élément a un binding de force `preferred` ou `example`
- **THEN** aucun Diagnostic terminologique SHALL être émis

#### Scenario: Code appartenant au ValueSet

- **WHEN** le code appartient au ValueSet
- **THEN** aucun Diagnostic terminologique SHALL être émis

#### Scenario: Appartenance indécidable

- **WHEN** le ValueSet est inconnu, utilise des filtres, des exclusions ou un CodeSystem fragmentaire/inconnu, et chaque `coding` porte un `system`
- **THEN** aucun Diagnostic `TERMINOLOGY_001` SHALL être émis

#### Scenario: Coding sans system

- **WHEN** un `Coding` lié en `required` porte un `code` mais pas de `system`, même si l'appartenance au ValueSet est indécidable
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` SHALL être émis

#### Scenario: CodeableConcept avec un coding valide

- **WHEN** un `CodeableConcept` contient au moins un `coding` appartenant au ValueSet
- **THEN** aucun Diagnostic `TERMINOLOGY_001` SHALL être émis

#### Scenario: CodeableConcept texte seul sous binding required

- **WHEN** un `CodeableConcept` lié en `required` n'a que `text` et aucun `coding` portant un code
- **THEN** un Diagnostic d'erreur `TERMINOLOGY_001` SHALL être émis avec un message commençant par `CodeableConcept with no coded Coding at`

#### Scenario: CodeableConcept texte seul sous binding extensible

- **WHEN** un `CodeableConcept` lié en `extensible` n'a que `text`
- **THEN** aucun Diagnostic SHALL être émis

#### Scenario: Coding XML unique

- **WHEN** un `CodeableConcept` XML lié en `required` contient un seul `<coding>` hors du ValueSet (objet et non tableau)
- **THEN** un Diagnostic `TERMINOLOGY_001` SHALL être émis comme pour un tableau d'un élément

#### Scenario: Élément imbriqué dans un datatype

- **WHEN** `Patient.name[0].use` contient un code hors du ValueSet requis
- **THEN** un Diagnostic `TERMINOLOGY_001` SHALL être émis au `path` `Patient.name[0].use`

#### Scenario: Variante de choix liée

- **WHEN** une variante de choix typée `Coding` (ex. `versionAlgorithmCoding`) est hors du ValueSet lié
- **THEN** un Diagnostic `TERMINOLOGY_001` SHALL être émis, le binding s'appliquant au type résolu de la variante

#### Scenario: Binding épinglé à une autre version

- **WHEN** le binding `required` nomme `url|version` et que le ValueSet chargé a une autre version
- **THEN** aucun Diagnostic `TERMINOLOGY_001` SHALL être émis (indécidé)
