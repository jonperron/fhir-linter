## ADDED Requirements

### Requirement: Évaluation singleton

Les opérateurs et fonctions qui attendent une seule valeur SHALL appliquer l'évaluation singleton (N1 §4.5). Là où un booléen est attendu (`and`, `or`, `xor`, `implies`, `not()`), une collection d'un seul élément non booléen SHALL compter comme `true` et une collection de plusieurs éléments SHALL retourner `Err(EvalError::Type(...))`. L'arithmétique, la négation unaire, `&`, `<`, `<=`, `>`, `>=`, l'opérande gauche de `in` et l'opérande droit de `contains` SHALL retourner `Err(EvalError::Type(...))` sur plusieurs éléments. Un élément `Null` seul (valeur de remplacement) SHALL compter comme une collection vide.

#### Scenario: Élément non booléen dans un contexte booléen

- **WHEN** `42.not()` ou `false or 'foo'` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]` puis `[Bool(true)]`, l'élément non booléen comptant comme `true`

#### Scenario: Plusieurs éléments dans un contexte booléen

- **WHEN** `(1 | 2).not()` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: Plusieurs éléments en arithmétique ou comparaison

- **WHEN** `(1 | 2) + 1`, `-(1 | 2)`, `(1 | 2 | 3) & 'b'`, `(1 | 2) < 3`, `('a' | 'c' | 'd') in 'b'` ou `'b' contains ('a' | 'c')` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: Valeur de remplacement Null

- **WHEN** un opérande est une collection contenant uniquement `Null`
- **THEN** il SHALL être traité comme une collection vide

---

### Requirement: Égalité et équivalence

L'opérateur `=` SHALL comparer deux collections élément par élément dans l'ordre ; des longueurs différentes donnent `[Bool(false)]` et une collection vide d'un côté donne une collection vide. Les objets SHALL être comparés propriété par propriété, récursivement. Un objet comparé à une primitive SHALL donner une collection vide (raccourci jusqu'à F10).

L'opérateur `~` SHALL ne jamais retourner de collection vide : `{} ~ {}` vaut `true` et un seul côté vide vaut `false`. Il SHALL ignorer l'ordre des collections, comparer les chaînes sans tenir compte de la casse en traitant chaque caractère d'espacement comme une espace, sans fusionner les suites d'espaces, arrondir les décimaux à la précision de l'opérande le moins précis, et ignorer `id` dans les objets.

Les dates, date-heures et heures SHALL être comparées précision par précision (secondes et millisecondes comptent pour une seule précision), les fuseaux étant normalisés en UTC. Une précision différente avec des parties communes égales SHALL donner une collection vide, de même qu'un fuseau présent d'un seul côté, y compris une `Date` comparée à une `DateTime` avec fuseau. `sort()`, `min()` et `max()` SHALL continuer d'ordonner les date-heures par leur texte.

#### Scenario: Collections de longueurs différentes

- **WHEN** `(1 | 2) ~ (1 | 2 | 3)` ou `(1 | 1) = (1 | 2 | {})` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: Collections dans un ordre différent

- **WHEN** `(1 | 2) = (2 | 1)` puis `(1 | 2 | 3) ~ (3 | 2 | 1)` sont évalués
- **THEN** les résultats SHALL être `[Bool(false)]` puis `[Bool(true)]`

#### Scenario: Équivalence avec collections vides

- **WHEN** `{} ~ {}` puis `1 ~ {}` sont évalués
- **THEN** les résultats SHALL être `[Bool(true)]` puis `[Bool(false)]`

#### Scenario: Équivalence de chaînes

- **WHEN** `'a B\tc' ~ 'A b c'` puis `'a  b' ~ 'a b'` sont évalués
- **THEN** les résultats SHALL être `[Bool(true)]` puis `[Bool(false)]`

#### Scenario: Équivalence de décimaux

- **WHEN** `1.2 / 1.8 ~ 0.67` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]` (arrondi à la précision de `0.67`)

#### Scenario: Date-heures avec fuseaux différents

- **WHEN** `@2012-04-15T15:00:00+02:00 = @2012-04-15T16:00:00+03:00` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: Secondes et millisecondes

- **WHEN** `@2012-04-15T15:30:31 = @2012-04-15T15:30:31.0` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: Précisions différentes

- **WHEN** `@2012-01 = @2012` ou `@2012-04-15 = @2012-04-15T10:00:00` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Fuseau d'un seul côté

- **WHEN** `@2012-04-15T15:00:00Z != @2012-04-15T10:00:00` ou `@2012-04-16 = @2012-04-15T23:00:00-05:00` est évalué
- **THEN** le résultat SHALL être une collection vide

---

### Requirement: Opérateurs booléens

`and`, `or` et `implies` SHALL suivre la logique à trois valeurs de FHIRPath. Quand un côté suffit à décider du résultat (`false and x`, `true or x`, `false implies x`, `x implies true`), une erreur produite par l'autre côté SHALL être ignorée ; le côté gauche SHALL court-circuiter l'évaluation du côté droit.

#### Scenario: Le côté gauche décide

- **WHEN** `false and (1 | 2).not()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]` sans erreur

#### Scenario: Le côté droit décide

- **WHEN** `(1 | 2).not() or true` ou `(1 | 2).not() implies true` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]` sans erreur

#### Scenario: Aucun côté ne décide

- **WHEN** `(1 | 2).not() and true` ou `(1 | 2).not() xor true` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

---

### Requirement: Division par zéro

`/`, `div` et `mod` SHALL retourner une collection vide quand le diviseur vaut zéro (N1 §6.6), sans erreur.

#### Scenario: Division par zéro

- **WHEN** `1 / 0`, `5 div 0`, `5 mod 0` ou `5 mod 0.0` est évalué
- **THEN** le résultat SHALL être une collection vide

## MODIFIED Requirements

### Requirement: Fonctions d'existence

Le système SHALL implémenter `empty()`, `exists()`, `exists(criteria)`, `all(criteria)`, `allTrue()`, `anyTrue()`, `allFalse()`, `anyFalse()`.

#### Scenario: empty sur collection vide

- **WHEN** `empty()` est évalué sur une collection vide
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: exists avec critère

- **WHEN** `exists(x | x > 0)` est évalué sur une collection d'entiers
- **THEN** le résultat SHALL être `[Bool(true)]` si au moins un élément satisfait le critère


#### Scenario: Critère non booléen

- **WHEN** le critère de `where`, `exists` ou `all` produit pour un élément autre chose qu'un booléen seul ou une collection vide (ex. `(1 | 2).exists('a')`)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: allTrue sur des éléments non booléens

- **WHEN** `allTrue()`, `anyTrue()`, `allFalse()` ou `anyFalse()` est évalué sur une collection contenant un élément non booléen
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

### Requirement: Fonctions de filtrage

Le système SHALL implémenter `where(criteria)`, `select(projection)`, `repeat(projection)`, `ofType(type)`, `is(type)`, `as(type)`.

L'argument de `ofType`, `is` et `as` est un identifiant de nom de type. La correspondance de type (partagée par `ofType`, les fonctions `is`/`as` et les opérateurs `is`/`as`) SHALL accepter les types primitifs FHIRPath ainsi qu'un objet ressource dont `resourceType` est exactement égal au nom de type. Elle ne SHALL pas gérer l'héritage (`Resource`, `DomainResource`), les noms de types complexes FHIR, ni les noms qualifiés (`FHIR.X`, `System.X`).

L'opérateur `is` (`X is Type`) SHALL avoir exactement la même sémantique que la fonction `is(Type)` (N1 §6.3.1) : collection vide en entrée -> collection vide, un seul élément -> `[Bool(true)]` ou `[Bool(false)]`, plus d'un élément -> `Err(EvalError::Type(...))`.

L'opérateur `as` (`X as Type`) SHALL avoir la même sémantique que la fonction `as(Type)` : collection vide en entrée -> collection vide, un seul élément -> l'élément s'il correspond, sinon une collection vide, plus d'un élément -> `Err(EvalError::Type(...))`.

Le critère de `where` SHALL produire pour chaque élément un booléen seul ou une collection vide, sinon `Err(EvalError::Type(...))`.

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


#### Scenario: l'opérateur as suit la sémantique de as()

- **WHEN** `(1 | 2) as Integer` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`, comme pour `(1 | 2).as(Integer)`

### Requirement: Fonctions de collections

Le système SHALL implémenter `distinct()`, `isDistinct()`, `subsetOf(other)`, `supersetOf(other)`, `intersect(other)`, `exclude(other)`, `combine(other, [preserveOrder])`, `coalesce(values...)`, ainsi que l'opérateur `|` (union).

`combine()` diffère de `|` : il conserve les doublons.
`coalesce()` retourne le premier résultat non vide parmi ses arguments.
L'union `|` et `intersect()` SHALL éliminer les doublons des deux opérandes, y compris ceux présents dans un seul opérande.

#### Scenario: distinct élimine les doublons

- **WHEN** `distinct()` est évalué sur une collection avec doublons
- **THEN** le résultat SHALL contenir chaque valeur unique une seule fois, dans l'ordre d'apparition

#### Scenario: intersect retourne les éléments communs

- **WHEN** `intersect(other)` est évalué
- **THEN** le résultat SHALL contenir uniquement les éléments présents dans les deux collections

#### Scenario: combine conserve les doublons

- **WHEN** `(1 | 2).combine(2 | 3)` est évalué
- **THEN** le résultat SHALL être `[1, 2, 2, 3]` (doublons conservés)

#### Scenario: coalesce retourne le premier non vide

- **WHEN** `coalesce({}, 'second', 'third')` est évalué
- **THEN** le résultat SHALL être `[String('second')]`


#### Scenario: union et intersect dédupliquent les deux opérandes

- **WHEN** `1 | 2.combine(2)` puis `(1 | 2).intersect(1.combine(1))` sont évalués
- **THEN** les résultats SHALL être `[1, 2]` puis `[1]`, chaque valeur n'apparaissant qu'une fois

### Requirement: Fonctions diverses

Le système SHALL implémenter `not()`, `iif(condition, trueResult, otherwiseResult)`, `trace(name)`, `hasValue()`, `getValue()`, `type()`. Les autres fonctions diverses de FHIRPath ne sont pas implémentées (voir « Limitations connues »).

La condition de `iif` SHALL être un booléen seul ou une collection vide, sinon `Err(EvalError::Type(...))` ; `iif` appelé sur une collection de plusieurs éléments SHALL retourner `Err(EvalError::Type(...))`. `not()` SHALL appliquer l'évaluation singleton dans un contexte booléen.

#### Scenario: not inverse un booléen

- **WHEN** `true.not()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: iif retourne la branche correcte

- **WHEN** `iif(true, 'yes', 'no')` est évalué
- **THEN** le résultat SHALL être `[String("yes")]`


#### Scenario: iif avec une condition invalide

- **WHEN** `iif(1 | 2 | 3, true, false)` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

### Requirement: Erreurs d'évaluation

Le système SHALL exposer `EvalError` couvrant toutes les erreurs d'exécution possibles.

#### Scenario: Variantes d'EvalError

- **WHEN** une erreur se produit à l'évaluation
- **THEN** `EvalError` SHALL avoir les variantes : `Type(String)` (erreur de type), `UndefinedFunction(String)` (fonction inconnue), `Arity { name, expected, got }` (mauvais nombre d'arguments), `DivisionByZero` (conservée pour la compatibilité de l'enum public mais plus jamais produite, la division par zéro donnant une collection vide), `Parse(ParseError)` (erreur de parse propagée)

### Requirement: Exécution de la suite de tests officielle FHIRPath

Le crate `fhir-fhirpath` SHALL fournir un test d'intégration exécutant `r5/fhirpath/tests-fhir-r5.xml` de `fhir-test-cases`. Chaque cas SHALL être évalué avec `evaluate` sur son `inputfile` (un nom de fichier nu, sans partie répertoire, sous `r5/`, JSON ou XML analysé par `fhir-parser` puis converti par `resource_to_value`) ou sur un contexte vide ; un attribut `context` SHALL être évalué d'abord et son résultat sert de contexte. Un cas réussit quand les sorties correspondent en type et en valeur (ordre conservé sauf `ordered="false"` ; `predicate="true"` convertit le résultat en booléen), ou, pour une expression `invalid`, quand l'analyse ou l'évaluation échoue avec une erreur autre que `EvalError::UndefinedFunction` (une fonction absente ne prouve pas l'invalidité). Une panique, une entrée non chargeable, un `inputfile` comportant une partie répertoire ou une erreur de contexte compte comme un échec. Les cas dont le mode (du test ou du groupe) est `cda`, `tx` ou `custom` SHALL être ignorés et comptés. Le test SHALL afficher réussites/exécutés, ignorés, total et chaque cas en échec, et SHALL échouer si les nombres de cas exécutés et ignorés diffèrent exactement de (1226, 121) (dérive du chargeur ou du filtre d'ignorance) ou si moins de 817 cas réussissent (plancher de régression). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le fichier de la suite est introuvable, le test SHALL être ignoré, sauf en CI (`CI` défini) où il SHALL échouer ; toute autre erreur de lecture de ce fichier SHALL faire échouer le test.

#### Scenario: Réussites sous le plancher

- **WHEN** moins de 817 cas de la suite réussissent
- **THEN** le test SHALL échouer

#### Scenario: Dérive du nombre de cas

- **WHEN** le nombre de cas exécutés diffère de 1226 ou le nombre de cas ignorés diffère de 121
- **THEN** le test SHALL échouer

#### Scenario: Expression invalide évaluée sans erreur

- **WHEN** un cas marqué `invalid` est analysé et évalué sans erreur
- **THEN** le cas SHALL compter comme un échec

#### Scenario: Expression invalide utilisant une fonction non implémentée

- **WHEN** l'évaluation d'un cas marqué `invalid` échoue avec `EvalError::UndefinedFunction`
- **THEN** le cas SHALL compter comme un échec

#### Scenario: Fichier d'entrée avec une partie répertoire

- **WHEN** l'`inputfile` d'un cas n'est pas un nom de fichier nu (par exemple `../patient.json`)
- **THEN** le cas SHALL compter comme un échec sans lire le fichier

#### Scenario: Sous-module absent

- **WHEN** le fichier de la suite est introuvable
- **THEN** le test SHALL se terminer sans échec si `CI` n'est pas défini, et SHALL échouer si `CI` est défini

#### Scenario: Erreur de lecture de la suite

- **WHEN** la lecture du fichier de la suite échoue pour une raison autre que son absence
- **THEN** le test SHALL échouer
