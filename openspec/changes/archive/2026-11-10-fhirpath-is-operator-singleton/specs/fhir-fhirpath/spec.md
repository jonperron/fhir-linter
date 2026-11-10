## MODIFIED Requirements

### Requirement: Fonctions de filtrage

Le système SHALL implémenter `where(criteria)`, `select(projection)`, `repeat(projection)`, `ofType(type)`, `is(type)`, `as(type)`.

L'argument de `ofType`, `is` et `as` est un identifiant de nom de type. La correspondance de type (partagée par `ofType`, les fonctions `is`/`as` et les opérateurs `is`/`as`) SHALL accepter les types primitifs FHIRPath ainsi qu'un objet ressource dont `resourceType` est exactement égal au nom de type. Elle ne SHALL pas gérer l'héritage (`Resource`, `DomainResource`), les noms de types complexes FHIR, ni les noms qualifiés (`FHIR.X`, `System.X`).

L'opérateur `is` (`X is Type`) SHALL avoir exactement la même sémantique que la fonction `is(Type)` (N1 §6.3.1) : collection vide en entrée -> collection vide, un seul élément -> `[Bool(true)]` ou `[Bool(false)]`, plus d'un élément -> `Err(EvalError::Type(...))`.

#### Scenario: where filtre les éléments

- **WHEN** `where(active = true)` est évalué sur une collection d'objets
- **THEN** le résultat SHALL contenir uniquement les éléments pour lesquels le critère est vrai

#### Scenario: ofType filtre par type FHIRPath

- **WHEN** `ofType(String)` est évalué sur une collection mixte
- **THEN** le résultat SHALL contenir uniquement les valeurs de type `String`

#### Scenario: ofType filtre par resourceType

- **WHEN** `ofType(Patient)` est évalué sur une collection de ressources
- **THEN** le résultat SHALL contenir uniquement les objets dont `resourceType` vaut exactement `Patient`

#### Scenario: is et as sur un seul élément

- **WHEN** `is(Patient)` ou `as(Patient)` est évalué sur une collection d'un seul élément
- **THEN** `is` SHALL retourner `[Bool(true)]` ou `[Bool(false)]`, et `as` SHALL retourner l'élément s'il correspond, sinon une collection vide

#### Scenario: is et as sur une collection vide

- **WHEN** `is(type)` ou `as(type)` est évalué sur une collection vide
- **THEN** le résultat SHALL être une collection vide

#### Scenario: is et as sur plusieurs éléments

- **WHEN** `is(type)` ou `as(type)` est évalué sur une collection de plus d'un élément
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` (test officiel `testFHIRPathAsFunction21`)

#### Scenario: l'opérateur is suit la sémantique de is()

- **WHEN** `X is Type` est évalué avec `X` vide, d'un seul élément, ou de plusieurs éléments
- **THEN** le résultat SHALL être respectivement une collection vide, `[Bool(true)]`/`[Bool(false)]`, ou `Err(EvalError::Type(...))`, comme pour `X.is(Type)`
