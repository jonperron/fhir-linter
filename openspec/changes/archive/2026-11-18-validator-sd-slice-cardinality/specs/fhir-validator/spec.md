## ADDED Requirements

### Requirement: Validation des profils — cardinalité des tranches (DEFINITION_002)

Pour toute StructureDefinition (profil ou spécialisation) ayant un `differential`, le système SHALL, pour chaque `differential.element[i]` portant `slicing`, déterminer le maximum n de l'élément découpé : son `max` énoncé, sinon le `max` de l'élément de même chemin dans le snapshot de la `baseDefinition` (Registry). Si n est `*` ou mal formé, aucun contrôle SHALL être effectué. Selon R5 (profiling, Slice Cardinality), le système SHALL émettre une erreur `DEFINITION_002` : (a) à `<ressource>.differential.element[<i>].max` pour chaque tranche (id `<id du découpé>:<nom>`, les re-tranches `a/b` exclues) dont le `max` énoncé est `*` ou supérieur à n; (b) à `<ressource>.differential.element[<index du découpé>]` si la somme des `min` énoncés des tranches (absent = 0, addition saturante) dépasse n. Les éléments sans `id` SHALL être ignorés. DEFINITION_001 reste limité aux profils.

#### Scenario: minimums des tranches supérieurs au maximum

- **WHEN** les `min` des tranches de `Observation.component` totalisent 2 et le découpé a `max` 1 (sd-card-slice)
- **THEN** une erreur `DEFINITION_002` SHALL être émise à `<ressource>.differential.element[0]`

#### Scenario: maximum d'une tranche supérieur à celui du découpé

- **WHEN** le découpé a `max` 2 et une tranche a `max` `*`
- **THEN** une erreur `DEFINITION_002` SHALL être émise à `<ressource>.differential.element[1].max`

#### Scenario: maximum hérité de la définition de base

- **WHEN** le découpé `Observation.value[x]` n'énonce pas de `max` (0..1 dans la base) et deux tranches ont `min` 1
- **THEN** une erreur `DEFINITION_002` SHALL être émise à `<ressource>.differential.element[0]`

#### Scenario: cas sans diagnostic

- **WHEN** les tranches respectent le maximum, ou le maximum de base est `*`, ou les éléments n'ont pas d'`id`
- **THEN** aucun Diagnostic `DEFINITION_002` SHALL être émis

#### Scenario: minimums énormes

- **WHEN** les `min` des tranches valent `i64::MAX` et le découpé a `max` 1
- **THEN** une seule erreur `DEFINITION_002` SHALL être émise, sans débordement

### Requirement: Validation des profils — min d'un enfant de tranche (DEFINITION_003)

Pour toute StructureDefinition ayant un `differential`, le système SHALL émettre une erreur `DEFINITION_003` (eld-2, Min <= Max) à `<ressource>.differential.element[<i>].min` lorsqu'un enfant de tranche (id dont un segment antérieur au dernier porte un nom de tranche, le dernier non) énonce un `min` supérieur au `max` énoncé de son homologue non découpé dans le même différentiel (id sans noms de tranche); le message SHALL commencer par « No instance can meet the cardinality rules for <id> ». Les éléments sans `id` SHALL être ignorés.

#### Scenario: min d'un enfant de tranche supérieur au max de l'homologue

- **WHEN** `Observation.component:a.value[x]` a `min` 1 et `Observation.component.value[x]` a `max` 0 (sd-card-child)
- **THEN** une erreur `DEFINITION_003` SHALL être émise à `<ressource>.differential.element[4].min`

#### Scenario: min compatible

- **WHEN** le `min` de l'enfant de tranche ne dépasse pas le `max` de l'homologue
- **THEN** aucun Diagnostic `DEFINITION_003` SHALL être émis
